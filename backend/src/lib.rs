pub mod aggregators;
pub mod auth;
pub mod catalog;
pub mod config;
pub mod contacts;
pub mod cv;
pub mod discovery;
pub mod error;
pub mod llm;
pub mod matching;
pub mod models;
pub mod routes;
pub mod scheduler;
pub mod skills;
pub mod sources;
pub mod state;
pub mod tailor;

use sqlx::postgres::PgPoolOptions;

pub async fn connect(database_url: &str) -> anyhow::Result<sqlx::PgPool> {
    let pool = PgPoolOptions::new().max_connections(10).connect(database_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    catalog::seed(&pool).await?;
    Ok(pool)
}
