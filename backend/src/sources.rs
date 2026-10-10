//! Job discovery from public, documented company job-board APIs.
//!
//! Greenhouse, Lever and Ashby publish each company's open roles as JSON for anyone to read.
//! LinkedIn, Naukri and Indeed are deliberately NOT scraped here (their terms forbid it);
//! users add those jobs via the share sheet or, later, job-alert emails.

use std::sync::OnceLock;

use anyhow::{anyhow, Context};
use chrono::{DateTime, TimeZone, Utc};
use regex::Regex;
use serde_json::Value;

use crate::models::NewJob;

pub async fn fetch(http: &reqwest::Client, kind: &str, board: &str, company: &str) -> anyhow::Result<Vec<NewJob>> {
    let board_enc: String = board.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_' || *c == '.').collect();
    if board_enc.is_empty() {
        return Err(anyhow!("invalid board name"));
    }
    let url = match kind {
        "greenhouse" => format!("https://boards-api.greenhouse.io/v1/boards/{board_enc}/jobs?content=true"),
        "lever" => format!("https://api.lever.co/v0/postings/{board_enc}?mode=json"),
        "ashby" => format!("https://api.ashbyhq.com/posting-api/job-board/{board_enc}"),
        other => return Err(anyhow!("unknown source kind {other}")),
    };
    let res = http.get(&url).send().await.with_context(|| format!("fetching {url}"))?;
    if !res.status().is_success() {
        return Err(anyhow!("{kind} board '{board}' returned {}", res.status()));
    }
    let body: Value = res.json().await.context("board response was not JSON")?;
    let company = if company.is_empty() { board } else { company };
    Ok(match kind {
        "greenhouse" => parse_greenhouse(&body, company),
        "lever" => parse_lever(&body, company),
        _ => parse_ashby(&body, company),
    })
}

