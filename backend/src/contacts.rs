//! Extract recruiter / HR emails from job descriptions (no paid enrichment).

use std::sync::OnceLock;

use regex::Regex;

fn email_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)(?:mailto:)?([a-z0-9._%+\-]+@[a-z0-9.\-]+\.[a-z]{2,})").expect("email regex"))
}

fn context_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)(?:apply\s+(?:to|at)|send\s+(?:your\s+)?(?:cv|resume|application)\s+to|email\s*(?:us|to)?|contact|reach\s+out)\s*[:\-]?\s*(?:mailto:)?([a-z0-9._%+\-]+@[a-z0-9.\-]+\.[a-z]{2,})",
        )
        .expect("context email regex")
    })
}

fn is_noise(email: &str) -> bool {
    let local = email.split('@').next().unwrap_or("").to_lowercase();
    let domain = email.split('@').nth(1).unwrap_or("").to_lowercase();
    let noise_local = [
        "noreply", "no-reply", "donotreply", "do-not-reply", "mailer-daemon", "bounce", "notifications", "newsletter",
        "unsubscribe",
    ];
    if noise_local.iter().any(|n| local.contains(n)) {
        return true;
    }
    let noise_domain = ["example.com", "email.com", "sentry.io", "github.com", "linkedin.com", "indeed.com"];
    noise_domain.iter().any(|d| domain == *d || domain.ends_with(&format!(".{d}")))
}

fn preference_score(email: &str) -> i32 {
    let lower = email.to_lowercase();
    let local = lower.split('@').next().unwrap_or("");
    let mut score = 0;
    for (needle, pts) in [
        ("careers", 50),
        ("talent", 45),
        ("recruit", 45),
        ("hiring", 40),
        ("hr", 35),
        ("jobs", 30),
        ("people", 25),
        ("apply", 20),
    ] {
        if local.contains(needle) || lower.contains(needle) {
            score += pts;
        }
    }
    score
}

/// Best single HR/recruiter email from a job description, if any.
pub fn extract_hr_email(description: &str) -> Option<String> {
    if description.trim().is_empty() {
        return None;
    }

    let mut best: Option<(i32, String)> = None;

    for cap in context_re().captures_iter(description) {
        let Some(m) = cap.get(1) else { continue };
        let email = m.as_str().to_lowercase();
        if is_noise(&email) {
            continue;
        }
        let score = preference_score(&email) + 100;
        if best.as_ref().map(|(s, _)| score > *s).unwrap_or(true) {
            best = Some((score, email));
        }
    }

    for cap in email_re().captures_iter(description) {
        let Some(m) = cap.get(1) else { continue };
        let email = m.as_str().to_lowercase();
        if is_noise(&email) {
            continue;
        }
        let score = preference_score(&email);
        if best.as_ref().map(|(s, _)| score > *s).unwrap_or(true) {
            best = Some((score, email));
        }
    }

    best.map(|(_, e)| e)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_mailto_and_prefers_hr() {
        let d = "Apply via the form. Questions: mailto:hr@acme.com or noreply@acme.com";
        assert_eq!(extract_hr_email(d).as_deref(), Some("hr@acme.com"));
    }

    #[test]
    fn prefers_contextual_apply_to() {
        let d = "Send your CV to talent@startup.io. Also see info@startup.io for office hours.";
        assert_eq!(extract_hr_email(d).as_deref(), Some("talent@startup.io"));
    }

    #[test]
    fn skips_noise() {
        assert!(extract_hr_email("Contact noreply@boards.greenhouse.io").is_none());
    }
}
