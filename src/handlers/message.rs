use actix::{fut::wrap_future, prelude::*, Handler, Message as ActixMessage};
use auth_service::core::{hasher::Hasher, repository::Repository, token_manager::TokenManager};
use serde::{Deserialize, Serialize};
use serde_json::to_string;

use super::ws::WS;

#[derive(Serialize, Deserialize, ActixMessage)]
#[rtype(result = "()")]
pub struct Message {
    from: String,
    payload: String,
}

impl Message {
    pub fn new(from: String, payload: String) -> Self {
        Self { from, payload }
    }
}

impl<R, H, T> Handler<Message> for WS<R, H, T>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
{
    type Result = ();

    fn handle(&mut self, msg: Message, ctx: &mut Self::Context) -> Self::Result {
        ctx.text(to_string(&msg).unwrap());
    }
}
