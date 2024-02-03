use crate::core::repository::{InsertChatMessage, Repository};
use crate::ws::actor::WS;
use crate::ws::messages::{InMessage, OutMessage, Outcome};
use actix::fut::wrap_future;
use actix::{AsyncContext, Handler};
use auth_service::core::{
    hasher::Hasher, repository::Repository as AuthRepository,
    token_manager::TokenManager,
};
use serde::Serialize;

use super::messages::{
    Accept, AddFriend, FriendRequests, InChatMessage, OutChatMessage,
    OutcomeType,
};

impl<R, H, T, F> Handler<InMessage> for WS<R, H, T, F>
where
    R: AuthRepository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
    F: Repository + Clone + Unpin + 'static,
{
    type Result = ();
    fn handle(
        &mut self,
        InMessage {
            user_id,
            to,
            content,
        }: InMessage,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        let self_addr = ctx.address();
        let addrs = self.addrs.clone();
        // let repo = self.friends_store.clone();
        ctx.spawn(wrap_future(async move {
            if let Some(dest_addr) = addrs.read().await.get(&to) {
                dest_addr.do_send(OutMessage {
                    from: user_id,
                    content,
                });
                return;
            }
            self_addr.do_send(Outcome::<()>::error(
                OutcomeType::Message,
                404,
                "destination not found".into(),
            ));
        }));
    }
}

impl<R, H, T, F> Handler<OutMessage> for WS<R, H, T, F>
where
    R: AuthRepository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
    F: Repository + Clone + Unpin + 'static,
{
    type Result = ();
    fn handle(
        &mut self,
        out: OutMessage,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        ctx.notify(Outcome::success(OutcomeType::Message, out));
    }
}

impl<R, H, T, F> Handler<AddFriend> for WS<R, H, T, F>
where
    R: AuthRepository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
    F: Repository + Clone + Unpin + 'static,
{
    type Result = ();
    fn handle(
        &mut self,
        AddFriend { user_id }: AddFriend,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        ctx.notify(Outcome::success(OutcomeType::AddFriend, user_id));
    }
}

impl<R, H, T, F> Handler<FriendRequests> for WS<R, H, T, F>
where
    R: AuthRepository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
    F: Repository + Clone + Unpin + 'static,
{
    type Result = ();
    fn handle(
        &mut self,
        _: FriendRequests,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        let self_id = self.user_id.clone();
        let self_addr = ctx.address();
        let friends_store = self.friends_store.clone();
        let addrs = self.addrs.clone();
        ctx.spawn(wrap_future(async move {
            let requests = friends_store
                .pending_friend_requests(&self_id)
                .await
                .unwrap();
            self_addr.do_send(Outcome::success(
                OutcomeType::FriendRequests,
                requests,
            ));
        }));
    }
}

impl<R, H, T, F> Handler<Accept> for WS<R, H, T, F>
where
    R: AuthRepository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
    F: Repository + Clone + Unpin + 'static,
{
    type Result = ();
    fn handle(
        &mut self,
        Accept { id }: Accept,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        let self_id = self.user_id.clone();
        let self_addr = ctx.address();
        let friends_store = self.friends_store.clone();
        let addrs = self.addrs.clone();
        ctx.spawn(wrap_future(async move {
            let req = friends_store.get_friend_request(&id).await.unwrap();
            friends_store.accept_friend_request(&id).await.unwrap();
            if let Some(addr) = addrs.read().await.get(&req.from) {
                addr.do_send(Outcome::success(OutcomeType::Accept, ()));
            }
        }));
    }
}

impl<R, H, T, O, F> Handler<Outcome<O>> for WS<R, H, T, F>
where
    R: AuthRepository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
    O: Serialize,
    F: Repository + Clone + Unpin + 'static,
{
    type Result = ();
    fn handle(
        &mut self,
        out: Outcome<O>,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        ctx.text(serde_json::to_string(&out).unwrap());
    }
}

impl<R, H, T, F> Handler<InChatMessage> for WS<R, H, T, F>
where
    R: AuthRepository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
    F: Repository + Clone + Unpin + 'static,
{
    type Result = ();
    fn handle(
        &mut self,
        InChatMessage { to, content }: InChatMessage,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        let repo = self.friends_store.clone();
        let self_id = self.user_id.clone();
        let self_addr = ctx.address();
        let addrs = self.addrs.clone();
        ctx.spawn(wrap_future(async move {
            if let Err(e) = repo
                .insert_chat_message(&InsertChatMessage {
                    from: self_id.clone(),
                    to: to.clone(),
                    content: content.clone(),
                })
                .await
            {
                self_addr.do_send(Outcome::<()>::error(
                    OutcomeType::ChatMessage,
                    500,
                    e.to_string(),
                ));
                return;
            }
            if let Some(addr) = addrs.read().await.get(&to) {
                addr.do_send(OutChatMessage {
                    from: self_id,
                    content,
                })
            }
        }));
    }
}

impl<R, H, T, F> Handler<OutChatMessage> for WS<R, H, T, F>
where
    R: AuthRepository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
    F: Repository + Clone + Unpin + 'static,
{
    type Result = ();
    fn handle(
        &mut self,
        msg: OutChatMessage,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        ctx.text(
            serde_json::to_string(&Outcome::success(
                OutcomeType::ChatMessage,
                msg,
            ))
            .unwrap(),
        );
    }
}
