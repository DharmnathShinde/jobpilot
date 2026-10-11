use axum::extract::{Path, Query, State};
use axum::response::Html;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::types::Json as SqlJson;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::discovery;
use crate::error::{AppError, AppResult};
use crate::models::{Job, NewJob, TailoredResume};
use crate::state::AppState;
use crate::tailor;

const JOB_COLS: &str = "id, source, external_id, title, company, location, url, description, posted_at, required_skills,
    min_years, max_years, score, matched_skills, missing_skills, explanation, queue_status, created_at,
    hr_email, hr_email_source, outreach_status";

#[derive(Deserialize)]
pub struct ManualJob {
    pub title: String,
    #[serde(default)]
    pub company: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub description: String,
    /// "manual" (typed in) or "share" (from the share sheet)
    #[serde(default)]
    pub via: Option<String>,
}

/// Add a job the user found themselves (LinkedIn/Naukri share sheet or typed in).
pub async fn create(State(st): State<AppState>, AuthUser(uid): AuthUser, Json(j): Json<ManualJob>) -> AppResult<Json<Job>> {
    if j.title.trim().is_empty() {
        return Err(AppError::BadRequest("title is required".into()));
    }
    if j.description.trim().len() < 30 {
        return Err(AppError::BadRequest("paste the job description so we can score it".into()));
    }
    let source = if j.via.as_deref() == Some("share") { "share" } else { "manual" };
    // Same link added twice (from any source) → same job.
    if !j.url.trim().is_empty() {
        let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM jobs WHERE user_id = $1 AND url = $2)")
            .bind(uid)
            .bind(j.url.trim())
            .fetch_one(&st.pool)
            .await?;
        if exists {
            return Err(AppError::Conflict("you've already added this job".into()));
        }
    }
    let external_id = if j.url.trim().is_empty() { Uuid::new_v4().to_string() } else { j.url.trim().to_string() };
    let new = NewJob {
        source: source.into(),
        external_id,
        title: j.title.trim().into(),
        company: j.company.trim().into(),
        location: j.location.trim().into(),
        url: j.url.trim().into(),
        description: j.description.trim().into(),
        posted_at: None,
    };
    let profile = discovery::load_profile(&st.pool, uid).await?;
    let prefs = discovery::load_preferences(&st.pool, uid).await?;
    match discovery::store_job(&st.pool, uid, &new, &profile, &prefs).await? {
        Some(job) => Ok(Json(job)),
        None => Err(AppError::Conflict("you've already added this job".into())),
    }
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub status: Option<String>,
    pub min_score: Option<i32>,
    pub limit: Option<i64>,
}

pub async fn list(State(st): State<AppState>, AuthUser(uid): AuthUser, Query(q): Query<ListQuery>) -> AppResult<Json<Vec<Job>>> {
    let status = q.status.unwrap_or_else(|| "new".into());
    let rows = sqlx::query_as::<_, Job>(&format!(
        "SELECT {JOB_COLS} FROM jobs WHERE user_id = $1 AND queue_status = $2 AND score >= $3
         ORDER BY score DESC, created_at DESC LIMIT $4"
    ))
    .bind(uid)
    .bind(status)
    .bind(q.min_score.unwrap_or(0))
    .bind(q.limit.unwrap_or(200).clamp(1, 1000))
    .fetch_all(&st.pool)
    .await?;
    Ok(Json(rows))
}

#[derive(Deserialize)]
pub struct HrEmailReq {
    pub email: Option<String>,
}

/// Set or clear the HR/recruiter email (manual override).
pub async fn set_hr_email(
    State(st): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(id): Path<Uuid>,
    Json(r): Json<HrEmailReq>,
) -> AppResult<Json<Job>> {
    let email = r.email.map(|e| e.trim().to_lowercase()).filter(|e| !e.is_empty());
    if let Some(ref e) = email {
        if !e.contains('@') || e.len() < 5 {
            return Err(AppError::BadRequest("enter a valid email".into()));
        }
    }
    let source = email.as_ref().map(|_| "manual".to_string());
    let n = sqlx::query("UPDATE jobs SET hr_email = $3, hr_email_source = $4 WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(uid)
        .bind(&email)
        .bind(&source)
        .execute(&st.pool)
        .await?
        .rows_affected();
    if n == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Json(load_job(&st, uid, id).await?))
}

#[derive(Serialize)]
pub struct OutreachDraft {
    pub to: String,
    pub subject: String,
    pub body: String,
}

