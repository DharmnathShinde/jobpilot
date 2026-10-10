//! Builds a per-job resume from the master profile.
//!
//! Rule: tailoring may reorder and select, never invent. Every line in the output exists in
//! the master profile; skills the job wants but the user lacks are reported, not added.

use crate::models::{ExperienceItem, Profile, ResumeChange};
use crate::skills;

pub struct Tailored {
    pub content: Profile,
    pub changes: Vec<ResumeChange>,
    pub keyword_coverage: i32,
}

pub fn tailor(profile: &Profile, job_title: &str, required_skills: &[String]) -> Tailored {
    let mut out = profile.clone();
    let mut changes = Vec::new();
    let req_lower: Vec<String> = required_skills.iter().map(|s| s.to_lowercase()).collect();
    let is_required = |s: &str| req_lower.contains(&skills::canonicalize(s).to_lowercase());

    // 1. Skills the job asks for go first.
    let (mut front, back): (Vec<String>, Vec<String>) = profile.skills.iter().cloned().partition(|s| is_required(s));
    front.sort_by_key(|s| {
        let c = skills::canonicalize(s).to_lowercase();
        req_lower.iter().position(|r| *r == c).unwrap_or(usize::MAX)
    });
    if !front.is_empty() && front.first() != profile.skills.first() {
        changes.push(ResumeChange {
            kind: "reordered".into(),
            text: format!("Listed {} first in skills", front.iter().take(3).cloned().collect::<Vec<_>>().join(", ")),
        });
    }
    out.skills = front.into_iter().chain(back).collect();

    // 2. Within each role, bullets that mention the job's skills move up.
    let mut new_exp: Vec<ExperienceItem> = Vec::with_capacity(profile.experience.len());
    for item in profile.experience.iter() {
        let mut ranked: Vec<(usize, usize, String)> = item
            .bullets
            .iter()
            .enumerate()
            .map(|(i, b)| (relevance(b, required_skills), i, b.clone()))
            .collect();
        ranked.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let bullets: Vec<String> = ranked.into_iter().map(|(_, _, b)| b).collect();
        if bullets.first() != item.bullets.first() {
            if let Some(top) = bullets.first() {
                changes.push(ResumeChange {
                    kind: "reordered".into(),
                    text: format!("Moved \"{}\" to the top of {}", short(top, 48), item.company),
                });
            }
        }
        new_exp.push(ExperienceItem { bullets, ..item.clone() });
    }

    // 3. Roles: keep chronological order (recruiters expect it), but note the most relevant one.
    out.experience = sqlx::types::Json(new_exp);

    // 4. Report gaps honestly.
    let have: Vec<String> = profile.skills.iter().map(|s| skills::canonicalize(s).to_lowercase()).collect();
    let missing: Vec<&String> = required_skills.iter().filter(|r| !have.contains(&r.to_lowercase())).collect();
    for m in &missing {
        changes.push(ResumeChange {
            kind: "not_added".into(),
            text: format!("{m} not added (not in your profile)"),
        });
    }

    let coverage = if required_skills.is_empty() {
        100
    } else {
        (100.0 * (required_skills.len() - missing.len()) as f32 / required_skills.len() as f32).round() as i32
    };

    if changes.iter().all(|c| c.kind == "not_added") {
        changes.insert(
            0,
            ResumeChange { kind: "reordered".into(), text: format!("Your profile already leads with what {job_title} needs") },
        );
    }

    Tailored { content: out, changes, keyword_coverage: coverage }
}

fn relevance(text: &str, required: &[String]) -> usize {
    let found = skills::extract_skills(text);
    found.iter().filter(|f| required.iter().any(|r| r.eq_ignore_ascii_case(f))).count()
}

