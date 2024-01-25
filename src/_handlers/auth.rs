use crate::AddrMap;
use auth_service::core::{
    hasher::Hasher, repository::Repository, service::Service as AuthService,
    token_manager::TokenManager,
};
use serde::{Deserialize, Serialize};

use actix_web::{
    error::{Error, ErrorForbidden},
    web::{Data, Json},
    HttpRequest,
};

#[derive(Deserialize)]
pub struct Login {
    phone: String,
    password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    token: String,
}

pub async fn login<R, H, T>(
    Json(Login { phone, password }): Json<Login>,
    auth_service: Data<AuthService<R, H, T>>,
) -> Result<Json<LoginResponse>, Error>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
{
    let token = auth_service
        .login_by_password(&phone, &password)
        .await
        .map_err(ErrorForbidden)?;
    Ok(Json(LoginResponse { token }))
}

#[derive(Deserialize)]
pub struct Signup {
    phone: String,
    password: String,
}

#[derive(Serialize)]
pub struct SignupResponse {
    token: String,
}

pub async fn signup<R, H, T>(
    Json(Signup { phone, password }): Json<Signup>,
    auth_service: Data<AuthService<R, H, T>>,
) -> Result<Json<SignupResponse>, Error>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
{
    let token = auth_service
        .signup(&phone, &password)
        .await
        .map_err(ErrorForbidden)?;
    Ok(Json(SignupResponse { token }))
}

pub async fn logout<R, H, T>(req: HttpRequest, addrs: Data<AddrMap>) -> Result<(), Error>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
{
    if let Some(uid) = req.headers().get("X-User-ID") {
        let id = uid.to_str().unwrap();
        addrs.write().await.remove(id);
        return Ok(());
    }
    Err(ErrorForbidden("Invalid token"))
}
