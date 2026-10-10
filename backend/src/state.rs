use std::time::Duration;

use sqlx::PgPool;

use crate::config::Config;
use crate::llm::Llm;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Config,
    pub http: reqwest::Client,
    pub llm: Option<Llm>,
}

impl AppState {
    pub fn new(pool: PgPool, config: Config) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("JobPilot/0.1 (+job search assistant)")
            .build()?;
        let llm = config.anthropic_api_key.clone().map(|key| Llm::new(http.clone(), key, config.llm_model.clone()));
        Ok(Self { pool, config, http, llm })
    }
}
