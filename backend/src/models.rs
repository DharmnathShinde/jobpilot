use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ExperienceItem {
    pub company: String,
    pub role: String,
    #[serde(default)]
    pub start: String,
    #[serde(default)]
    pub end: String,
    #[serde(default)]
    pub bullets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ProjectItem {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct EducationItem {
    pub institution: String,
    #[serde(default)]
    pub degree: String,
    #[serde(default)]
    pub year: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct LinkItem {
    pub label: String,
    pub url: String,
}

/// The master profile. Everything the app writes on the user's behalf comes from here.
#[derive(Debug, Clone, Serialize, Deserialize, Default, FromRow)]
pub struct Profile {
    #[serde(skip_deserializing)]
    pub user_id: Uuid,
    #[serde(default)]
    pub full_name: String,
    #[serde(default)]
    pub headline: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub years_experience: f32,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub experience: Json<Vec<ExperienceItem>>,
    #[serde(default)]
    pub projects: Json<Vec<ProjectItem>>,
    #[serde(default)]
    pub education: Json<Vec<EducationItem>>,
    #[serde(default)]
    pub links: Json<Vec<LinkItem>>,
    #[serde(skip_serializing, default)]
    pub raw_cv_text: String,
    #[serde(default)]
    pub confirmed: bool,
    #[serde(skip_deserializing, default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Preferences {
    #[serde(skip_deserializing)]
    pub user_id: Uuid,
    #[serde(default)]
    pub target_roles: Vec<String>,
    #[serde(default)]
    pub locations: Vec<String>,
    #[serde(default = "yes")]
    pub remote_ok: bool,
    pub expected_ctc_lpa: Option<f32>,
    pub notice_days: Option<i32>,
    #[serde(default = "default_min_score")]
    pub min_score: i32,
    #[serde(default = "default_run_time")]
    pub run_time: NaiveTime,
    #[serde(default = "default_tz")]
    pub timezone: String,
    #[serde(default = "default_days")]
    pub active_days: Vec<i32>,
    #[serde(default = "default_max_jobs")]
    pub max_jobs_per_run: i32,
    #[serde(default = "yes")]
    pub schedule_enabled: bool,
    #[serde(skip_deserializing)]
    pub last_run_on: Option<NaiveDate>,
}

fn yes() -> bool {
    true
}
fn default_min_score() -> i32 {
    50
}
fn default_run_time() -> NaiveTime {
    NaiveTime::from_hms_opt(7, 0, 0).unwrap()
}
fn default_tz() -> String {
    "Asia/Kolkata".into()
}
fn default_days() -> Vec<i32> {
    vec![1, 2, 3, 4, 5]
}
fn default_max_jobs() -> i32 {
    20
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct JobSource {
    pub id: Uuid,
    pub kind: String,
    pub board: String,
    pub company: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Job {
    pub id: Uuid,
    pub source: String,
    pub external_id: String,
    pub title: String,
    pub company: String,
    pub location: String,
    pub url: String,
    pub description: String,
    pub posted_at: Option<DateTime<Utc>>,
    pub required_skills: Vec<String>,
    pub min_years: Option<f32>,
    pub max_years: Option<f32>,
    pub score: i32,
    pub matched_skills: Vec<String>,
    pub missing_skills: Vec<String>,
    pub explanation: String,
    pub queue_status: String,
    pub created_at: DateTime<Utc>,
}

/// A job as fetched from a source, before it is stored and scored.
#[derive(Debug, Clone, PartialEq)]
pub struct NewJob {
    pub source: String,
    pub external_id: String,
    pub title: String,
    pub company: String,
    pub location: String,
    pub url: String,
    pub description: String,
    pub posted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResumeChange {
    /// reordered | wording | not_added
    pub kind: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct TailoredResume {
    pub id: Uuid,
    pub job_id: Uuid,
    pub content: Json<Profile>,
    pub changes: Json<Vec<ResumeChange>>,
    pub keyword_coverage: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Application {
    pub id: Uuid,
    pub job_id: Uuid,
    pub status: String,
    pub resume_id: Option<Uuid>,
    pub notes: String,
    pub next_reminder: Option<NaiveDate>,
    pub applied_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    // joined from jobs
    pub title: String,
    pub company: String,
    pub location: String,
    pub url: String,
    pub score: i32,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ApplicationEvent {
    pub id: Uuid,
    pub kind: String,
    pub note: String,
    pub at: DateTime<Utc>,
}

pub const APPLICATION_STATUSES: &[&str] = &["saved", "applied", "replied", "interview", "offer", "rejected"];
