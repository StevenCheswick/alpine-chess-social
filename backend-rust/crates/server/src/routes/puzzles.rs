use axum::{extract::Query, Extension, Json};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use sqlx::PgPool;

use crate::auth::middleware::AuthUser;
use crate::db::analysis;
use crate::error::AppError;

#[derive(Deserialize)]
pub struct PuzzleQuery {
    pub theme: Option<String>,
}

/// GET /api/puzzles?theme=Fork
pub async fn get_puzzles(
    Extension(pool): Extension<PgPool>,
    user: AuthUser,
    Query(params): Query<PuzzleQuery>,
) -> Result<Json<JsonValue>, AppError> {
    let theme_filter = params.theme.as_deref();

    let puzzles = analysis::get_user_puzzles(&pool, user.id, theme_filter).await?;
    let themes = analysis::get_user_puzzle_themes(&pool, user.id).await?;

    let total = puzzles.len();

    Ok(Json(serde_json::json!({
        "puzzles": puzzles,
        "total": total,
        "themes": themes,
    })))
}
