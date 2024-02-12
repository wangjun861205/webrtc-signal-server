use super::PostgresRepository;
use crate::core::error::{Error, Result};
use crate::core::repository::{
    ChatMessage, Friend, FriendRequest, FriendRequestStatus, InsertChatMessage,
    Repository, User, UserType,
};
use sqlx::{query, query_scalar, types::Uuid};

impl Repository for PostgresRepository {
    async fn add_friend_request(&self, from: &str, to: &str) -> Result<String> {
        Ok(query!(
            r#"INSERT INTO friend_requests (id, "from", "to", status) VALUES ($1, $2, $3, 'Pending') 
	    ON CONFLICT ("from", "to") DO UPDATE SET status = 'Pending'
	    RETURNING id
	    "#,
            Uuid::new_v4().to_string(),
            from,
            to,
        )
	.fetch_one(&self.pool)
	.await
	.map_err(|e| Error::wrap("failed to insert friend request".into(), 500, e))?.id)
    }
    async fn get_friend_request(&self, id: &str) -> Result<FriendRequest> {
        if let Some(record) = query!(
		r#"SELECT id, "from", "to", status FROM friend_requests WHERE id = $1"#,
		id,
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
		return Ok(FriendRequest{id: record.id, from: record.from, to: record.to, status});
	}
        Err(Error::new(
            format!("friend request not found(id: {})", id),
            404,
        ))
    }
    async fn accept_friend_request(&self, id: &str) -> Result<()> {
        query!(
            "UPDATE friend_requests SET status = 'Accepted' WHERE id = $1",
            id,
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
            id,
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
		to,
	)
	.fetch_all(&self.pool)
	.await
	.map_err(|e| Error::wrap("failed to get friend requests".into(), 500, e))?
	.into_iter()
	.map(|record| {
		FriendRequest { id: record.id, from: record.from, to: record.to, status: FriendRequestStatus::Pending }
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
                    WHEN u.fid = $1 THEN u.tid
                    ELSE u.fid
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
            user_id,
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
		user_id,
		friend_id,
	)
	.fetch_one(&self.pool)
	.await
	.map_err(|e| Error::wrap("failed to get friend requests".into(), 500, e))?.exists.unwrap())
    }

    async fn search_user(
        &self,
        user_id: &str,
        phone: &str,
    ) -> Result<Option<crate::core::repository::User>> {
        Ok(query!(r#"
        SELECT
            u.id AS id,
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
        user_id,
        phone)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::wrap("failed to search user".into(), 500, e))?
        .map(|record| {
            User {
                id: record.id,
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

    async fn latest_chat_messages_with_others(
        &self,
        self_id: &str,
        other_id: &str,
        limit: i64,
    ) -> Result<Vec<crate::core::repository::ChatMessage>> {
        Ok(query!(
            r#"
                SELECT 
                    id::VARCHAR AS id,
                    "from"::VARCHAR AS "from",
                    "to"::VARCHAR AS "to",
                    content AS content,
                    CASE
                        WHEN "from" = $1 THEN true
                        ELSE false
                    END AS is_out
                FROM 
                    messages
                WHERE 
                    ("from" = $1 AND "to" = $2) OR ("from" = $2 AND "to" = $1)
                ORDER BY sent_at DESC
                LIMIT $3
            "#,
            Uuid::parse_str(self_id).unwrap(),
            Uuid::parse_str(other_id).unwrap(),
            limit,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            Error::wrap("failed to get latest messages".into(), 500, e)
        })?
        .into_iter()
        .map(|record| ChatMessage {
            id: record.id.unwrap(),
            from: record.from.unwrap(),
            to: record.to.unwrap(),
            content: record.content,
            is_out: record.is_out.unwrap(),
        })
        .collect())
    }

    async fn insert_chat_message(
        &self,
        create: &InsertChatMessage,
    ) -> Result<String> {
        Ok(query_scalar!(
            r#"INSERT INTO messages ("from", "to", content) VALUES ($1, $2, $3) RETURNING id::VARCHAR"#,
            Uuid::parse_str(&create.from).unwrap(),
            Uuid::parse_str(&create.to).unwrap(),
            &create.content,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::wrap("failed to insert message".into(), 500, e))?
        .unwrap())
    }

    async fn get_avatar(&self, self_id: &str) -> Result<Option<String>> {
        query_scalar!(
            "
        SELECT avatar FROM users WHERE id = $1",
            self_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::wrap("failed to get avatar".into(), 500, e))
    }

    async fn update_avatar(
        &self,
        self_id: &str,
        upload_id: &str,
    ) -> Result<()> {
        query!(
            "UPDATE users SET avatar = $1 WHERE id = $2",
            upload_id,
            self_id,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| Error::wrap("failed to update avatar".into(), 500, e))?;
        Ok(())
    }
}
