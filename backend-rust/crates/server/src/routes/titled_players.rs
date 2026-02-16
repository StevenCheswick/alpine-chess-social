use axum::{Extension, Json};
use serde_json::Value as JsonValue;
use sqlx::PgPool;

use crate::db::titled_players;
use crate::error::AppError;

/// POST /api/admin/titled-players/refresh
/// Fetches all titled players from Chess.com API and reloads the cache.
pub async fn refresh_titled_players(
    Extension(pool): Extension<PgPool>,
) -> Result<Json<JsonValue>, AppError> {
    tracing::info!("Refreshing titled players from Chess.com API...");
    let count = titled_players::seed_from_chesscom(&pool).await?;
    Ok(Json(serde_json::json!({
        "success": true,
        "playersLoaded": count,
        "cacheSize": titled_players::cache_size(),
    })))
}

/// POST /api/admin/backfill-titled-tags
/// Iterates all games and applies titled opponent tags retroactively.
pub async fn backfill_titled_tags(
    Extension(pool): Extension<PgPool>,
) -> Result<Json<JsonValue>, AppError> {
    tracing::info!("Backfilling titled opponent tags for all games...");

    use sqlx::Row;

    // Get all games that don't already have a "titled" tag
    let rows = sqlx::query(
        r#"SELECT ug.id, ug.opponent
           FROM user_games ug
           WHERE NOT EXISTS (
               SELECT 1 FROM game_tags gt WHERE gt.game_id = ug.id AND gt.tag = 'titled'
           )"#,
    )
    .fetch_all(&pool)
    .await
    .map_err(AppError::Sqlx)?;

    let mut title_pairs: Vec<(i64, String)> = Vec::new();

    for row in &rows {
        let game_id: i64 = row.try_get("id").unwrap_or(0);
        let opponent: String = row.try_get("opponent").unwrap_or_default();

        // Check against the in-memory titled players cache
        if let Some(title) = titled_players::lookup(&opponent) {
            title_pairs.push((game_id, title));
        }
    }

    let tagged_count = title_pairs.len();
    if !title_pairs.is_empty() {
        titled_players::insert_title_tags(&pool, &title_pairs).await?;
    }

    tracing::info!("Backfill complete: {} games tagged", tagged_count);

    Ok(Json(serde_json::json!({
        "success": true,
        "totalTagged": tagged_count,
        "gamesChecked": rows.len(),
    })))
}
