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

/// Expands a user location preference into related place strings (Dubai ↔ UAE, etc.).
fn location_synonyms(pref: &str) -> Vec<String> {
    let p = pref.to_lowercase();
    let mut out = vec![p.clone()];
    let uae = ["dubai", "uae", "united arab emirates", "abu dhabi", "sharjah", "ajman", "ras al khaimah", "fujairah", "middle east", "gcc"];
    if uae.iter().any(|s| p.contains(s)) {
        out.extend(uae.iter().map(|s| (*s).to_string()));
    }
    let india = ["bengaluru", "bangalore", "mumbai", "pune", "hyderabad", "delhi", "noida", "gurgaon", "gurugram", "chennai", "india"];
    if india.iter().any(|s| p.contains(s)) {
        if p.contains("bengaluru") || p.contains("bangalore") {
            out.push("bengaluru".into());
            out.push("bangalore".into());
        }
    }
    out.sort();
    out.dedup();
    out
}

fn location_matches(job_loc: &str, pref: &str) -> bool {
    let loc = job_loc.to_lowercase();
    location_synonyms(pref).iter().any(|s| !s.is_empty() && loc.contains(s.as_str()))
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
        || locations.iter().any(|l| location_matches(&loc, l))
        || loc.is_empty();
    role_ok && loc_ok
}

pub const KINDS: [&str; 3] = ["greenhouse", "lever", "ashby"];

/// Pulls the platform and board slug out of a Greenhouse, Lever or Ashby careers link.
pub fn parse_careers_url(text: &str) -> Option<(String, String)> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"(?i)(greenhouse\.io|lever\.co|ashbyhq\.com)/(?:embed/job_board\?for=)?([A-Za-z0-9_.-]+)").unwrap());
    let c = re.captures(text)?;
    let host = c[1].to_lowercase();
    let kind = if host.starts_with("greenhouse") { "greenhouse" } else if host.starts_with("lever") { "lever" } else { "ashby" };
    Some((kind.to_string(), c[2].to_lowercase()))
}

/// Likely board slugs for a company name ("Acme Labs Pvt Ltd") or its own site ("acme.com/careers").
pub fn slug_guesses(query: &str) -> Vec<String> {
    static HOST: OnceLock<Regex> = OnceLock::new();
    let host = HOST.get_or_init(|| Regex::new(r"^(?:https?://)?(?:www\.|careers\.|jobs\.)?([a-z0-9-]+)\.[a-z.]{2,}(?:/|$)").unwrap());
    const SUFFIXES: [&str; 14] = ["inc", "ltd", "limited", "pvt", "private", "technologies", "technology", "labs", "corp", "corporation", "co", "software", "hq", "india"];

    let q = query.trim().to_lowercase();
    if !q.contains(' ') {
        if let Some(c) = host.captures(&q) {
            return vec![c[1].to_string()];
        }
    }
    let words: Vec<String> = q
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(String::from)
        .collect();
    let core: Vec<&str> = words.iter().map(String::as_str).filter(|w| !SUFFIXES.contains(w)).collect();
    let mut out: Vec<String> = Vec::new();
    for g in [words.concat(), core.concat(), core.join("-"), core.first().map(|w| w.to_string()).unwrap_or_default()] {
        if g.len() >= 2 && !out.contains(&g) {
            out.push(g);
        }
    }
    out
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
    fn careers_links_and_slugs() {
        assert_eq!(parse_careers_url("https://boards.greenhouse.io/Razorpay/jobs/123"), Some(("greenhouse".into(), "razorpay".into())));
        assert_eq!(parse_careers_url("jobs.lever.co/cred"), Some(("lever".into(), "cred".into())));
        assert_eq!(parse_careers_url("https://jobs.ashbyhq.com/ramp?x=1"), Some(("ashby".into(), "ramp".into())));
        assert_eq!(parse_careers_url("Acme"), None);
        assert_eq!(slug_guesses("https://www.razorpay.com/careers"), vec!["razorpay"]);
        assert_eq!(slug_guesses("Urban Company"), vec!["urbancompany", "urban-company", "urban"]);
        assert_eq!(slug_guesses("Acme Labs Pvt Ltd"), vec!["acmelabspvtltd", "acme"]);
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
        let remote = NewJob { location: "Remote - India".into(), ..job.clone() };
        assert!(passes_filters(&remote, &roles, &["Pune".into()], true));
        assert!(!passes_filters(&remote, &roles, &["Pune".into()], false));
        let dubai = NewJob { location: "Dubai, UAE".into(), ..job };
        assert!(passes_filters(&dubai, &roles, &["UAE".into()], true));
        assert!(passes_filters(&dubai, &roles, &["Abu Dhabi".into()], true));
    }
}
