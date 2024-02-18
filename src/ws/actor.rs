use actix::ActorContext;
use actix_web::{
    error::{ErrorForbidden, ErrorInternalServerError},
    web::Data,
    web::{Payload, Query},
    Error, HttpRequest, HttpResponse,
};
use serde::Deserialize;

use actix::{Actor, Addr, AsyncContext, StreamHandler};
use actix_web_actors::ws::{
    self, Message as WSMessage, ProtocolError, WebsocketContext,
};
use auth_service::core::{
    hasher::Hasher, repository::Repository as AuthRepository,
    service::Service as AuthService, token_manager::TokenManager,
};
use serde_json::from_str;

use crate::{
    core::{
        notifier::Notifier,
        repository::{AddrStore, Repository},
    },
    stores::addr::AddrMap,
    ws::messages::InMessage,
};

use super::messages::{Accept, InChatMessage, Income, Outcome, OutcomeType};

pub struct WS<R, N, S>
where
    R: Repository + Clone + Unpin + 'static,
    N: Notifier + Clone + Unpin + 'static,
    S: AddrStore + Clone + Unpin + 'static,
{
    pub(crate) user_id: String,
    pub(crate) repo: R,
    pub(crate) notifier: N,
    pub(crate) addrs: S,
}

impl<R, N, S> WS<R, N, S>
where
    R: Repository + Clone + Unpin + 'static,
    N: Notifier + Clone + Unpin + 'static,
    S: AddrStore + Clone + Unpin + 'static,
{
    pub fn new(user_id: String, repo: R, notifier: N, addrs: S) -> Self {
        Self {
            user_id,
            repo,
            notifier,
            addrs,
        }
    }
}

impl<R, N, S> Actor for WS<R, N, S>
where
    R: Repository + Clone + Unpin + 'static,
    N: Notifier + Clone + Unpin + 'static,
    S: AddrStore + Clone + Unpin + 'static,
{
    type Context = WebsocketContext<Self>;
}

impl<R, N, S> StreamHandler<Result<WSMessage, ProtocolError>> for WS<R, N, S>
where
    R: Repository + Clone + Unpin + 'static,
    N: Notifier + Clone + Unpin + 'static,
    S: AddrStore + Clone + Unpin + 'static,
{
    fn handle(
        &mut self,
        item: Result<WSMessage, ProtocolError>,
        ctx: &mut Self::Context,
    ) {
        match item {
            Ok(WSMessage::Ping(msg)) => ctx.pong(&msg),
            Ok(WSMessage::Text(text)) => match from_str::<Income>(&text) {
                Ok(msg) => match msg {
                    Income::Message { to, content } => {
                        ctx.notify(InMessage {
                            user_id: self.user_id.clone(),
                            to,
                            content,
                        });
                    }
                    Income::ChatMessage { to, content } => {
                        ctx.notify(InChatMessage { to, content })
                    }
                    Income::Accept { id } => ctx.notify(Accept { id }),
                },
                Err(err) => {
                    ctx.notify(Outcome::<()>::error(
                        OutcomeType::Other,
                        400,
                        err.to_string(),
                    ));
                }
            },
            Ok(WSMessage::Close(_)) => {
                ctx.stop();
            }

            _ => {
                ctx.notify(Outcome::<()>::error(
                    OutcomeType::Other,
                    400,
                    "unsupported message type".into(),
                ));
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
    let (addr, resp) = ws::start_with_addr(
        WS::new(
            user_id.clone(),
            friends_stores.as_ref().clone(),
            notifier.as_ref().clone(),
            addrs.as_ref().clone(),
        ),
        &req,
        stream,
    )
    .map_err(ErrorInternalServerError)?;
    addrs
        .add_addr(&user_id, addr.recipient())
        .await
        .map_err(ErrorInternalServerError)?;
    Ok(resp)
}
