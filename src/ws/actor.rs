use actix::{ActorContext, Handler};
use actix_web::{
    error::{ErrorForbidden, ErrorInternalServerError},
    web::Data,
    web::{Payload, Query},
    Error, HttpRequest, HttpResponse,
};
use log::error;
use serde::Deserialize;

use actix::{Actor, Addr, AsyncContext, StreamHandler};
use actix_web_actors::ws::{
    self, Message as WSMessage, ProtocolError, WebsocketContext,
};
use auth_service::core::{
    hasher::Hasher, repository::Repository as AuthRepository,
    service::Service as AuthService, token_manager::TokenManager,
};
use serde_json::{from_str, to_string};

use crate::{
    core::{
        message::Message,
        notifier::Notifier,
        repository::{AddrStore, Repository},
    },
    stores::addr::AddrMap,
};

pub struct WS;

impl WS {
    pub fn new() -> Self {
        Self {}
    }
}

impl Actor for WS {
    type Context = WebsocketContext<Self>;
}

impl StreamHandler<Result<WSMessage, ProtocolError>> for WS {
    fn handle(
        &mut self,
        item: Result<WSMessage, ProtocolError>,
        ctx: &mut Self::Context,
    ) {
        match item {
            Ok(WSMessage::Ping(msg)) => ctx.pong(&msg),
            _ => {}
        }
    }
}

impl Handler<Message> for WS {
    type Result = ();
    fn handle(
        &mut self,
        msg: Message,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        match to_string(&msg) {
            Ok(msg) => ctx.text(msg),
            Err(e) => {
                error!("failed to serialize message: {}", e);
            }
        }
    }
}

#[derive(Deserialize)]
pub(crate) struct Index {
    pub(crate) auth_token: String,
}

pub(crate) async fn index<R, H, T, F, N, S>(
    req: HttpRequest,
    stream: Payload,
    addrs: Data<AddrMap>,
    auth_service: Data<AuthService<R, H, T>>,
    friends_stores: Data<F>,
    notifier: Data<N>,
    Query(Index { auth_token }): Query<Index>,
) -> Result<HttpResponse, Error>
where
    R: AuthRepository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
    F: Repository + Clone + Unpin + 'static,
    N: Notifier + Clone + Unpin + 'static,
    S: AddrStore + Clone + Unpin + 'static,
{
    let user_id = auth_service
        .verify_token(&auth_token)
        .await
        .map_err(ErrorForbidden)?;
    let (addr, resp) = ws::start_with_addr(WS::new(), &req, stream)
        .map_err(ErrorInternalServerError)?;
    addrs
        .add_addr(&user_id, addr.recipient())
        .await
        .map_err(ErrorInternalServerError)?;
    Ok(resp)
}
