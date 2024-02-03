use actix_web::{
    error::{ErrorForbidden, ErrorInternalServerError},
    http::StatusCode,
    web::{Data, Json, Path, Query},
    HttpResponse, Result,
};
use auth_service::core::{
    hasher::Hasher, repository::Repository as AuthRepository,
    service::Service as AuthService, token_manager::TokenManager,
};
use serde::{Deserialize, Serialize};

use crate::{
    core::repository::{
        ChatMessage, Friend, FriendRequest, InsertChatMessage, Repository, User,
    },
    utils::UserID,
    ws::messages,
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

pub(crate) async fn my_friends<F>(
    UserID(uid): UserID,
    friends_store: Data<F>,
    Query(Pagination { limit, offset }): Query<Pagination>,
) -> Result<Json<Vec<Friend>>>
where
    F: Repository,
{
    Ok(Json(
        friends_store
            .friends(&uid, limit, offset)
            .await
            .map_err(ErrorInternalServerError)?,
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

pub(crate) async fn add_friend<F>(
    friends_store: Data<F>,
    addrs: Data<AddrMap>,
    UserID(uid): UserID,
    Json(AddFriend { friend_id }): Json<AddFriend>,
) -> Result<Json<AddFriendResp>>
where
    F: Repository,
{
    let id = friends_store
        .add_friend_request(&uid, &friend_id)
        .await
        .map_err(ErrorInternalServerError)?;
    if let Some(addr) = addrs.read().await.get(&friend_id) {
        addr.do_send(messages::AddFriend { user_id: uid });
    }
    Ok(Json(AddFriendResp { id }))
}

pub(crate) async fn accept_request<F>(
    friends_store: Data<F>,
    addrs: Data<AddrMap>,
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
        .accept_friend_request(&id.0)
        .await
        .map_err(ErrorInternalServerError)?;
    if let Some(addr) = addrs.read().await.get(&req.from) {
        addr.do_send(messages::Accept {
            id: id.0.to_owned(),
        });
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
) -> Result<Json<Vec<FriendRequest>>>
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
pub(crate) struct LatestChatMessagesWithOthers {
    to: String,
}

pub(crate) async fn latest_messages_with_others<F>(
    repository: Data<F>,
    UserID(uid): UserID,
    Query(LatestChatMessagesWithOthers { to }): Query<
        LatestChatMessagesWithOthers,
    >,
) -> Result<Json<Vec<ChatMessage>>>
where
    F: Repository,
{
    let messages = repository
        .latest_chat_messages_with_others(&uid, &to, 20)
        .await
        .map_err(|e| ErrorInternalServerError(e))?;
    Ok(Json(messages))
}
