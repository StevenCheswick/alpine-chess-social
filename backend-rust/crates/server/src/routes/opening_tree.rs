use axum::{extract::Query, Extension, Json};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use sqlx::PgPool;

use crate::auth::middleware::AuthUser;
use crate::db::{games, opening_tree as ot_db};
use crate::error::AppError;

const MAX_DEPTH: usize = 15;

#[derive(Deserialize)]
pub struct OpeningTreeQuery {
    pub color: String,
    pub rebuild: Option<bool>,
}

/// GET /api/opening-tree
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

    let account_id = user.id;

    // Check cache
    if !q.rebuild.unwrap_or(false) {
        if let Some(cached) = ot_db::get_cached_opening_tree(&pool, account_id, &color).await? {
            return Ok(Json(serde_json::json!({
                "color": color,
                "rootNode": cached["tree"],
                "totalGames": cached["totalGames"],
                "depth": MAX_DEPTH,
            })));
        }
    }

    // Build tree
    let color_games = games::get_user_games_by_color(&pool, account_id, &color).await?;

    if color_games.is_empty() {
        return Ok(Json(serde_json::json!({
            "color": color,
            "rootNode": {
                "move": "start",
                "fen": "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                "games": 0,
                "wins": 0,
                "losses": 0,
                "draws": 0,
                "winRate": 0,
                "children": [],
            },
            "totalGames": 0,
            "depth": MAX_DEPTH,
        })));
    }

    // Convert games to format expected by opening tree builder
    let tree_games: Vec<chess_core::opening_tree::TreeGame> = color_games
        .iter()
        .filter_map(|g| {
            let result = g["result"].as_str().unwrap_or("D").to_string();
            let tcn = g["tcn"].as_str().map(|s| s.to_string());

            // Decode TCN to SAN moves if available
            let moves = if let Some(ref tcn_str) = tcn {
                chess_core::tcn::decode_tcn_to_san(tcn_str).unwrap_or_default()
            } else {
                vec![]
            };

            if moves.is_empty() {
                return None;
            }

            Some(chess_core::opening_tree::TreeGame { moves, result })
        })
        .collect();

    let tree = chess_core::opening_tree::build_opening_tree(&tree_games, MAX_DEPTH);
    let root_node = chess_core::opening_tree::convert_tree_for_response(&tree);

    // Cache it
    ot_db::save_opening_tree(&pool, account_id, &color, &root_node, tree_games.len() as i32).await?;

    Ok(Json(serde_json::json!({
        "color": color,
        "rootNode": root_node,
        "totalGames": tree_games.len(),
        "depth": MAX_DEPTH,
    })))
}
