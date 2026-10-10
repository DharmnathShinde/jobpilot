//! The shared catalog of company career pages that every user follows by default:
//! seeding it, the cron refresh of each board's open roles, and turning users'
//! "add this company" requests into new catalog entries.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::NewJob;
use crate::sources;
use crate::state::AppState;

/// Verified Greenhouse / Lever / Ashby boards loaded on startup (new entries only).
const SEED: &str = include_str!("../data/companies.json");
/// A board is fetched again once its last check is this old.
const STALE_AFTER_HOURS: i32 = 20;
const CONCURRENCY: usize = 12;
const MAX_DESCRIPTION: usize = 12_000;
/// Boards that fail this many checks in a row (removed or renamed) stop being checked.
const MAX_FAILS: i32 = 7;
const REQUESTS_PER_TICK: i64 = 25;

static BUSY: AtomicBool = AtomicBool::new(false);

struct BusyGuard;
impl Drop for BusyGuard {
    fn drop(&mut self) {
        BUSY.store(false, Ordering::Release);
    }
}

pub async fn seed(pool: &PgPool) -> anyhow::Result<u64> {
    let rows: serde_json::Value = serde_json::from_str(SEED)?;
    let n = sqlx::query(
        "INSERT INTO companies (id, kind, board, name)
         SELECT gen_random_uuid(), kind, board, name FROM jsonb_to_recordset($1) AS t(kind text, board text, name text)
         ON CONFLICT (kind, board) DO NOTHING",
    )
    .bind(sqlx::types::Json(rows))
    .execute(pool)
    .await?
    .rows_affected();
    Ok(n)
}

#[derive(Debug, Default, Serialize)]
pub struct TickSummary {
    pub requests_resolved: usize,
    pub refreshed: usize,
    pub failed: usize,
    /// Another refresh was already running.
    pub skipped: bool,
}

/// Resolves pending requests, then refreshes stale boards until `budget` runs out.
/// Whatever is left is picked up by the next tick.
pub async fn tick(state: &AppState, budget: Duration) -> anyhow::Result<TickSummary> {
    if BUSY.swap(true, Ordering::AcqRel) {
        return Ok(TickSummary { skipped: true, ..Default::default() });
    }
    let _guard = BusyGuard;
    let deadline = Instant::now() + budget;
    let mut summary = TickSummary { requests_resolved: resolve_requests(state).await?, ..Default::default() };

    let stale: Vec<(Uuid, String, String, String)> = sqlx::query_as(
        "SELECT id, kind, board, name FROM companies
         WHERE active AND (last_checked_at IS NULL OR last_checked_at < now() - make_interval(hours => $1))
         ORDER BY last_checked_at NULLS FIRST LIMIT 2000",
    )
    .bind(STALE_AFTER_HOURS)
    .fetch_all(&state.pool)
    .await?;

    for chunk in stale.chunks(CONCURRENCY) {
        if Instant::now() >= deadline {
            break;
        }
        let mut set = tokio::task::JoinSet::new();
        for (id, kind, board, name) in chunk.iter().cloned() {
            let st = state.clone();
            set.spawn(async move { refresh_company(&st, id, &kind, &board, &name).await });
        }
        while let Some(r) = set.join_next().await {
            match r {
                Ok(Ok(())) => summary.refreshed += 1,
                Ok(Err(e)) => {
                    tracing::warn!(error = %e, "catalog board refresh failed");
                    summary.failed += 1;
                }
                Err(e) => {
                    tracing::error!(error = %e, "catalog refresh task panicked");
                    summary.failed += 1;
                }
            }
        }
    }
    if summary.refreshed + summary.failed + summary.requests_resolved > 0 {
        tracing::info!(refreshed = summary.refreshed, failed = summary.failed, requests = summary.requests_resolved, left = stale.len() - summary.refreshed - summary.failed, "catalog tick");
    }
    Ok(summary)
}

async fn refresh_company(st: &AppState, id: Uuid, kind: &str, board: &str, name: &str) -> anyhow::Result<()> {
    match sources::fetch(&st.http, kind, board, name).await {
        Ok(jobs) => Ok(store_jobs(&st.pool, id, &jobs).await?),
        Err(e) => {
            sqlx::query("UPDATE companies SET fail_count = fail_count + 1, active = fail_count + 1 < $2, last_checked_at = now() WHERE id = $1")
                .bind(id)
                .bind(MAX_FAILS)
                .execute(&st.pool)
                .await?;
            Err(e)
        }
    }
}

