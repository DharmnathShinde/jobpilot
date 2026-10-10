use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{AppError, AppResult};
use crate::models::{Application, ApplicationEvent, APPLICATION_STATUSES};
use crate::state::AppState;

const APP_SELECT: &str = "SELECT a.id, a.job_id, a.status, a.resume_id, a.notes, a.next_reminder, a.applied_at, a.updated_at,
    j.title, j.company, j.location, j.url, j.score
    FROM applications a JOIN jobs j ON j.id = a.job_id";

#[derive(Deserialize)]
pub struct CreateReq {
    pub job_id: Uuid,
    #[serde(default = "applied")]
    pub status: String,
    pub resume_id: Option<Uuid>,
}
fn applied() -> String {
    "applied".into()
}

/// Called when the user confirms "Yes, I submitted" (or saves a job to track).
pub async fn create(State(st): State<AppState>, AuthUser(uid): AuthUser, Json(r): Json<CreateReq>) -> AppResult<Json<Application>> {
    if !APPLICATION_STATUSES.contains(&r.status.as_str()) {
        return Err(AppError::BadRequest("unknown status".into()));
    }
    let owns: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM jobs WHERE id = $1 AND user_id = $2)").bind(r.job_id).bind(uid).fetch_one(&st.pool).await?;
    if !owns {
        return Err(AppError::NotFound);
    }
    let mut tx = st.pool.begin().await?;
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO applications (id, user_id, job_id, status, resume_id, applied_at)
         VALUES ($1, $2, $3, $4, $5, CASE WHEN $4 = 'saved' THEN NULL ELSE now() END)
         ON CONFLICT (user_id, job_id) DO UPDATE SET status = EXCLUDED.status, resume_id = COALESCE(EXCLUDED.resume_id, applications.resume_id),
             applied_at = COALESCE(applications.applied_at, EXCLUDED.applied_at), updated_at = now()
         RETURNING id",
    )
    .bind(Uuid::new_v4())
    .bind(uid)
    .bind(r.job_id)
    .bind(&r.status)
    .bind(r.resume_id)
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO application_events (id, application_id, kind) VALUES ($1, $2, $3)").bind(Uuid::new_v4()).bind(id).bind(&r.status).execute(&mut *tx).await?;
    if r.status == "applied" {
        sqlx::query("UPDATE jobs SET queue_status = 'applied' WHERE id = $1").bind(r.job_id).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(Json(load(&st, uid, id).await?))
}

async fn load(st: &AppState, uid: Uuid, id: Uuid) -> AppResult<Application> {
    sqlx::query_as::<_, Application>(&format!("{APP_SELECT} WHERE a.id = $1 AND a.user_id = $2"))
        .bind(id)
        .bind(uid)
        .fetch_optional(&st.pool)
        .await?
        .ok_or(AppError::NotFound)
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub status: Option<String>,
}

pub async fn list(State(st): State<AppState>, AuthUser(uid): AuthUser, Query(q): Query<ListQuery>) -> AppResult<Json<Value>> {
    let rows = sqlx::query_as::<_, Application>(&format!(
        "{APP_SELECT} WHERE a.user_id = $1 AND ($2::text IS NULL OR a.status = $2) ORDER BY a.updated_at DESC"
    ))
    .bind(uid)
    .bind(q.status)
    .fetch_all(&st.pool)
    .await?;
    let counts: Vec<(String, i64)> = sqlx::query_as("SELECT status, count(*) FROM applications WHERE user_id = $1 GROUP BY status").bind(uid).fetch_all(&st.pool).await?;
    let counts: serde_json::Map<String, Value> = APPLICATION_STATUSES
        .iter()
        .map(|s| (s.to_string(), json!(counts.iter().find(|(k, _)| k == s).map_or(0, |(_, n)| *n))))
        .collect();
    Ok(Json(json!({ "counts": counts, "applications": rows })))
}

pub async fn get(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(id): Path<Uuid>) -> AppResult<Json<Value>> {
    let app = load(&st, uid, id).await?;
    let events = sqlx::query_as::<_, ApplicationEvent>("SELECT id, kind, note, at FROM application_events WHERE application_id = $1 ORDER BY at")
        .bind(id)
        .fetch_all(&st.pool)
        .await?;
    Ok(Json(json!({ "application": app, "events": events })))
}

#[derive(Deserialize)]
pub struct UpdateReq {
    pub status: Option<String>,
    pub notes: Option<String>,
    /// Some(None) clears the reminder.
    #[serde(default, deserialize_with = "double_option")]
    pub next_reminder: Option<Option<NaiveDate>>,
    pub event_note: Option<String>,
}

/// Distinguishes a missing field (None) from an explicit null (Some(None)).
fn double_option<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Option<NaiveDate>>, D::Error> {
    Option::<NaiveDate>::deserialize(d).map(Some)
}

pub async fn update(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(id): Path<Uuid>, Json(r): Json<UpdateReq>) -> AppResult<Json<Application>> {
    let current = load(&st, uid, id).await?;
    if let Some(s) = &r.status {
        if !APPLICATION_STATUSES.contains(&s.as_str()) {
            return Err(AppError::BadRequest("unknown status".into()));
        }
    }
    let mut tx = st.pool.begin().await?;
    sqlx::query(
        "UPDATE applications SET status = COALESCE($2, status), notes = COALESCE($3, notes),
             next_reminder = CASE WHEN $4 THEN $5 ELSE next_reminder END,
             applied_at = CASE WHEN COALESCE($2, status) <> 'saved' THEN COALESCE(applied_at, now()) ELSE applied_at END,
             updated_at = now()
         WHERE id = $1",
    )
    .bind(id)
    .bind(&r.status)
    .bind(&r.notes)
    .bind(r.next_reminder.is_some())
    .bind(r.next_reminder.flatten())
    .execute(&mut *tx)
    .await?;
    if let Some(s) = r.status.as_ref().filter(|s| **s != current.status) {
        sqlx::query("INSERT INTO application_events (id, application_id, kind, note) VALUES ($1, $2, $3, $4)")
            .bind(Uuid::new_v4())
            .bind(id)
            .bind(s)
            .bind(r.event_note.clone().unwrap_or_default())
            .execute(&mut *tx)
            .await?;
    } else if let Some(note) = &r.event_note {
        sqlx::query("INSERT INTO application_events (id, application_id, kind, note) VALUES ($1, $2, 'note', $3)").bind(Uuid::new_v4()).bind(id).bind(note).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(Json(load(&st, uid, id).await?))
}
