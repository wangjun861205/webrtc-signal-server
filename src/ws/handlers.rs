use std::collections::HashMap;
use std::ops::Add;

use crate::core::notifier::Notifier;
use crate::core::repository::{
    AddrStore, Friend, InsertChatMessage, Repository,
};
use crate::ws::actor::WS;
use crate::ws::messages::{InMessage, OutMessage, Outcome};
use actix::fut::wrap_future;
use actix::{AsyncContext, Handler};
use log::error;
use serde::Serialize;

use super::messages::{
    Accept, AddFriend, InChatMessage, OutChatMessage, OutcomeType,
};

impl<R, N, S> Handler<InMessage> for WS<R, N, S>
where
    R: Repository + Clone + Unpin + 'static,
    N: Notifier + Clone + Unpin + 'static,
    S: AddrStore + Clone + Unpin + 'static,
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
        // let addrs = self.addrs.clone();
        let mut notifier = self.notifier.clone();
        ctx.spawn(wrap_future(async move {
            // if let Some(dest_addr) = addrs.read().await.get(&to) {
            //     dest_addr.do_send(OutMessage {
            //         from: user_id,
            //         content,
            //     });
            //     return;
            // } else {
            //     match notifier.get_token(&to).await {
            //         Ok(token) => {
            //             if let Some(token) = token {
            //                 if let Err(e) = notifier
            //                     .send_notification::<HashMap<&str, String>>(
            //                         &token,
            //                         "message",
            //                         "new webrtc message",
            //                         HashMap::from_iter(
            //                             vec![
            //                                 ("sender", user_id),
            //                                 ("content", content),
            //                             ]
            //                             .into_iter(),
            //                         ),
            //                     )
            //                     .await
            //                 {
            //                     error!("failed to send notification: {}", e);
            //                 }
            //             }
            //         }
            //         Err(e) => {
            //             error!("failed to get notification token: {}", e);
            //         }
            //     }
            // }
            // self_addr.do_send(Outcome::<()>::error(
            //     OutcomeType::Message,
            //     404,
            //     "destination not found".into(),
            // ));
        }));
    }
}

impl<R, N, S> Handler<OutMessage> for WS<R, N, S>
where
    R: Repository + Clone + Unpin + 'static,
    N: Notifier + Clone + Unpin + 'static,
    S: AddrStore + Clone + Unpin + 'static,
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

impl<R, N, S> Handler<AddFriend> for WS<R, N, S>
where
    R: Repository + Clone + Unpin + 'static,
    N: Notifier + Clone + Unpin + 'static,
    S: AddrStore + Clone + Unpin + 'static,
{
    type Result = ();
    fn handle(
        &mut self,
        msg: AddFriend,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        ctx.notify(Outcome::success(OutcomeType::AddFriend, msg));
    }
}

impl<R, N, S> Handler<Accept> for WS<R, N, S>
where
    R: Repository + Clone + Unpin + 'static,
    N: Notifier + Clone + Unpin + 'static,
    S: AddrStore + Clone + Unpin + 'static,
{
    type Result = ();
    fn handle(
        &mut self,
        Accept { id }: Accept,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        let repo = self.repo.clone();
        // let addrs = self.addrs.clone();
        ctx.spawn(wrap_future(async move {
            // let req = friends_store.get_friend_request(&id).await.unwrap();
            // friends_store.accept_friend_request(&id).await.unwrap();
            // if let Some(addr) = addrs.read().await.get(&req.from) {
            //     match friends_store.get_friend(&req.to).await {
            //         Ok(friend) => {
            //             addr.do_send(Outcome::success(
            //                 OutcomeType::Accept,
            //                 Friend {
            //                     id: friend.id,
            //                     phone: friend.phone,
            //                     avatar: friend.avatar,
            //                 },
            //             ));
            //         }
            //         Err(err) => {
            //             error!("{}", err);
            //         }
            //     }
            // }
        }));
    }
}

impl<O, R, N, S> Handler<Outcome<O>> for WS<R, N, S>
where
    O: Serialize,
    R: Repository + Clone + Unpin + 'static,
    N: Notifier + Clone + Unpin + 'static,
    S: AddrStore + Clone + Unpin + 'static,
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

impl<R, N, S> Handler<InChatMessage> for WS<R, N, S>
where
    R: Repository + Clone + Unpin + 'static,
    N: Notifier + Clone + Unpin + 'static,
    S: AddrStore + Clone + Unpin + 'static,
{
    type Result = ();
    fn handle(
        &mut self,
        InChatMessage { to, content }: InChatMessage,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        let repo = self.repo.clone();
        let self_id = self.user_id.clone();
        let self_addr = ctx.address();
        // let addrs = self.addrs.clone();
        ctx.spawn(wrap_future(async move {
            match repo
                .insert_chat_message(&InsertChatMessage {
                    from: self_id.clone(),
                    to: to.clone(),
                    mime_type: "text/plain".into(),
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
                Ok(msg) => match repo.get_phone(&self_id).await {
                    Ok(phone) => {
                        // if let Some(addr) = addrs.read().await.get(&to) {
                        //     addr.do_send(OutChatMessage {
                        //         id: msg.id.clone(),
                        //         from: self_id,
                        //         phone,
                        //         content,
                        //     });
                        //     if let Err(e) = repo.mark_as_read(&msg.id).await {
                        //         error!("{}", e);
                        //     }
                        // }
                    }
                    Err(e) => {
                        error!("{}", e);
                    }
                },
            }
        }));
    }
}

impl<R, N, S> Handler<OutChatMessage> for WS<R, N, S>
where
    R: Repository + Clone + Unpin + 'static,
    N: Notifier + Clone + Unpin + 'static,
    S: AddrStore + Clone + Unpin + 'static,
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
