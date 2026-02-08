use axum::{extract::Path, extract::Query, Extension, Json};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use sqlx::PgPool;

use crate::auth::middleware::AuthUser;
use crate::db::posts;
use crate::error::AppError;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePostRequest {
    pub content: String,
    pub post_type: String,
    pub game_id: Option<i64>,
    pub key_position_index: Option<i32>,
}

#[derive(Deserialize)]
pub struct PaginationQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// POST /api/posts
pub async fn create_post(
    Extension(pool): Extension<PgPool>,
    user: AuthUser,
    Json(req): Json<CreatePostRequest>,
) -> Result<Json<JsonValue>, AppError> {
    let content = req.content.trim().to_string();
    if content.is_empty() {
        return Err(AppError::BadRequest("Content cannot be empty".into()));
    }
    if content.len() > 2000 {
        return Err(AppError::BadRequest(
            "Content must be at most 2000 characters".into(),
        ));
    }
    if req.post_type != "text" && req.post_type != "game_share" {
        return Err(AppError::BadRequest("Invalid post type".into()));
    }
    if req.post_type == "game_share" && req.game_id.is_none() {
        return Err(AppError::BadRequest(
            "gameId is required for game_share posts".into(),
        ));
    }

    let post_id = posts::create_post(
        &pool,
        user.id,
        &req.post_type,
        &content,
        req.game_id,
        req.key_position_index.unwrap_or(0),
    )
    .await?;

    let post = posts::get_post_by_id(&pool, post_id)
        .await?
        .ok_or(AppError::Internal("Failed to create post".into()))?;

    Ok(Json(post))
}

/// GET /api/posts
pub async fn get_posts(
    Extension(pool): Extension<PgPool>,
    Query(q): Query<PaginationQuery>,
    _user: AuthUser,
) -> Result<Json<JsonValue>, AppError> {
    let limit = q.limit.unwrap_or(20).max(1).min(100);
    let offset = q.offset.unwrap_or(0).max(0);

    let posts_list = posts::get_posts(&pool, limit, offset).await?;
    let total = posts::get_posts_count(&pool).await?;

    Ok(Json(serde_json::json!({
        "posts": posts_list,
        "total": total,
        "hasMore": offset + posts_list.len() as i64 > total,
    })))
}

/// GET /api/users/{username}/posts
pub async fn get_user_posts(
    Extension(pool): Extension<PgPool>,
    Path(username): Path<String>,
    Query(q): Query<PaginationQuery>,
) -> Result<Json<JsonValue>, AppError> {
    let limit = q.limit.unwrap_or(20).min(100);
    let offset = q.offset.unwrap_or(0).max(0);

    let posts_list = posts::get_posts_by_username(&pool, &username, limit, offset).await?;
    let total = posts::get_posts_count_by_username(&pool, &username).await?;

    Ok(Json(serde_json::json!({
        "posts": posts_list,
        "total": total,
        "hasMore": offset + posts_list.len() as i64 > total,
    })))
}
