use crate::core::{error::Error, ws::WS};
use actix::{Actor, Addr};

pub trait AddrKeeper {
    type Actor: Actor;
    async fn login(&self, id: &str) -> Result<(), Error>;
    async fn logout(&self, id: &str) -> Result<(), Error>;
    async fn get_addr(&self, id: &str) -> Option<Addr<Self::Actor>>;
}
