pub mod core;
pub mod handlers;

use core::ws::WS;
use std::{collections::HashMap, sync::Arc};

use actix::Addr;
use actix_web::{
    dev::Transform,
    middleware::Logger,
    web::{get, post, Data, Path, Payload},
    App, Error, HttpRequest, HttpResponse, HttpServer,
};
use actix_web_actors::ws;
use auth_service::{
    core::service::Service as AuthService, hashers::sha::ShaHasher,
    middlewares::actix_web::AuthTokenMiddleware, repositories::memory::MemoryRepository,
    token_managers::jwt::JWTTokenManager,
};
use hmac::{Hmac, Mac};
use nb_from_env::{FromEnv, FromEnvDerive};
use sha2::Sha256;
use tokio::sync::RwLock;

#[derive(FromEnvDerive)]
struct Config {
    listen_address: String,
    auth_token_secret: String,
}

type AddrMap = Arc<RwLock<HashMap<String, Addr<WS>>>>;

async fn index(
    req: HttpRequest,
    stream: Payload,
    id: Path<(String,)>,
    map: Data<AddrMap>,
) -> Result<HttpResponse, Error> {
    ws::start(WS::new(id.0.clone(), map.as_ref().clone()), &req, stream)
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
            .wrap(Logger::default())
            .wrap(AuthTokenMiddleware::new(
                "X-Auth-Token",
                jwt_token_manager.clone(),
            ))
            .app_data(Data::new(map.clone()))
            .route("/ws/{id}", get().to(index))
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
            )
    })
    .bind(config.listen_address)
    .expect("failed to bind to listen address")
    .run()
    .await
}
