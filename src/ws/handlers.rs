use crate::ws::actor::WS;
use crate::ws::messages::{InMessage, OutMessage, Outcome};
use actix::fut::wrap_future;
use actix::{AsyncContext, Handler};
use auth_service::core::{hasher::Hasher, repository::Repository, token_manager::TokenManager};
use serde::Serialize;

use super::messages::{AcquireFriends, Greet, Online, OutcomeType};

impl<R, H, T> Handler<InMessage> for WS<R, H, T>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
{
    type Result = ();
    fn handle(&mut self, InMessage { user_id, to, content }: InMessage, ctx: &mut Self::Context) -> Self::Result {
        let self_addr = ctx.address();
        let auth_service = self.auth_service.clone();
        let addrs = self.addrs.clone();
        ctx.spawn(wrap_future(async move {
            if let Some(dest_addr) = addrs.read().await.get(&to) {
                dest_addr.do_send(OutMessage { from: user_id, content });
                return;
            }
            self_addr.do_send(Outcome::<()>::error(OutcomeType::Message, 404, "destination not found".into()));
        }));
    }
}

impl<R, H, T> Handler<OutMessage> for WS<R, H, T>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
{
    type Result = ();
    fn handle(&mut self, out: OutMessage, ctx: &mut Self::Context) -> Self::Result {
        ctx.notify(Outcome::success(OutcomeType::Message, out));
    }
}

impl<R, H, T> Handler<Online> for WS<R, H, T>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
{
    type Result = ();
    fn handle(&mut self, _: Online, ctx: &mut Self::Context) -> Self::Result {
        let addrs = self.addrs.clone();
        let self_addr = ctx.address();
        let self_id = self.user_id.clone();
        ctx.spawn(wrap_future(async move {
            addrs.write().await.insert(self_id.clone(), self_addr);
            for (id, addr) in addrs.read().await.iter() {
                if id != &self_id {
                    addr.do_send(Greet { user_id: self_id.clone() });
                }
            }
        }));
    }
}

impl<R, H, T> Handler<Greet> for WS<R, H, T>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
{
    type Result = ();
    fn handle(&mut self, Greet { user_id }: Greet, ctx: &mut Self::Context) -> Self::Result {
        if self.friends.contains(&user_id) {
            return;
        }
        self.friends.insert(user_id.clone());
        ctx.notify(Outcome::success(OutcomeType::Greet, user_id.clone()));
        let addrs = self.addrs.clone();
        let self_id = self.user_id.clone();
        ctx.spawn(wrap_future(async move {
            if let Some(addr) = addrs.read().await.get(&user_id) {
                addr.do_send(Greet { user_id: self_id.clone() });
            }
        }));
    }
}

impl<R, H, T> Handler<AcquireFriends> for WS<R, H, T>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
{
    type Result = ();
    fn handle(&mut self, _: AcquireFriends, ctx: &mut Self::Context) -> Self::Result {
        ctx.notify(Outcome::success(OutcomeType::AcquireFriends, self.friends.iter().map(|f| f.to_owned()).collect::<Vec<String>>()));
    }
}

impl<R, H, T, O> Handler<Outcome<O>> for WS<R, H, T>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
    O: Serialize,
{
    type Result = ();
    fn handle(&mut self, out: Outcome<O>, ctx: &mut Self::Context) -> Self::Result {
        ctx.text(serde_json::to_string(&out).unwrap());
    }
}
