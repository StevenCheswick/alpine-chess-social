use axum::{extract::Path, Extension, Json};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::auth::middleware::{AuthUser, MaybeAuthUser};
use crate::db::{accounts, games};
use crate::error::AppError;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileResponse {
    pub id: i64,
    pub username: String,
    pub display_name: String,
    pub chess_com_username: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: String,
    pub games_count: i64,
    pub is_own_profile: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProfileRequest {
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub chess_com_username: Option<String>,
}

pub async fn get_user_profile(
    Extension(pool): Extension<PgPool>,
    Path(username): Path<String>,
    maybe_user: MaybeAuthUser,
) -> Result<Json<ProfileResponse>, AppError> {
    let profile = accounts::get_public_profile(&pool, &username)
        .await?
        .ok_or(AppError::NotFound("User not found".into()))?;

    let games_count = games::get_user_games_count(&pool, profile.id, None).await?;

    let is_own_profile = maybe_user
        .0
        .as_ref()
        .map(|u| u.id == profile.id)
        .unwrap_or(false);

    Ok(Json(ProfileResponse {
        id: profile.id,
        username: profile.username.clone(),
        display_name: profile
            .display_name
            .clone()
            .unwrap_or_else(|| profile.username.clone()),
        chess_com_username: profile.chess_com_username.clone(),
        bio: profile.bio.clone(),
        avatar_url: profile.avatar_url.clone(),
        created_at: profile.created_at.to_rfc3339(),
        games_count,
        is_own_profile,
    }))
}

pub async fn update_profile(
    Extension(pool): Extension<PgPool>,
    user: AuthUser,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<super::auth::UserResponse>, AppError> {
    // Validate
    if let Some(ref dn) = req.display_name {
        if dn.is_empty() {
            return Err(AppError::BadRequest("Display name cannot be empty".into()));
        }
        if dn.len() > 50 {
            return Err(AppError::BadRequest(
                "Display name must be at most 50 characters".into(),
            ));
        }
    }
    if let Some(ref bio) = req.bio {
        if bio.len() > 500 {
            return Err(AppError::BadRequest(
                "Bio must be at most 500 characters".into(),
            ));
        }
    }
    if let Some(ref cc) = req.chess_com_username {
        if cc.len() > 50 {
            return Err(AppError::BadRequest(
                "Chess.com username must be at most 50 characters".into(),
            ));
        }
    }

    let updated = accounts::update_account(
        &pool,
        user.id,
        req.display_name.as_deref(),
        req.bio.as_deref(),
        req.chess_com_username.as_deref(),
    )
    .await?;

    Ok(Json(super::auth::UserResponse {
        id: updated.id,
        username: updated.username.clone(),
        display_name: updated
            .display_name
            .clone()
            .unwrap_or_else(|| updated.username.clone()),
        email: updated.email.clone(),
        chess_com_username: updated.chess_com_username.clone(),
        bio: updated.bio.clone(),
        avatar_url: updated.avatar_url.clone(),
        created_at: updated.created_at.to_rfc3339(),
        is_verified: false,
        follower_count: 0,
        following_count: 0,
    }))
}
