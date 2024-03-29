use super::PostgresRepository;
use auth_service::core::{
    entities::CreateUser, error::Error, repository::Repository,
};
use sqlx::{query, query_scalar};
use uuid::Uuid;

impl Repository for PostgresRepository {
    async fn exists_credential(
        &self,
        identifier: &str,
        password: &str,
    ) -> Result<bool, Error> {
        Ok(query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM users WHERE phone = $1 AND password = $2)",
            identifier,
            password,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::FailedToGetID(Box::new(e)))?
        .unwrap())
    }

    async fn get_id_by_key(
        &self,
        key: &str,
    ) -> Result<Option<String>, auth_service::core::error::Error> {
        Ok(
            query!("SELECT id::VARCHAR FROM users WHERE session_key = $1", key)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| Error::FailedToGetID(Box::new(e)))?
                .map(|r| r.id),
        )
    }

    async fn get_password_salt(
        &self,
        identifier: &str,
    ) -> Result<Option<String>, auth_service::core::error::Error> {
        Ok(query!(
            "SELECT password_salt FROM users WHERE phone = $1",
            identifier
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::FailedToGetPasswordSalt(Box::new(e)))?
        .map(|r| r.password_salt))
    }

    async fn set_key(
        &self,
        identifier: &str,
        key: &str,
    ) -> Result<(), auth_service::core::error::Error> {
        query!(
            "UPDATE users SET session_key = $1 WHERE phone = $2",
            key,
            identifier
        )
        .execute(&self.pool)
        .await
        .map_err(|e| Error::FailedToSetKey(Box::new(e)))?;
        Ok(())
    }

    async fn exists_user(&self, phone: &str) -> Result<bool, Error> {
        let exists = sqlx::query!(
            "SELECT EXISTS (SELECT 1 FROM users WHERE phone = $1)",
            phone
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::FailedToCheckExists(Box::new(e)))?;
        Ok(exists.exists.unwrap())
    }

    async fn insert_user(&self, user: &CreateUser) -> Result<String, Error> {
        let id = sqlx::query!(
            "INSERT INTO users (id, phone, password, password_salt) VALUES ($1, $2, $3, $4) RETURNING id",
            Uuid::new_v4().to_string(),
            user.identifier,
            user.password,
            user.password_salt
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::FailedToInsertUser(Box::new(e)))?
        .id;
        Ok(id)
    }

    async fn delete_key(&self, identifier: &str) -> Result<(), Error> {
        query!(
            "UPDATE users SET session_key = NULL WHERE phone = $1",
            identifier
        )
        .execute(&self.pool)
        .await
        .map_err(|e| Error::FailedToDeleteKey(Box::new(e)))?;
        Ok(())
    }
}
