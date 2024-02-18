use crate::core::{message::Message, repository::AddrStore};
use actix::{Actor, Addr, Recipient};

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub(crate) struct AddrMap {
    pub(crate) map: Arc<RwLock<HashMap<String, Recipient<Message>>>>,
}

impl AddrMap {
    pub(crate) fn new() -> Self {
        Self {
            map: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl AddrStore for AddrMap {
    async fn add_addr(
        &self,
        id: &str,
        addr: Recipient<Message>,
    ) -> crate::core::error::Result<()> {
        self.map.write().await.insert(id.to_owned(), addr);
        Ok(())
    }

    async fn get_addr(
        &self,
        id: &str,
    ) -> crate::core::error::Result<Option<Recipient<Message>>> {
        Ok(self.map.read().await.get(id).cloned())
    }

    async fn remove_addr(&self, id: &str) -> crate::core::error::Result<()> {
        self.map.write().await.remove(id);
        Ok(())
    }
}
