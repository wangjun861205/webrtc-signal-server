use crate::core::error::Result;
use serde::Serialize;

pub trait Notifier {
    async fn update_token(&self, uid: &str, token: &str) -> Result<()>;
    async fn get_token(&self, uid: &str) -> Result<Option<String>>;
    async fn send_notification<T>(
        &mut self,
        to: &str,
        title: &str,
        body: &str,
        data: T,
    ) -> Result<()>
    where
        T: Serialize;
}
