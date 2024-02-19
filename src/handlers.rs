use std::collections::HashMap;

use crate::{
    core::{
        error::Error,
        message::{
            ChatPayload, FriendAccept, FriendRequest, Message, SystemMessage,
        },
        repository::{self, ChatMessage as RepoChatMessage, InsertChatMessage},
    },
    ws::actor::WS,
};
use actix::{Actor, ActorContext, Context, Handler};
use actix_multipart::Multipart;
use actix_web::{
    error::{
        ErrorForbidden, ErrorInternalServerError, ErrorUnauthorized,
        ErrorUnprocessableEntity,
    },
    http::StatusCode,
    web::{Data, Json, Path, Query},
    HttpResponse, Result,
};
use auth_service::core::{
    hasher::Hasher, repository::Repository as AuthRepository,
    service::Service as AuthService, token_manager::TokenManager,
};
use futures_util::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use upload_service::core::{
    repository::Repository as UploadRepository,
    service::Service as UploadService, store::Store as UploadStore,
};

use crate::{
    core::{
        notifier::Notifier,
        repository::{AddrStore, Repository, Session, User},
    },
    stores::postgres::PostgresRepository,
    utils::UserID,
    AddrMap,
};

#[derive(Debug, Deserialize)]
pub(crate) struct Pagination {
    pub(crate) limit: i64,
    pub(crate) offset: i64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Login {
    phone: String,
    password: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct LoginResp {
    token: String,
}

pub(crate) async fn login<R, H, T>(
    auth_service: Data<AuthService<R, H, T>>,
    Json(Login { phone, password }): Json<Login>,
) -> Result<Json<LoginResp>>
where
    R: AuthRepository + Clone,
    H: Hasher + Clone,
    T: TokenManager + Clone,
{
    let token = auth_service
        .login(&phone, &password)
        .await
        .map_err(ErrorForbidden)?;

    Ok(Json(LoginResp { token }))
}

#[derive(Debug, Deserialize)]
pub(crate) struct Signup {
    phone: String,
    password: String,
}

pub(crate) async fn signup<R, H, T>(
    auth_service: Data<AuthService<R, H, T>>,
    Json(Signup { phone, password }): Json<Signup>,
) -> Result<HttpResponse>
where
    R: AuthRepository + Clone,
    H: Hasher + Clone,
    T: TokenManager + Clone,
{
    auth_service
        .signup(&phone, &password)
        .await
        .map_err(ErrorInternalServerError)?;
    Ok(HttpResponse::Ok().finish())
}

pub(crate) async fn my_friends<R>(
    UserID(uid): UserID,
    repo: Data<R>,
) -> Result<Json<Vec<User>>>
where
    R: Repository,
{
    Ok(Json(
        repo.friends(&uid).await.map_err(ErrorInternalServerError)?,
    ))
}

#[derive(Debug, Serialize)]
pub enum UserType {
    Stranger,
    Friend,
    Myself,
}

#[derive(Debug, Deserialize)]
pub struct SearchUser {
    phone: String,
}

pub(crate) async fn search_user<F>(
    friends_store: Data<F>,
    UserID(uid): UserID,
    Query(SearchUser { phone }): Query<SearchUser>,
) -> Result<Json<Option<User>>>
where
    F: Repository,
{
    Ok(Json(
        friends_store
            .search_user(&uid, &phone)
            .await
            .map_err(ErrorInternalServerError)?,
    ))
}

#[derive(Debug, Deserialize)]
pub(crate) struct AddFriend {
    friend_id: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct AddFriendResp {
    id: String,
}

pub(crate) async fn add_friend<R, N, S>(
    repo: Data<R>,
    addrs: Data<S>,
    UserID(uid): UserID,
    Json(AddFriend { friend_id }): Json<AddFriend>,
) -> Result<Json<AddFriendResp>>
where
    R: Repository + Clone + Unpin + 'static,
    N: Notifier + Clone + Unpin + 'static,
    S: AddrStore + Clone + Unpin + 'static,
{
    let id = repo
        .add_friend_request(&uid, &friend_id)
        .await
        .map_err(ErrorInternalServerError)?;
    let user = repo
        .get_user(&uid)
        .await
        .map_err(ErrorInternalServerError)?;
    if let Some(addr) = addrs
        .get_addr(&friend_id)
        .await
        .map_err(ErrorInternalServerError)?
    {
        addr.do_send(Message::System(SystemMessage::FriendRequest {
            id: id.clone(),
            phone: user.phone,
            avatar: user.avatar,
        }))
    }
    Ok(Json(AddFriendResp { id }))
}

pub(crate) async fn accept_request<R, N, S>(
    friends_store: Data<R>,
    addrs: Data<S>,
    UserID(uid): UserID,
    id: Path<(String,)>,
) -> Result<HttpResponse>
where
    R: Repository + Clone + Unpin + 'static,
    N: Notifier + Clone + Unpin + 'static,
    S: AddrStore + Clone + Unpin + 'static,
{
    let req = friends_store
        .get_friend_request(&id.0)
        .await
        .map_err(ErrorInternalServerError)?;
    if req.to != uid {
        return Err(ErrorForbidden("not your request"));
    }
    friends_store
        .accept_friend_request(&id.0)
        .await
        .map_err(ErrorInternalServerError)?;
    if let Some(addr) = addrs
        .get_addr(&req.from)
        .await
        .map_err(ErrorInternalServerError)?
    {
        addr.do_send(Message::System(SystemMessage::FriendAccept {
            id: id.0.to_owned(),
        }));
    }
    Ok(HttpResponse::new(StatusCode::OK))
}

pub(crate) async fn reject_request<F>(
    friends_store: Data<F>,
    UserID(uid): UserID,
    id: Path<(String,)>,
) -> Result<HttpResponse>
where
    F: Repository,
{
    let req = friends_store
        .get_friend_request(&id.0)
        .await
        .map_err(ErrorInternalServerError)?;
    if req.to != uid {
        return Err(ErrorForbidden("not your request"));
    }
    friends_store
        .reject_friend_request(&id.0)
        .await
        .map_err(ErrorInternalServerError)?;
    Ok(HttpResponse::new(StatusCode::OK))
}

pub(crate) async fn my_requests<F>(
    friends_store: Data<F>,
    UserID(uid): UserID,
) -> Result<Json<Vec<crate::core::repository::FriendRequest>>>
where
    F: Repository,
{
    let reqs = friends_store
        .pending_friend_requests(&uid)
        .await
        .map_err(ErrorInternalServerError)?;
    Ok(Json(reqs))
}

#[derive(Debug, Serialize)]
pub(crate) struct NumOfFriendRequestsResp {
    count: usize,
}

pub(crate) async fn num_of_friend_requests<F>(
    friends_store: Data<F>,
    UserID(uid): UserID,
) -> Result<Json<NumOfFriendRequestsResp>>
where
    F: Repository,
{
    let count = friends_store
        .pending_friend_requests(&uid)
        .await
        .map_err(ErrorInternalServerError)?
        .len();
    Ok(Json(NumOfFriendRequestsResp { count }))
}

#[derive(Debug, Deserialize)]
pub(crate) struct ChatMessageHistory {
    to: String,
    before: Option<String>,
}

pub(crate) async fn chat_message_history<F>(
    repository: Data<F>,
    UserID(uid): UserID,
    Query(ChatMessageHistory { to, before }): Query<ChatMessageHistory>,
) -> Result<Json<Vec<RepoChatMessage>>>
where
    F: Repository,
{
    let messages = repository
        .chat_message_history(&uid, &to, 20, before.as_deref())
        .await
        .map_err(ErrorInternalServerError)?;
    Ok(Json(messages))
}

#[derive(Debug, Serialize)]
pub(crate) struct UploadResponse {
    ids: Vec<String>,
}

pub(crate) async fn upload<R, S>(
    upload_service: Data<UploadService<R, S>>,
    mut payload: Multipart,
    UserID(uid): UserID,
) -> Result<Json<UploadResponse>>
where
    R: UploadRepository + Clone,
    S: UploadStore + Clone,
{
    let mut ids = Vec::new();
    while let Some(field) = payload.next().await {
        if let Ok(f) = field {
            let filename =
                f.content_disposition().get_filename().unwrap().to_owned();
            let chunk = f.map_err(|e| {
                anyhow::Error::msg(format!(
                    "failed to read uploaded file: {}",
                    e
                ))
            });
            let id = upload_service
                .upload(chunk, &filename, &uid, Some((1 << 24) - 1))
                .await
                .map_err(ErrorInternalServerError)?;
            ids.push(id);
        }
    }
    Ok(Json(UploadResponse { ids }))
}

pub(crate) async fn download<R, S>(
    upload_service: Data<UploadService<R, S>>,
    id: Path<String>,
) -> Result<HttpResponse>
where
    R: UploadRepository + Clone,
    S: UploadStore + Clone,
{
    let id = id.into_inner();
    let stream = upload_service
        .download(&id)
        .await
        .map_err(ErrorInternalServerError)?;
    Ok(HttpResponse::Ok().streaming(stream))
}

#[derive(Debug, Deserialize)]
pub(crate) struct UpsertAvatarRequest {
    upload_id: String,
}

pub(crate) async fn upsert_avatar(
    repo: Data<PostgresRepository>,
    UserID(uid): UserID,
    Json(UpsertAvatarRequest { upload_id }): Json<UpsertAvatarRequest>,
) -> Result<HttpResponse> {
    repo.update_avatar(&uid, &upload_id)
        .await
        .map_err(ErrorInternalServerError)?;
    Ok(HttpResponse::new(StatusCode::OK))
}

pub(crate) async fn my_avatar<UR, US>(
    repo: Data<PostgresRepository>,
    UserID(uid): UserID,
    upload_service: Data<UploadService<UR, US>>,
) -> Result<HttpResponse>
where
    UR: UploadRepository + Clone,
    US: UploadStore + Clone,
{
    let avatar = repo
        .get_avatar(&uid)
        .await
        .map_err(ErrorInternalServerError)?;
    if let Some(avatar) = avatar {
        let stream = upload_service
            .download(&avatar)
            .await
            .map_err(ErrorInternalServerError)?;
        return Ok(HttpResponse::Ok().streaming(stream));
    }
    Ok(HttpResponse::Ok().finish())
}

pub(crate) async fn get_user_avatar<UR, US>(
    repo: Data<PostgresRepository>,
    uid: Path<(String,)>,
    upload_service: Data<UploadService<UR, US>>,
) -> Result<HttpResponse>
where
    UR: UploadRepository + Clone,
    US: UploadStore + Clone,
{
    let avatar = repo
        .get_avatar(&uid.0)
        .await
        .map_err(ErrorInternalServerError)?;
    if let Some(avatar) = avatar {
        let stream = upload_service
            .download(&avatar)
            .await
            .map_err(ErrorInternalServerError)?;
        return Ok(HttpResponse::Ok().streaming(stream));
    }
    Ok(HttpResponse::Ok().finish())
}

#[derive(Debug, Deserialize)]
pub(crate) struct UpdateNotificationToken {
    token: String,
}

pub(crate) async fn update_notification_token<N>(
    notifier: Data<N>,
    UserID(uid): UserID,
    Json(UpdateNotificationToken { token }): Json<UpdateNotificationToken>,
) -> Result<HttpResponse>
where
    N: Notifier + Clone,
{
    notifier
        .update_token(&uid, &token)
        .await
        .map_err(ErrorInternalServerError)?;
    Ok(HttpResponse::Ok().finish())
}

pub(crate) async fn my_sessions<R>(
    repo: Data<R>,
    UserID(uid): UserID,
) -> Result<Json<Vec<Session>>>
where
    R: Repository + Clone,
{
    Ok(Json(
        repo.sessions(&uid)
            .await
            .map_err(ErrorInternalServerError)?,
    ))
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SendChatMessage {
    to: String,
    mime_type: String,
    content: String,
}

pub(crate) async fn send_chat_message<R, N, S>(
    repo: Data<R>,
    addrs: Data<S>,
    notifier: Data<N>,
    UserID(uid): UserID,
    Json(SendChatMessage {
        to,
        mime_type,
        content,
    }): Json<SendChatMessage>,
) -> Result<Json<RepoChatMessage>>
where
    R: Repository + Clone + Unpin + 'static,
    N: Notifier + Clone + Unpin + 'static,
    S: AddrStore + Clone + Unpin + 'static,
{
    let inserted = repo
        .insert_chat_message(&InsertChatMessage {
            from: uid.clone(),
            to: to.clone(),
            mime_type: mime_type.clone(),
            content: content.clone(),
        })
        .await
        .map_err(ErrorInternalServerError)?;
    let user = repo
        .get_user(&uid)
        .await
        .map_err(ErrorInternalServerError)?;
    let chat_msg = Message::Chat {
        from: uid.clone(),
        phone: user.phone.clone(),
        payload: ChatPayload { id: inserted.id.clone(), mime_type, content },
    };
    if let Some(addr) = addrs
        .get_addr(&to)
        .await
        .map_err(ErrorInternalServerError)?
    {
        addr.do_send(chat_msg);
    } else if let Some(fcm_token) = notifier.get_token(&to).await.map_err(ErrorInternalServerError)? {
        notifier
            .send_notification(
                &fcm_token,
                "Chat message",
                "You got a chat message just now",
                [("phone", user.phone), ("typ", "Chat".into())].into_iter().collect::<HashMap<&str, String>>(),
            )
            .await
            .map_err(ErrorInternalServerError)?;
        }
        
    Ok(Json(inserted))
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SendRTCMessage {
    to: String,
    typ: String,
    payload: String,
}

pub(crate) async fn send_rtc_message<R, N, S>(
    repo: Data<R>,
    addrs: Data<S>,
    notifier: Data<N>,
    UserID(uid): UserID,
    Json(SendRTCMessage { to, typ, payload }): Json<SendRTCMessage>,
) -> Result<HttpResponse>
where
    R: Repository + Clone + Unpin + 'static,
    N: Notifier + Clone + Unpin + 'static,
    S: AddrStore + Clone + Unpin + 'static,
{
    let user = repo
        .get_user(&uid)
        .await
        .map_err(ErrorInternalServerError)?;
    let rtc_msg = Message::RTC {
        from: uid,
        phone: user.phone,
        payload,
    };
    if let Some(addr) = addrs
        .get_addr(&to)
        .await
        .map_err(ErrorInternalServerError)?
    {
        addr.do_send(rtc_msg);
        return Ok(HttpResponse::Ok().finish());
    }
    if typ != "Offer" {
        return Err(ErrorUnprocessableEntity("could not forward to user"));
    }
    notifier
        .send_notification(
            &to,
            "RTC message",
            "You got an RTC message just now",
            rtc_msg,
        )
        .await
        .map_err(ErrorInternalServerError)?;
    Ok(HttpResponse::Ok().finish())
}

pub(crate) async fn offline<S>(
    addrs: Data<S>,
    UserID(uid): UserID,
) -> Result<HttpResponse>
where
    S: AddrStore + Clone + Unpin + 'static,
{
    addrs
        .remove_addr(&uid)
        .await
        .map_err(ErrorInternalServerError)?;
    Ok(HttpResponse::Ok().finish())
}

pub(crate) async fn mark_as_read<R>(
    repo: Data<R>,
    UserID(uid): UserID,
    msg_id: Path<(String,)>,
) -> Result<HttpResponse>
where
    R: Repository + Clone + Unpin + 'static,
{
    repo.mark_as_read(&uid, &msg_id.0)
        .await
        .map_err(ErrorInternalServerError)?;
    Ok(HttpResponse::Ok().finish())
}

pub(crate) async fn verify_auth_token() -> HttpResponse {
    HttpResponse::Ok().finish()
}

pub(crate) async fn me<R>(repo: Data<R>, UserID(uid): UserID) -> Result<Json<User>> where R: Repository + Clone + 'static{
    Ok(Json(repo.get_user(&uid).await.map_err(ErrorInternalServerError)?))
}
