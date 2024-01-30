use anyhow::Error;
use auth_service::core::{
    entities::{CreateUser, User},
    repository::Repository,
};
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub(crate) struct PostgresRepository {
    pool: PgPool,
}

impl PostgresRepository {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl Repository for PostgresRepository {
    async fn exists_user(&self, phone: &str) -> Result<bool, Error> {
        let exists = sqlx::query!(
            "SELECT EXISTS (SELECT 1 FROM users WHERE phone = $1)",
            phone
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(exists.exists.unwrap())
    }

    async fn fetch_user(&self, phone: &str) -> Result<Option<User>, Error> {
        Ok(sqlx::query!(
            "SELECT id::VARCHAR, phone, password, password_salt, created_at, updated_at FROM users WHERE phone = $1",
            phone
        )
        .fetch_optional(&self.pool)
        .await?
        .map(|u| User { id: u.id.unwrap(), phone: u.phone, password: u.password, password_salt: u.password_salt, created_at: u.created_at, updated_at: u.updated_at}))
    }

    async fn insert_user(&self, user: &CreateUser) -> Result<String, Error> {
        let id = sqlx::query!(
            "INSERT INTO users (phone, password, password_salt) VALUES ($1, $2, $3) RETURNING id::VARCHAR",
            user.phone,
            user.password,
            user.password_salt
        )
        .fetch_one(&self.pool)
        .await?
        .id
        .unwrap();
        Ok(id)
    }
}
