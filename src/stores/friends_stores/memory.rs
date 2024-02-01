use crate::core::{
    error::Error,
    store::{Friend, FriendRequest, FriendRequestStatus, FriendsStore},
};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct MemoryFriendsStore {
    requests: Arc<RwLock<Vec<FriendRequest>>>,
}

impl MemoryFriendsStore {
    pub fn new() -> Self {
        Self {
            requests: Arc::new(RwLock::new(vec![])),
        }
    }
}

impl FriendsStore for MemoryFriendsStore {
    async fn get_friend_request(
        &self,
        id: &str,
    ) -> crate::core::error::Result<FriendRequest> {
        self.requests
            .read()
            .await
            .iter()
            .find(|r| r.id == id)
            .map(|r| r.clone())
            .ok_or(Error::new("not found request".into(), 404))
    }
    async fn accept_friend_request(
        &self,
        id: &str,
    ) -> crate::core::error::Result<()> {
        if let Some(r) =
            self.requests.write().await.iter_mut().find(|r| r.id == id)
        {
            r.status = FriendRequestStatus::Accepted;
        }
        Ok(())
    }
    async fn add_friend_request(
        &self,
        from: &str,
        to: &str,
    ) -> crate::core::error::Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        self.requests.write().await.push(FriendRequest {
            id: id.clone(),
            from: from.to_owned(),
            to: to.to_owned(),
            status: FriendRequestStatus::Pending,
        });
        Ok(id)
    }
    async fn friends(
        &self,
        user_id: &str,
        limit: i64,
        offset: i64,
    ) -> crate::core::error::Result<Vec<Friend>> {
        // Ok(self
        //     .requests
        //     .read()
        //     .await
        //     .iter()
        //     .filter(|r| {
        //         r.status == FriendRequestStatus::Accepted
        //             && (r.from == user_id || r.to == user_id)
        //     })
        //     .map(|r| {
        //         if r.from == user_id {
        //             r.to.clone()
        //         } else {
        //             r.from.clone()
        //         }
        //     })
        //     .collect())
        unimplemented!()
    }
    async fn is_friend(
        &self,
        user_id: &str,
        friend_id: &str,
    ) -> crate::core::error::Result<bool> {
        Ok(self.requests.read().await.iter().any(|r| {
            r.status == FriendRequestStatus::Accepted
                && (r.from == user_id && r.to == friend_id
                    || r.from == friend_id && r.to == user_id)
        }))
    }
    async fn pending_friend_requests(
        &self,
        to: &str,
    ) -> crate::core::error::Result<Vec<FriendRequest>> {
        Ok(self
            .requests
            .read()
            .await
            .iter()
            .filter(|r| r.to == to && r.status == FriendRequestStatus::Pending)
            .cloned()
            .collect())
    }
    async fn reject_friend_request(
        &self,
        id: &str,
    ) -> crate::core::error::Result<()> {
        if let Some(r) =
            self.requests.write().await.iter_mut().find(|r| r.id == id)
        {
            r.status = FriendRequestStatus::Rejected;
        }
        Ok(())
    }

    async fn search_user(
        &self,
        phone: &str,
        user_id: &str,
    ) -> crate::core::error::Result<Option<crate::core::store::User>> {
        unimplemented!()
    }
}
