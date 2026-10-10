use axum::extract::{Multipart, Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::cv;
use crate::discovery;
use crate::error::{AppError, AppResult};
use crate::models::{JobSource, Preferences, Profile};
use crate::skills;
use crate::state::AppState;

pub async fn get_profile(State(st): State<AppState>, AuthUser(uid): AuthUser) -> AppResult<Json<Profile>> {
    Ok(Json(discovery::load_profile(&st.pool, uid).await?))
}

/// Upload a CV (multipart field `file`). Returns an unconfirmed draft for the user to review.
pub async fn upload_cv(State(st): State<AppState>, AuthUser(uid): AuthUser, mut form: Multipart) -> AppResult<Json<Value>> {
    let mut file: Option<(String, Vec<u8>)> = None;
    while let Some(field) = form.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        if field.name() == Some("file") {
            let name = field.file_name().unwrap_or("cv").to_string();
            let bytes = field.bytes().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
            file = Some((name, bytes.to_vec()));
        }
    }
    let (name, bytes) = file.ok_or_else(|| AppError::BadRequest("attach the CV as a field named 'file'".into()))?;
    if bytes.len() > 5 * 1024 * 1024 {
        return Err(AppError::BadRequest("CV must be 5 MB or smaller".into()));
    }
    // PDF parsing is CPU-bound; keep it off the async workers.
    let text = tokio::task::spawn_blocking(move || cv::extract_text(&name, &bytes))
        .await
        .map_err(|e| anyhow::anyhow!(e))?
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    let (draft, method) = match &st.llm {
        Some(llm) => match cv::llm_profile(llm, &text).await {
            Ok(p) => (p, "ai"),
            Err(e) => {
                tracing::warn!(error = %e, "LLM CV parse failed, using rules");
                (cv::heuristic_profile(&text), "rules")
            }
        },
        None => (cv::heuristic_profile(&text), "rules"),
    };
    let saved = save_profile(&st, uid, draft, false).await?;
    Ok(Json(json!({ "profile": saved, "method": method })))
}

/// Replace the profile with the reviewed version. `confirmed: true` unlocks matching runs.
pub async fn put_profile(State(st): State<AppState>, AuthUser(uid): AuthUser, Json(p): Json<Profile>) -> AppResult<Json<Profile>> {
    let confirmed = p.confirmed;
    let saved = save_profile(&st, uid, p, confirmed).await?;
    discovery::rescore_all(&st.pool, uid).await?;
    Ok(Json(saved))
}

async fn save_profile(st: &AppState, uid: Uuid, mut p: Profile, confirmed: bool) -> AppResult<Profile> {
    let mut seen = Vec::<String>::new();
    p.skills = p
        .skills
        .iter()
        .map(|s| skills::canonicalize(s))
        .filter(|s| !s.is_empty())
        .filter(|s| {
            let k = s.to_lowercase();
            if seen.contains(&k) {
                false
            } else {
                seen.push(k);
                true
            }
        })
        .collect();
    let raw = if p.raw_cv_text.is_empty() { None } else { Some(p.raw_cv_text.clone()) };
    let row = sqlx::query_as::<_, Profile>(
        "UPDATE profiles SET full_name=$2, headline=$3, location=$4, email=$5, phone=$6, summary=$7, years_experience=$8,
                skills=$9, experience=$10, projects=$11, education=$12, links=$13,
                raw_cv_text=COALESCE($14, raw_cv_text), confirmed=$15, updated_at=now()
         WHERE user_id=$1 RETURNING *",
    )
    .bind(uid)
    .bind(p.full_name.trim())
    .bind(p.headline.trim())
    .bind(p.location.trim())
    .bind(p.email.trim())
    .bind(p.phone.trim())
    .bind(p.summary.trim())
    .bind(p.years_experience.max(0.0))
    .bind(&p.skills)
    .bind(&p.experience)
    .bind(&p.projects)
    .bind(&p.education)
    .bind(&p.links)
    .bind(raw)
    .bind(confirmed)
    .fetch_one(&st.pool)
    .await?;
    Ok(row)
}

