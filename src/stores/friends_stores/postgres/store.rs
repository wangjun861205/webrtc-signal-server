use crate::core::error::{Error, Result};
use crate::core::store::{FriendRequest, FriendRequestStatus, FriendsStore};
use sqlx::{query, types::Uuid, PgPool};

#[derive(Debug, Clone)]
pub(crate) struct PostgresFriendsStore {
    pool: PgPool,
}

impl PostgresFriendsStore {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl FriendsStore for PostgresFriendsStore {
    async fn add_friend_request(&self, from: &str, to: &str) -> Result<String> {
        Ok(query!(
            r#"INSERT INTO friend_requests ("from", "to", status) VALUES ($1, $2, 'Pending') 
	    ON CONFLICT ("from", "to") DO UPDATE SET status = 'Pending'
	    RETURNING id::VARCHAR
	    "#,
            Uuid::parse_str(from).map_err(|e| Error::wrap(
                "invalid from id".into(),
                400,
                e
            ))?,
            Uuid::parse_str(to).map_err(|e| Error::wrap(
                "invalid from id".into(),
                400,
                e
            ))?,
        )
	.fetch_one(&self.pool)
	.await
	.map_err(|e| Error::wrap("failed to insert friend request".into(), 500, e))?.id.unwrap())
    }
    async fn get_friend_request(&self, id: &str) -> Result<FriendRequest> {
        if let Some(record) = query!(
		r#"SELECT id::VARCHAR, "from"::VARCHAR, "to"::VARCHAR, status FROM friend_requests WHERE id = $1"#,
		Uuid::parse_str(id).map_err(|e| Error::wrap(format!("invalid id(id: {})", id), 400, e))?,
	)
	.fetch_optional(&self.pool)
	.await
	.map_err(|e| Error::wrap(format!("failed to get friend request(id: {})", id), 500, e))? {
		let status = match record.status.as_ref() {
			"Pending" => FriendRequestStatus::Pending,
			"Accepted" => FriendRequestStatus::Accepted,
			"Rejected" => FriendRequestStatus::Rejected,
			_ => return Err(Error::new(format!("invalid request status: {}", record.status), 500))
		};
		return Ok(FriendRequest{id: record.id.unwrap(), from: record.from.unwrap(), to: record.to.unwrap(), status});
	}
        Err(Error::new(
            format!("friend request not found(id: {})", id),
            404,
        ))
    }
    async fn accept_friend_request(&self, id: &str) -> Result<()> {
        query!(
            "UPDATE friend_requests SET status = 'Accepted' WHERE id = $1",
            Uuid::parse_str(id).map_err(|e| Error::wrap(
                format!("invalid id(id: {})", id),
                400,
                e
            ))?
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            Error::wrap("failed to update friend request".into(), 500, e)
        })?;
        Ok(())
    }
    async fn reject_friend_request(&self, id: &str) -> Result<()> {
        query!(
            "UPDATE friend_requests SET status = 'Rejected' WHERE id = $1",
            Uuid::parse_str(id).map_err(|e| Error::wrap(
                format!("invalid id(id: {})", id),
                400,
                e
            ))?
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            Error::wrap("failed to update friend request".into(), 500, e)
        })?;
        Ok(())
    }
    async fn pending_friend_requests(
        &self,
        to: &str,
    ) -> Result<Vec<FriendRequest>> {
        Ok(query!(
		r#"SELECT id::VARCHAR, "from"::VARCHAR, "to"::VARCHAR FROM friend_requests WHERE status = 'Pending' AND "to" = $1"#,
		Uuid::parse_str(to).map_err(|e| Error::wrap(format!("invalid to id(id: {}", to), 400, e))?
	)
	.fetch_all(&self.pool)
	.await
	.map_err(|e| Error::wrap("failed to get friend requests".into(), 500, e))?
	.into_iter()
	.map(|record| {
		FriendRequest { id: record.id.unwrap(), from: record.from.unwrap(), to: record.to.unwrap(), status: FriendRequestStatus::Pending }
	})
	.collect())
    }
    async fn friends(&self, user_id: &str) -> Result<Vec<String>> {
        Ok(query!(
		r#"SELECT id::VARCHAR FROM friend_requests WHERE status = 'Accepted' AND ("from" = $1 OR "to" = $1)"#,
		Uuid::parse_str(user_id).map_err(|e| Error::wrap(format!("invalid to id(id: {}", user_id), 400, e))?
	)
	.fetch_all(&self.pool)
	.await
	.map_err(|e| Error::wrap("failed to get friend requests".into(), 500, e))?
	.into_iter()
	.map(|record| {
		record.id.unwrap()
	})
	.collect())
    }
    async fn is_friend(&self, user_id: &str, friend_id: &str) -> Result<bool> {
        Ok(query!(
		r#"SELECT EXISTS(SELECT 1 FROM friend_requests WHERE status = 'Accepted' AND ("from" = $1 AND "to" = $2) OR ("from" = $2 AND "to" = $1))"#,
		Uuid::parse_str(user_id).map_err(|e| Error::wrap(format!("invalid to id(id: {}", user_id), 400, e))?,
		Uuid::parse_str(friend_id).map_err(|e| Error::wrap(format!("invalid to id(id: {}", friend_id), 400, e))?
	)
	.fetch_one(&self.pool)
	.await
	.map_err(|e| Error::wrap("failed to get friend requests".into(), 500, e))?.exists.unwrap())
    }
}
