use std::collections::HashMap;

use crate::core::notifier::Notifier;
use crate::core::repository::{InsertChatMessage, Repository};
use crate::ws::actor::WS;
use crate::ws::messages::{InMessage, OutMessage, Outcome};
use actix::fut::wrap_future;
use actix::{AsyncContext, Handler};
use log::error;
use serde::Serialize;

use super::messages::{
    Accept, AddFriend, InChatMessage, OutChatMessage, OutcomeType,
};

impl<F, N> Handler<InMessage> for WS<F, N>
where
    F: Repository + Clone + Unpin + 'static,
    N: Notifier + Clone + Unpin + 'static,
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
        let mut notifier = self.notifier.clone();
        ctx.spawn(wrap_future(async move {
            if let Some(dest_addr) = addrs.read().await.get(&to) {
                dest_addr.do_send(OutMessage {
                    from: user_id,
                    content,
                });
                return;
            } else {
                match notifier.get_token(&to).await {
                    Ok(token) => {
                        if let Some(token) = token {
                            if let Err(e) = notifier
                                .send_notification::<HashMap<&str, String>>(
                                    &token,
                                    "message",
                                    "new webrtc message",
                                    HashMap::from_iter(
                                        vec![
                                            ("sender", user_id),
                                            ("content", content),
                                        ]
                                        .into_iter(),
                                    ),
                                )
                                .await
                            {
                                error!("failed to send notification: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("failed to get notification token: {}", e);
                    }
                }
            }
            self_addr.do_send(Outcome::<()>::error(
                OutcomeType::Message,
                404,
                "destination not found".into(),
            ));
        }));
    }
}

impl<F, N> Handler<OutMessage> for WS<F, N>
where
    F: Repository + Clone + Unpin + 'static,
    N: Notifier + Clone + Unpin + 'static,
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

impl<F, N> Handler<AddFriend> for WS<F, N>
where
    F: Repository + Clone + Unpin + 'static,
    N: Notifier + Clone + Unpin + 'static,
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

impl<F, N> Handler<Accept> for WS<F, N>
where
    F: Repository + Clone + Unpin + 'static,
    N: Notifier + Clone + Unpin + 'static,
{
    type Result = ();
    fn handle(
        &mut self,
        Accept { id }: Accept,
        ctx: &mut Self::Context,
    ) -> Self::Result {
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

impl<O, F, N> Handler<Outcome<O>> for WS<F, N>
where
    O: Serialize,
    F: Repository + Clone + Unpin + 'static,
    N: Notifier + Clone + Unpin + 'static,
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

impl<F, N> Handler<InChatMessage> for WS<F, N>
where
    F: Repository + Clone + Unpin + 'static,
    N: Notifier + Clone + Unpin + 'static,
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
            match repo
                .insert_chat_message(&InsertChatMessage {
                    from: self_id.clone(),
                    to: to.clone(),
                    content: content.clone(),
                })
                .await
            {
                Err(e) => {
                    self_addr.do_send(Outcome::<()>::error(
                        OutcomeType::ChatMessage,
                        500,
                        e.to_string(),
                    ));
                }
                Ok(id) => {
                    if let Some(addr) = addrs.read().await.get(&to) {
                        addr.do_send(OutChatMessage {
                            id,
                            from: self_id,
                            content,
                        })
                    }
                }
            }
        }));
    }
}

impl<F, N> Handler<OutChatMessage> for WS<F, N>
where
    F: Repository + Clone + Unpin + 'static,
    N: Notifier + Clone + Unpin + 'static,
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