/// Short outreach email draft; user sends from their own mail app.
pub async fn outreach_draft(
    State(st): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<OutreachDraft>> {
    let job = load_job(&st, uid, id).await?;
    let to = job.hr_email.clone().filter(|e| !e.is_empty()).ok_or_else(|| AppError::BadRequest("add an HR email first".into()))?;
    let profile = discovery::load_profile(&st.pool, uid).await?;
    let name = if profile.full_name.is_empty() { "there".into() } else { profile.full_name.clone() };
    let company = if job.company.is_empty() { "your team".to_string() } else { job.company.clone() };
    let headline = if profile.headline.is_empty() {
        String::new()
    } else {
        format!(" I'm a {}.", profile.headline.to_lowercase())
    };
    let skills = if job.matched_skills.is_empty() {
        String::new()
    } else {
        let s = job.matched_skills.iter().take(4).cloned().collect::<Vec<_>>().join(", ");
        format!(" My background includes {s}.")
    };
    let subject = format!("Application: {} at {}", job.title, company);
    let body = format!(
        "Hi,\n\nI saw the {title} role at {company} and would like to apply.{headline}{skills}\n\nI'd welcome the chance to discuss how I can contribute. My resume is attached / available on request.\n\nBest regards,\n{name}",
        title = job.title,
        company = company,
        headline = headline,
        skills = skills,
        name = name,
    );
    sqlx::query("UPDATE jobs SET outreach_status = 'drafted' WHERE id = $1 AND user_id = $2 AND outreach_status = 'none'")
        .bind(id)
        .bind(uid)
        .execute(&st.pool)
        .await?;
    Ok(Json(OutreachDraft { to, subject, body }))
}

#[derive(Deserialize)]
pub struct OutreachStatusReq {
    pub status: String,
}

pub async fn set_outreach_status(
    State(st): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(id): Path<Uuid>,
    Json(r): Json<OutreachStatusReq>,
) -> AppResult<Json<Job>> {
    if !["none", "drafted", "sent"].contains(&r.status.as_str()) {
        return Err(AppError::BadRequest("status must be none, drafted or sent".into()));
    }
    let n = sqlx::query("UPDATE jobs SET outreach_status = $3 WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(uid)
        .bind(&r.status)
        .execute(&st.pool)
        .await?
        .rows_affected();
    if n == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Json(load_job(&st, uid, id).await?))
}

pub async fn get(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(id): Path<Uuid>) -> AppResult<Json<Job>> {
    Ok(Json(load_job(&st, uid, id).await?))
}

async fn load_job(st: &AppState, uid: Uuid, id: Uuid) -> AppResult<Job> {
    sqlx::query_as::<_, Job>(&format!("SELECT {JOB_COLS} FROM jobs WHERE id = $1 AND user_id = $2"))
        .bind(id)
        .bind(uid)
        .fetch_optional(&st.pool)
        .await?
        .ok_or(AppError::NotFound)
}

#[derive(Deserialize)]
pub struct StatusReq {
    pub status: String,
}

/// Queue triage: new → saved / skipped.
pub async fn set_status(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(id): Path<Uuid>, Json(r): Json<StatusReq>) -> AppResult<Json<Job>> {
    if !["new", "saved", "skipped"].contains(&r.status.as_str()) {
        return Err(AppError::BadRequest("status must be new, saved or skipped".into()));
    }
    let n = sqlx::query("UPDATE jobs SET queue_status = $3 WHERE id = $1 AND user_id = $2").bind(id).bind(uid).bind(&r.status).execute(&st.pool).await?.rows_affected();
    if n == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Json(load_job(&st, uid, id).await?))
}

pub async fn tailor_resume(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(id): Path<Uuid>) -> AppResult<Json<TailoredResume>> {
    let job = load_job(&st, uid, id).await?;
    let profile = discovery::load_profile(&st.pool, uid).await?;
    if !profile.confirmed {
        return Err(AppError::BadRequest("confirm your profile before tailoring resumes".into()));
    }
    let t = tailor::tailor(&profile, &job.title, &job.required_skills);
    let row = sqlx::query_as::<_, TailoredResume>(
        "INSERT INTO tailored_resumes (id, user_id, job_id, content, changes, keyword_coverage) VALUES ($1,$2,$3,$4,$5,$6)
         RETURNING id, job_id, content, changes, keyword_coverage, created_at",
    )
    .bind(Uuid::new_v4())
    .bind(uid)
    .bind(id)
    .bind(SqlJson(&t.content))
    .bind(SqlJson(&t.changes))
    .bind(t.keyword_coverage)
    .fetch_one(&st.pool)
    .await?;
    Ok(Json(row))
}

