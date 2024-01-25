use actix::Addr;
use actix_web::{web::Data, web::Payload, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use actix::{Actor, AsyncContext, StreamHandler};
use actix_web_actors::ws::{Message as WSMessage, ProtocolError, WebsocketContext};
use auth_service::core::{hasher::Hasher, repository::Repository, service::Service as AuthService, token_manager::TokenManager};
use serde_json::from_str;

use crate::messages::{AcquireFriends, InMessage, Income, Login, Logout, Signup, WSError};

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
    pub fn new(addrs: Arc<RwLock<HashMap<String, Addr<Self>>>>, auth_service: Data<AuthService<R, H, T>>) -> Self {
        Self { addrs, auth_service }
    }
}

impl<R, H, T> Actor for WS<R, H, T>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
{
    type Context = WebsocketContext<Self>;
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
            Ok(WSMessage::Text(text)) => match from_str::<Income>(&text) {
                Ok(income) => match income {
                    Income::Login { username, password } => ctx.notify(Login { username, password }),
                    Income::Logout { token } => ctx.notify(Logout { token }),
                    Income::Signup { username, password } => ctx.notify(Signup { username, password }),
                    Income::AcquireFriends { token } => ctx.notify(AcquireFriends { token }),
                    Income::Message { token, to, content } => ctx.notify(InMessage { token, to, content }),
                },
                Err(err) => {
                    ctx.notify(WSError { status: 400, reason: err.to_string() });
                }
            },
            Ok(WSMessage::Close(_)) => {}

            _ => {
                ctx.notify(WSError {
                    status: 400,
                    reason: "unsupported message type".into(),
                });
            }
        }
    }
}

type AddrMap<R, H, T> = Arc<RwLock<HashMap<String, Addr<WS<R, H, T>>>>>;

pub(crate) async fn index<R, H, T>(req: HttpRequest, stream: Payload, map: Data<AddrMap<R, H, T>>, auth_service: Data<AuthService<R, H, T>>) -> Result<HttpResponse, Error>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
{
    ws::start(WS::new(map.as_ref().clone(), auth_service.clone()), &req, stream)
}
