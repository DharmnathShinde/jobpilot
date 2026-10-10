//! CV upload: file → text → draft master profile.
//! The draft is always unconfirmed; the user reviews it in the app before anything uses it.

use std::io::Read;
use std::sync::OnceLock;

use anyhow::{anyhow, Context};
use regex::Regex;
use sqlx::types::Json;

use crate::llm::Llm;
use crate::models::{EducationItem, ExperienceItem, LinkItem, Profile, ProjectItem};
use crate::skills;

pub fn extract_text(filename: &str, bytes: &[u8]) -> anyhow::Result<String> {
    let lower = filename.to_lowercase();
    let text = if lower.ends_with(".pdf") || bytes.starts_with(b"%PDF") {
        pdf_extract::extract_text_from_mem(bytes).map_err(|e| anyhow!("could not read PDF: {e}"))?
    } else if lower.ends_with(".docx") || bytes.starts_with(b"PK") {
        docx_text(bytes)?
    } else {
        String::from_utf8(bytes.to_vec()).context("file is not PDF, DOCX or UTF-8 text")?
    };
    let text = text.replace('\u{0}', "");
    if text.trim().len() < 40 {
        return Err(anyhow!("couldn't find readable text in this file; if it's a scanned PDF, upload a text-based one"));
    }
    Ok(text)
}

fn docx_text(bytes: &[u8]) -> anyhow::Result<String> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).context("not a valid DOCX file")?;
    let mut xml = String::new();
    zip.by_name("word/document.xml").context("DOCX has no document body")?.read_to_string(&mut xml)?;
    // Paragraph ends become newlines, then strip every tag.
    let xml = xml.replace("</w:p>", "\n").replace("<w:tab/>", "\t").replace("<w:br/>", "\n");
    static TAG: OnceLock<Regex> = OnceLock::new();
    let tag = TAG.get_or_init(|| Regex::new(r"<[^>]+>").unwrap());
    let text = tag.replace_all(&xml, "");
    Ok(text.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'"))
}

/// Rule-based draft: name, contact details, skills and years. Experience is left for the user.
pub fn heuristic_profile(text: &str) -> Profile {
    static EMAIL: OnceLock<Regex> = OnceLock::new();
    static PHONE: OnceLock<Regex> = OnceLock::new();
    static YEARS: OnceLock<Regex> = OnceLock::new();
    static URL: OnceLock<Regex> = OnceLock::new();
    let email = EMAIL.get_or_init(|| Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}").unwrap());
    let phone = PHONE.get_or_init(|| Regex::new(r"(?:\+91[\s-]?)?[6-9]\d{4}[\s-]?\d{5}").unwrap());
    let years = YEARS.get_or_init(|| Regex::new(r"(?i)(\d{1,2})(?:\.\d)?\+?\s*(?:years?|yrs?)").unwrap());
    let url = URL.get_or_init(|| Regex::new(r"(?i)\b(?:https?://)?(?:www\.)?((?:github\.com|linkedin\.com/in|gitlab\.com)/[A-Za-z0-9_-]+)").unwrap());

    let lines: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let full_name = lines
        .iter()
        .find(|l| {
            let words = l.split_whitespace().count();
            (2..=4).contains(&words) && l.chars().all(|c| c.is_alphabetic() || c.is_whitespace() || c == '.')
        })
        .map(|s| s.to_string())
        .unwrap_or_default();
    let headline = lines
        .iter()
        .skip_while(|l| **l != full_name)
        .nth(1)
        .filter(|l| l.len() < 80 && !email.is_match(l))
        .map(|s| s.to_string())
        .unwrap_or_default();

    let years_experience = years
        .captures_iter(text)
        .filter_map(|c| c[1].parse::<f32>().ok())
        .filter(|y| *y <= 40.0)
        .fold(0.0f32, f32::max);

    let links = url
        .captures_iter(text)
        .map(|c| {
            let u = c[1].to_string();
            let label = if u.contains("github") { "GitHub" } else if u.contains("linkedin") { "LinkedIn" } else { "GitLab" };
            LinkItem { label: label.into(), url: format!("https://{u}") }
        })
        .fold(Vec::<LinkItem>::new(), |mut acc, l| {
            if !acc.iter().any(|a| a.url == l.url) {
                acc.push(l);
            }
            acc
        });

    Profile {
        full_name,
        headline,
        email: email.find(text).map(|m| m.as_str().to_string()).unwrap_or_default(),
        phone: phone.find(text).map(|m| m.as_str().to_string()).unwrap_or_default(),
        years_experience,
        skills: skills::extract_skills(text),
        links: Json(links),
        raw_cv_text: text.to_string(),
        confirmed: false,
        ..Default::default()
    }
}

