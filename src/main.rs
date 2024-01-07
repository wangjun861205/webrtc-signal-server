use actix::{Actor, StreamHandler};
use actix_web::{
    middleware::Logger,
    web::{get, Payload},
    HttpRequest, HttpResponse, HttpServer,
};
use actix_web::{App, Error};
use actix_web_actors::ws;
use nb_from_env::{FromEnv, FromEnvDerive};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

#[derive(FromEnvDerive)]
struct Config {
    listen_address: String,
}

struct WS;

impl Actor for WS {
    type Context = ws::WebsocketContext<Self>;
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WS {
    fn handle(&mut self, item: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match item {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => ctx.text(text),
            _ => (),
        }
    }
}

async fn index(req: HttpRequest, stream: Payload) -> Result<HttpResponse, Error> {
    ws::start(WS {}, &req, stream)
}

#[derive(Serialize, Deserialize)]
struct Message<'a> {
    from: String,
    to: String,
    #[serde(borrow)]
    payload: &'a RawValue,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    let config = Config::from_env();
    HttpServer::new(|| {
        App::new()
            .wrap(Logger::default())
            .route("/ws", get().to(index))
    })
    .bind(config.listen_address)
    .expect("failed to bind to listen address")
    .run()
    .await
}
