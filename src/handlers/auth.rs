use std::hash::Hash;

use actix_web::{
    http::StatusCode,
    web::{Data, Json},
    HttpResponse,
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

pub async fn signup<R, H, T>(
    auth_service: Data<AuthService<R, H, T>>,
    Json(Signup { phone, password }): Json<Signup>,
) -> HttpResponse
where
    R: Repository + Clone,
    H: Hasher + Clone,
    T: TokenManager + Clone,
{
    match auth_service.signup(&phone, &password).await {
        Ok(id) => HttpResponse::build(StatusCode::OK).body(id),
        Err(err) => HttpResponse::build(StatusCode::BAD_REQUEST).body(err.to_string()),
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Login {
    phone: String,
    password: String,
}

pub async fn login<R, H, T>(
    auth_service: Data<AuthService<R, H, T>>,
    Json(Login { phone, password }): Json<Login>,
) -> HttpResponse
where
    R: Repository + Clone,
    H: Hasher + Clone,
    T: TokenManager + Clone,
{
    match auth_service.login_by_password(&phone, &password).await {
        Ok(token) => HttpResponse::build(StatusCode::OK).body(token),
        Err(err) => HttpResponse::build(StatusCode::BAD_REQUEST).body(err.to_string()),
    }
}
