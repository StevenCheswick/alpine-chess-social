use axum::{Extension, Json};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::auth::{jwt, middleware::AuthUser, password};
use crate::config::Config;
use crate::db::accounts;
use crate::error::AppError;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub chess_com_username: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
    pub id: i64,
    pub username: String,
    pub display_name: String,
    pub email: String,
    pub chess_com_username: Option<String>,
    pub lichess_username: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: String,
    pub is_verified: bool,
    pub follower_count: i32,
    pub following_count: i32,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub user: UserResponse,
    pub token: String,
}

fn account_to_response(a: &accounts::Account) -> UserResponse {
    UserResponse {
        id: a.id,
        username: a.username.clone(),
        display_name: a
            .display_name
            .clone()
            .unwrap_or_else(|| a.username.clone()),
        email: a.email.clone(),
        chess_com_username: a.chess_com_username.clone(),
        lichess_username: a.lichess_username.clone(),
        bio: a.bio.clone(),
        avatar_url: a.avatar_url.clone(),
        created_at: a.created_at.to_rfc3339(),
        is_verified: false,
        follower_count: 0,
        following_count: 0,
    }
}

pub async fn register(
    Extension(pool): Extension<PgPool>,
    Extension(config): Extension<Config>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    // Validate username
    if req.username.len() < 3 {
        return Err(AppError::BadRequest(
            "Username must be at least 3 characters".into(),
        ));
    }
    if req.username.len() > 20 {
        return Err(AppError::BadRequest(
            "Username must be at most 20 characters".into(),
        ));
    }
    let username_re = Regex::new(r"^[a-zA-Z0-9_]+$").unwrap();
    if !username_re.is_match(&req.username) {
        return Err(AppError::BadRequest(
            "Username can only contain letters, numbers, and underscores".into(),
        ));
    }

    // Validate password
    if req.password.len() < 8 {
        return Err(AppError::BadRequest(
            "Password must be at least 8 characters".into(),
        ));
    }

    // Check uniqueness
    if accounts::email_exists(&pool, &req.email).await? {
        return Err(AppError::BadRequest("Email already registered".into()));
    }
    if accounts::username_exists(&pool, &req.username).await? {
        return Err(AppError::BadRequest("Username already taken".into()));
    }

    // Hash password with argon2
    let hash = password::hash_password(&req.password)
        .map_err(|e| AppError::Internal(format!("Password hash error: {e}")))?;

    let account_id = accounts::create_account(
        &pool,
        &req.username,
        &req.email,
        &hash,
        req.chess_com_username.as_deref().unwrap_or(""),
    )
    .await?;

    let account = accounts::get_account_by_id(&pool, account_id)
        .await?
        .ok_or_else(|| AppError::Internal("Failed to retrieve created account".into()))?;

    let token = jwt::create_token(account_id, &config.jwt_secret, config.jwt_expire_hours)
        .map_err(|e| AppError::Internal(format!("Token creation error: {e}")))?;

    Ok(Json(AuthResponse {
        user: account_to_response(&account),
        token,
    }))
}

pub async fn login(
    Extension(pool): Extension<PgPool>,
    Extension(config): Extension<Config>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let account = accounts::get_account_by_email(&pool, &req.email)
        .await?
        .ok_or(AppError::BadRequest("Invalid email or password".into()))?;

    let (valid, needs_rehash) = password::verify_password(&req.password, &account.password_hash)
        .map_err(|e| AppError::Internal(format!("Password verify error: {e}")))?;

    if !valid {
        return Err(AppError::BadRequest("Invalid email or password".into()));
    }

    // Transparently rehash bcrypt -> argon2 on successful login
    if needs_rehash {
        if let Ok(new_hash) = password::hash_password(&req.password) {
            let _ = accounts::update_password_hash(&pool, account.id, &new_hash).await;
        }
    }

    let token = jwt::create_token(account.id, &config.jwt_secret, config.jwt_expire_hours)
        .map_err(|e| AppError::Internal(format!("Token creation error: {e}")))?;

    Ok(Json(AuthResponse {
        user: account_to_response(&account),
        token,
    }))
}

pub async fn me(user: AuthUser) -> Result<Json<UserResponse>, AppError> {
    Ok(Json(UserResponse {
        id: user.id,
        username: user.username.clone(),
        display_name: user
            .display_name
            .clone()
            .unwrap_or_else(|| user.username.clone()),
        email: user.email.clone(),
        chess_com_username: user.chess_com_username.clone(),
        lichess_username: user.lichess_username.clone(),
        bio: user.bio.clone(),
        avatar_url: user.avatar_url.clone(),
        created_at: user.created_at.to_rfc3339(),
        is_verified: false,
        follower_count: 0,
        following_count: 0,
    }))
}
