use actix::{fut::wrap_future, Addr};
use actix_web::{
    error::ErrorForbidden,
    web::Data,
    web::{Payload, Query},
    Error, HttpRequest, HttpResponse,
};
use actix_web_actors::ws;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

use actix::{Actor, AsyncContext, StreamHandler};
use actix_web_actors::ws::{Message as WSMessage, ProtocolError, WebsocketContext};
use auth_service::core::{hasher::Hasher, repository::Repository, service::Service as AuthService, token_manager::TokenManager};
use serde_json::from_str;

use crate::ws::messages::InMessage;

use super::messages::{AcquireFriends, Income, Online, Outcome, OutcomeType};

pub(crate) struct WS<R, H, T>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
{
    pub(crate) user_id: String,
    pub(crate) addrs: Arc<RwLock<HashMap<String, Addr<Self>>>>,
    pub(crate) auth_service: Data<AuthService<R, H, T>>,
    pub(crate) friends: HashSet<String>,
}

impl<R, H, T> WS<R, H, T>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
{
    pub fn new(user_id: String, addrs: Arc<RwLock<HashMap<String, Addr<Self>>>>, auth_service: Data<AuthService<R, H, T>>) -> Self {
        Self {
            user_id,
            addrs,
            auth_service,
            friends: HashSet::new(),
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
                Ok(msg) => match msg {
                    Income::Online => {
                        ctx.notify(Online);
                    }
                    Income::AcquireFriends => {
                        ctx.notify(AcquireFriends);
                    }
                    Income::Message { to, content } => {
                        ctx.notify(InMessage {
                            user_id: self.user_id.clone(),
                            to,
                            content,
                        });
                    }
                },
                Err(err) => {
                    ctx.notify(Outcome::<()>::error(OutcomeType::Other, 400, err.to_string()));
                }
            },
            Ok(WSMessage::Close(_)) => {}

            _ => {
                ctx.notify(Outcome::<()>::error(OutcomeType::Other, 400, "unsupported message type".into()));
            }
        }
    }
}

type AddrMap<R, H, T> = Arc<RwLock<HashMap<String, Addr<WS<R, H, T>>>>>;

#[derive(Deserialize)]
pub(crate) struct Index {
    pub(crate) auth_token: String,
}

pub(crate) async fn index<R, H, T>(
    req: HttpRequest,
    stream: Payload,
    map: Data<AddrMap<R, H, T>>,
    auth_service: Data<AuthService<R, H, T>>,
    Query(Index { auth_token }): Query<Index>,
) -> Result<HttpResponse, Error>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
{
    let user_id = auth_service.verify_token(&auth_token).await.map_err(ErrorForbidden)?;
    ws::start(WS::new(user_id, map.as_ref().clone(), auth_service.clone()), &req, stream)
}
