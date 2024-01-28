use crate::core::store::FriendsStore;
use crate::ws::actor::WS;
use crate::ws::messages::{InMessage, OutMessage, Outcome};
use actix::fut::wrap_future;
use actix::{AsyncContext, Handler};
use auth_service::core::{
    hasher::Hasher, repository::Repository, token_manager::TokenManager,
};
use serde::Serialize;

use super::messages::{
    Accept, AcquireFriends, AddFriend, FriendRequests, Greet, Online,
    OutcomeType,
};

impl<R, H, T, F> Handler<InMessage> for WS<R, H, T, F>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
    F: FriendsStore + Clone + Unpin + 'static,
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
        let auth_service = self.auth_service.clone();
        let addrs = self.addrs.clone();
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
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
    F: FriendsStore + Clone + Unpin + 'static,
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

impl<R, H, T, F> Handler<Online> for WS<R, H, T, F>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
    F: FriendsStore + Clone + Unpin + 'static,
{
    type Result = ();
    fn handle(&mut self, _: Online, ctx: &mut Self::Context) -> Self::Result {
        let addrs = self.addrs.clone();
        let self_addr = ctx.address();
        let self_id = self.user_id.clone();
        let friends_store = self.friends_store.clone();
        ctx.spawn(wrap_future(async move {
            let friends = friends_store.friends(&self_id).await.unwrap();
            addrs
                .write()
                .await
                .insert(self_id.clone(), self_addr.clone());
            self_addr.do_send(Outcome::success(OutcomeType::Online, friends));
        }));
    }
}

// impl<R, H, T, F> Handler<Greet> for WS<R, H, T, F>
// where
//     R: Repository + Clone + 'static,
//     H: Hasher + Clone + 'static,
//     T: TokenManager + Clone + 'static,
//     F: FriendsStore + Clone + Unpin + 'static,
// {
//     type Result = ();
//     fn handle(&mut self, Greet { user_id }: Greet, ctx: &mut Self::Context) -> Self::Result {
//         let addrs = self.addrs.clone();
//         let self_id = self.user_id.clone();
//         ctx.spawn(wrap_future(async move {
//             if self.friends_store.is_friend(self_id.clone(), user_id.clone()).await.unwrap() {
//                 return;
//             }
//             self.friends.insert(user_id.clone());
//             ctx.notify(Outcome::success(OutcomeType::Greet, user_id.clone()));
//             if let Some(addr) = addrs.read().await.get(&user_id) {
//                 addr.do_send(Greet { user_id: self_id.clone() });
//             }
//         }));
//     }
// }

impl<R, H, T, F> Handler<AcquireFriends> for WS<R, H, T, F>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
    F: FriendsStore + Clone + Unpin + 'static,
{
    type Result = ();
    fn handle(
        &mut self,
        _: AcquireFriends,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        let self_id = self.user_id.clone();
        let self_addr = ctx.address();
        let friends_store = self.friends_store.clone();
        ctx.spawn(wrap_future(async move {
            let friends = friends_store.friends(&self_id).await.unwrap();
            self_addr.do_send(Outcome::success(
                OutcomeType::AcquireFriends,
                friends,
            ));
        }));
    }
}

impl<R, H, T, F> Handler<AddFriend> for WS<R, H, T, F>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
    F: FriendsStore + Clone + Unpin + 'static,
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
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
    F: FriendsStore + Clone + Unpin + 'static,
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
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
    F: FriendsStore + Clone + Unpin + 'static,
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
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
    O: Serialize,
    F: FriendsStore + Clone + Unpin + 'static,
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
