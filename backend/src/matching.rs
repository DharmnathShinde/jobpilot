//! Scores a job against the master profile.
//!
//! Score out of 100:
//! - 70 points: share of the job's required skills the user has
//! - 20 points: experience fit against the job's stated year range
//! - 10 points: title fit against the user's headline and target roles

use std::sync::OnceLock;

use regex::Regex;

use crate::models::Profile;
use crate::skills;

#[derive(Debug, Clone, PartialEq)]
pub struct MatchResult {
    pub score: i32,
    pub required_skills: Vec<String>,
    pub matched_skills: Vec<String>,
    pub missing_skills: Vec<String>,
    pub min_years: Option<f32>,
    pub max_years: Option<f32>,
    pub explanation: String,
}

/// Pulls a year range like "4-7 years", "5+ years", "3 to 5 yrs" out of a job description.
pub fn experience_range(text: &str) -> (Option<f32>, Option<f32>) {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r"(?i)(\d{1,2})(?:\.\d)?\s*(\+|plus)?\s*(?:(?:-|–|to)\s*(\d{1,2}))?\s*\+?\s*(?:years?|yrs?)").unwrap()
    });
    let mut best: Option<(f32, Option<f32>)> = None;
    for caps in re.captures_iter(text) {
        let min: f32 = caps[1].parse().unwrap_or(0.0);
        if min > 30.0 {
            continue;
        }
        let max = caps.get(3).and_then(|m| m.as_str().parse::<f32>().ok()).filter(|m| *m >= min);
        // Prefer the largest minimum mentioned: "2 years of React, 5+ years overall" means 5.
        if best.map_or(true, |(b, _)| min > b) {
            best = Some((min, max));
        }
    }
    match best {
        Some((min, max)) => (Some(min), max),
        None => (None, None),
    }
}

pub fn score(profile: &Profile, title: &str, description: &str, target_roles: &[String]) -> MatchResult {
    let full_text = format!("{title}\n{description}");
    let required = skills::extract_skills(&full_text);

    let have: Vec<String> = profile.skills.iter().map(|s| skills::canonicalize(s)).collect();
    let have_lower: Vec<String> = have.iter().map(|s| s.to_lowercase()).collect();
    let (matched, missing): (Vec<String>, Vec<String>) =
        required.iter().cloned().partition(|s| have_lower.contains(&s.to_lowercase()));

    let skill_points = if required.is_empty() {
        35.0 // unknown requirements: neutral
    } else {
        70.0 * matched.len() as f32 / required.len() as f32
    };

    let (min_years, max_years) = experience_range(description);
    let yrs = profile.years_experience;
    let exp_points = match (min_years, max_years) {
        (None, _) => 14.0,
        (Some(min), max) => {
            let max = max.unwrap_or(min + 6.0);
            if yrs >= min && yrs <= max + 2.0 {
                20.0
            } else if yrs >= min - 1.0 && yrs < min {
                12.0
            } else if yrs > max + 2.0 {
                10.0 // overqualified, still worth a look
            } else {
                0.0
            }
        }
    };

    let title_points = title_fit(title, &profile.headline, target_roles);
    let total = (skill_points + exp_points + title_points).round().clamp(0.0, 100.0) as i32;

    let explanation = explain(&matched, &missing, &required, yrs, min_years, max_years);

    MatchResult {
        score: total,
        required_skills: required,
        matched_skills: matched,
        missing_skills: missing,
        min_years,
        max_years,
        explanation,
    }
}

fn title_fit(title: &str, headline: &str, target_roles: &[String]) -> f32 {
    let words = |s: &str| -> Vec<String> {
        s.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 2 && !STOP.contains(w))
            .map(String::from)
            .collect()
    };
    let title_words = words(title);
    if title_words.is_empty() {
        return 5.0;
    }
    let mut best = 0.0f32;
    for candidate in target_roles.iter().map(String::as_str).chain(std::iter::once(headline)) {
        let cw = words(candidate);
        if cw.is_empty() {
            continue;
        }
        let overlap = cw.iter().filter(|w| title_words.contains(w)).count() as f32;
        best = best.max(overlap / cw.len() as f32);
    }
    10.0 * best
}

const STOP: &[&str] = &["and", "the", "for", "with", "senior", "junior", "lead", "sr", "jr", "ii", "iii"];

fn explain(
    matched: &[String],
    missing: &[String],
    required: &[String],
    yrs: f32,
    min: Option<f32>,
    max: Option<f32>,
) -> String {
    let mut parts = Vec::new();
    if required.is_empty() {
        parts.push("The listing doesn't name specific skills.".to_string());
    } else {
        parts.push(format!("You have {} of the {} skills it asks for", matched.len(), required.len()));
        if !missing.is_empty() {
            let shown: Vec<&str> = missing.iter().take(3).map(String::as_str).collect();
            parts[0].push_str(&format!("; missing {}", shown.join(", ")));
        }
        parts[0].push('.');
    }
    if let Some(min) = min {
        let range = match max {
            Some(max) => format!("{min:.0}–{max:.0} years"),
            None => format!("{min:.0}+ years"),
        };
        let fit = if yrs >= min { "fits" } else { "is below" };
        parts.push(format!("Your {yrs:.0} years {fit} the {range} asked."));
    }
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> Profile {
        Profile {
            headline: "Senior Mobile Engineer".into(),
            years_experience: 5.0,
            skills: vec!["React Native".into(), "typescript".into(), "Redux".into(), "Jest".into()],
            ..Default::default()
        }
    }

    #[test]
    fn year_ranges() {
        assert_eq!(experience_range("We need 4-7 years of experience"), (Some(4.0), Some(7.0)));
        assert_eq!(experience_range("5+ years building apps"), (Some(5.0), None));
        assert_eq!(experience_range("3 to 5 yrs"), (Some(3.0), Some(5.0)));
        assert_eq!(experience_range("2 years of React, 6+ years overall"), (Some(6.0), None));
        assert_eq!(experience_range("no numbers here"), (None, None));
    }

    #[test]
    fn strong_match_scores_high() {
        let r = score(
            &profile(),
            "Senior React Native Engineer",
            "Build our app with React Native, TypeScript and Redux. 4-7 years experience.",
            &["React Native Engineer".into()],
        );
        assert_eq!(r.matched_skills, vec!["React Native", "TypeScript", "Redux"]);
        assert!(r.missing_skills.is_empty());
        assert!(r.score >= 90, "score was {}", r.score);
    }

    #[test]
    fn gaps_lower_the_score_and_are_named() {
        let r = score(
            &profile(),
            "Mobile Engineer",
            "React Native, GraphQL, Detox and Kotlin. 4-7 years.",
            &[],
        );
        assert_eq!(r.missing_skills, vec!["GraphQL", "Kotlin", "Detox"]);
        assert!(r.score < 60, "score was {}", r.score);
        assert!(r.explanation.contains("missing GraphQL, Kotlin, Detox"));
    }

    #[test]
    fn under_experienced_gets_no_experience_points() {
        let mut junior = profile();
        junior.years_experience = 1.0;
        let low = score(&junior, "Staff Engineer", "React Native. 4+ years.", &[]);
        let high = score(&profile(), "Staff Engineer", "React Native. 4+ years.", &[]);
        assert_eq!(high.score - low.score, 20);
    }
}