pub fn parse_greenhouse(body: &Value, company: &str) -> Vec<NewJob> {
    body["jobs"]
        .as_array()
        .map(|jobs| {
            jobs.iter()
                .filter_map(|j| {
                    Some(NewJob {
                        source: "greenhouse".into(),
                        external_id: j["id"].as_i64().map(|i| i.to_string()).or_else(|| j["id"].as_str().map(String::from))?,
                        title: j["title"].as_str()?.trim().to_string(),
                        company: company.to_string(),
                        location: j["location"]["name"].as_str().unwrap_or("").to_string(),
                        url: j["absolute_url"].as_str().unwrap_or("").to_string(),
                        // Greenhouse returns HTML-escaped HTML in `content`.
                        description: html_to_text(&unescape(j["content"].as_str().unwrap_or(""))),
                        posted_at: j["updated_at"].as_str().and_then(parse_rfc3339),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn parse_lever(body: &Value, company: &str) -> Vec<NewJob> {
    body.as_array()
        .map(|jobs| {
            jobs.iter()
                .filter_map(|j| {
                    let mut desc = j["descriptionPlain"].as_str().unwrap_or("").to_string();
                    if let Some(lists) = j["lists"].as_array() {
                        for l in lists {
                            desc.push_str("\n\n");
                            desc.push_str(l["text"].as_str().unwrap_or(""));
                            desc.push('\n');
                            desc.push_str(&html_to_text(l["content"].as_str().unwrap_or("")));
                        }
                    }
                    Some(NewJob {
                        source: "lever".into(),
                        external_id: j["id"].as_str()?.to_string(),
                        title: j["text"].as_str()?.trim().to_string(),
                        company: company.to_string(),
                        location: j["categories"]["location"].as_str().unwrap_or("").to_string(),
                        url: j["hostedUrl"].as_str().unwrap_or("").to_string(),
                        description: desc.trim().to_string(),
                        posted_at: j["createdAt"].as_i64().and_then(|ms| Utc.timestamp_millis_opt(ms).single()),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn parse_ashby(body: &Value, company: &str) -> Vec<NewJob> {
    body["jobs"]
        .as_array()
        .map(|jobs| {
            jobs.iter()
                .filter(|j| j["isListed"].as_bool().unwrap_or(true))
                .filter_map(|j| {
                    let location = match (j["location"].as_str(), j["isRemote"].as_bool()) {
                        (Some(l), Some(true)) => format!("{l} (Remote)"),
                        (Some(l), _) => l.to_string(),
                        (None, Some(true)) => "Remote".into(),
                        _ => String::new(),
                    };
                    Some(NewJob {
                        source: "ashby".into(),
                        external_id: j["id"].as_str()?.to_string(),
                        title: j["title"].as_str()?.trim().to_string(),
                        company: company.to_string(),
                        location,
                        url: j["jobUrl"].as_str().unwrap_or("").to_string(),
                        description: j["descriptionPlain"]
                            .as_str()
                            .map(String::from)
                            .unwrap_or_else(|| html_to_text(j["descriptionHtml"].as_str().unwrap_or(""))),
                        posted_at: j["publishedAt"].as_str().and_then(parse_rfc3339),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Keeps jobs whose title matches a target role and whose location fits the preferences.
pub fn passes_filters(job: &NewJob, target_roles: &[String], locations: &[String], remote_ok: bool) -> bool {
    let title = job.title.to_lowercase();
    let role_ok = target_roles.is_empty()
        || target_roles.iter().any(|r| {
            let words: Vec<String> = r.to_lowercase().split_whitespace().filter(|w| w.len() > 2).map(String::from).collect();
            // every meaningful word of the role appears in the title ("react native engineer" ⊂ "senior react native engineer")
            !words.is_empty() && words.iter().all(|w| title.contains(w.as_str()))
        });
    let loc = job.location.to_lowercase();
    let is_remote = loc.contains("remote") || loc.contains("anywhere");
    let loc_ok = locations.is_empty()
        || (remote_ok && is_remote)
        || locations.iter().any(|l| loc.contains(&l.to_lowercase()))
        || (loc.is_empty());
    role_ok && loc_ok
}

fn parse_rfc3339(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc))
}

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'").replace("&amp;", "&")
}

pub fn html_to_text(html: &str) -> String {
    static BLOCK: OnceLock<Regex> = OnceLock::new();
    static TAG: OnceLock<Regex> = OnceLock::new();
    static SPACES: OnceLock<Regex> = OnceLock::new();
    let block = BLOCK.get_or_init(|| Regex::new(r"(?i)</?(p|div|br|li|ul|ol|h[1-6])[^>]*>").unwrap());
    let tag = TAG.get_or_init(|| Regex::new(r"<[^>]+>").unwrap());
    let spaces = SPACES.get_or_init(|| Regex::new(r"\n\s*\n\s*\n+").unwrap());
    let t = block.replace_all(html, "\n");
    let t = tag.replace_all(&t, "");
    let t = t.replace("&nbsp;", " ").replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&#39;", "'").replace("&quot;", "\"");
    spaces.replace_all(t.trim(), "\n\n").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn greenhouse() {
        let body = json!({"jobs": [{
            "id": 4012345,
            "title": "Senior React Native Engineer",
            "absolute_url": "https://boards.greenhouse.io/acme/jobs/4012345",
            "location": {"name": "Bengaluru, India"},
            "updated_at": "2026-10-08T10:00:00-04:00",
            "content": "&lt;p&gt;We use &lt;strong&gt;React Native&lt;/strong&gt; &amp;amp; TypeScript.&lt;/p&gt;"
        }]});
        let jobs = parse_greenhouse(&body, "Acme");
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].external_id, "4012345");
        assert_eq!(jobs[0].description, "We use React Native & TypeScript.");
        assert_eq!(jobs[0].location, "Bengaluru, India");
        assert!(jobs[0].posted_at.is_some());
    }

    #[test]
    fn lever() {
        let body = json!([{
            "id": "abc-123",
            "text": "Backend Engineer (Rust)",
            "hostedUrl": "https://jobs.lever.co/acme/abc-123",
            "categories": {"location": "Pune", "commitment": "Full-time"},
            "descriptionPlain": "Build services in Rust.",
            "lists": [{"text": "Requirements", "content": "<li>3+ years</li><li>PostgreSQL</li>"}],
            "createdAt": 1791600000000i64
        }]);
        let jobs = parse_lever(&body, "Acme");
        assert_eq!(jobs[0].title, "Backend Engineer (Rust)");
        assert!(jobs[0].description.contains("PostgreSQL"));
        assert!(jobs[0].description.contains("Requirements"));
    }

    #[test]
    fn ashby() {
        let body = json!({"jobs": [
            {"id": "u1", "title": "Mobile Engineer", "location": "Mumbai", "isRemote": true, "jobUrl": "https://jobs.ashbyhq.com/acme/u1", "descriptionPlain": "React Native", "publishedAt": "2026-10-01T00:00:00Z", "isListed": true},
            {"id": "u2", "title": "Hidden", "isListed": false}
        ]});
        let jobs = parse_ashby(&body, "Acme");
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].location, "Mumbai (Remote)");
    }

    #[test]
    fn filters() {
        let job = NewJob {
            source: "lever".into(),
            external_id: "1".into(),
            title: "Senior React Native Engineer".into(),
            company: "Acme".into(),
            location: "Bengaluru".into(),
            url: String::new(),
            description: String::new(),
            posted_at: None,
        };
        let roles = vec!["React Native Engineer".to_string()];
        assert!(passes_filters(&job, &roles, &["Bengaluru".into()], true));
        assert!(!passes_filters(&job, &roles, &["Pune".into()], true));
        assert!(!passes_filters(&job, &["Data Scientist".into()], &[], true));
        let remote = NewJob { location: "Remote - India".into(), ..job };
        assert!(passes_filters(&remote, &roles, &["Pune".into()], true));
        assert!(!passes_filters(&remote, &roles, &["Pune".into()], false));
    }
}
