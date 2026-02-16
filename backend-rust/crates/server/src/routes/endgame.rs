use axum::{Extension, Json};
use serde_json::Value as JsonValue;
use sqlx::PgPool;

use crate::auth::middleware::AuthUser;
use crate::db::analysis;
use crate::error::AppError;

/// GET /api/games/endgame-stats
pub async fn get_endgame_stats(
    Extension(pool): Extension<PgPool>,
    user: AuthUser,
) -> Result<Json<JsonValue>, AppError> {
    let stats = analysis::get_user_endgame_stats(&pool, user.id).await?;
    Ok(Json(stats))
}
