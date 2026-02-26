use axum::{extract::Query, http::HeaderMap, Extension, Json};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use shakmaty::{Chess, Position, uci::UciMove, san::San};
use sqlx::PgPool;
use std::collections::{HashMap, HashSet};

use crate::book_cache::{self, BOOK_CACHE};
use crate::config::Config;
use crate::db::analysis::{first_bad_move, phase_accuracy};
use crate::error::AppError;
use crate::routes::dashboard;
use crate::routes::trainer::check_admin_secret;

#[derive(Deserialize)]
pub struct BookCheckQuery {
    pub fen: String,
    pub san: String,
}

#[derive(Serialize)]
pub struct BookCheckResponse {
    pub is_book: bool,
    pub games: Option<i32>,
    pub white_wins: Option<i32>,
    pub draws: Option<i32>,
    pub black_wins: Option<i32>,
}

/// GET /api/opening-book/check?fen=...&san=...
/// Check if a move exists in the opening book.
/// Uses in-memory cache for instant lookups, falls back to DB if cache is empty.
pub async fn check_book_move(
    Extension(pool): Extension<PgPool>,
    Query(q): Query<BookCheckQuery>,
) -> Result<Json<BookCheckResponse>, AppError> {
    // Try in-memory cache first (instant lookup)
    if !BOOK_CACHE.is_empty() {
        if let Some(stats) = book_cache::lookup(&q.fen, &q.san) {
            return Ok(Json(BookCheckResponse {
                is_book: true,
                games: Some(stats.games),
                white_wins: Some(stats.white_wins),
                draws: Some(stats.draws),
                black_wins: Some(stats.black_wins),
            }));
        } else {
            // Cache is loaded but move not found - it's not a book move
            return Ok(Json(BookCheckResponse {
                is_book: false,
                games: None,
                white_wins: None,
                draws: None,
                black_wins: None,
            }));
        }
    }

    // Fallback to database query (cache not loaded)
    let normalized_fen = book_cache::normalize_fen(&q.fen);

    let row: Option<(i32, i32, i32, i32)> = sqlx::query_as(
        r#"SELECT games, white_wins, draws, black_wins
           FROM opening_book
           WHERE parent_fen = $1 AND move_san = $2"#,
    )
    .bind(&normalized_fen)
    .bind(&q.san)
    .fetch_optional(&pool)
    .await
    .map_err(AppError::Sqlx)?;

    match row {
        Some((games, white_wins, draws, black_wins)) => Ok(Json(BookCheckResponse {
            is_book: true,
            games: Some(games),
            white_wins: Some(white_wins),
            draws: Some(draws),
            black_wins: Some(black_wins),
        })),
        None => Ok(Json(BookCheckResponse {
            is_book: false,
            games: None,
            white_wins: None,
            draws: None,
            black_wins: None,
        })),
    }
}

/// Classify a move by cp_loss using the same thresholds as the analysis worker.
fn classify_by_cp_loss(cp_loss: i64) -> &'static str {
    if cp_loss <= 0 {
        "best"
    } else if cp_loss < 10 {
        "excellent"
    } else if cp_loss < 50 {
        "good"
    } else if cp_loss < 100 {
        "inaccuracy"
    } else if cp_loss < 200 {
        "mistake"
    } else {
        "blunder"
    }
}

