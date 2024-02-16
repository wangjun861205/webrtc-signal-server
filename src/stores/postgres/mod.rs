pub(crate) mod auth;
pub(crate) mod store;
pub(crate) mod upload;

use futures_util::lock::Mutex;
use snowflake::SnowflakeIdGenerator;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub(crate) struct PostgresRepository {
    pool: PgPool,
    id_generator: Arc<Mutex<SnowflakeIdGenerator>>,
}

impl PostgresRepository {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self {
            pool,
            id_generator: Arc::new(Mutex::new(SnowflakeIdGenerator::new(1, 1))),
        }
    }
}
