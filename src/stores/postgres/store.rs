use super::PostgresRepository;
use crate::core::error::{Error, Result};
use crate::core::repository::{
    ChatMessage, Friend, FriendRequest, FriendRequestStatus, InsertChatMessage,
    Repository, Session, User, UserType,
};
use sqlx::{query, query_as, query_scalar, types::Uuid};

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
                f.id,
                f.phone,
                COUNT(m.id) AS unread_count
            FROM
                (SELECT 
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
            ) AS f
                LEFT JOIN messages AS m ON f.id = m."from" AND m."to" = $1 AND has_read = false
            GROUP BY  f.id, f.phone
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
            unread_count: record.unread_count.unwrap_or(0),
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

    async fn chat_message_history(
        &self,
        self_id: &str,
        other_id: &str,
        limit: i64,
        before: Option<&str>,
    ) -> Result<Vec<crate::core::repository::ChatMessage>> {
        Ok(query!(
            r#"
                WITH 
                    unread_ids AS (
                        UPDATE messages SET has_read = true WHERE (("from" = $1 AND "to" = $2) OR ("from" = $2 AND "to" = $1)) AND has_read = false RETURNING id
                    ),
                    last_message AS (
                        SELECT id FROM messages WHERE (("from" = $1 AND "to" = $2) OR ("from" = $2 AND "to" = $1)) ORDER BY id DESC LIMIT 1
                    )
                SELECT 
                    id,
                    "from",
                    "to",
                    content,
                    sent_at
                FROM 
                    (SELECT 
                        id,
                        CASE WHEN "from" != $1 THEN "from" ELSE '' END AS "from",
                        "to",
                        content,
                        sent_at
                    FROM 
                        messages
                    WHERE 
                        (
                            ("from" = $1 AND "to" = $2) 
                            OR ("from" = $2 AND "to" = $1)
                        ) 
                        AND id < CASE WHEN $4 = null THEN $4 ELSE (SELECT id FROM last_message) END
                    ORDER BY id DESC
                    LIMIT GREATEST($3, (SELECT COUNT(*) FROM unread_ids LIMIT 1))) AS m
                ORDER BY m.id ASC;
            "#,
            self_id,
            other_id,
            limit,
            before,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            Error::wrap("failed to get latest messages".into(), 500, e)
        })?
        .into_iter()
        .map(|record| ChatMessage {
            id: record.id,
            from: record.from.unwrap(),
            to: record.to,
            content: record.content,
            sent_at: record.sent_at,
        })
        .collect())
    }

    async fn insert_chat_message(
        &self,
        create: &InsertChatMessage,
    ) -> Result<String> {
        query_scalar!(
            r#"INSERT INTO messages (id, "from", "to", content) VALUES (UUID_GENERATE_V1()::VARCHAR, $1, $2, $3) RETURNING id"#,
            &create.from,
            &create.to,
            &create.content,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::wrap("failed to insert message".into(), 500, e))
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

    async fn sessions(
        &self,
        user_id: &str,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<crate::core::repository::Session>> {
        Ok(query!(r#"
            SELECT DISTINCT
                peer_ids.peer_id AS peer_id,
                users.phone AS peer_phone,
                SUM(CASE WHEN messages.has_read = false THEN 1 ELSE 0 END) OVER (PARTITION BY peer_ids.peer_id ORDER BY messages.id) AS unread_count,
                LAST_VALUE(messages.content) OVER (PARTITION BY peer_ids.peer_id ORDER BY messages.id) AS latest_content
            FROM (SELECT DISTINCT
                    CASE WHEN "from" = $1 THEN "to" ELSE "from" END AS peer_id
                FROM messages
                WHERE "from" = $1 OR "to" = $1) AS peer_ids
                JOIN users ON peer_ids.peer_id = users.id
                JOIN messages ON peer_ids.peer_id = messages."from" OR peer_ids.peer_id = messages."to"
            LIMIT $2 
            OFFSET $3
            "#, user_id, limit, offset).fetch_all(&self.pool)
            .await
            .map_err(|e| Error::wrap("failed to get sessions".into(), 500, e))?
            .into_iter()
            .map(|record| Session {
                peer_id: record.peer_id.unwrap(),
                peer_phone: record.peer_phone,
                unread_count: record.unread_count.unwrap(),
                latest_content: record.latest_content,
            })
            .collect())
    }
}
