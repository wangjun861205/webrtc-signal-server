use actix::{fut::wrap_future, ActorContext, Addr};
use actix_web::{
    error::{ErrorForbidden, ErrorInternalServerError},
    web::Data,
    web::{Payload, Query},
    Error, HttpRequest, HttpResponse,
};
use actix_web_actors::ws::{self, CloseCode, CloseReason};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use actix::{Actor, AsyncContext, StreamHandler};
use actix_web_actors::ws::{
    Message as WSMessage, ProtocolError, WebsocketContext,
};
use auth_service::core::{
    hasher::Hasher, repository::Repository, service::Service as AuthService,
    token_manager::TokenManager,
};
use serde_json::from_str;

use crate::{core::store::FriendsStore, ws::messages::InMessage};

use super::messages::{
    Accept, AcquireFriends, AddFriend, FriendRequests, Income, Online, Outcome,
    OutcomeType,
};

pub struct WS<R, H, T, F>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
    F: FriendsStore + Clone + Unpin + 'static,
{
    pub(crate) user_id: String,
    pub(crate) addrs: Arc<RwLock<HashMap<String, Addr<Self>>>>,
    pub(crate) auth_service: Data<AuthService<R, H, T>>,
    pub(crate) friends_store: F,
}

impl<R, H, T, F> WS<R, H, T, F>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
    F: FriendsStore + Clone + Unpin + 'static,
{
    pub fn new(
        user_id: String,
        addrs: Arc<RwLock<HashMap<String, Addr<Self>>>>,
        auth_service: Data<AuthService<R, H, T>>,
        friends_store: F,
    ) -> Self {
        Self {
            user_id,
            addrs,
            auth_service,
            friends_store,
        }
    }
}

impl<R, H, T, F> Actor for WS<R, H, T, F>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
    F: FriendsStore + Clone + Unpin + 'static,
{
    type Context = WebsocketContext<Self>;
}

impl<R, H, T, F> StreamHandler<Result<WSMessage, ProtocolError>>
    for WS<R, H, T, F>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
    F: FriendsStore + Clone + Unpin + 'static,
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
                    Income::AddFriend { user_id } => {
                        ctx.notify(AddFriend { user_id })
                    }
                    Income::FriendRequests => ctx.notify(FriendRequests),
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
                ctx.close(Some(CloseReason {
                    code: CloseCode::Normal,
                    description: Some("as proposed".into()),
                }));
                ctx.terminate();
                let addrs = self.addrs.clone();
                let self_id = self.user_id.clone();
                ctx.wait(wrap_future(async move {
                    addrs.write().await.remove(&self_id);
                }));
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

type AddrMap<R, H, T, F> = Arc<RwLock<HashMap<String, Addr<WS<R, H, T, F>>>>>;

#[derive(Deserialize)]
pub(crate) struct Index {
    pub(crate) auth_token: String,
}

pub(crate) async fn index<R, H, T, F>(
    req: HttpRequest,
    stream: Payload,
    map: Data<AddrMap<R, H, T, F>>,
    auth_service: Data<AuthService<R, H, T>>,
    friends_stores: Data<F>,
    Query(Index { auth_token }): Query<Index>,
) -> Result<HttpResponse, Error>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
    F: FriendsStore + Clone + Unpin + 'static,
{
    let user_id = auth_service
        .verify_token(&auth_token)
        .await
        .map_err(ErrorForbidden)?;

    let (addr, resp) = ws::start_with_addr(
        WS::new(
            user_id.clone(),
            map.as_ref().clone(),
            auth_service.clone(),
            friends_stores.as_ref().clone(),
        ),
        &req,
        stream,
    )
    .map_err(ErrorInternalServerError)?;
    map.write().await.insert(user_id.clone(), addr);
    Ok(resp)
}