/// Replaces a board's stored roles with the ones it lists now.
pub async fn store_jobs(pool: &PgPool, company_id: Uuid, jobs: &[NewJob]) -> sqlx::Result<()> {
    let mut seen = HashSet::new();
    let jobs: Vec<&NewJob> = jobs.iter().filter(|j| seen.insert(j.external_id.as_str())).collect();
    let ids: Vec<&str> = jobs.iter().map(|j| j.external_id.as_str()).collect();
    let titles: Vec<&str> = jobs.iter().map(|j| j.title.as_str()).collect();
    let locations: Vec<&str> = jobs.iter().map(|j| j.location.as_str()).collect();
    let urls: Vec<&str> = jobs.iter().map(|j| j.url.as_str()).collect();
    let descriptions: Vec<String> = jobs.iter().map(|j| clean_description(&j.description)).collect();
    let posted: Vec<Option<chrono::DateTime<chrono::Utc>>> = jobs.iter().map(|j| j.posted_at).collect();

    // now() is fixed for the whole transaction, so rows not touched here are the ones that disappeared.
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO catalog_jobs (company_id, external_id, title, location, url, description, posted_at, seen_at)
         SELECT $1, t.*, now() FROM UNNEST($2::text[], $3::text[], $4::text[], $5::text[], $6::text[], $7::timestamptz[]) AS t
         ON CONFLICT (company_id, external_id) DO UPDATE SET title = EXCLUDED.title, location = EXCLUDED.location, url = EXCLUDED.url,
             description = EXCLUDED.description, posted_at = EXCLUDED.posted_at, seen_at = EXCLUDED.seen_at",
    )
    .bind(company_id)
    .bind(&ids)
    .bind(&titles)
    .bind(&locations)
    .bind(&urls)
    .bind(&descriptions)
    .bind(&posted)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM catalog_jobs WHERE company_id = $1 AND seen_at < now()").bind(company_id).execute(&mut *tx).await?;
    sqlx::query("UPDATE companies SET job_count = $2, fail_count = 0, active = true, last_checked_at = now() WHERE id = $1")
        .bind(company_id)
        .bind(jobs.len() as i32)
        .execute(&mut *tx)
        .await?;
    tx.commit().await
}

fn clean_description(s: &str) -> String {
    let s = s.replace('\0', "");
    if s.len() <= MAX_DESCRIPTION {
        return s;
    }
    let mut end = MAX_DESCRIPTION;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].to_string()
}

/// A catalog company matching a typed name or a careers link, if there is one.
pub async fn find_in_catalog(pool: &PgPool, query: &str) -> sqlx::Result<Option<Uuid>> {
    if let Some((kind, board)) = sources::parse_careers_url(query) {
        return sqlx::query_scalar("SELECT id FROM companies WHERE active AND kind = $1 AND board = $2").bind(kind).bind(board).fetch_optional(pool).await;
    }
    sqlx::query_scalar("SELECT id FROM companies WHERE active AND (lower(name) = lower($1) OR board = ANY($2)) ORDER BY job_count DESC LIMIT 1")
        .bind(query.trim())
        .bind(sources::slug_guesses(query))
        .fetch_optional(pool)
        .await
}

async fn resolve_requests(st: &AppState) -> anyhow::Result<usize> {
    let pending: Vec<(Uuid, String)> =
        sqlx::query_as("SELECT id, query FROM company_requests WHERE status = 'pending' ORDER BY created_at LIMIT $1")
            .bind(REQUESTS_PER_TICK)
            .fetch_all(&st.pool)
            .await?;
    for (id, query) in &pending {
        let company = match find_in_catalog(&st.pool, query).await? {
            Some(c) => Some(c),
            None => discover_board(st, query).await?,
        };
        sqlx::query(
            "UPDATE company_requests SET status = CASE WHEN $2::uuid IS NULL THEN 'not_found' ELSE 'added' END,
                 company_id = $2, resolved_at = now() WHERE id = $1",
        )
        .bind(id)
        .bind(company)
        .execute(&st.pool)
        .await?;
        tracing::info!(query = %query, found = company.is_some(), "company request resolved");
    }
    Ok(pending.len())
}

/// Looks for the company's board on Greenhouse, Lever and Ashby; adds the first one with open roles.
async fn discover_board(st: &AppState, query: &str) -> anyhow::Result<Option<Uuid>> {
    let (guesses, typed_name): (Vec<(String, String)>, Option<String>) = match sources::parse_careers_url(query) {
        Some(kb) => (vec![kb], None),
        None => {
            let looks_like_link = !query.trim().contains(' ') && query.contains('.');
            let slugs = sources::slug_guesses(query);
            let guesses = slugs.iter().flat_map(|s| sources::KINDS.iter().map(move |k| (k.to_string(), s.clone()))).collect();
            (guesses, (!looks_like_link).then(|| query.trim().to_string()))
        }
    };
    for (kind, board) in guesses {
        let Ok(jobs) = sources::fetch(&st.http, &kind, &board, "").await else { continue };
        if jobs.is_empty() {
            continue;
        }
        let name = match typed_name.clone() {
            Some(n) => n,
            None => board_name(st, &kind, &board).await,
        };
        let jobs: Vec<NewJob> = jobs.into_iter().map(|j| NewJob { company: name.clone(), ..j }).collect();
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO companies (id, kind, board, name, origin) VALUES ($1, $2, $3, $4, 'request')
             ON CONFLICT (kind, board) DO UPDATE SET active = true RETURNING id",
        )
        .bind(Uuid::new_v4())
        .bind(&kind)
        .bind(&board)
        .bind(&name)
        .fetch_one(&st.pool)
        .await?;
        store_jobs(&st.pool, id, &jobs).await?;
        return Ok(Some(id));
    }
    Ok(None)
}

/// Display name for a board found from a link: Greenhouse publishes it; otherwise tidy the slug.
async fn board_name(st: &AppState, kind: &str, board: &str) -> String {
    if kind == "greenhouse" {
        let url = format!("https://boards-api.greenhouse.io/v1/boards/{board}");
        if let Ok(res) = st.http.get(&url).send().await {
            if let Ok(v) = res.json::<serde_json::Value>().await {
                if let Some(n) = v["name"].as_str().filter(|n| !n.trim().is_empty()) {
                    return n.trim().to_string();
                }
            }
        }
    }
    board
        .split(['-', '_', '.'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            c.next().map(|f| f.to_uppercase().chain(c).collect::<String>()).unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}
