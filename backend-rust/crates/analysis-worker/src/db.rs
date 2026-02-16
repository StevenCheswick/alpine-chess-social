//! Database queries for game fetching and analysis storage

use serde_json::Value as JsonValue;
use sqlx::PgPool;

use crate::error::WorkerError;

/// Game data needed for analysis
#[derive(Debug)]
pub struct GameData {
    pub id: i64,
    pub tcn: String,
    pub user_color: String,
}

/// Fetch game data by ID
pub async fn fetch_game(pool: &PgPool, game_id: i64) -> Result<Option<GameData>, WorkerError> {
    let row: Option<(i64, String, String)> = sqlx::query_as(
        "SELECT id, tcn, user_color FROM user_games WHERE id = $1",
    )
    .bind(game_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(id, tcn, user_color)| GameData {
        id,
        tcn,
        user_color,
    }))
}

/// Save game analysis results
pub async fn save_game_analysis(
    pool: &PgPool,
    game_id: i64,
    analysis: &JsonValue,
) -> Result<(), WorkerError> {
    let moves = &analysis["moves"];

    // Pre-compute dashboard stats
    let phase_acc = serde_json::json!({
        "white": phase_accuracy(moves, true),
        "black": phase_accuracy(moves, false),
    });
    let first_inacc = serde_json::json!({
        "white": first_inaccuracy_move(moves, true),
        "black": first_inaccuracy_move(moves, false),
    });

    let puzzles = analysis.get("puzzles").cloned().unwrap_or(JsonValue::Null);
    let endgame_segments = analysis
        .get("endgame_segments")
        .cloned()
        .unwrap_or(JsonValue::Null);

    sqlx::query(
        r#"INSERT INTO game_analysis (
            game_id, white_accuracy, black_accuracy,
            white_avg_cp_loss, black_avg_cp_loss,
            white_classifications, black_classifications,
            moves, phase_accuracy, first_inaccuracy_move,
            puzzles, endgame_segments
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        ON CONFLICT (game_id) DO UPDATE SET
            white_accuracy = EXCLUDED.white_accuracy,
            black_accuracy = EXCLUDED.black_accuracy,
            white_avg_cp_loss = EXCLUDED.white_avg_cp_loss,
            black_avg_cp_loss = EXCLUDED.black_avg_cp_loss,
            white_classifications = EXCLUDED.white_classifications,
            black_classifications = EXCLUDED.black_classifications,
            moves = EXCLUDED.moves,
            phase_accuracy = EXCLUDED.phase_accuracy,
            first_inaccuracy_move = EXCLUDED.first_inaccuracy_move,
            puzzles = EXCLUDED.puzzles,
            endgame_segments = EXCLUDED.endgame_segments"#,
    )
    .bind(game_id)
    .bind(analysis["white_accuracy"].as_f64().unwrap_or(0.0))
    .bind(analysis["black_accuracy"].as_f64().unwrap_or(0.0))
    .bind(
        analysis
            .get("white_avg_cp_loss")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
    )
    .bind(
        analysis
            .get("black_avg_cp_loss")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
    )
    .bind(&analysis["white_classifications"])
    .bind(&analysis["black_classifications"])
    .bind(moves)
    .bind(&phase_acc)
    .bind(&first_inacc)
    .bind(&puzzles)
    .bind(&endgame_segments)
    .execute(pool)
    .await?;

    // Mark game as analyzed
    sqlx::query("UPDATE user_games SET analyzed_at = NOW() WHERE id = $1")
        .bind(game_id)
        .execute(pool)
        .await?;

    // Save tags from puzzle themes
    if let Some(tags) = analysis.get("tags").and_then(|t| t.as_array()) {
        // Delete old tags
        sqlx::query("DELETE FROM game_tags WHERE game_id = $1")
            .bind(game_id)
            .execute(pool)
            .await?;

        // Insert new tags
        for tag in tags {
            if let Some(tag_str) = tag.as_str() {
                sqlx::query(
                    "INSERT INTO game_tags (game_id, tag) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                )
                .bind(game_id)
                .bind(tag_str)
                .execute(pool)
                .await?;
            }
        }

        // Update denormalized tags JSON in user_games
        sqlx::query("UPDATE user_games SET tags = $2 WHERE id = $1")
            .bind(game_id)
            .bind(serde_json::to_value(tags).unwrap_or_default())
            .execute(pool)
            .await?;
    }

    Ok(())
}

/// Check if a move exists in the opening book
pub async fn is_book_move(pool: &PgPool, fen: &str, san: &str) -> bool {
    let normalized = normalize_fen(fen);
    let result: Result<Option<(i32,)>, _> = sqlx::query_as(
        "SELECT games FROM opening_book WHERE parent_fen = $1 AND move_san = $2",
    )
    .bind(&normalized)
    .bind(san)
    .fetch_optional(pool)
    .await;

    matches!(result, Ok(Some(_)))
}

/// Normalize FEN to 4 parts for opening book lookup
fn normalize_fen(fen: &str) -> String {
    fen.split_whitespace().take(4).collect::<Vec<_>>().join(" ")
}

/// Calculate per-phase accuracy from moves
fn phase_accuracy(moves: &JsonValue, is_white: bool) -> JsonValue {
    let arr = match moves.as_array() {
        Some(a) => a,
        None => {
            return serde_json::json!({"opening": null, "middlegame": null, "endgame": null})
        }
    };

    let mut buckets: [Vec<f64>; 3] = [vec![], vec![], vec![]]; // opening, middlegame, endgame
    let mut user_move_num = 0;

    for (i, mv) in arr.iter().enumerate() {
        let is_user = (i % 2 == 0) == is_white;
        if !is_user {
            continue;
        }
        user_move_num += 1;

        let class = mv
            .get("classification")
            .and_then(|c| c.as_str())
            .unwrap_or("");
        if class == "book" || class == "forced" {
            continue;
        }

        let cp_loss = mv.get("cp_loss").and_then(|v| v.as_f64()).unwrap_or(0.0);

        match user_move_num {
            1..=10 => buckets[0].push(cp_loss),
            11..=25 => buckets[1].push(cp_loss),
            _ => buckets[2].push(cp_loss),
        }
    }

    let names = ["opening", "middlegame", "endgame"];
    let mut result = serde_json::Map::new();
    for (idx, losses) in buckets.iter().enumerate() {
        if losses.is_empty() {
            result.insert(names[idx].to_string(), JsonValue::Null);
        } else {
            let avg: f64 = losses.iter().sum::<f64>() / losses.len() as f64;
            let acc = 100.0 / (1.0 + avg / 100.0).sqrt();
            result.insert(
                names[idx].to_string(),
                serde_json::json!((acc * 10.0).round() / 10.0),
            );
        }
    }

    JsonValue::Object(result)
}

/// Find the first inaccuracy/mistake/blunder move number
fn first_inaccuracy_move(moves: &JsonValue, is_white: bool) -> i64 {
    let arr = match moves.as_array() {
        Some(a) => a,
        None => return 0,
    };

    let bad = ["inaccuracy", "mistake", "blunder"];
    let mut user_move_num = 0;

    for (i, mv) in arr.iter().enumerate() {
        let is_user = (i % 2 == 0) == is_white;
        if !is_user {
            continue;
        }
        user_move_num += 1;
        let class = mv
            .get("classification")
            .and_then(|c| c.as_str())
            .unwrap_or("");
        if bad.contains(&class) {
            return user_move_num;
        }
    }

    0
}
