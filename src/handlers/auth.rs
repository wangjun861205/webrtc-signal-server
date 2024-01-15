use std::fmt::Debug;

use actix_web::{
    error::{ErrorInternalServerError, ErrorUnauthorized},
    web::{Data, Json},
    Error,
};
use auth_service::core::{
    hasher::Hasher, repository::Repository, service::Service as AuthService,
    token_manager::TokenManager,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Signup {
    phone: String,
    password: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SignupResp {
    token: String,
}

pub async fn signup<R, H, T>(
    auth_service: Data<AuthService<R, H, T>>,
    Json(Signup { phone, password }): Json<Signup>,
) -> Result<Json<SignupResp>, Error>
where
    R: Repository + Clone,
    H: Hasher + Clone,
    T: TokenManager + Clone,
{
    auth_service
        .signup(&phone, &password)
        .await
        .map(|token| Json(SignupResp { token }))
        .map_err(ErrorInternalServerError)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Login {
    phone: String,
    password: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LoginResp {
    token: String,
}

pub async fn login<R, H, T>(
    auth_service: Data<AuthService<R, H, T>>,
    Json(Login { phone, password }): Json<Login>,
) -> Result<Json<LoginResp>, Error>
where
    R: Repository + Clone,
    H: Hasher + Clone,
    T: TokenManager + Clone,
{
    auth_service
        .login_by_password(&phone, &password)
        .await
        .map(|token| Json(LoginResp { token }))
        .map_err(ErrorUnauthorized)
}
