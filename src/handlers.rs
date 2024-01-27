use std::hash::Hash;

use actix_web::{
    error::{ErrorForbidden, ErrorInternalServerError},
    web::{Data, Json},
    HttpRequest, HttpResponse, Result,
};
use auth_service::core::{hasher::Hasher, repository::Repository, service::Service as AuthService, token_manager::TokenManager};
use serde::{Deserialize, Serialize};

use crate::AddrMap;

#[derive(Debug, Deserialize)]
pub(crate) struct Login {
    phone: String,
    password: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct LoginResp {
    token: String,
}

pub(crate) async fn login<R, H, T>(auth_service: Data<AuthService<R, H, T>>, Json(Login { phone, password }): Json<Login>) -> Result<Json<LoginResp>>
where
    R: Repository + Clone,
    H: Hasher + Clone,
    T: TokenManager + Clone,
{
    let token = auth_service.login_by_password(&phone, &password).await.map_err(ErrorForbidden)?;

    Ok(Json(LoginResp { token }))
}

#[derive(Debug, Deserialize)]
pub(crate) struct Signup {
    phone: String,
    password: String,
}

pub(crate) async fn signup<R, H, T>(auth_service: Data<AuthService<R, H, T>>, Json(Signup { phone, password }): Json<Signup>) -> Result<HttpResponse>
where
    R: Repository + Clone,
    H: Hasher + Clone,
    T: TokenManager + Clone,
{
    auth_service.signup(&phone, &password).await.map_err(ErrorInternalServerError)?;
    Ok(HttpResponse::Ok().finish())
}

pub(crate) async fn acquire_friends(req: HttpRequest, addrs: Data<AddrMap>) -> Result<Json<Vec<String>>> {
    let uid = req.headers().get("X-User-ID").ok_or(ErrorForbidden("auth header not exists"))?;
    let friends = addrs.read().await.keys().cloned().collect::<Vec<String>>();
    Ok(Json(friends))
}
