//! Optional free-tier job aggregators (Adzuna + JSearch). Skipped when API keys are unset.

use anyhow::{anyhow, Context};
use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::config::Config;
use crate::models::{NewJob, Preferences};

fn parse_dt(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc)).or_else(|| {
        chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%SZ")
            .ok()
            .map(|n| n.and_utc())
            .or_else(|| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok().map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc()))
    })
}

/// Search phrases for aggregator queries: each target role × each location (plus empty for remote).
fn search_queries(prefs: &Preferences) -> Vec<(String, String)> {
    let roles = if prefs.target_roles.is_empty() {
        vec!["software engineer".into()]
    } else {
        prefs.target_roles.clone()
    };
    let mut locs: Vec<String> = prefs.locations.clone();
    if locs.is_empty() || prefs.remote_ok {
        if !locs.iter().any(|l| l.eq_ignore_ascii_case("remote")) {
            locs.push(String::new());
        }
    }
    if locs.is_empty() {
        locs.push(String::new());
    }
    let mut out = Vec::new();
    for r in &roles {
        for l in &locs {
            out.push((r.clone(), l.clone()));
        }
    }
    out.truncate(12); // stay within free-tier call budgets
    out
}

pub async fn fetch_all(http: &reqwest::Client, config: &Config, prefs: &Preferences) -> (Vec<NewJob>, Vec<String>) {
    let mut jobs = Vec::new();
    let mut errors = Vec::new();
    if config.adzuna_app_id.is_some() && config.adzuna_app_key.is_some() {
        match fetch_adzuna(http, config, prefs).await {
            Ok(mut j) => jobs.append(&mut j),
            Err(e) => {
                tracing::warn!(error = %e, "adzuna fetch failed");
                errors.push(format!("adzuna: {e}"));
            }
        }
    }
    if config.jsearch_api_key.is_some() {
        match fetch_jsearch(http, config, prefs).await {
            Ok(mut j) => jobs.append(&mut j),
            Err(e) => {
                tracing::warn!(error = %e, "jsearch fetch failed");
                errors.push(format!("jsearch: {e}"));
            }
        }
    }
    (jobs, errors)
}

async fn fetch_adzuna(http: &reqwest::Client, config: &Config, prefs: &Preferences) -> anyhow::Result<Vec<NewJob>> {
    let app_id = config.adzuna_app_id.as_deref().ok_or_else(|| anyhow!("missing adzuna id"))?;
    let app_key = config.adzuna_app_key.as_deref().ok_or_else(|| anyhow!("missing adzuna key"))?;
    let country = config.adzuna_country.as_str();
    let mut out = Vec::new();
    for (role, loc) in search_queries(prefs) {
        let mut url = reqwest::Url::parse(&format!("https://api.adzuna.com/v1/api/jobs/{country}/search/1"))?;
        {
            let mut q = url.query_pairs_mut();
            q.append_pair("app_id", app_id);
            q.append_pair("app_key", app_key);
            q.append_pair("results_per_page", "50");
            q.append_pair("what", &role);
            if !loc.is_empty() {
                q.append_pair("where", &loc);
            }
            q.append_pair("content-type", "application/json");
        }
        let res = http.get(url).send().await.context("adzuna request")?;
        if res.status().as_u16() == 429 {
            return Err(anyhow!("rate limited"));
        }
        if !res.status().is_success() {
            return Err(anyhow!("HTTP {}", res.status()));
        }
        let body: Value = res.json().await.context("adzuna json")?;
        for j in body["results"].as_array().cloned().unwrap_or_default() {
            let id = j["id"].as_i64().map(|n| n.to_string()).or_else(|| j["id"].as_str().map(|s| s.to_string())).unwrap_or_default();
            if id.is_empty() {
                continue;
            }
            let title = j["title"].as_str().unwrap_or("").to_string();
            let company = j["company"]["display_name"].as_str().unwrap_or("").to_string();
            let location = j["location"]["display_name"].as_str().unwrap_or("").to_string();
            let url = j["redirect_url"].as_str().unwrap_or("").to_string();
            let description = j["description"].as_str().unwrap_or("").to_string();
            let posted_at = j["created"].as_str().and_then(parse_dt);
            out.push(NewJob {
                source: "adzuna".into(),
                external_id: id,
                title,
                company,
                location,
                url,
                description,
                posted_at,
            });
        }
    }
    Ok(out)
}

async fn fetch_jsearch(http: &reqwest::Client, config: &Config, prefs: &Preferences) -> anyhow::Result<Vec<NewJob>> {
    let key = config.jsearch_api_key.as_deref().ok_or_else(|| anyhow!("missing jsearch key"))?;
    let mut out = Vec::new();
    for (role, loc) in search_queries(prefs) {
        let query = if loc.is_empty() {
            format!("{role} remote")
        } else {
            format!("{role} in {loc}")
        };
        let mut url = reqwest::Url::parse("https://jsearch.p.rapidapi.com/search")?;
        {
            let mut q = url.query_pairs_mut();
            q.append_pair("query", &query);
            q.append_pair("page", "1");
            q.append_pair("num_pages", "1");
            q.append_pair("date_posted", "month");
        }
        let res = http
            .get(url)
            .header("x-rapidapi-key", key)
            .header("x-rapidapi-host", "jsearch.p.rapidapi.com")
            .send()
            .await
            .context("jsearch request")?;
        if res.status().as_u16() == 429 {
            return Err(anyhow!("rate limited"));
        }
        if !res.status().is_success() {
            return Err(anyhow!("HTTP {}", res.status()));
        }
        let body: Value = res.json().await.context("jsearch json")?;
        for j in body["data"].as_array().cloned().unwrap_or_default() {
            let id = j["job_id"].as_str().unwrap_or("").to_string();
            if id.is_empty() {
                continue;
            }
            let title = j["job_title"].as_str().unwrap_or("").to_string();
            let company = j["employer_name"].as_str().unwrap_or("").to_string();
            let location = [
                j["job_city"].as_str().unwrap_or(""),
                j["job_state"].as_str().unwrap_or(""),
                j["job_country"].as_str().unwrap_or(""),
            ]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(", ");
            let location = if j["job_is_remote"].as_bool() == Some(true) && !location.to_lowercase().contains("remote") {
                if location.is_empty() {
                    "Remote".into()
                } else {
                    format!("{location} (Remote)")
                }
            } else {
                location
            };
            let url = j["job_apply_link"]
                .as_str()
                .or_else(|| j["job_google_link"].as_str())
                .unwrap_or("")
                .to_string();
            let description = j["job_description"].as_str().unwrap_or("").to_string();
            let posted_at = j["job_posted_at_datetime_utc"].as_str().and_then(parse_dt);
            out.push(NewJob {
                source: "jsearch".into(),
                external_id: id,
                title,
                company,
                location,
                url,
                description,
                posted_at,
            });
        }
    }
    Ok(out)
}
