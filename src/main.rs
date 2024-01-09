pub mod core;

use core::addr_keeper::AddrKeeper;

use actix::{Actor, StreamHandler};
use actix_web::{
    error::ErrorInternalServerError,
    http::StatusCode,
    middleware::Logger,
    web::{get, Data, Path, Payload},
    App, Error, HttpRequest, HttpResponse, HttpServer,
};
use actix_web_actors::ws;
use nb_from_env::{FromEnv, FromEnvDerive};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use tokio::sync::mpsc::{Receiver, Sender};

#[derive(FromEnvDerive)]
struct Config {
    listen_address: String,
}

// async fn index<K>(
//     req: HttpRequest,
//     stream: Payload,
//     id: Path<(String,)>,
//     keeper: Data<K>,
// ) -> Result<HttpResponse, Error>
// where
//     K: AddrKeeper,
// {
//     let (tx, rx) = keeper
//         .login(&id.0)
//         .map_err(|err| ErrorInternalServerError(err))?;
//     ws::start(WS { tx, rx }, &req, stream)
//     Ok(HttpResponse::new(StatusCode::OK))
// }

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    let config = Config::from_env();
    HttpServer::new(|| {
        App::new().wrap(Logger::default())
        // .route("/ws/:id", get().to(index))
    })
    .bind(config.listen_address)
    .expect("failed to bind to listen address")
    .run()
    .await
}
