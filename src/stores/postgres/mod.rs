pub(crate) mod auth;
pub(crate) mod upload;
pub(crate) mod store;

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
