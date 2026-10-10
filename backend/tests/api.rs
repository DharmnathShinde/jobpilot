//! End-to-end API tests against a real Postgres database.
//! Run with DATABASE_URL set (see .env). Skipped when it isn't.

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use jobpilot::config::Config;
use jobpilot::state::AppState;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

async fn app() -> Option<Router> {
    let _ = dotenvy::dotenv();
    let url = std::env::var("DATABASE_URL").ok()?;
    let config = Config {
        database_url: url.clone(),
        jwt_secret: "test-secret".into(),
        bind_addr: "127.0.0.1:0".into(),
        anthropic_api_key: None,
        llm_model: "unused".into(),
        scheduler_enabled: false,
        cron_secret: Some("test-cron-secret-123456".into()),
        allow_signups: true,
    };
    let pool = jobpilot::connect(&url).await.expect("db");
    Some(jobpilot::routes::router(AppState::new(pool, config).unwrap()))
}

async fn call(app: &Router, method: &str, path: &str, token: Option<&str>, body: Option<Value>) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(path);
    if let Some(t) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    let req = match body {
        Some(b) => req.header(header::CONTENT_TYPE, "application/json").body(Body::from(b.to_string())).unwrap(),
        None => req.body(Body::empty()).unwrap(),
    };
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v = serde_json::from_slice(&bytes).unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into()));
    (status, v)
}

async fn register(app: &Router) -> String {
    let email = format!("t{}@example.com", Uuid::new_v4().simple());
    let (s, v) = call(app, "POST", "/auth/register", None, Some(json!({ "email": email, "password": "correct horse", "name": "Aarav Sharma" }))).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    v["token"].as_str().unwrap().to_string()
}

const CV: &str = "Aarav Sharma\nSenior Mobile Engineer\nPune · aarav@example.com · +91 98765 43210\ngithub.com/aaravdev\n\n\
5 years building React Native apps with TypeScript, Redux and Jest. Payments and checkout flows.\n";

const JD: &str = "Kestrel Pay is hiring a Senior React Native Engineer. You'll own checkout in our merchant app. \
Requirements: React Native, TypeScript, Redux, GraphQL. 4-7 years of experience. Bengaluru, hybrid.";

