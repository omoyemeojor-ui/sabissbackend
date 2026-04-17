use std::time::Duration;

use anyhow::{Context, Result};
use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::config::environment::Environment;

pub type DbPool = PgPool;

pub async fn create_pool(env: &Environment) -> Result<DbPool> {
    PgPoolOptions::new()
        .max_connections(env.db_max_connections)
        .acquire_timeout(Duration::from_millis(env.db_acquire_timeout_ms))
        .connect(&env.database_url)
        .await
        .context("failed to connect to postgres")
}
