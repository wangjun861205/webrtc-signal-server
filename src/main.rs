pub mod core;

use core::ws::WS;
use std::{collections::HashMap, sync::Arc};

use actix::Addr;
use actix_web::{
    middleware::Logger,
    web::{get, Data, Path, Payload},
    App, Error, HttpRequest, HttpResponse, HttpServer,
};
use actix_web_actors::ws;
use nb_from_env::{FromEnv, FromEnvDerive};
use tokio::sync::RwLock;

#[derive(FromEnvDerive)]
struct Config {
    listen_address: String,
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
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(Data::new(map.clone()))
            .route("/ws/{id}", get().to(index))
    })
    .bind(config.listen_address)
    .expect("failed to bind to listen address")
    .run()
    .await
}
