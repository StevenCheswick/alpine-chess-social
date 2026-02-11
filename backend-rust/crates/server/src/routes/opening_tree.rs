use axum::{extract::Query, Extension, Json};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use sqlx::PgPool;

use crate::auth::middleware::AuthUser;
use crate::db::opening_moves;
use crate::error::AppError;

const MAX_DEPTH: usize = 15;
const STARTING_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

#[derive(Deserialize)]
pub struct OpeningTreeQuery {
    pub color: String,
    pub fen: Option<String>,
}

/// GET /api/opening-tree?color=white&fen=...
/// Returns the children of a single position. No fen = root position.
pub async fn get_opening_tree(
    Extension(pool): Extension<PgPool>,
    Query(q): Query<OpeningTreeQuery>,
    user: AuthUser,
) -> Result<Json<JsonValue>, AppError> {
    let color = q.color.to_lowercase();
    if color != "white" && color != "black" {
        return Err(AppError::BadRequest(
            "Color must be 'white' or 'black'".into(),
        ));
    }

    let parent_fen = q.fen.as_deref().unwrap_or(STARTING_FEN);
    let account_id = user.id;

    // Trigger backfill only if no stats exist at all (first time after migration)
    if !opening_moves::has_opening_stats(&pool, account_id).await? {
        opening_moves::populate_opening_stats(&pool, account_id).await?;
    }

    // Query ONLY the children of this position
    let rows = opening_moves::get_children(&pool, account_id, &color, parent_fen).await?;

    let mut children: Vec<JsonValue> = rows
        .iter()
        .map(|row| {
            let win_rate = if row.games > 0 {
                ((row.wins as f64 / row.games as f64) * 1000.0).round() / 10.0
            } else {
                0.0
            };

            let mut node = serde_json::json!({
                "move": row.move_san,
                "fen": row.result_fen,
                "games": row.games,
                "wins": row.wins,
                "losses": row.losses,
                "draws": row.draws,
                "winRate": win_rate,
            });

            if let Some(cp) = row.eval_cp {
                node["evalCp"] = serde_json::json!(cp);
            }

            node
        })
        .collect();

    // Sort by game count descending
    children.sort_by(|a, b| {
        b["games"]
            .as_i64()
            .unwrap_or(0)
            .cmp(&a["games"].as_i64().unwrap_or(0))
    });

    // Sum stats from children for the current node
    let total_games: i64 = children.iter().map(|c| c["games"].as_i64().unwrap_or(0)).sum();
    let total_wins: i64 = children.iter().map(|c| c["wins"].as_i64().unwrap_or(0)).sum();
    let total_losses: i64 = children.iter().map(|c| c["losses"].as_i64().unwrap_or(0)).sum();
    let total_draws: i64 = children.iter().map(|c| c["draws"].as_i64().unwrap_or(0)).sum();
    let win_rate = if total_games > 0 {
        ((total_wins as f64 / total_games as f64) * 1000.0).round() / 10.0
    } else {
        0.0
    };

    Ok(Json(serde_json::json!({
        "color": color,
        "fen": parent_fen,
        "games": total_games,
        "wins": total_wins,
        "losses": total_losses,
        "draws": total_draws,
        "winRate": win_rate,
        "children": children,
        "totalGames": total_games,
        "depth": MAX_DEPTH,
    })))
}