/// POST /api/admin/opening-book/reclassify
/// Sync all game_analysis move classifications against the current opening_book table.
/// Moves in the book get classified as "book"; moves no longer in the book get
/// reclassified by cp_loss. Recalculates accuracy/stats for any changed game.
pub async fn reclassify_book_moves(
    headers: HeaderMap,
    Extension(pool): Extension<PgPool>,
    Extension(config): Extension<Config>,
) -> Result<Json<JsonValue>, AppError> {
    check_admin_secret(&headers, &config)?;

    // 1. Load the full opening book from DB into a HashMap<fen, HashSet<san>>
    let book_rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT parent_fen, move_san FROM opening_book",
    )
    .fetch_all(&pool)
    .await
    .map_err(AppError::Sqlx)?;

    let mut book: HashMap<String, HashSet<String>> = HashMap::new();
    for (fen, san) in &book_rows {
        book.entry(fen.clone()).or_default().insert(san.clone());
    }

    tracing::info!(
        "Reclassify: loaded {} book positions ({} total moves)",
        book.len(),
        book_rows.len()
    );

    // 2. Fetch all game_analysis rows (game_id + moves JSONB)
    let rows: Vec<(i64, JsonValue)> = sqlx::query_as(
        "SELECT game_id, moves FROM game_analysis WHERE moves IS NOT NULL",
    )
    .fetch_all(&pool)
    .await
    .map_err(AppError::Sqlx)?;

    let games_checked = rows.len() as i64;
    let mut games_updated: i64 = 0;
    let mut moves_reclassified: i64 = 0;

    // 3. Process each game
    for (game_id, moves_json) in &rows {
        let moves_arr = match moves_json.as_array() {
            Some(a) => a,
            None => continue,
        };

        let mut new_moves = moves_arr.clone();
        let mut changed = false;
        let mut pos = Chess::default();

        for (ply, mv) in moves_arr.iter().enumerate() {
            let uci_str = mv.get("move").and_then(|v| v.as_str()).unwrap_or("");
            let old_class = mv
                .get("classification")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            // Replay the move with shakmaty to get the SAN and normalized FEN
            let uci_move: UciMove = match uci_str.parse() {
                Ok(m) => m,
                Err(_) => {
                    // Can't parse — skip this move, advance position if possible
                    continue;
                }
            };
            let legal_move = match uci_move.to_move(&pos) {
                Ok(m) => m,
                Err(_) => continue,
            };
            let san = San::from_move(&pos, legal_move.clone()).to_string();

            // Normalize FEN: first 4 fields (position, side, castling, ep)
            let fen = {
                let full = shakmaty::fen::Fen::from_position(&pos, shakmaty::EnPassantMode::Legal).to_string();
                book_cache::normalize_fen(&full)
            };

            // Check book membership
            let is_in_book = book
                .get(&fen)
                .map(|moves| moves.contains(&san))
                .unwrap_or(false);

            let new_class = if is_in_book {
                "book"
            } else if old_class == "forced" {
                // Don't reclassify forced moves
                "forced"
            } else {
                let cp_loss = mv.get("cp_loss").and_then(|v| v.as_i64()).unwrap_or(0);
                classify_by_cp_loss(cp_loss)
            };

            if new_class != old_class {
                if let Some(obj) = new_moves[ply].as_object_mut() {
                    obj.insert(
                        "classification".to_string(),
                        JsonValue::String(new_class.to_string()),
                    );
                }
                changed = true;
                moves_reclassified += 1;
            }

            // Advance position
            pos.play_unchecked(legal_move);
        }

        if !changed {
            continue;
        }

        // Recalculate stats from the updated moves
        let new_moves_val = JsonValue::Array(new_moves.clone());

        // Classification counts
        let mut white_class = serde_json::Map::new();
        let mut black_class = serde_json::Map::new();
        for key in ["book", "best", "excellent", "good", "inaccuracy", "mistake", "blunder", "forced"] {
            white_class.insert(key.to_string(), serde_json::json!(0));
            black_class.insert(key.to_string(), serde_json::json!(0));
        }

        let mut white_cp_total: f64 = 0.0;
        let mut black_cp_total: f64 = 0.0;
        let mut white_counted: u32 = 0;
        let mut black_counted: u32 = 0;

        for (i, mv) in new_moves.iter().enumerate() {
            let class = mv
                .get("classification")
                .and_then(|v| v.as_str())
                .unwrap_or("best");
            let cp_loss = mv.get("cp_loss").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let is_white = i % 2 == 0;

            let class_map = if is_white {
                &mut white_class
            } else {
                &mut black_class
            };
            if let Some(count) = class_map.get_mut(class) {
                *count = serde_json::json!(count.as_i64().unwrap_or(0) + 1);
            }

            // Accuracy: skip book and forced moves
            if class != "book" && class != "forced" {
                if is_white {
                    white_cp_total += cp_loss;
                    white_counted += 1;
                } else {
                    black_cp_total += cp_loss;
                    black_counted += 1;
                }
            }
        }

        let white_accuracy = if white_counted > 0 {
            let acpl = white_cp_total / white_counted as f64;
            (100.0 * (1.0 / (1.0 + acpl / 100.0)).sqrt()).max(0.0).min(100.0)
        } else {
            100.0
        };
        let black_accuracy = if black_counted > 0 {
            let acpl = black_cp_total / black_counted as f64;
            (100.0 * (1.0 / (1.0 + acpl / 100.0)).sqrt()).max(0.0).min(100.0)
        } else {
            100.0
        };
        let white_avg_cp = if white_counted > 0 {
            white_cp_total / white_counted as f64
        } else {
            0.0
        };
        let black_avg_cp = if black_counted > 0 {
            black_cp_total / black_counted as f64
        } else {
            0.0
        };

        // Phase accuracy + first bad move (reuse existing helpers)
        let phase_acc = serde_json::json!({
            "white": phase_accuracy(&new_moves_val, true),
            "black": phase_accuracy(&new_moves_val, false),
        });
        let first_inacc = serde_json::json!({
            "white": first_bad_move(&new_moves_val, true, &["inaccuracy", "mistake", "blunder"]),
            "black": first_bad_move(&new_moves_val, false, &["inaccuracy", "mistake", "blunder"]),
            "white_mistake": first_bad_move(&new_moves_val, true, &["mistake", "blunder"]),
            "black_mistake": first_bad_move(&new_moves_val, false, &["mistake", "blunder"]),
            "white_blunder": first_bad_move(&new_moves_val, true, &["blunder"]),
            "black_blunder": first_bad_move(&new_moves_val, false, &["blunder"]),
        });

        // Update the row
        sqlx::query(
            r#"UPDATE game_analysis SET
                moves = $1,
                white_accuracy = $2,
                black_accuracy = $3,
                white_avg_cp_loss = $4,
                black_avg_cp_loss = $5,
                white_classifications = $6,
                black_classifications = $7,
                phase_accuracy = $8,
                first_inaccuracy_move = $9
            WHERE game_id = $10"#,
        )
        .bind(&new_moves_val)
        .bind(white_accuracy)
        .bind(black_accuracy)
        .bind(white_avg_cp)
        .bind(black_avg_cp)
        .bind(&JsonValue::Object(white_class))
        .bind(&JsonValue::Object(black_class))
        .bind(&phase_acc)
        .bind(&first_inacc)
        .bind(game_id)
        .execute(&pool)
        .await
        .map_err(AppError::Sqlx)?;

        games_updated += 1;
    }

    // Invalidate dashboard cache so users see updated stats immediately
    dashboard::invalidate_stats_cache();

    tracing::info!(
        "Reclassify complete: checked={}, updated={}, moves_changed={}",
        games_checked,
        games_updated,
        moves_reclassified
    );

    Ok(Json(serde_json::json!({
        "games_checked": games_checked,
        "games_updated": games_updated,
        "moves_reclassified": moves_reclassified,
    })))
}
