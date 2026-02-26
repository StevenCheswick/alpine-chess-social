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
        "white": first_bad_move(moves, true, &["inaccuracy", "mistake", "blunder"]),
        "black": first_bad_move(moves, false, &["inaccuracy", "mistake", "blunder"]),
        "white_mistake": first_bad_move(moves, true, &["mistake", "blunder"]),
        "black_mistake": first_bad_move(moves, false, &["mistake", "blunder"]),
        "white_blunder": first_bad_move(moves, true, &["blunder"]),
        "black_blunder": first_bad_move(moves, false, &["blunder"]),
    });

    let puzzles = analysis.get("puzzles").cloned().unwrap_or(JsonValue::Null);
    let endgame_segments = analysis
        .get("endgame_segments")
        .cloned()
        .unwrap_or(JsonValue::Null);

    let tags = analysis.get("tags").and_then(|t| t.as_array());
    let tags_json = tags
        .map(|t| serde_json::to_value(t).unwrap_or_default())
        .unwrap_or(JsonValue::Null);

    let mut tx = pool.begin().await?;

    // 1. Upsert game_analysis
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
    .execute(&mut *tx)
    .await?;

    // 2. Mark game as analyzed + update denormalized tags (single UPDATE)
    sqlx::query("UPDATE user_games SET analyzed_at = NOW(), tags = $2 WHERE id = $1")
        .bind(game_id)
        .bind(&tags_json)
        .execute(&mut *tx)
        .await?;

    // 3. Replace analysis tags only (preserve title tags like "titled", "GM", etc.)
    if let Some(tags) = tags {
        let analysis_tags = &[
            "queen_sacrifice", "rook_sacrifice", "smothered_mate",
            "king_mate", "castling_mate", "en_passant_mate",
        ];
        sqlx::query(
            "DELETE FROM game_tags WHERE game_id = $1 AND tag = ANY($2::text[])"
        )
            .bind(game_id)
            .bind(analysis_tags)
            .execute(&mut *tx)
            .await?;

        let tag_strings: Vec<&str> = tags.iter().filter_map(|t| t.as_str()).collect();
        if !tag_strings.is_empty() {
            sqlx::query(
                "INSERT INTO game_tags (game_id, tag)
                 SELECT $1, UNNEST($2::text[])
                 ON CONFLICT DO NOTHING",
            )
            .bind(game_id)
            .bind(&tag_strings)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;

    Ok(())
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

/// Find the first move matching any of the given classifications
fn first_bad_move(moves: &JsonValue, is_white: bool, bad: &[&str]) -> i64 {
    let arr = match moves.as_array() {
        Some(a) => a,
        None => return 0,
    };

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
