use actix::{fut::wrap_future, Addr};
use actix_web::{web::Data, web::Payload, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use actix::{Actor, AsyncContext, Message, StreamHandler};
use actix_web_actors::ws::{Message as WSMessage, ProtocolError, WebsocketContext};
use auth_service::core::{
    hasher::Hasher, repository::Repository, service::Service as AuthService,
    token_manager::TokenManager,
};
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};

use super::common::{In, Out, ResponseType};

#[derive(Serialize, Deserialize, Message)]
#[rtype(result = "()")]
struct LoginSuccess;

pub(crate) struct WS<R, H, T>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
{
    pub(crate) addrs: Arc<RwLock<HashMap<String, Addr<Self>>>>,
    pub(crate) auth_service: Data<AuthService<R, H, T>>,
}

impl<R, H, T> WS<R, H, T>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
{
    pub fn new(
        addrs: Arc<RwLock<HashMap<String, Addr<Self>>>>,
        auth_service: Data<AuthService<R, H, T>>,
    ) -> Self {
        Self {
            addrs,
            auth_service,
        }
    }
}

impl<R, H, T> Actor for WS<R, H, T>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
{
    type Context = WebsocketContext<Self>;

    // fn started(&mut self, ctx: &mut Self::Context) {
    //     ctx.notify(Login(self.id.clone()));
    // }
}

impl<R, H, T> StreamHandler<Result<WSMessage, ProtocolError>> for WS<R, H, T>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
{
    fn handle(&mut self, item: Result<WSMessage, ProtocolError>, ctx: &mut Self::Context) {
        match item {
            Ok(WSMessage::Ping(msg)) => ctx.pong(&msg),
            Ok(WSMessage::Text(text)) => match from_str::<In>(&text) {
                Ok(payload) => {
                    let auth_service = self.auth_service.clone();
                    let addrs = self.addrs.clone();
                    let self_addr = ctx.address();
                    ctx.spawn(wrap_future(async move {
                        if let Ok(id) = auth_service
                            .token_manager
                            .verify_token(&payload.token)
                            .await
                        {
                            addrs.write().await.insert(id.clone(), self_addr.clone());
                            self_addr.do_send(payload);
                            return;
                        }
                        self_addr.do_send(Out::new(
                            "server",
                            403,
                            ResponseType::Error,
                            "invalid token",
                        ))
                    }));
                }
                Err(err) => {
                    ctx.notify(Out::new(
                        "server",
                        400,
                        ResponseType::Error,
                        err.to_string(),
                    ));
                }
            },
            Ok(WSMessage::Close(_)) => {}

            _ => {
                let payload = Out::new(
                    "server",
                    400,
                    ResponseType::Error,
                    "Unsupported websocket message type",
                );
                ctx.text(to_string(&payload).unwrap());
            }
        }
    }
}

type AddrMap<R, H, T> = Arc<RwLock<HashMap<String, Addr<WS<R, H, T>>>>>;

pub async fn index<R, H, T>(
    req: HttpRequest,
    stream: Payload,
    map: Data<AddrMap<R, H, T>>,
    auth_service: Data<AuthService<R, H, T>>,
) -> Result<HttpResponse, Error>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
{
    ws::start(
        WS::new(map.as_ref().clone(), auth_service.clone()),
        &req,
        stream,
    )
}
