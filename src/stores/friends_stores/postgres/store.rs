use crate::core::error::{Error, Result};
use crate::core::store::{
    Friend, FriendRequest, FriendRequestStatus, FriendsStore, User, UserType,
};
use sqlx::query_as;
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
    async fn friends(
        &self,
        user_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Friend>> {
        Ok(query!(
            r#"
            SELECT 
                CASE 
                    WHEN u.fid = $1 THEN u.tid::VARCHAR
                    ELSE u.fid::VARCHAR
                END AS id,
                CASE 
                    WHEN u.fid = $1 THEN u.tphone
                    ELSE u.fphone
                END AS phone
            FROM (
                SELECT
                    f.id AS fid,
                    f.phone AS fphone,
                    t.id AS tid,
                    t.phone AS tphone
                FROM
                    users AS f
                    JOIN friend_requests AS fr ON fr."from" = f.id
                    JOIN users AS t ON fr."to" = t.id
                WHERE fr.status = 'Accepted' AND (fr."from" = $1 OR fr."to" = $1)
                LIMIT $2
                OFFSET $3
            ) AS u
            "#,
            Uuid::parse_str(user_id).map_err(|e| Error::wrap(
                format!("invalid to id(id: {}", user_id),
                400,
                e
            ))?,
            limit,
            offset
        )
       .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            Error::wrap("failed to get friend requests".into(), 500, e)
        })?
        .into_iter()
        .map(|record| Friend {
            id: record.id.unwrap(),
            phone: record.phone.unwrap(),
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

    async fn search_user(
        &self,
        user_id: &str,
        phone: &str,
    ) -> Result<Option<crate::core::store::User>> {
        Ok(query!(r#"
        SELECT
            u.id::VARCHAR AS id,
            u.phone AS phone,
            CASE 
                WHEN f.status = 'Pending' THEN 'Requested'
                WHEN t.status = 'Pending' THEN 'Requesting'
                WHEN f.status = 'Accepted' OR t.status = 'Accepted' THEN 'Friend'
                WHEN u.id = $1 THEN 'Myself'
                ELSE 'Stranger'
            END AS typ
        FROM users AS u
        LEFT JOIN friend_requests AS f ON u.id = f."from" AND f."to" = $1
        LEFT JOIN friend_requests AS t ON u.id = t."to" AND t."from" = $1
        WHERE u.phone = $2"#, 
        Uuid::parse_str(user_id).map_err(|e| Error::wrap("invalid user id".into(), 400, e))?,
        phone)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::wrap("failed to search user".into(), 500, e))?
        .map(|record| {
            User {
                id: record.id.unwrap(),
                phone: record.phone,
                typ: match record.typ.unwrap().as_ref() {
                    "Requesting" => UserType::Requesting,
                    "Requested" => UserType::Requested,
                    "Friend" => UserType::Friend,
                    "Myself" => UserType::Myself,
                    "Stranger" => UserType::Stranger,
                    _ => unreachable!(),
                }
            }
        }))
    }
}
