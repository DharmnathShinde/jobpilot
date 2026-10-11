mod account;
mod applications;
mod companies;
mod jobs;
mod profile;

use axum::extract::{DefaultBodyLimit, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::error::{AppError, AppResult};
use crate::scheduler;
use crate::state::AppState;

/// Called by an external cron (GitHub Actions) so free hosts that sleep still run the morning job.
async fn internal_tick(State(st): State<AppState>, headers: HeaderMap) -> AppResult<Json<serde_json::Value>> {
    let expected = st.config.cron_secret.as_deref().ok_or(AppError::NotFound)?;
    let given = headers.get("x-cron-secret").and_then(|v| v.to_str().ok()).unwrap_or("");
    // constant-time-ish comparison
    if given.len() != expected.len() || given.bytes().zip(expected.bytes()).fold(0u8, |acc, (a, b)| acc | (a ^ b)) != 0 {
        return Err(AppError::Unauthorized);
    }
    // The GitHub Action allows 300s; leave room for the morning runs after the refresh.
    let catalog = scheduler::refresh_catalog(&st, std::time::Duration::from_secs(150)).await;
    let runs = scheduler::tick_once_with(&st, true).await?;
    Ok(Json(json!({ "runs": runs, "catalog": catalog })))
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(|| async { Json(json!({ "ok": true })) }))
        .route("/auth/register", post(account::register))
        .route("/auth/login", post(account::login))
        .route("/me", get(account::me).delete(account::delete_account))
        .route("/profile", get(profile::get_profile).put(profile::put_profile))
        .route("/profile/cv", post(profile::upload_cv))
        .route("/preferences", get(profile::get_preferences).put(profile::put_preferences))
        .route("/sources", get(profile::list_sources).post(profile::add_source))
        .route("/sources/:id", axum::routing::delete(profile::delete_source))
        .route("/companies", get(companies::list))
        .route("/companies/requests", get(companies::list_requests).post(companies::create_request))
        .route("/today", get(jobs::today))
        .route("/runs", post(jobs::run_now))
        .route("/jobs", get(jobs::list).post(jobs::create))
        .route("/jobs/:id", get(jobs::get))
        .route("/jobs/:id/status", post(jobs::set_status))
        .route("/jobs/:id/hr-email", axum::routing::patch(jobs::set_hr_email))
        .route("/jobs/:id/outreach-draft", post(jobs::outreach_draft))
        .route("/jobs/:id/outreach-status", post(jobs::set_outreach_status))
        .route("/jobs/:id/tailor", post(jobs::tailor_resume))
        .route("/jobs/:id/answers", get(jobs::answers))
        .route("/resumes/:id", get(jobs::get_resume))
        .route("/resumes/:id/html", get(jobs::get_resume_html))
        .route("/applications", get(applications::list).post(applications::create))
        .route("/applications/:id", get(applications::get).patch(applications::update))
        .route("/internal/tick", post(internal_tick))
        .layer(DefaultBodyLimit::max(6 * 1024 * 1024))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
