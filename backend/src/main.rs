use jobpilot::config::Config;
use jobpilot::state::AppState;
use jobpilot::{connect, routes, scheduler};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,tower_http=info,sqlx=warn")))
        .init();

    let config = Config::from_env()?;
    let pool = connect(&config.database_url).await?;
    let state = AppState::new(pool, config.clone())?;

    if config.anthropic_api_key.is_none() {
        tracing::warn!("ANTHROPIC_API_KEY not set: CV parsing uses rule-based extraction");
    }
    if config.scheduler_enabled {
        scheduler::spawn(state.clone());
    }

    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    tracing::info!("JobPilot API listening on {}", config.bind_addr);
    axum::serve(listener, routes::router(state)).await?;
    Ok(())
}