pub async fn get_preferences(State(st): State<AppState>, AuthUser(uid): AuthUser) -> AppResult<Json<Preferences>> {
    Ok(Json(discovery::load_preferences(&st.pool, uid).await?))
}

pub async fn put_preferences(State(st): State<AppState>, AuthUser(uid): AuthUser, Json(p): Json<Preferences>) -> AppResult<Json<Preferences>> {
    if p.timezone.parse::<chrono_tz::Tz>().is_err() {
        return Err(AppError::BadRequest(format!("unknown timezone {}", p.timezone)));
    }
    if p.active_days.iter().any(|d| !(1..=7).contains(d)) {
        return Err(AppError::BadRequest("active_days must be 1 (Mon) to 7 (Sun)".into()));
    }
    let row = sqlx::query_as::<_, Preferences>(
        "UPDATE preferences SET target_roles=$2, locations=$3, remote_ok=$4, expected_ctc_lpa=$5, notice_days=$6,
                min_score=$7, run_time=$8, timezone=$9, active_days=$10, max_jobs_per_run=$11, schedule_enabled=$12, updated_at=now()
         WHERE user_id=$1 RETURNING *",
    )
    .bind(uid)
    .bind(&p.target_roles)
    .bind(&p.locations)
    .bind(p.remote_ok)
    .bind(p.expected_ctc_lpa)
    .bind(p.notice_days)
    .bind(p.min_score.clamp(0, 100))
    .bind(p.run_time)
    .bind(&p.timezone)
    .bind(&p.active_days)
    .bind(p.max_jobs_per_run.clamp(1, 100))
    .bind(p.schedule_enabled)
    .fetch_one(&st.pool)
    .await?;
    discovery::rescore_all(&st.pool, uid).await?;
    Ok(Json(row))
}

#[derive(Deserialize)]
pub struct NewSource {
    pub kind: String,
    pub board: String,
    #[serde(default)]
    pub company: String,
}

pub async fn list_sources(State(st): State<AppState>, AuthUser(uid): AuthUser) -> AppResult<Json<Vec<JobSource>>> {
    let rows = sqlx::query_as::<_, JobSource>("SELECT id, kind, board, company, created_at FROM job_sources WHERE user_id = $1 ORDER BY created_at")
        .bind(uid)
        .fetch_all(&st.pool)
        .await?;
    Ok(Json(rows))
}

pub async fn add_source(State(st): State<AppState>, AuthUser(uid): AuthUser, Json(s): Json<NewSource>) -> AppResult<Json<JobSource>> {
    if !["greenhouse", "lever", "ashby"].contains(&s.kind.as_str()) {
        return Err(AppError::BadRequest("kind must be greenhouse, lever or ashby".into()));
    }
    let board = s.board.trim().to_lowercase();
    if board.is_empty() || !board.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c)) {
        return Err(AppError::BadRequest("board is the company's slug, e.g. 'razorpay'".into()));
    }
    let row = sqlx::query_as::<_, JobSource>(
        "INSERT INTO job_sources (id, user_id, kind, board, company) VALUES ($1,$2,$3,$4,$5)
         ON CONFLICT (user_id, kind, board) DO UPDATE SET company = EXCLUDED.company
         RETURNING id, kind, board, company, created_at",
    )
    .bind(Uuid::new_v4())
    .bind(uid)
    .bind(&s.kind)
    .bind(&board)
    .bind(s.company.trim())
    .fetch_one(&st.pool)
    .await?;
    Ok(Json(row))
}

pub async fn delete_source(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(id): Path<Uuid>) -> AppResult<Json<Value>> {
    let n = sqlx::query("DELETE FROM job_sources WHERE id = $1 AND user_id = $2").bind(id).bind(uid).execute(&st.pool).await?.rows_affected();
    if n == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Json(json!({ "deleted": true })))
}
