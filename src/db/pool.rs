//! PostgreSQL connection pool.

use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

pub type DbPool = sqlx::PgPool;

pub async fn create_pool(database_url: &str) -> Result<DbPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await
}