const CV_SYSTEM: &str = "You extract structured data from a CV. Use ONLY facts written in the CV. \
Never invent, infer or embellish companies, dates, numbers, skills or achievements. \
Leave a field empty if the CV does not state it. Reply with one JSON object and nothing else.";

const CV_SCHEMA: &str = r#"{
  "full_name": "", "headline": "", "location": "", "email": "", "phone": "",
  "summary": "", "years_experience": 0,
  "skills": [""],
  "experience": [{"company": "", "role": "", "start": "", "end": "", "bullets": [""]}],
  "projects": [{"name": "", "description": "", "url": ""}],
  "education": [{"institution": "", "degree": "", "year": ""}],
  "links": [{"label": "", "url": ""}]
}"#;

/// LLM draft, merged over the rule-based one so contact details and skills are never lost.
pub async fn llm_profile(llm: &Llm, text: &str) -> anyhow::Result<Profile> {
    let base = heuristic_profile(text);
    let clipped: String = text.chars().take(24_000).collect();
    let prompt = format!("Return JSON with exactly this shape:\n{CV_SCHEMA}\n\nCV:\n\"\"\"\n{clipped}\n\"\"\"");
    let v = llm.complete_json(CV_SYSTEM, &prompt, 4000).await?;

    let s = |k: &str| v[k].as_str().unwrap_or("").trim().to_string();
    let or = |a: String, b: String| if a.is_empty() { b } else { a };
    let list = |k: &str| -> Vec<String> {
        v[k].as_array()
            .map(|a| a.iter().filter_map(|x| x.as_str()).map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect())
            .unwrap_or_default()
    };

    let mut skills_out: Vec<String> = list("skills").iter().map(|s| skills::canonicalize(s)).collect();
    for s in &base.skills {
        if !skills_out.iter().any(|x| x.eq_ignore_ascii_case(s)) {
            skills_out.push(s.clone());
        }
    }
    let experience: Vec<ExperienceItem> = serde_json::from_value(v["experience"].clone()).unwrap_or_default();
    let projects: Vec<ProjectItem> = serde_json::from_value(v["projects"].clone()).unwrap_or_default();
    let education: Vec<EducationItem> = serde_json::from_value(v["education"].clone()).unwrap_or_default();
    let mut links: Vec<LinkItem> = serde_json::from_value(v["links"].clone()).unwrap_or_default();
    for l in base.links.iter() {
        if !links.iter().any(|x| x.url == l.url) {
            links.push(l.clone());
        }
    }

    Ok(Profile {
        full_name: or(s("full_name"), base.full_name),
        headline: or(s("headline"), base.headline),
        location: s("location"),
        email: or(s("email"), base.email),
        phone: or(s("phone"), base.phone),
        summary: s("summary"),
        years_experience: v["years_experience"].as_f64().map(|y| y as f32).filter(|y| *y > 0.0).unwrap_or(base.years_experience),
        skills: skills_out,
        experience: Json(experience.into_iter().filter(|e| !e.company.is_empty() || !e.role.is_empty()).collect()),
        projects: Json(projects.into_iter().filter(|p| !p.name.is_empty()).collect()),
        education: Json(education.into_iter().filter(|e| !e.institution.is_empty()).collect()),
        links: Json(links.into_iter().filter(|l| !l.url.is_empty()).collect()),
        raw_cv_text: text.to_string(),
        confirmed: false,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CV: &str = "Aarav Sharma\nSenior Mobile Engineer\nPune · aarav@example.com · +91 98765 43210\ngithub.com/aaravdev\n\n\
        5 years building React Native apps with TypeScript and Redux. Payments and checkout flows.\n\
        Experience\nFinlo — Mobile Engineer (2022–now)\n";

    #[test]
    fn heuristic_draft() {
        let p = heuristic_profile(CV);
        assert_eq!(p.full_name, "Aarav Sharma");
        assert_eq!(p.headline, "Senior Mobile Engineer");
        assert_eq!(p.email, "aarav@example.com");
        assert_eq!(p.phone, "+91 98765 43210");
        assert_eq!(p.years_experience, 5.0);
        assert!(p.skills.contains(&"React Native".to_string()));
        assert!(p.skills.contains(&"Payments".to_string()));
        assert_eq!(p.links[0].url, "https://github.com/aaravdev");
        assert!(!p.confirmed);
    }

    #[test]
    fn plain_text_upload() {
        let t = extract_text("cv.txt", CV.as_bytes()).unwrap();
        assert!(t.contains("Aarav"));
        assert!(extract_text("cv.txt", b"too short").is_err());
    }
}
