#![feature(result_flattening)]
#![allow(async_fn_in_trait)]

pub mod core;
pub mod handlers;
pub mod notifiers;
pub mod stores;
pub mod utils;
pub mod ws;

use core::notifier;
use notifiers::fcm::FCMNotifier;
use sqlx::postgres::PgPoolOptions;
use std::{collections::HashMap, env, sync::Arc};
use stores::postgres::PostgresRepository;
use ws::actor::WS;

use actix::Addr;
use actix_web::{
    middleware::Logger,
    web::{get, post, put, route, scope, Data},
    App, HttpServer,
};
use auth_service::{
    core::service::Service as AuthService, hashers::sha::ShaHasher,
    middlewares::actix_web::AuthTokenMiddleware,
    token_managers::jwt::JWTTokenManager,
};
use hmac::{Hmac, Mac};
use nb_from_env::{FromEnv, FromEnvDerive};
use tokio::sync::RwLock;
use upload_service::{
    core::service::Service as UploadService, stores::local_fs::LocalFSStore,
};

#[derive(FromEnvDerive)]
struct Config {
    listen_address: String,
    auth_token_secret: String,
}

type AddrMap =
    Arc<RwLock<HashMap<String, Addr<WS<PostgresRepository, FCMNotifier>>>>>;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    let config = Config::from_env();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    let map: AddrMap = Arc::new(RwLock::new(HashMap::new()));
    let db_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL environment variable not set");
    let pg_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("failed to connect to postgresql");
    let repository = PostgresRepository::new(pg_pool.clone());
    let auth_hasher = ShaHasher {};
    let jwt_token_manager: JWTTokenManager<Hmac<sha2::Sha256>> =
        JWTTokenManager::new(
            Hmac::new_from_slice(config.auth_token_secret.as_bytes())
                .expect("invalid auth token secret"),
        );
    let auth_service = AuthService::new(
        repository.clone(),
        auth_hasher,
        jwt_token_manager.clone(),
    );
    let upload_service = UploadService::new(
        repository.clone(),
        LocalFSStore::new(
            env::var("UPLOAD_STORE_PATH")
                .expect("Environment varialble UPLOAD_STORE_PATH not set"),
        ),
    );
    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(auth_service.clone()))
            .app_data(Data::new(map.clone()))
            .app_data(Data::new(repository.clone()))
            .app_data(Data::new(upload_service.clone()))
            .app_data(Data::new(FCMNotifier::new(pg_pool.clone())))
            .wrap(Logger::default())
            .route(
                "/login",
                post().to(handlers::login::<
                    PostgresRepository,
                    ShaHasher,
                    JWTTokenManager<Hmac<sha2::Sha256>>,
                >),
            )
            .route(
                "/signup",
                post().to(handlers::signup::<
                    PostgresRepository,
                    ShaHasher,
                    JWTTokenManager<Hmac<sha2::Sha256>>,
                >),
            )
            .route(
                "/ws",
                get().to(ws::actor::index::<
                    PostgresRepository,
                    ShaHasher,
                    JWTTokenManager<Hmac<sha2::Sha256>>,
                    PostgresRepository,
                    FCMNotifier,
                >),
            )
            .service(
                scope("/apis/v1")
                    .wrap(AuthTokenMiddleware::new(
                        "X-Auth-Token",
                        "X-User-ID",
                        auth_service.clone(),
                    ))
                    .service(
                        scope("/users")
                            .route(
                                "",
                                get().to(handlers::search_user::<
                                    PostgresRepository,
                                >),
                            )
                            .route(
                                "/{uid}/avatar",
                                get().to(handlers::get_user_avatar::<
                                    PostgresRepository,
                                    LocalFSStore,
                                >),
                            ),
                    )
                    .service(
                        scope("friends")
                            .route(
                                "",
                                get().to(handlers::my_friends::<
                                    PostgresRepository,
                                >),
                            )
                            .service(
                                scope("requests")
                                    .route(
                                        "",
                                        post().to(handlers::add_friend::<
                                            PostgresRepository,
                                        >),
                                    )
                                    .route(
                                        "",
                                        get().to(handlers::my_requests::<
                                            PostgresRepository,
                                        >),
                                    )
                                    .route(
                                        "/{id}/accept",
                                        put().to(handlers::accept_request::<
                                            PostgresRepository,
                                        >),
                                    )
                                    .route(
                                        "/{id}/reject",
                                        put().to(handlers::reject_request::<
                                            PostgresRepository,
                                        >),
                                    )
                                    .route(
                                        "/count",
                                        get().to(
                                            handlers::num_of_friend_requests::<
                                                PostgresRepository,
                                            >,
                                        ),
                                    ),
                            ),
                    )
                    .service(scope("/chat_messages").route(
                        "",
                        get().to(handlers::chat_message_history::<
                            PostgresRepository,
                        >),
                    ))
                    .service(
                        scope("/uploads")
                            .route(
                                "",
                                post().to(handlers::upload::<
                                    PostgresRepository,
                                    LocalFSStore,
                                >),
                            )
                            .route(
                                "/{id}",
                                get().to(handlers::download::<
                                    PostgresRepository,
                                    LocalFSStore,
                                >),
                            ),
                    )
                    .service(
                        scope("/me")
                            .route(
                                "/avatar",
                                get().to(handlers::my_avatar::<
                                    PostgresRepository,
                                    LocalFSStore,
                                >),
                            )
                            .route("/avatar", put().to(handlers::upsert_avatar))
                            .route(
                                "/notification_token",
                                put().to(
                                    handlers::update_notification_token::<
                                        FCMNotifier,
                                    >,
                                ),
                            )
                            .route(
                                "/sessions",
                                get().to(handlers::my_sessions::<
                                    PostgresRepository,
                                >),
                            ),
                    )
                    .service(scope("/friends").route(
                        "",
                        get().to(handlers::my_friends::<PostgresRepository>),
                    )),
            )
    })
    .bind(config.listen_address)
    .expect("failed to bind to listen address")
    .run()
    .await
}
