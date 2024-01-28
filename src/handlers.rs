use std::{collections::HashSet, hash::Hash};

use actix_web::{
    error::{ErrorForbidden, ErrorInternalServerError},
    http::{header::Accept, StatusCode},
    web::{Data, Json, Path},
    HttpRequest, HttpResponse, Result,
};
use auth_service::core::{
    hasher::Hasher, repository::Repository, service::Service as AuthService,
    token_manager::TokenManager,
};
use serde::{Deserialize, Serialize};

use crate::{
    core::store::{FriendRequest, FriendsStore},
    utils::UserID,
    ws::messages,
    AddrMap,
};

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
    R: Repository + Clone,
    H: Hasher + Clone,
    T: TokenManager + Clone,
{
    let token = auth_service
        .login_by_password(&phone, &password)
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
    R: Repository + Clone,
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
) -> Result<Json<Vec<String>>>
where
    F: FriendsStore,
{
    Ok(Json(
        friends_store
            .friends(&uid)
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

#[derive(Debug, Serialize)]
pub struct User {
    id: String,
    typ: UserType,
}

pub(crate) async fn all_users<F>(
    addrs: Data<AddrMap>,
    friends_store: Data<F>,
    UserID(uid): UserID,
) -> Result<Json<Vec<User>>>
where
    F: FriendsStore,
{
    let ids = addrs.read().await.keys().cloned().collect::<Vec<String>>();
    let friends: HashSet<String> = friends_store
        .friends(&uid)
        .await
        .map_err(ErrorInternalServerError)?
        .into_iter()
        .collect();
    Ok(Json(
        ids.into_iter()
            .map(|id| User {
                id: id.clone(),
                typ: if id == uid {
                    UserType::Myself
                } else if friends.contains(&id) {
                    UserType::Friend
                } else {
                    UserType::Stranger
                },
            })
            .collect(),
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
    F: FriendsStore,
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
    F: FriendsStore,
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
    F: FriendsStore,
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
    F: FriendsStore,
{
    let reqs = friends_store
        .pending_friend_requests(&uid)
        .await
        .map_err(ErrorInternalServerError)?;
    Ok(Json(reqs))
}
