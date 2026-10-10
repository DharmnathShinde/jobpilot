use anyhow::Context;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub bind_addr: String,
    /// Optional. Without it, CV parsing and drafting fall back to deterministic rules.
    pub anthropic_api_key: Option<String>,
    pub llm_model: String,
    /// Turn the per-minute scheduler loop off (tests, one-off CLI runs).
    pub scheduler_enabled: bool,
    /// Refresh the shared company catalog on each tick (off in tests: it calls ~600 job boards).
    pub catalog_refresh: bool,
    /// Shared secret for POST /internal/tick (an external cron wakes sleeping free hosts).
    pub cron_secret: Option<String>,
    /// Set to false once your own account exists so nobody else can sign up.
    pub allow_signups: bool,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let _ = dotenvy::dotenv();
        Ok(Self {
            database_url: std::env::var("DATABASE_URL").context("DATABASE_URL is not set")?,
            jwt_secret: std::env::var("JWT_SECRET").context("JWT_SECRET is not set")?,
            // Hosts like Render pass PORT; BIND_ADDR wins if set.
            bind_addr: std::env::var("BIND_ADDR")
                .ok()
                .or_else(|| std::env::var("PORT").ok().map(|p| format!("0.0.0.0:{p}")))
                .unwrap_or_else(|| "0.0.0.0:8080".into()),
            anthropic_api_key: std::env::var("ANTHROPIC_API_KEY").ok().filter(|k| !k.is_empty()),
            llm_model: std::env::var("LLM_MODEL").unwrap_or_else(|_| "claude-haiku-5-5".into()),
            scheduler_enabled: std::env::var("SCHEDULER_ENABLED").map(|v| v != "false").unwrap_or(true),
            catalog_refresh: std::env::var("CATALOG_REFRESH").map(|v| v != "false").unwrap_or(true),
            cron_secret: std::env::var("CRON_SECRET").ok().filter(|s| s.len() >= 16),
            allow_signups: std::env::var("ALLOW_SIGNUPS").map(|v| v != "false").unwrap_or(true),
        })
    }
}
