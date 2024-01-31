#![feature(result_flattening)]

pub mod core;
pub mod handlers;
pub mod stores;
pub mod utils;
pub mod ws;

use sqlx::postgres::PgPoolOptions;
use std::{collections::HashMap, env, sync::Arc};
use stores::{
    auth_repositories::postgres::repository::PostgresRepository,
    friends_stores::postgres::store::PostgresFriendsStore,
};
use ws::actor::WS;

use actix::Addr;
use actix_web::{
    middleware::Logger,
    web::{get, post, put, scope, Data},
    App, HttpServer,
};
use auth_service::{
    core::service::Service as AuthService, hashers::sha::ShaHasher,
    middlewares::actix_web::AuthTokenMiddleware,
    token_managers::jwt::JWTTokenManager,
};
use hmac::{Hmac, Mac};
// use middlewares::cors::CORSMiddleware;
use nb_from_env::{FromEnv, FromEnvDerive};
use tokio::sync::RwLock;

#[derive(FromEnvDerive)]
struct Config {
    listen_address: String,
    auth_token_secret: String,
}

type AddrMap = Arc<
    RwLock<
        HashMap<
            String,
            Addr<
                WS<
                    PostgresRepository,
                    ShaHasher,
                    JWTTokenManager<Hmac<sha2::Sha256>>,
                    PostgresFriendsStore,
                >,
            >,
        >,
    >,
>;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    let config = Config::from_env();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    let map: AddrMap = Arc::new(RwLock::new(HashMap::new()));
    let db_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL environment variable not set");
    let auth_repository = PostgresRepository::new(
        PgPoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await
            .expect("failed to connect to postgresql"),
    );
    let auth_hasher = ShaHasher {};
    let jwt_token_manager: JWTTokenManager<Hmac<sha2::Sha256>> =
        JWTTokenManager::new(
            Hmac::new_from_slice(config.auth_token_secret.as_bytes())
                .expect("invalid auth token secret"),
        );
    let auth_service = AuthService::new(
        auth_repository,
        auth_hasher,
        jwt_token_manager.clone(),
    );
    let friends_store = PostgresFriendsStore::new(
        PgPoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await
            .expect("failed to connect to postgresql"),
    );
    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(auth_service.clone()))
            .app_data(Data::new(map.clone()))
            .app_data(Data::new(friends_store.clone()))
            // .wrap(CORSMiddleware)
            .wrap(Logger::default())
            .service(
                scope("/apis/v1").service(
                    scope("")
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
                                PostgresFriendsStore,
                            >),
                        )
                        .service(
                            scope("")
                                .wrap(AuthTokenMiddleware::new(
                                    "X-Auth-Token",
                                    "X-User-ID",
                                    jwt_token_manager.clone(),
                                ))
                                .route(
                                    "users",
                                    get().to(handlers::search_user::<
                                        PostgresFriendsStore,
                                    >),
                                )
                                .service(
                                    scope("friends")
                                    .route(
                                        "",
                                        get().to(handlers::my_friends::<
                                            PostgresFriendsStore,
                                        >),
                                    )
                                    .service(
                                        scope("requests")
                                        .route(
                                        "",
                                        post().to(handlers::add_friend::<
                                            PostgresFriendsStore,
                                        >))
                                        .route(
                                            "",
                                            get().to(handlers::my_requests::<
                                                PostgresFriendsStore,
                                            >),
                                        )
                                        .route(
                                            "/{id}/accept",
                                            put().to(
                                                handlers::accept_request::<
                                                    PostgresFriendsStore,
                                                >,
                                            ),
                                        )
                                        .route(
                                            "/{id}/reject",
                                            put().to(
                                                handlers::reject_request::<
                                                    PostgresFriendsStore,
                                                >,
                                            ),
                                        )
                                        .route(
                                            "/count",
                                            get().to(handlers::num_of_friend_requests::<PostgresFriendsStore>)
                                        )
                                    )
                                ),
                        ),
                ),
            )
    })
    .bind(config.listen_address)
    .expect("failed to bind to listen address")
    .run()
    .await
}
