pub mod core;
pub mod handlers;
pub mod middlewares;

use core::ws::WS;
use std::{collections::HashMap, sync::Arc};

use actix::Addr;
use actix_web::{
    middleware::Logger,
    web::{get, post, scope, Data},
    App, HttpServer,
};
use auth_service::{
    core::service::Service as AuthService, hashers::sha::ShaHasher,
    middlewares::actix_web::AuthTokenMiddleware, repositories::memory::MemoryRepository,
    token_managers::jwt::JWTTokenManager,
};
use hmac::{Hmac, Mac};
use middlewares::cors::CORSMiddleware;
use nb_from_env::{FromEnv, FromEnvDerive};
use sha2::Sha256;
use tokio::sync::RwLock;

#[derive(FromEnvDerive)]
struct Config {
    listen_address: String,
    auth_token_secret: String,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    let config = Config::from_env();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    let map: Arc<RwLock<HashMap<String, Addr<WS>>>> = Arc::new(RwLock::new(HashMap::new()));
    let auth_repository = MemoryRepository::default();
    let auth_hasher = ShaHasher {};
    let jwt_token_manager: JWTTokenManager<Hmac<sha2::Sha256>> = JWTTokenManager::new(
        Hmac::new_from_slice(config.auth_token_secret.as_bytes())
            .expect("invalid auth token secret"),
    );
    let auth_service = AuthService::new(auth_repository, auth_hasher, jwt_token_manager.clone());
    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(auth_service.clone()))
            .app_data(Data::new(map.clone()))
            .wrap(CORSMiddleware)
            .wrap(Logger::default())
            .service(
                scope("/apis/v1")
                    .service(
                        scope("/auth")
                            .route(
                                "/login",
                                post().to(handlers::auth::login::<
                                    MemoryRepository,
                                    ShaHasher,
                                    JWTTokenManager<Hmac<Sha256>>,
                                >),
                            )
                            .route(
                                "/signup",
                                post().to(handlers::auth::signup::<
                                    MemoryRepository,
                                    ShaHasher,
                                    JWTTokenManager<Hmac<Sha256>>,
                                >),
                            ),
                    )
                    .service(
                        scope("")
                            .wrap(AuthTokenMiddleware::new(
                                "X-Auth-Token",
                                "X-User-ID",
                                jwt_token_manager.clone(),
                            ))
                            .route("/ws", get().to(handlers::ws::index)),
                    ),
            )
    })
    .bind(config.listen_address)
    .expect("failed to bind to listen address")
    .run()
    .await
}
