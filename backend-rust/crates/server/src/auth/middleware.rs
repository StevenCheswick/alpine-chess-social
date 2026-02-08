use axum::{
    extract::FromRequestParts,
    http::request::Parts,
};
use sqlx::PgPool;

use crate::auth::jwt;
use crate::config::Config;
use crate::error::AppError;

/// Authenticated user extracted from the Authorization header.
/// Use as an extractor in route handlers that require auth.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AuthUser {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub display_name: Option<String>,
    pub chess_com_username: Option<String>,
    pub lichess_username: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let pool = parts
            .extensions
            .get::<PgPool>()
            .ok_or(AppError::Internal("Missing database pool".into()))?
            .clone();

        let config = parts
            .extensions
            .get::<Config>()
            .ok_or(AppError::Internal("Missing config".into()))?
            .clone();

        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or(AppError::Unauthorized)?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .or_else(|| auth_header.strip_prefix("bearer "))
            .ok_or(AppError::Unauthorized)?;

        let claims = jwt::verify_token(token, &config.jwt_secret)
            .ok_or(AppError::Unauthorized)?;

        let account = sqlx::query_as::<_, AuthUser>(
            r#"SELECT
                id, username, email, display_name,
                chess_com_username, lichess_username,
                bio, avatar_url, created_at
            FROM accounts WHERE id = $1"#,
        )
        .bind(claims.user_id)
        .fetch_optional(&pool)
        .await
        .map_err(AppError::Sqlx)?
        .ok_or(AppError::Unauthorized)?;

        Ok(account)
    }
}

/// Optional auth — returns None if no valid token present.
#[derive(Debug, Clone)]
pub struct MaybeAuthUser(pub Option<AuthUser>);

impl<S> FromRequestParts<S> for MaybeAuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match AuthUser::from_request_parts(parts, state).await {
            Ok(user) => Ok(MaybeAuthUser(Some(user))),
            Err(_) => Ok(MaybeAuthUser(None)),
        }
    }
}