async fn load_resume(st: &AppState, uid: Uuid, id: Uuid) -> AppResult<TailoredResume> {
    sqlx::query_as::<_, TailoredResume>("SELECT id, job_id, content, changes, keyword_coverage, created_at FROM tailored_resumes WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(uid)
        .fetch_optional(&st.pool)
        .await?
        .ok_or(AppError::NotFound)
}

pub async fn get_resume(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(id): Path<Uuid>) -> AppResult<Json<TailoredResume>> {
    Ok(Json(load_resume(&st, uid, id).await?))
}

pub async fn get_resume_html(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(id): Path<Uuid>) -> AppResult<Html<String>> {
    let r = load_resume(&st, uid, id).await?;
    Ok(Html(tailor::render_html(&r.content)))
}

#[derive(Serialize)]
pub struct Answer {
    pub key: &'static str,
    pub label: &'static str,
    pub value: String,
}

/// Copy-ready answers for the application form, built only from the profile and preferences.
pub async fn answers(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(id): Path<Uuid>) -> AppResult<Json<Vec<Answer>>> {
    let job = load_job(&st, uid, id).await?;
    let profile = discovery::load_profile(&st.pool, uid).await?;
    let prefs = discovery::load_preferences(&st.pool, uid).await?;

    let skills_line = match job.matched_skills.len() {
        0 => String::new(),
        1 => job.matched_skills[0].clone(),
        n => format!("{} and {}", job.matched_skills[..n.min(3) - 1].join(", "), job.matched_skills[n.min(3) - 1]),
    };
    // Most relevant bullet from the user's own experience.
    let best_bullet = profile
        .experience
        .iter()
        .flat_map(|e| e.bullets.iter())
        .max_by_key(|b| crate::skills::extract_skills(b).iter().filter(|s| job.required_skills.contains(s)).count())
        .cloned();

    let company = if job.company.is_empty() { "your team".to_string() } else { job.company.clone() };
    let mut why = String::new();
    if profile.years_experience > 0.0 && !profile.headline.is_empty() {
        why.push_str(&format!("I've spent {:.0} years as a {}", profile.years_experience, profile.headline.to_lowercase()));
        if !skills_line.is_empty() {
            why.push_str(&format!(", working with {skills_line}"));
        }
        why.push_str(". ");
    } else if !skills_line.is_empty() {
        why.push_str(&format!("My experience with {skills_line} lines up with this role. "));
    }
    if let Some(b) = &best_bullet {
        why.push_str(&format!("Most relevant: {}. ", b.trim_end_matches('.')));
    }
    why.push_str(&format!("I'd like to bring that to the {} role at {company}.", job.title));

    let mut out = vec![Answer { key: "why_role", label: "Why this role?", value: why }];
    if let Some(n) = prefs.notice_days {
        out.push(Answer { key: "notice", label: "Notice period", value: format!("{n} days") });
    }
    if let Some(c) = prefs.expected_ctc_lpa {
        out.push(Answer { key: "ctc", label: "Expected CTC", value: format!("₹{c} LPA") });
    }
    out.push(Answer { key: "experience", label: "Total experience", value: format!("{:.0} years", profile.years_experience) });
    Ok(Json(out))
}

pub async fn run_now(State(st): State<AppState>, AuthUser(uid): AuthUser) -> AppResult<Json<discovery::RunSummary>> {
    let confirmed: bool = sqlx::query_scalar("SELECT confirmed FROM profiles WHERE user_id = $1").bind(uid).fetch_one(&st.pool).await?;
    if !confirmed {
        return Err(AppError::BadRequest("confirm your profile first".into()));
    }
    Ok(Json(discovery::run_for_user(&st, uid, "manual").await?))
}

/// Home screen: queue size, today's progress, top matches, last run.
pub async fn today(State(st): State<AppState>, AuthUser(uid): AuthUser) -> AppResult<Json<Value>> {
    let prefs = discovery::load_preferences(&st.pool, uid).await?;
    let tz: chrono_tz::Tz = prefs.timezone.parse().unwrap_or(chrono_tz::Asia::Kolkata);
    let start_of_day = chrono::Utc::now().with_timezone(&tz).date_naive().and_hms_opt(0, 0, 0).unwrap()
        .and_local_timezone(tz).single().map(|d| d.with_timezone(&chrono::Utc)).unwrap_or_else(chrono::Utc::now);

    let (new_count,): (i64,) = sqlx::query_as("SELECT count(*) FROM jobs WHERE user_id = $1 AND queue_status = 'new' AND score >= $2")
        .bind(uid).bind(prefs.min_score).fetch_one(&st.pool).await?;
    let (reviewed,): (i64,) = sqlx::query_as("SELECT count(*) FROM jobs WHERE user_id = $1 AND queue_status <> 'new' AND created_at >= $2")
        .bind(uid).bind(start_of_day).fetch_one(&st.pool).await?;
    let (applied,): (i64,) = sqlx::query_as("SELECT count(*) FROM applications WHERE user_id = $1 AND applied_at >= $2")
        .bind(uid).bind(start_of_day).fetch_one(&st.pool).await?;
    let top = sqlx::query_as::<_, Job>(&format!(
        "SELECT {JOB_COLS} FROM jobs WHERE user_id = $1 AND queue_status = 'new' AND score >= $2 ORDER BY score DESC, created_at DESC LIMIT 5"
    ))
    .bind(uid).bind(prefs.min_score).fetch_all(&st.pool).await?;
    let last_run: Option<(chrono::DateTime<chrono::Utc>, i32)> =
        sqlx::query_as("SELECT started_at, added FROM runs WHERE user_id = $1 ORDER BY started_at DESC LIMIT 1").bind(uid).fetch_optional(&st.pool).await?;

    Ok(Json(json!({
        "queue_count": new_count,
        "reviewed_today": reviewed,
        "applied_today": applied,
        "run_time": prefs.run_time.format("%H:%M").to_string(),
        "last_run": last_run.map(|(at, added)| json!({ "at": at, "added": added })),
        "top_matches": top,
    })))
}
