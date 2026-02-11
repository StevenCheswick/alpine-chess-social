use std::collections::HashMap;

use serde_json::Value as JsonValue;
use sqlx::PgPool;

use crate::db::opening_moves;
use crate::error::AppError;

pub async fn save_game_analysis(
    pool: &PgPool,
    game_id: i64,
    analysis: &JsonValue,
) -> Result<(), AppError> {
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
    let endgame_segments = analysis.get("endgame_segments").cloned().unwrap_or(JsonValue::Null);

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
    .bind(analysis.get("white_avg_cp_loss").and_then(|v| v.as_f64()).unwrap_or(0.0))
    .bind(analysis.get("black_avg_cp_loss").and_then(|v| v.as_f64()).unwrap_or(0.0))
    .bind(&analysis["white_classifications"])
    .bind(&analysis["black_classifications"])
    .bind(moves)
    .bind(&phase_acc)
    .bind(&first_inacc)
    .bind(&puzzles)
    .bind(&endgame_segments)
    .execute(pool)
    .await
    .map_err(AppError::Sqlx)?;

    // Mark game as analyzed
    sqlx::query("UPDATE user_games SET analyzed_at = NOW() WHERE id = $1")
        .bind(game_id)
        .execute(pool)
        .await
        .map_err(AppError::Sqlx)?;

    // Save tags from cook() themes + endgame types (if present in payload)
    if let Some(tags) = analysis.get("tags").and_then(|t| t.as_array()) {
        // Delete old tags
        sqlx::query("DELETE FROM game_tags WHERE game_id = $1")
            .bind(game_id)
            .execute(pool)
            .await
            .map_err(AppError::Sqlx)?;

        // Insert new tags
        for tag in tags {
            if let Some(tag_str) = tag.as_str() {
                sqlx::query(
                    "INSERT INTO game_tags (game_id, tag) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                )
                .bind(game_id)
                .bind(tag_str)
                .execute(pool)
                .await
                .map_err(AppError::Sqlx)?;
            }
        }

        // Update denormalized tags JSON in user_games
        sqlx::query("UPDATE user_games SET tags = $2 WHERE id = $1")
            .bind(game_id)
            .bind(serde_json::to_value(tags).unwrap_or_default())
            .execute(pool)
            .await
            .map_err(AppError::Sqlx)?;
    }

    // Enrich opening stats with eval data from this analysis
    opening_moves::enrich_opening_evals(pool, game_id).await?;

    Ok(())
}

pub async fn get_game_analysis(
    pool: &PgPool,
    game_id: i64,
) -> Result<Option<JsonValue>, AppError> {
    use sqlx::Row;

    let row = sqlx::query(
        r#"SELECT white_accuracy, black_accuracy, white_avg_cp_loss, black_avg_cp_loss,
                  white_classifications, black_classifications, moves,
                  puzzles, endgame_segments
           FROM game_analysis WHERE game_id = $1"#,
    )
    .bind(game_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Sqlx)?;

    Ok(row.map(|r| {
        let mut result = serde_json::json!({
            "white_accuracy": r.try_get::<f64, _>("white_accuracy").unwrap_or(0.0),
            "black_accuracy": r.try_get::<f64, _>("black_accuracy").unwrap_or(0.0),
            "white_avg_cp_loss": r.try_get::<f64, _>("white_avg_cp_loss").unwrap_or(0.0),
            "black_avg_cp_loss": r.try_get::<f64, _>("black_avg_cp_loss").unwrap_or(0.0),
            "white_classifications": r.try_get::<JsonValue, _>("white_classifications").unwrap_or(JsonValue::Object(serde_json::Map::new())),
            "black_classifications": r.try_get::<JsonValue, _>("black_classifications").unwrap_or(JsonValue::Object(serde_json::Map::new())),
            "moves": r.try_get::<JsonValue, _>("moves").unwrap_or(JsonValue::Array(vec![])),
            "isComplete": true,
        });

        if let Ok(Some(puzzles)) = r.try_get::<Option<JsonValue>, _>("puzzles") {
            result["puzzles"] = puzzles;
        }
        if let Ok(Some(segments)) = r.try_get::<Option<JsonValue>, _>("endgame_segments") {
            result["endgame_segments"] = segments;
        }

        result
    }))
}

/// Get analyzed game stats for dashboard charts.
pub async fn get_user_game_stats(
    pool: &PgPool,
    user_id: i64,
) -> Result<Vec<JsonValue>, AppError> {
    use sqlx::Row;

    let rows = sqlx::query(
        r#"SELECT ug.id, ug.date, ug.user_rating, ug.result, ug.user_color,
                  ug.opponent, ug.opponent_rating,
                  ga.white_accuracy, ga.black_accuracy,
                  ga.white_classifications, ga.black_classifications,
                  ga.phase_accuracy, ga.first_inaccuracy_move
           FROM user_games ug
           INNER JOIN game_analysis ga ON ug.id = ga.game_id
           WHERE ug.user_id = $1
           ORDER BY ug.date ASC"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Sqlx)?;

    Ok(rows
        .into_iter()
        .map(|r| {
            let user_color: String = r.try_get("user_color").unwrap_or_default();
            let is_white = user_color.to_lowercase() == "white";
            let color_key = if is_white { "white" } else { "black" };
            let accuracy: f64 = if is_white {
                r.try_get("white_accuracy").unwrap_or(0.0)
            } else {
                r.try_get("black_accuracy").unwrap_or(0.0)
            };
            let classifications: JsonValue = if is_white {
                r.try_get("white_classifications").unwrap_or(JsonValue::Object(serde_json::Map::new()))
            } else {
                r.try_get("black_classifications").unwrap_or(JsonValue::Object(serde_json::Map::new()))
            };

            let phase_accuracy_val: Option<JsonValue> = r.try_get("phase_accuracy").unwrap_or(None);
            let phase_acc = phase_accuracy_val
                .as_ref()
                .and_then(|pa| pa.get(color_key).cloned())
                .unwrap_or(JsonValue::Object(serde_json::Map::new()));

            let first_inaccuracy_val: Option<JsonValue> = r.try_get("first_inaccuracy_move").unwrap_or(None);
            let first_inaccuracy = first_inaccuracy_val
                .as_ref()
                .and_then(|fi| fi.get(color_key))
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            serde_json::json!({
                "id": r.try_get::<i64, _>("id").unwrap_or(0),
                "date": r.try_get::<Option<String>, _>("date").unwrap_or(None),
                "user_rating": r.try_get::<Option<i32>, _>("user_rating").unwrap_or(None),
                "result": r.try_get::<String, _>("result").unwrap_or_default(),
                "opponent": r.try_get::<String, _>("opponent").unwrap_or_default(),
                "opponent_rating": r.try_get::<Option<i32>, _>("opponent_rating").unwrap_or(None),
                "user_color": user_color,
                "accuracy": accuracy,
                "classifications": classifications,
                "phase_accuracy": phase_acc,
                "first_inaccuracy": first_inaccuracy,
            })
        })
        .collect())
}

// ---- Pre-computed stats helpers ----

fn phase_accuracy(moves: &JsonValue, is_white: bool) -> JsonValue {
    let arr = match moves.as_array() {
        Some(a) => a,
        None => return serde_json::json!({"opening": null, "middlegame": null, "endgame": null}),
    };

    let mut buckets: [Vec<f64>; 3] = [vec![], vec![], vec![]]; // opening, middlegame, endgame
    let mut user_move_num = 0;

    for (i, mv) in arr.iter().enumerate() {
        let is_user = (i % 2 == 0) == is_white;
        if !is_user {
            continue;
        }
        user_move_num += 1;

        let class = mv.get("classification").and_then(|c| c.as_str()).unwrap_or("");
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

/// Get all puzzles for a user, enriched with game metadata.
/// Optionally filters by theme (case-insensitive substring match).
pub async fn get_user_puzzles(
    pool: &PgPool,
    user_id: i64,
    theme_filter: Option<&str>,
) -> Result<Vec<JsonValue>, AppError> {
    use sqlx::Row;

    let rows = sqlx::query(
        r#"SELECT ug.id AS game_id, ug.opponent, ug.date, ug.user_color, ug.source,
                  ga.puzzles
           FROM user_games ug
           INNER JOIN game_analysis ga ON ug.id = ga.game_id
           WHERE ug.user_id = $1 AND ga.puzzles IS NOT NULL
           ORDER BY ug.date DESC"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Sqlx)?;

    let mut result = Vec::new();
    let mut puzzle_idx: u64 = 0;

    for row in &rows {
        let game_id: i64 = row.try_get("game_id").unwrap_or(0);
        let opponent: String = row.try_get("opponent").unwrap_or_default();
        let date: Option<String> = row.try_get("date").unwrap_or(None);
        let user_color: String = row.try_get("user_color").unwrap_or_default();
        let source: String = row.try_get("source").unwrap_or_default();
        let puzzles_json: Option<JsonValue> = row.try_get("puzzles").unwrap_or(None);

        let puzzles = match puzzles_json.and_then(|v| v.as_array().cloned()) {
            Some(arr) => arr,
            None => continue,
        };

        for puzzle in puzzles {
            // Apply theme filter if provided
            if let Some(filter) = theme_filter {
                let themes = puzzle
                    .get("themes")
                    .and_then(|t| t.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                if !themes.iter().any(|t| t.eq_ignore_ascii_case(filter)) {
                    continue;
                }
            }

            let mut enriched = puzzle.clone();
            if let Some(obj) = enriched.as_object_mut() {
                obj.insert("id".to_string(), serde_json::json!(format!("p{puzzle_idx}")));
                obj.insert("gameId".to_string(), serde_json::json!(game_id));
                obj.insert("opponent".to_string(), serde_json::json!(opponent));
                obj.insert("date".to_string(), serde_json::json!(date));
                obj.insert("userColor".to_string(), serde_json::json!(user_color));
                obj.insert("source".to_string(), serde_json::json!(source));
            }

            result.push(enriched);
            puzzle_idx += 1;
        }
    }

    Ok(result)
}

/// Get theme counts across all user puzzles.
pub async fn get_user_puzzle_themes(
    pool: &PgPool,
    user_id: i64,
) -> Result<HashMap<String, i64>, AppError> {
    use sqlx::Row;

    let rows = sqlx::query(
        r#"SELECT ga.puzzles
           FROM user_games ug
           INNER JOIN game_analysis ga ON ug.id = ga.game_id
           WHERE ug.user_id = $1 AND ga.puzzles IS NOT NULL"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Sqlx)?;

    let mut counts: HashMap<String, i64> = HashMap::new();

    for row in &rows {
        let puzzles_json: Option<JsonValue> = row.try_get("puzzles").unwrap_or(None);
        let puzzles = match puzzles_json.and_then(|v| v.as_array().cloned()) {
            Some(arr) => arr,
            None => continue,
        };

        for puzzle in puzzles {
            if let Some(themes) = puzzle.get("themes").and_then(|t| t.as_array()) {
                for theme in themes {
                    if let Some(t) = theme.as_str() {
                        *counts.entry(t.to_string()).or_insert(0) += 1;
                    }
                }
            }
        }
    }

    Ok(counts)
}

/// Per-endgame-type stats aggregated across all analyzed games for a user.
pub async fn get_user_endgame_stats(
    pool: &PgPool,
    user_id: i64,
) -> Result<JsonValue, AppError> {
    use sqlx::Row;

    // Unnest endgame_segments JSONB arrays, join with user_games for color info,
    // then aggregate per endgame_type.
    let rows = sqlx::query(
        r#"
        WITH segments AS (
            SELECT
                ug.id AS game_id,
                LOWER(ug.user_color) AS user_color,
                seg->>'endgame_type' AS endgame_type,
                (seg->>'white_cp_loss')::double precision AS white_cp_loss,
                (seg->>'white_moves')::int AS white_moves,
                (seg->>'black_cp_loss')::double precision AS black_cp_loss,
                (seg->>'black_moves')::int AS black_moves
            FROM user_games ug
            INNER JOIN game_analysis ga ON ug.id = ga.game_id,
            jsonb_array_elements(ga.endgame_segments) AS seg
            WHERE ug.user_id = $1
              AND ga.endgame_segments IS NOT NULL
              AND jsonb_array_length(ga.endgame_segments) > 0
        )
        SELECT
            endgame_type,
            COUNT(DISTINCT game_id)::bigint AS games,
            SUM(CASE WHEN user_color = 'white' THEN white_cp_loss ELSE black_cp_loss END) AS user_total_cp_loss,
            SUM(CASE WHEN user_color = 'white' THEN white_moves ELSE black_moves END)::bigint AS user_total_moves,
            SUM(CASE WHEN user_color = 'white' THEN black_cp_loss ELSE white_cp_loss END) AS opp_total_cp_loss,
            SUM(CASE WHEN user_color = 'white' THEN black_moves ELSE white_moves END)::bigint AS opp_total_moves
        FROM segments
        WHERE endgame_type IS NOT NULL
        GROUP BY endgame_type
        ORDER BY games DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Sqlx)?;

    let mut total_games_with_endgame: i64 = 0;
    let type_stats: Vec<JsonValue> = rows
        .iter()
        .map(|r| {
            let endgame_type: String = r.try_get("endgame_type").unwrap_or_default();
            let games: i64 = r.try_get("games").unwrap_or(0);
            let user_total_cp: f64 = r.try_get("user_total_cp_loss").unwrap_or(0.0);
            let user_total_moves: i64 = r.try_get("user_total_moves").unwrap_or(0);
            let opp_total_cp: f64 = r.try_get("opp_total_cp_loss").unwrap_or(0.0);
            let opp_total_moves: i64 = r.try_get("opp_total_moves").unwrap_or(0);

            total_games_with_endgame += games;

            let user_avg = if user_total_moves > 0 {
                (user_total_cp / user_total_moves as f64 * 10.0).round() / 10.0
            } else {
                0.0
            };
            let opp_avg = if opp_total_moves > 0 {
                (opp_total_cp / opp_total_moves as f64 * 10.0).round() / 10.0
            } else {
                0.0
            };

            serde_json::json!({
                "type": endgame_type,
                "games": games,
                "userAvgCpLoss": user_avg,
                "opponentAvgCpLoss": opp_avg,
            })
        })
        .collect();

    Ok(serde_json::json!({
        "totalGamesWithEndgame": total_games_with_endgame,
        "typeStats": type_stats,
    }))
}

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
        let class = mv.get("classification").and_then(|c| c.as_str()).unwrap_or("");
        if bad.contains(&class) {
            return user_move_num;
        }
    }

    0
}
