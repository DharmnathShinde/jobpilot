//! Per-user morning runs. Every minute, refresh stale catalog boards, then find users whose
//! local run time has passed today and who haven't run yet, and run discovery for each.

use std::time::Duration;

use chrono::{Datelike, NaiveDate, NaiveTime, Utc};
use chrono_tz::Tz;
use uuid::Uuid;

use crate::{catalog, discovery};
use crate::state::AppState;

pub fn spawn(state: AppState) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(60));
        loop {
            tick.tick().await;
            if let Err(e) = tick_once(&state).await {
                tracing::error!(error = %e, "scheduler tick failed");
            }
        }
    });
}

/// Decides whether a user is due. Pure, so it can be tested without a clock or database.
pub fn is_due(
    now_utc: chrono::DateTime<Utc>,
    tz_name: &str,
    run_time: NaiveTime,
    active_days: &[i32],
    last_run_on: Option<NaiveDate>,
) -> Option<NaiveDate> {
    let tz: Tz = tz_name.parse().unwrap_or(chrono_tz::Asia::Kolkata);
    let local = now_utc.with_timezone(&tz);
    let today = local.date_naive();
    let weekday = local.weekday().number_from_monday() as i32;
    let due = active_days.contains(&weekday) && local.time() >= run_time && last_run_on.map_or(true, |d| d < today);
    due.then_some(today)
}

/// Finds due users and runs discovery for each. Returns how many runs started.
/// `wait` = true runs them inline (used by the cron endpoint so the host stays awake until done).
pub async fn tick_once_with(state: &AppState, wait: bool) -> anyhow::Result<usize> {
    let rows: Vec<(Uuid, String, NaiveTime, Vec<i32>, Option<NaiveDate>)> = sqlx::query_as(
        "SELECT p.user_id, p.timezone, p.run_time, p.active_days, p.last_run_on
         FROM preferences p JOIN profiles pr ON pr.user_id = p.user_id
         WHERE p.schedule_enabled AND pr.confirmed",
    )
    .fetch_all(&state.pool)
    .await?;

    let now = Utc::now();
    let mut started = 0;
    for (user_id, tz, run_time, days, last) in rows {
        let Some(today) = is_due(now, &tz, run_time, &days, last) else { continue };
        // Claim the run first so a slow run is never started twice.
        let claimed = sqlx::query("UPDATE preferences SET last_run_on = $2 WHERE user_id = $1 AND (last_run_on IS NULL OR last_run_on < $2)")
            .bind(user_id)
            .bind(today)
            .execute(&state.pool)
            .await?
            .rows_affected();
        if claimed == 0 {
            continue;
        }
        started += 1;
        if wait {
            if let Err(e) = discovery::run_for_user(state, user_id, "schedule").await {
                tracing::error!(user = %user_id, error = %e, "scheduled run failed");
            }
        } else {
            let st = state.clone();
            tokio::spawn(async move {
                if let Err(e) = discovery::run_for_user(&st, user_id, "schedule").await {
                    tracing::error!(user = %user_id, error = %e, "scheduled run failed");
                }
            });
        }
    }
    Ok(started)
}

/// Catalog refresh first, so morning runs score fresh roles. A refresh that runs out of
/// `budget` carries on at the next tick.
pub async fn refresh_catalog(state: &AppState, budget: Duration) -> Option<catalog::TickSummary> {
    if !state.config.catalog_refresh {
        return None;
    }
    match catalog::tick(state, budget).await {
        Ok(s) => Some(s),
        Err(e) => {
            tracing::error!(error = %e, "catalog refresh failed");
            None
        }
    }
}

pub async fn tick_once(state: &AppState) -> anyhow::Result<()> {
    refresh_catalog(state, Duration::from_secs(45)).await;
    tick_once_with(state, false).await.map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn due_after_local_run_time_once_a_day() {
        let seven = NaiveTime::from_hms_opt(7, 0, 0).unwrap();
        let weekdays = [1, 2, 3, 4, 5];
        // Mon 12 Oct 2026, 01:00 UTC = 06:30 IST → not yet
        let early = Utc.with_ymd_and_hms(2026, 10, 12, 1, 0, 0).unwrap();
        assert_eq!(is_due(early, "Asia/Kolkata", seven, &weekdays, None), None);
        // 02:00 UTC = 07:30 IST → due
        let later = Utc.with_ymd_and_hms(2026, 10, 12, 2, 0, 0).unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 10, 12).unwrap();
        assert_eq!(is_due(later, "Asia/Kolkata", seven, &weekdays, None), Some(today));
        // already ran today → not due
        assert_eq!(is_due(later, "Asia/Kolkata", seven, &weekdays, Some(today)), None);
        // Saturday 10 Oct → not an active day
        let sat = Utc.with_ymd_and_hms(2026, 10, 10, 4, 0, 0).unwrap();
        assert_eq!(is_due(sat, "Asia/Kolkata", seven, &weekdays, None), None);
    }
}
