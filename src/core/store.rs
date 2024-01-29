use crate::core::error::Result;
use crate::ws::actor::WS;
use actix::Addr;
use auth_service::core::{
    hasher::Hasher, repository::Repository, token_manager::TokenManager,
};
use serde::Serialize;

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

pub trait FriendsStore {
    async fn add_friend_request(&self, from: &str, to: &str) -> Result<String>;
    async fn get_friend_request(&self, id: &str) -> Result<FriendRequest>;
    async fn accept_friend_request(&self, id: &str) -> Result<()>;
    async fn reject_friend_request(&self, id: &str) -> Result<()>;
    async fn pending_friend_requests(
        &self,
        to: &str,
    ) -> Result<Vec<FriendRequest>>;
    async fn friends(&self, user_id: &str) -> Result<Vec<String>>;
    async fn is_friend(&self, user_id: &str, friend_id: &str) -> Result<bool>;
}

pub trait AddrStore<R, H, T, F>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
    F: FriendsStore + Clone + Unpin + 'static,
{
    async fn add_addr(
        &self,
        id: String,
        addr: Addr<WS<R, H, T, F>>,
    ) -> Result<()>;
    async fn remove_addr(&self, id: String) -> Result<()>;
    async fn get_addr(&self, id: String) -> Result<Addr<WS<R, H, T, F>>>;
    async fn get_all_addrs(&self) -> Result<Vec<Addr<WS<R, H, T, F>>>>;
    async fn get_all_ids(&self) -> Result<Vec<String>>;
}