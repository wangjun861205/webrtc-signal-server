use crate::core::{error::Result, message::Message};
use crate::ws::actor::WS;
use actix::{Actor, Addr, Recipient, WeakAddr};
use chrono::{DateTime, Utc};
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
    pub phone: String,
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
    pub avatar: Option<String>,
    pub typ: UserType,
}

#[derive(Clone, Serialize)]
pub(crate) struct Friend {
    pub id: String,
    pub phone: String,
    pub avatar: Option<String>,
}

#[derive(Clone, Serialize)]
pub(crate) struct Session {
    pub(crate) peer_id: String,
    pub(crate) peer_phone: String,
    pub(crate) unread_count: i64,
    pub(crate) latest_content: Option<String>,
}

#[derive(Clone, Serialize)]
pub(crate) struct ChatMessage {
    pub(crate) id: String,
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) content: String,
    pub(crate) sent_at: DateTime<chrono::Utc>,
    pub(crate) has_read: bool,
    pub(crate) mime_type: String,
}

#[derive(Clone, Serialize)]
pub struct InsertChatMessage {
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) mime_type: String,
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
    async fn friends(&self, user_id: &str) -> Result<Vec<Friend>>;
    async fn sessions(&self, user_id: &str) -> Result<Vec<Session>>;
    async fn is_friend(&self, user_id: &str, friend_id: &str) -> Result<bool>;
    async fn search_user(
        &self,
        user_id: &str,
        phone: &str,
    ) -> Result<Option<User>>;

    async fn chat_message_history(
        &self,
        self_id: &str,
        other_id: &str,
        limit: i64,
        before: Option<&str>,
    ) -> Result<Vec<ChatMessage>>;

    async fn insert_chat_message(
        &self,
        create: &InsertChatMessage,
    ) -> Result<ChatMessage>;

    async fn update_avatar(&self, self_id: &str, upload_id: &str)
        -> Result<()>;
    async fn get_avatar(&self, self_id: &str) -> Result<Option<String>>;
    async fn mark_as_read(&self, msg_id: &str) -> Result<()>;
    async fn get_friend(&self, id: &str) -> Result<Friend>;
    async fn get_phone(&self, id: &str) -> Result<String>;
}

pub trait AddrStore {
    async fn add_addr(&self, id: &str, addr: Recipient<Message>) -> Result<()>;
    async fn remove_addr(&self, id: &str) -> Result<()>;
    async fn get_addr(&self, id: &str) -> Result<Option<Recipient<Message>>>;
}
