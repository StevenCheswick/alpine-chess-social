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
        r#"SELECT ug.id, ug.opponent, ug.source
           FROM user_games ug
           WHERE NOT EXISTS (
               SELECT 1 FROM game_tags gt WHERE gt.game_id = ug.id AND gt.tag = 'titled'
           )"#,
    )
    .fetch_all(&pool)
    .await
    .map_err(AppError::Sqlx)?;

    let mut chess_com_tagged = 0usize;
    let mut lichess_tagged = 0usize;
    let mut title_pairs: Vec<(i64, String)> = Vec::new();

    for row in &rows {
        let game_id: i64 = row.try_get("id").unwrap_or(0);
        let opponent: String = row.try_get("opponent").unwrap_or_default();
        let source: String = row.try_get("source").unwrap_or_default();

        // For all sources, check against the in-memory titled players cache
        if let Some(title) = titled_players::lookup(&opponent) {
            title_pairs.push((game_id, title));
            if source == "lichess" {
                lichess_tagged += 1;
            } else {
                chess_com_tagged += 1;
            }
        }
    }

    if !title_pairs.is_empty() {
        titled_players::insert_title_tags(&pool, &title_pairs).await?;
    }

    let total = chess_com_tagged + lichess_tagged;
    tracing::info!(
        "Backfill complete: {} games tagged ({} Chess.com, {} Lichess)",
        total, chess_com_tagged, lichess_tagged
    );

    Ok(Json(serde_json::json!({
        "success": true,
        "totalTagged": total,
        "chessComTagged": chess_com_tagged,
        "lichessTagged": lichess_tagged,
        "gamesChecked": rows.len(),
    })))
}
