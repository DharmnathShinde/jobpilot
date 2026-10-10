//! The shared catalog of career pages, and requests to add companies to it.

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::catalog;
use crate::error::{AppError, AppResult};
use crate::models::{Company, CompanyRequest};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ListQuery {
    pub q: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Catalog companies, alphabetical, optionally filtered by name. Returns `{ total, companies }`.
pub async fn list(State(st): State<AppState>, AuthUser(_): AuthUser, Query(q): Query<ListQuery>) -> AppResult<Json<Value>> {
    let search = q.q.unwrap_or_default();
    let pattern = format!("%{}%", search.trim().replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
    let filter = "active AND (name ILIKE $1 OR board ILIKE $1)";
    let (total,): (i64,) = sqlx::query_as(&format!("SELECT count(*) FROM companies WHERE {filter}")).bind(&pattern).fetch_one(&st.pool).await?;
    let companies = sqlx::query_as::<_, Company>(&format!(
        "SELECT id, name, kind, board, job_count, last_checked_at FROM companies WHERE {filter} ORDER BY lower(name) LIMIT $2 OFFSET $3"
    ))
    .bind(&pattern)
    .bind(q.limit.unwrap_or(50).clamp(1, 200))
    .bind(q.offset.unwrap_or(0).max(0))
    .fetch_all(&st.pool)
    .await?;
    Ok(Json(json!({ "total": total, "companies": companies })))
}

const REQUEST_COLS: &str = "SELECT r.id, r.query, r.status, c.name AS company_name, r.created_at, r.resolved_at
    FROM company_requests r LEFT JOIN companies c ON c.id = r.company_id";

pub async fn list_requests(State(st): State<AppState>, AuthUser(uid): AuthUser) -> AppResult<Json<Vec<CompanyRequest>>> {
    let rows = sqlx::query_as::<_, CompanyRequest>(&format!("{REQUEST_COLS} WHERE r.user_id = $1 ORDER BY r.created_at DESC LIMIT 50"))
        .bind(uid)
        .fetch_all(&st.pool)
        .await?;
    Ok(Json(rows))
}

#[derive(Deserialize)]
pub struct NewRequest {
    /// A company name or a careers link.
    pub query: String,
}

/// "Add more career pages": already in the catalog → added now; otherwise the cron looks for it.
pub async fn create_request(State(st): State<AppState>, AuthUser(uid): AuthUser, Json(r): Json<NewRequest>) -> AppResult<Json<CompanyRequest>> {
    let query = r.query.trim();
    if query.chars().count() < 2 || query.chars().count() > 200 {
        return Err(AppError::BadRequest("enter a company name or its careers link".into()));
    }
    let pending: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM company_requests WHERE user_id = $1 AND lower(query) = lower($2) AND status = 'pending')",
    )
    .bind(uid)
    .bind(query)
    .fetch_one(&st.pool)
    .await?;
    if pending {
        return Err(AppError::Conflict("you've already asked for this company".into()));
    }
    let existing = catalog::find_in_catalog(&st.pool, query).await?;
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO company_requests (id, user_id, query, status, company_id, resolved_at)
         VALUES ($1, $2, $3, CASE WHEN $4::uuid IS NULL THEN 'pending' ELSE 'added' END, $4, CASE WHEN $4::uuid IS NULL THEN NULL ELSE now() END)",
    )
    .bind(id)
    .bind(uid)
    .bind(query)
    .bind(existing)
    .execute(&st.pool)
    .await?;
    let row = sqlx::query_as::<_, CompanyRequest>(&format!("{REQUEST_COLS} WHERE r.id = $1")).bind(id).fetch_one(&st.pool).await?;
    Ok(Json(row))
}