fn short(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max).collect::<String>().trim_end())
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Single-column, ATS-friendly HTML. The app turns it into a PDF on device (expo-print).
pub fn render_html(p: &Profile) -> String {
    let mut h = String::new();
    h.push_str("<!doctype html><html><head><meta charset=\"utf-8\"><title>");
    h.push_str(&esc(&p.full_name));
    h.push_str("</title><style>body{font-family:Helvetica,Arial,sans-serif;color:#111;max-width:720px;margin:32px auto;padding:0 24px;font-size:11pt;line-height:1.4}h1{font-size:20pt;margin:0}h2{font-size:11pt;text-transform:uppercase;letter-spacing:.06em;color:#3730A3;border-bottom:1px solid #ddd;padding-bottom:2px;margin:18px 0 6px}.meta{color:#444;margin:2px 0 0}.role{display:flex;justify-content:space-between;font-weight:bold;margin-top:8px}ul{margin:4px 0 0 18px;padding:0}li{margin:2px 0}</style></head><body>");
    h.push_str(&format!("<h1>{}</h1>", esc(&p.full_name)));
    let contact: Vec<String> = [&p.headline, &p.location, &p.email, &p.phone]
        .iter()
        .filter(|s| !s.is_empty())
        .map(|s| esc(s))
        .chain(p.links.iter().map(|l| esc(&l.url)))
        .collect();
    h.push_str(&format!("<p class=\"meta\">{}</p>", contact.join(" · ")));
    if !p.summary.is_empty() {
        h.push_str(&format!("<h2>Summary</h2><p>{}</p>", esc(&p.summary)));
    }
    if !p.skills.is_empty() {
        h.push_str(&format!("<h2>Skills</h2><p>{}</p>", p.skills.iter().map(|s| esc(s)).collect::<Vec<_>>().join(", ")));
    }
    if !p.experience.is_empty() {
        h.push_str("<h2>Experience</h2>");
        for e in p.experience.iter() {
            let dates = [e.start.as_str(), e.end.as_str()].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(" – ");
            h.push_str(&format!("<div class=\"role\"><span>{}, {}</span><span>{}</span></div>", esc(&e.role), esc(&e.company), esc(&dates)));
            if !e.bullets.is_empty() {
                h.push_str("<ul>");
                for b in &e.bullets {
                    h.push_str(&format!("<li>{}</li>", esc(b)));
                }
                h.push_str("</ul>");
            }
        }
    }
    if !p.projects.is_empty() {
        h.push_str("<h2>Projects</h2><ul>");
        for pr in p.projects.iter() {
            h.push_str(&format!("<li><strong>{}</strong> {}</li>", esc(&pr.name), esc(&pr.description)));
        }
        h.push_str("</ul>");
    }
    if !p.education.is_empty() {
        h.push_str("<h2>Education</h2>");
        for ed in p.education.iter() {
            h.push_str(&format!("<p>{} — {} {}</p>", esc(&ed.degree), esc(&ed.institution), esc(&ed.year)));
        }
    }
    h.push_str("</body></html>");
    h
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::types::Json;

    fn profile() -> Profile {
        Profile {
            full_name: "Aarav Sharma".into(),
            skills: vec!["Jest".into(), "Redux".into(), "React Native".into(), "TypeScript".into()],
            experience: Json(vec![ExperienceItem {
                company: "Finlo".into(),
                role: "Mobile Engineer".into(),
                start: "2022".into(),
                end: "now".into(),
                bullets: vec![
                    "Mentored two interns".into(),
                    "Rebuilt the checkout in TypeScript, cutting drop-off by 18%".into(),
                ],
            }]),
            ..Default::default()
        }
    }

    #[test]
    fn reorders_without_inventing() {
        let req = vec!["React Native".to_string(), "TypeScript".to_string(), "GraphQL".to_string()];
        let t = tailor(&profile(), "Senior RN Engineer", &req);
        assert_eq!(t.content.skills, vec!["React Native", "TypeScript", "Jest", "Redux"]);
        assert_eq!(t.content.experience[0].bullets[0], "Rebuilt the checkout in TypeScript, cutting drop-off by 18%");
        // nothing invented
        assert!(!t.content.skills.iter().any(|s| s == "GraphQL"));
        assert_eq!(t.content.experience[0].bullets.len(), 2);
        assert!(t.changes.iter().any(|c| c.kind == "not_added" && c.text.starts_with("GraphQL")));
        assert_eq!(t.keyword_coverage, 67);
    }

    #[test]
    fn html_escapes() {
        let mut p = profile();
        p.full_name = "<script>".into();
        assert!(render_html(&p).contains("&lt;script&gt;"));
    }
}
