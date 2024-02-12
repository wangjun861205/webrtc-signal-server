use actix::{fut::wrap_future, Addr};
use actix_web::{
    error::{ErrorForbidden, ErrorInternalServerError},
    web::Data,
    web::{Payload, Query},
    Error, HttpRequest, HttpResponse,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use actix::{Actor, AsyncContext, StreamHandler};
use actix_web_actors::ws::{
    self, Message as WSMessage, ProtocolError, WebsocketContext,
};
use auth_service::core::{
    hasher::Hasher, repository::Repository as AuthRepository,
    service::Service as AuthService, token_manager::TokenManager,
};
use serde_json::from_str;

use crate::{
    core::{notifier::Notifier, repository::Repository},
    ws::messages::InMessage,
};

use super::messages::{
    Accept, AddFriend, FriendRequests, InChatMessage, Income, Outcome,
    OutcomeType,
};

pub struct WS<F, N>
where
    F: Repository + Clone + Unpin + 'static,
    N: Notifier + Clone + Unpin + 'static,
{
    pub(crate) user_id: String,
    pub(crate) addrs: Arc<RwLock<HashMap<String, Addr<Self>>>>,
    pub(crate) friends_store: F,
    pub(crate) notifier: N,
}

impl<F, N> WS<F, N>
where
    F: Repository + Clone + Unpin + 'static,
    N: Notifier + Clone + Unpin + 'static,
{
    pub fn new(
        user_id: String,
        addrs: Arc<RwLock<HashMap<String, Addr<Self>>>>,
        friends_store: F,
        notifier: N,
    ) -> Self {
        Self {
            user_id,
            addrs,
            friends_store,
            notifier,
        }
    }
}

impl<F, N> Actor for WS<F, N>
where
    F: Repository + Clone + Unpin + 'static,
    N: Notifier + Clone + Unpin + 'static,
{
    type Context = WebsocketContext<Self>;
}

impl<F, N> StreamHandler<Result<WSMessage, ProtocolError>> for WS<F, N>
where
    F: Repository + Clone + Unpin + 'static,
    N: Notifier + Clone + Unpin + 'static,
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
                    Income::AddFriend { user_id } => {
                        ctx.notify(AddFriend { user_id })
                    }
                    // Income::FriendRequests => ctx.notify(FriendRequests),
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
                let addrs = self.addrs.clone();
                let self_id = self.user_id.clone();
                ctx.wait(wrap_future(async move {
                    addrs.write().await.remove(&self_id);
                }));
                // ctx.close(Some(CloseReason {
                //     code: CloseCode::Normal,
                //     description: Some("as proposed".into()),
                // }));
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

type AddrMap<F, N> = Arc<RwLock<HashMap<String, Addr<WS<F, N>>>>>;

#[derive(Deserialize)]
pub(crate) struct Index {
    pub(crate) auth_token: String,
}

pub(crate) async fn index<R, H, T, F, N>(
    req: HttpRequest,
    stream: Payload,
    map: Data<AddrMap<F, N>>,
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
{
    let user_id = auth_service
        .verify_token(&auth_token)
        .await
        .map_err(ErrorForbidden)?;

    let (addr, resp) = ws::start_with_addr(
        WS::new(
            user_id.clone(),
            map.as_ref().clone(),
            friends_stores.as_ref().clone(),
            notifier.as_ref().clone(),
        ),
        &req,
        stream,
    )
    .map_err(ErrorInternalServerError)?;
    map.write().await.insert(user_id.clone(), addr);
    Ok(resp)
}
