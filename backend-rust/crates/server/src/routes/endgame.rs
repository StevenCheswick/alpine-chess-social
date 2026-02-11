use axum::{Extension, Json};
use serde_json::Value as JsonValue;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::RwLock;

use crate::auth::middleware::AuthUser;
use crate::db::analysis;
use crate::error::AppError;

static ENDGAME_CACHE: std::sync::LazyLock<RwLock<HashMap<i64, JsonValue>>> =
    std::sync::LazyLock::new(|| RwLock::new(HashMap::new()));

/// GET /api/games/endgame-stats
pub async fn get_endgame_stats(
    Extension(pool): Extension<PgPool>,
    user: AuthUser,
) -> Result<Json<JsonValue>, AppError> {
    // Check cache
    if let Ok(cache) = ENDGAME_CACHE.read() {
        if let Some(cached) = cache.get(&user.id) {
            return Ok(Json(cached.clone()));
        }
    }

    let stats = analysis::get_user_endgame_stats(&pool, user.id).await?;

    // Store in cache
    if let Ok(mut cache) = ENDGAME_CACHE.write() {
        cache.insert(user.id, stats.clone());
    }

    Ok(Json(stats))
}
