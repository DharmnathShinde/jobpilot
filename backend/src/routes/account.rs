use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::{self, AuthUser};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct RegisterReq {
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Deserialize)]
pub struct LoginReq {
    pub email: String,
    pub password: String,
}

pub async fn register(State(st): State<AppState>, Json(req): Json<RegisterReq>) -> AppResult<Json<Value>> {
    if !st.config.allow_signups {
        return Err(AppError::BadRequest("sign-ups are closed on this server".into()));
    }
    let email = req.email.trim().to_lowercase();
    if !email.contains('@') || email.len() < 5 {
        return Err(AppError::BadRequest("enter a valid email".into()));
    }
    if req.password.len() < 8 {
        return Err(AppError::BadRequest("password must be at least 8 characters".into()));
    }
    let hash = auth::hash_password(&req.password)?;
    let id = Uuid::new_v4();
    let mut tx = st.pool.begin().await?;
    let inserted = sqlx::query("INSERT INTO users (id, email, password_hash, name) VALUES ($1, $2, $3, $4) ON CONFLICT (email) DO NOTHING")
        .bind(id)
        .bind(&email)
        .bind(&hash)
        .bind(req.name.trim())
        .execute(&mut *tx)
        .await?
        .rows_affected();
    if inserted == 0 {
        return Err(AppError::Conflict("an account with this email already exists".into()));
    }
    sqlx::query("INSERT INTO profiles (user_id, full_name, email) VALUES ($1, $2, $3)").bind(id).bind(req.name.trim()).bind(&email).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO preferences (user_id) VALUES ($1)").bind(id).execute(&mut *tx).await?;
    tx.commit().await?;
    let token = auth::issue_token(&st.config.jwt_secret, id)?;
    Ok(Json(json!({ "token": token, "user": { "id": id, "email": email, "name": req.name.trim() } })))
}

pub async fn login(State(st): State<AppState>, Json(req): Json<LoginReq>) -> AppResult<Json<Value>> {
    let email = req.email.trim().to_lowercase();
    let row: Option<(Uuid, String, String)> =
        sqlx::query_as("SELECT id, password_hash, name FROM users WHERE email = $1").bind(&email).fetch_optional(&st.pool).await?;
    let Some((id, hash, name)) = row else { return Err(AppError::Unauthorized) };
    if !auth::verify_password(&req.password, &hash) {
        return Err(AppError::Unauthorized);
    }
    let token = auth::issue_token(&st.config.jwt_secret, id)?;
    Ok(Json(json!({ "token": token, "user": { "id": id, "email": email, "name": name } })))
}

pub async fn me(State(st): State<AppState>, AuthUser(uid): AuthUser) -> AppResult<Json<Value>> {
    let (email, name): (String, String) = sqlx::query_as("SELECT email, name FROM users WHERE id = $1").bind(uid).fetch_one(&st.pool).await?;
    let confirmed: bool = sqlx::query_scalar("SELECT confirmed FROM profiles WHERE user_id = $1").bind(uid).fetch_one(&st.pool).await?;
    Ok(Json(json!({ "id": uid, "email": email, "name": name, "profile_confirmed": confirmed })))
}

/// Deletes the account and every row that belongs to it (all tables cascade from users).
pub async fn delete_account(State(st): State<AppState>, AuthUser(uid): AuthUser) -> AppResult<Json<Value>> {
    sqlx::query("DELETE FROM users WHERE id = $1").bind(uid).execute(&st.pool).await?;
    Ok(Json(json!({ "deleted": true })))
}
