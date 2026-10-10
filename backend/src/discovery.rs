//! A discovery run: fetch from the user's sources, filter, score, store the new matches.

use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::matching;
use crate::models::{Job, NewJob, Preferences, Profile};
use crate::sources;
use crate::state::AppState;

pub async fn load_profile(pool: &PgPool, user_id: Uuid) -> sqlx::Result<Profile> {
    sqlx::query_as::<_, Profile>("SELECT * FROM profiles WHERE user_id = $1").bind(user_id).fetch_one(pool).await
}

pub async fn load_preferences(pool: &PgPool, user_id: Uuid) -> sqlx::Result<Preferences> {
    sqlx::query_as::<_, Preferences>("SELECT * FROM preferences WHERE user_id = $1").bind(user_id).fetch_one(pool).await
}

/// Scores and stores one job. Returns None if this job was already stored for the user.
pub async fn store_job(
    pool: &PgPool,
    user_id: Uuid,
    job: &NewJob,
    profile: &Profile,
    prefs: &Preferences,
) -> sqlx::Result<Option<Job>> {
    let m = matching::score(profile, &job.title, &job.description, &prefs.target_roles);
    sqlx::query_as::<_, Job>(
        "INSERT INTO jobs (id, user_id, source, external_id, title, company, location, url, description, posted_at,
                           required_skills, min_years, max_years, score, matched_skills, missing_skills, explanation)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17)
         ON CONFLICT (user_id, source, external_id) DO NOTHING
         RETURNING id, source, external_id, title, company, location, url, description, posted_at, required_skills,
                   min_years, max_years, score, matched_skills, missing_skills, explanation, queue_status, created_at",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(&job.source)
    .bind(&job.external_id)
    .bind(&job.title)
    .bind(&job.company)
    .bind(&job.location)
    .bind(&job.url)
    .bind(&job.description)
    .bind(job.posted_at)
    .bind(&m.required_skills)
    .bind(m.min_years)
    .bind(m.max_years)
    .bind(m.score)
    .bind(&m.matched_skills)
    .bind(&m.missing_skills)
    .bind(&m.explanation)
    .fetch_optional(pool)
    .await
}

/// Re-scores every stored job after the profile or target roles change.
pub async fn rescore_all(pool: &PgPool, user_id: Uuid) -> sqlx::Result<u64> {
    let profile = load_profile(pool, user_id).await?;
    let prefs = load_preferences(pool, user_id).await?;
    let rows: Vec<(Uuid, String, String)> =
        sqlx::query_as("SELECT id, title, description FROM jobs WHERE user_id = $1").bind(user_id).fetch_all(pool).await?;
    let n = rows.len() as u64;
    for (id, title, description) in rows {
        let m = matching::score(&profile, &title, &description, &prefs.target_roles);
        sqlx::query(
            "UPDATE jobs SET required_skills=$2, min_years=$3, max_years=$4, score=$5, matched_skills=$6, missing_skills=$7, explanation=$8 WHERE id=$1",
        )
        .bind(id)
        .bind(&m.required_skills)
        .bind(m.min_years)
        .bind(m.max_years)
        .bind(m.score)
        .bind(&m.matched_skills)
        .bind(&m.missing_skills)
        .bind(&m.explanation)
        .execute(pool)
        .await?;
    }
    Ok(n)
}

#[derive(Debug, Serialize)]
pub struct RunSummary {
    pub run_id: Uuid,
    pub fetched: i32,
    pub added: i32,
    pub errors: Vec<String>,
}

pub async fn run_for_user(state: &AppState, user_id: Uuid, trigger: &str) -> anyhow::Result<RunSummary> {
    let pool = &state.pool;
    let run_id = Uuid::new_v4();
    sqlx::query("INSERT INTO runs (id, user_id, trigger) VALUES ($1, $2, $3)").bind(run_id).bind(user_id).bind(trigger).execute(pool).await?;

    let profile = load_profile(pool, user_id).await?;
    let prefs = load_preferences(pool, user_id).await?;
    let src: Vec<(String, String, String)> =
        sqlx::query_as("SELECT kind, board, company FROM job_sources WHERE user_id = $1").bind(user_id).fetch_all(pool).await?;

    let mut fetched = 0i32;
    let mut candidates = Vec::new();
    let mut errors = Vec::new();
    for (kind, board, company) in &src {
        match sources::fetch(&state.http, kind, board, company).await {
            Ok(jobs) => {
                fetched += jobs.len() as i32;
                candidates.extend(jobs.into_iter().filter(|j| sources::passes_filters(j, &prefs.target_roles, &prefs.locations, prefs.remote_ok)));
            }
            Err(e) => {
                tracing::warn!(user = %user_id, kind, board, error = %e, "source fetch failed");
                errors.push(format!("{kind}/{board}: {e}"));
            }
        }
    }

    // Best matches first, so the per-run cap keeps the strongest jobs.
    let mut scored: Vec<(i32, NewJob)> = candidates
        .into_iter()
        .map(|j| (matching::score(&profile, &j.title, &j.description, &prefs.target_roles).score, j))
        .filter(|(s, _)| *s >= prefs.min_score)
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0));

    let mut added = 0i32;
    for (_, job) in scored {
        if added >= prefs.max_jobs_per_run {
            break;
        }
        if store_job(pool, user_id, &job, &profile, &prefs).await?.is_some() {
            added += 1;
        }
    }

    sqlx::query("UPDATE runs SET finished_at = now(), fetched = $2, added = $3, errors = $4 WHERE id = $1")
        .bind(run_id)
        .bind(fetched)
        .bind(added)
        .bind(sqlx::types::Json(&errors))
        .execute(pool)
        .await?;
    tracing::info!(user = %user_id, trigger, fetched, added, "discovery run finished");
    Ok(RunSummary { run_id, fetched, added, errors })
}