#[tokio::test]
async fn full_daily_flow() {
    let Some(app) = app().await else { eprintln!("DATABASE_URL not set; skipping"); return };
    let token = register(&app).await;
    let t = Some(token.as_str());

    // Upload CV (multipart)
    let boundary = "XBOUNDARY";
    let body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"cv.txt\"\r\nContent-Type: text/plain\r\n\r\n{CV}\r\n--{boundary}--\r\n"
    );
    let req = Request::builder()
        .method("POST")
        .uri("/profile/cv")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, format!("multipart/form-data; boundary={boundary}"))
        .body(Body::from(body))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let v: Value = serde_json::from_slice(&res.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(v["method"], "rules");
    let mut profile = v["profile"].clone();
    assert_eq!(profile["full_name"], "Aarav Sharma");
    assert_eq!(profile["confirmed"], false);

    // Tailoring is blocked until the profile is confirmed
    let (s, job) = call(&app, "POST", "/jobs", t, Some(json!({ "title": "Senior React Native Engineer", "company": "Kestrel Pay", "location": "Bengaluru", "url": "https://example.com/jobs/1", "description": JD, "via": "share" }))).await;
    assert_eq!(s, StatusCode::OK, "{job}");
    let job_id = job["id"].as_str().unwrap().to_string();
    let (s, _) = call(&app, "POST", &format!("/jobs/{job_id}/tailor"), t, None).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    // Review + confirm, adding experience the rules couldn't parse
    profile["experience"] = json!([{ "company": "Finlo", "role": "Mobile Engineer", "start": "2022", "end": "now",
        "bullets": ["Mentored two interns", "Rebuilt the checkout in TypeScript, cutting drop-off by 18%"] }]);
    profile["confirmed"] = json!(true);
    let (s, p) = call(&app, "PUT", "/profile", t, Some(profile)).await;
    assert_eq!(s, StatusCode::OK, "{p}");
    assert_eq!(p["confirmed"], true);

    let (s, prefs) = call(&app, "PUT", "/preferences", t, Some(json!({ "target_roles": ["React Native Engineer"], "locations": ["Bengaluru"], "expected_ctc_lpa": 32, "notice_days": 60, "min_score": 40 }))).await;
    assert_eq!(s, StatusCode::OK, "{prefs}");

    // The job was re-scored after the profile changed
    let (_, job) = call(&app, "GET", &format!("/jobs/{job_id}"), t, None).await;
    assert_eq!(job["missing_skills"], json!(["GraphQL"]));
    assert!(job["score"].as_i64().unwrap() >= 70, "{job}");

    // Same link twice → conflict
    let (s, _) = call(&app, "POST", "/jobs", t, Some(json!({ "title": "Dup", "url": "https://example.com/jobs/1", "description": JD }))).await;
    assert_eq!(s, StatusCode::CONFLICT);

    let (s, today) = call(&app, "GET", "/today", t, None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(today["queue_count"], 1);
    assert_eq!(today["top_matches"][0]["id"], job_id.as_str());

    // Tailor, then render
    let (s, resume) = call(&app, "POST", &format!("/jobs/{job_id}/tailor"), t, None).await;
    assert_eq!(s, StatusCode::OK, "{resume}");
    assert_eq!(resume["content"]["experience"][0]["bullets"][0], "Rebuilt the checkout in TypeScript, cutting drop-off by 18%");
    assert!(resume["changes"].as_array().unwrap().iter().any(|c| c["text"].as_str().unwrap().starts_with("GraphQL not added")));
    let resume_id = resume["id"].as_str().unwrap();
    let (s, html) = call(&app, "GET", &format!("/resumes/{resume_id}/html"), t, None).await;
    assert_eq!(s, StatusCode::OK);
    assert!(html.as_str().unwrap().contains("Aarav Sharma"));

    let (_, answers) = call(&app, "GET", &format!("/jobs/{job_id}/answers"), t, None).await;
    let answers = answers.as_array().unwrap();
    assert_eq!(answers.iter().find(|a| a["key"] == "notice").unwrap()["value"], "60 days");
    assert_eq!(answers.iter().find(|a| a["key"] == "ctc").unwrap()["value"], "₹32 LPA");
    assert!(answers[0]["value"].as_str().unwrap().contains("checkout"));

    // Apply → tracker
    let (s, app_row) = call(&app, "POST", "/applications", t, Some(json!({ "job_id": job_id, "resume_id": resume_id }))).await;
    assert_eq!(s, StatusCode::OK, "{app_row}");
    assert_eq!(app_row["status"], "applied");
    let app_id = app_row["id"].as_str().unwrap();
    let (_, job) = call(&app, "GET", &format!("/jobs/{job_id}"), t, None).await;
    assert_eq!(job["queue_status"], "applied");

    let (s, upd) = call(&app, "PATCH", &format!("/applications/{app_id}"), t, Some(json!({ "status": "interview", "next_reminder": "2026-10-14", "event_note": "Tech round" }))).await;
    assert_eq!(s, StatusCode::OK, "{upd}");
    assert_eq!(upd["status"], "interview");
    assert_eq!(upd["next_reminder"], "2026-10-14");

    let (_, detail) = call(&app, "GET", &format!("/applications/{app_id}"), t, None).await;
    let kinds: Vec<&str> = detail["events"].as_array().unwrap().iter().map(|e| e["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds, vec!["applied", "interview"]);

    let (_, list) = call(&app, "GET", "/applications", t, None).await;
    assert_eq!(list["counts"]["interview"], 1);
    assert_eq!(list["counts"]["applied"], 0);

    let (_, today) = call(&app, "GET", "/today", t, None).await;
    assert_eq!(today["queue_count"], 0);
    assert_eq!(today["applied_today"], 1);

    // Clean up
    let (s, _) = call(&app, "DELETE", "/me", t, None).await;
    assert_eq!(s, StatusCode::OK);
}

#[tokio::test]
async fn auth_and_isolation() {
    let Some(app) = app().await else { return };
    let (s, _) = call(&app, "GET", "/today", None, None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, _) = call(&app, "GET", "/today", Some("not-a-jwt"), None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);

    let a = register(&app).await;
    let b = register(&app).await;
    let (_, job) = call(&app, "POST", "/jobs", Some(&a), Some(json!({ "title": "Engineer", "description": JD }))).await;
    let id = job["id"].as_str().unwrap();
    // B cannot see or act on A's job
    let (s, _) = call(&app, "GET", &format!("/jobs/{id}"), Some(&b), None).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    let (s, _) = call(&app, "POST", "/applications", Some(&b), Some(json!({ "job_id": id }))).await;
    assert_eq!(s, StatusCode::NOT_FOUND);

    // Wrong password
    let (s, _) = call(&app, "POST", "/auth/login", None, Some(json!({ "email": "nobody@example.com", "password": "whatever1" }))).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);

    call(&app, "DELETE", "/me", Some(&a), None).await;
    call(&app, "DELETE", "/me", Some(&b), None).await;
}

#[tokio::test]
async fn validation() {
    let Some(app) = app().await else { return };
    let t = register(&app).await;
    let (s, _) = call(&app, "PUT", "/preferences", Some(&t), Some(json!({ "timezone": "Mars/Base" }))).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, _) = call(&app, "POST", "/sources", Some(&t), Some(json!({ "kind": "linkedin", "board": "x" }))).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, src) = call(&app, "POST", "/sources", Some(&t), Some(json!({ "kind": "greenhouse", "board": "Acme", "company": "Acme" }))).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(src["board"], "acme");
    let (s, _) = call(&app, "POST", "/jobs", Some(&t), Some(json!({ "title": "x", "description": "short" }))).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    call(&app, "DELETE", "/me", Some(&t), None).await;
}

#[tokio::test]
async fn cron_tick_requires_secret() {
    let Some(app) = app().await else { return };
    let (s, _) = call(&app, "POST", "/internal/tick", None, None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let req = Request::builder().method("POST").uri("/internal/tick").header("x-cron-secret", "test-cron-secret-123456").body(Body::empty()).unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}
