//! A discovery run: take jobs from the shared catalog plus the user's own pages, filter, score,
//! store the new matches.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
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

/// Most catalog jobs that get scored in one run (after the title/location filter), newest first.
const MAX_CATALOG_CANDIDATES: usize = 1500;

type CatalogRow = (Uuid, String, String, String, String, String, String, Option<DateTime<Utc>>);

/// Jobs from the shared catalog that pass the user's role and location filters.
/// Returns (jobs looked at, candidates). Descriptions are loaded only for the candidates.
async fn catalog_candidates(pool: &PgPool, prefs: &Preferences) -> sqlx::Result<(usize, Vec<NewJob>)> {
    let rows: Vec<CatalogRow> = sqlx::query_as(
        "SELECT j.company_id, c.kind, j.external_id, j.title, c.name, j.location, j.url, j.posted_at
         FROM catalog_jobs j JOIN companies c ON c.id = j.company_id WHERE c.active",
    )
    .fetch_all(pool)
    .await?;
    let total = rows.len();

    let mut picked: Vec<(Uuid, NewJob)> = rows
        .into_iter()
        .map(|(company_id, kind, external_id, title, company, location, url, posted_at)| {
            (company_id, NewJob { source: kind, external_id, title, company, location, url, description: String::new(), posted_at })
        })
        .filter(|(_, j)| sources::passes_filters(j, &prefs.target_roles, &prefs.locations, prefs.remote_ok))
        .collect();
    picked.sort_by(|a, b| b.1.posted_at.cmp(&a.1.posted_at));
    picked.truncate(MAX_CATALOG_CANDIDATES);
    if picked.is_empty() {
        return Ok((total, Vec::new()));
    }

    let company_ids: Vec<Uuid> = picked.iter().map(|(c, _)| *c).collect();
    let external_ids: Vec<&str> = picked.iter().map(|(_, j)| j.external_id.as_str()).collect();
    let descriptions: HashMap<(Uuid, String), String> = sqlx::query_as::<_, (Uuid, String, String)>(
        "SELECT company_id, external_id, description FROM catalog_jobs
         WHERE (company_id, external_id) IN (SELECT * FROM UNNEST($1::uuid[], $2::text[]))",
    )
    .bind(&company_ids)
    .bind(&external_ids)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|(c, e, d)| ((c, e), d))
    .collect();

    let jobs = picked
        .into_iter()
        .map(|(c, mut j)| {
            j.description = descriptions.get(&(c, j.external_id.clone())).cloned().unwrap_or_default();
            j
        })
        .collect();
    Ok((total, jobs))
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

    let (catalog_fetched, mut candidates) = catalog_candidates(pool, &prefs).await?;
    let mut fetched = catalog_fetched as i32;
    let mut errors = Vec::new();

    // Pages the user added just for themselves are fetched live (unless the catalog already covers them).
    let src: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT kind, board, company FROM job_sources s WHERE user_id = $1
           AND NOT EXISTS (SELECT 1 FROM companies c WHERE c.active AND c.kind = s.kind AND c.board = s.board)",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
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
