use crate::core::error::Result;
use crate::ws::actor::WS;
use actix::Addr;
use auth_service::core::{
    hasher::Hasher, repository::Repository as AuthRepository,
    token_manager::TokenManager,
};
use serde::Serialize;

use super::notifier::Notifier;

#[derive(Clone, PartialEq, Serialize)]
pub enum FriendRequestStatus {
    Pending,
    Accepted,
    Rejected,
}

#[derive(Clone, Serialize)]
pub struct FriendRequest {
    pub id: String,
    pub from: String,
    pub to: String,
    pub status: FriendRequestStatus,
}

#[derive(Clone, Serialize)]
pub(crate) enum UserType {
    Myself,
    Friend,
    Requesting,
    Requested,
    Stranger,
}

#[derive(Clone, Serialize)]
pub(crate) struct User {
    pub id: String,
    pub phone: String,
    pub typ: UserType,
}

#[derive(Clone, Serialize)]
pub(crate) struct Friend {
    pub id: String,
    pub phone: String,
}

#[derive(Clone, Serialize)]
pub(crate) struct ChatMessage {
    pub(crate) id: String,
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) content: String,
    pub(crate) is_out: bool,
}

#[derive(Clone, Serialize)]
pub struct InsertChatMessage {
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) content: String,
}

pub trait Repository {
    async fn add_friend_request(&self, from: &str, to: &str) -> Result<String>;
    async fn get_friend_request(&self, id: &str) -> Result<FriendRequest>;
    async fn accept_friend_request(&self, id: &str) -> Result<()>;
    async fn reject_friend_request(&self, id: &str) -> Result<()>;
    async fn pending_friend_requests(
        &self,
        to: &str,
    ) -> Result<Vec<FriendRequest>>;
    async fn friends(
        &self,
        user_id: &str,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<Friend>>;
    async fn is_friend(&self, user_id: &str, friend_id: &str) -> Result<bool>;
    async fn search_user(
        &self,
        user_id: &str,
        phone: &str,
    ) -> Result<Option<User>>;

    async fn latest_chat_messages_with_others(
        &self,
        self_id: &str,
        other_id: &str,
        limit: i64,
    ) -> Result<Vec<ChatMessage>>;

    async fn insert_chat_message(
        &self,
        create: &InsertChatMessage,
    ) -> Result<String>;

    async fn update_avatar(&self, self_id: &str, upload_id: &str)
        -> Result<()>;
    async fn get_avatar(&self, self_id: &str) -> Result<Option<String>>;
}

pub trait AddrStore<F, N>
where
    F: Repository + Clone + Unpin + 'static,
    N: Notifier + Clone + Unpin + 'static,
{
    async fn add_addr(&self, id: String, addr: Addr<WS<F, N>>) -> Result<()>;
    async fn remove_addr(&self, id: String) -> Result<()>;
    async fn get_addr(&self, id: String) -> Result<Addr<WS<F, N>>>;
    async fn get_all_addrs(&self) -> Result<Vec<Addr<WS<F, N>>>>;
    async fn get_all_ids(&self) -> Result<Vec<String>>;
}
