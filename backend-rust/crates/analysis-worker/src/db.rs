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

    // Precompute opening stats for dashboard (after commit, non-transactional)
    precompute_opening_stats(pool, game_id).await?;

    Ok(())
}

/// Precompute opening mistake and clean-line rows for a single game.
async fn precompute_opening_stats(pool: &PgPool, game_id: i64) -> Result<(), WorkerError> {
    use sqlx::Row;

    let row = sqlx::query(
        r#"SELECT ug.user_id, ug.user_color, ga.moves
           FROM user_games ug
           INNER JOIN game_analysis ga ON ug.id = ga.game_id
           WHERE ug.id = $1"#,
    )
    .bind(game_id)
    .fetch_optional(pool)
    .await?;

    let row = match row {
        Some(r) => r,
        None => return Ok(()),
    };

    let user_id: i64 = row.try_get("user_id").unwrap_or(0);
    let user_color: String = row.try_get("user_color").unwrap_or_default();
    let color = user_color.to_lowercase();
    let is_white = color == "white";
    let moves_json: JsonValue = row.try_get("moves").unwrap_or(JsonValue::Null);

    let moves_arr = match moves_json.as_array() {
        Some(a) => a,
        None => return Ok(()),
    };

    let max_ply = moves_arr.len().min(30);

    // Delete old rows for this game
    sqlx::query("DELETE FROM game_opening_mistakes WHERE game_id = $1")
        .bind(game_id)
        .execute(pool)
        .await?;

    // Walk moves, collect mistakes and find first_mistake_ply
    let mut first_mistake_ply: Option<usize> = None;

    for ply in 0..max_ply {
        let mv = &moves_arr[ply];
        let is_user_move = (ply % 2 == 0) == is_white;
        if !is_user_move {
            continue;
        }

        let classification = mv.get("classification").and_then(|c| c.as_str()).unwrap_or("");
        if classification == "book" || classification == "forced" {
            continue;
        }

        let cp_loss = mv.get("cp_loss").and_then(|v| v.as_f64()).unwrap_or(0.0);

        if cp_loss >= 50.0 {
            if first_mistake_ply.is_none() {
                first_mistake_ply = Some(ply);
            }

            let line: String = moves_arr[..=ply]
                .iter()
                .filter_map(|m| m.get("move").and_then(|v| v.as_str()))
                .collect::<Vec<_>>()
                .join(" ");

            let move_san = mv.get("move").and_then(|v| v.as_str()).unwrap_or("");
            let best_move = mv.get("best_move").and_then(|v| v.as_str());

            sqlx::query(
                r#"INSERT INTO game_opening_mistakes (game_id, user_id, ply, move_san, cp_loss, best_move, color, line)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                   ON CONFLICT (game_id, ply) DO UPDATE SET
                       move_san = EXCLUDED.move_san,
                       cp_loss = EXCLUDED.cp_loss,
                       best_move = EXCLUDED.best_move,
                       color = EXCLUDED.color,
                       line = EXCLUDED.line"#,
            )
            .bind(game_id)
            .bind(user_id)
            .bind(ply as i16)
            .bind(move_san)
            .bind(cp_loss)
            .bind(best_move)
            .bind(&color)
            .bind(&line)
            .execute(pool)
            .await?;
        }
    }

    // Compute clean_up_to
    let total_moves = moves_arr.len();
    let clean_up_to = match first_mistake_ply {
        Some(fmp) if fmp > 0 => (fmp - 1).min(total_moves),
        Some(_) => 0,
        None => {
            let max_clean = if is_white { 29 } else { 30 };
            max_clean.min(total_moves)
        }
    };

    let line = if clean_up_to > 0 {
        moves_arr[..clean_up_to]
            .iter()
            .filter_map(|m| m.get("move").and_then(|v| v.as_str()))
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        String::new()
    };

    let mut cp_sum = 0.0;
    let mut cp_count = 0;
    for ply in 0..clean_up_to {
        let mv = &moves_arr[ply];
        let is_user_move = (ply % 2 == 0) == is_white;
        if !is_user_move {
            continue;
        }
        let classification = mv.get("classification").and_then(|c| c.as_str()).unwrap_or("");
        if classification == "book" || classification == "forced" {
            continue;
        }
        let cp_loss = mv.get("cp_loss").and_then(|v| v.as_f64()).unwrap_or(0.0);
        cp_sum += cp_loss;
        cp_count += 1;
    }
    let avg_cp_loss = if cp_count > 0 {
        (cp_sum / cp_count as f64 * 10.0).round() / 10.0
    } else {
        0.0
    };

    let clean_depth = (clean_up_to / 2 + clean_up_to % 2) as i16;

    if !line.is_empty() {
        sqlx::query(
            r#"INSERT INTO game_opening_clean_plies (game_id, user_id, color, clean_up_to, clean_depth, line, avg_cp_loss)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               ON CONFLICT (game_id) DO UPDATE SET
                   clean_up_to = EXCLUDED.clean_up_to,
                   clean_depth = EXCLUDED.clean_depth,
                   line = EXCLUDED.line,
                   avg_cp_loss = EXCLUDED.avg_cp_loss"#,
        )
        .bind(game_id)
        .bind(user_id)
        .bind(&color)
        .bind(clean_up_to as i16)
        .bind(clean_depth)
        .bind(&line)
        .bind(avg_cp_loss)
        .execute(pool)
        .await?;
    } else {
        sqlx::query("DELETE FROM game_opening_clean_plies WHERE game_id = $1")
            .bind(game_id)
            .execute(pool)
            .await?;
    }

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
