use serde_json::Value as JsonValue;
use sqlx::PgPool;

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

    sqlx::query(
        r#"INSERT INTO game_analysis (
            game_id, white_accuracy, black_accuracy,
            white_avg_cp_loss, black_avg_cp_loss,
            white_classifications, black_classifications,
            moves, phase_accuracy, first_inaccuracy_move
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        ON CONFLICT (game_id) DO UPDATE SET
            white_accuracy = EXCLUDED.white_accuracy,
            black_accuracy = EXCLUDED.black_accuracy,
            white_avg_cp_loss = EXCLUDED.white_avg_cp_loss,
            black_avg_cp_loss = EXCLUDED.black_avg_cp_loss,
            white_classifications = EXCLUDED.white_classifications,
            black_classifications = EXCLUDED.black_classifications,
            moves = EXCLUDED.moves,
            phase_accuracy = EXCLUDED.phase_accuracy,
            first_inaccuracy_move = EXCLUDED.first_inaccuracy_move"#,
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
    .execute(pool)
    .await
    .map_err(AppError::Sqlx)?;

    // Mark game as analyzed
    sqlx::query("UPDATE user_games SET analyzed_at = NOW() WHERE id = $1")
        .bind(game_id)
        .execute(pool)
        .await
        .map_err(AppError::Sqlx)?;

    Ok(())
}

pub async fn get_game_analysis(
    pool: &PgPool,
    game_id: i64,
) -> Result<Option<JsonValue>, AppError> {
    use sqlx::Row;

    let row = sqlx::query(
        r#"SELECT white_accuracy, black_accuracy, white_avg_cp_loss, black_avg_cp_loss,
                  white_classifications, black_classifications, moves
           FROM game_analysis WHERE game_id = $1"#,
    )
    .bind(game_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Sqlx)?;

    Ok(row.map(|r| {
        serde_json::json!({
            "white_accuracy": r.try_get::<f64, _>("white_accuracy").unwrap_or(0.0),
            "black_accuracy": r.try_get::<f64, _>("black_accuracy").unwrap_or(0.0),
            "white_avg_cp_loss": r.try_get::<f64, _>("white_avg_cp_loss").unwrap_or(0.0),
            "black_avg_cp_loss": r.try_get::<f64, _>("black_avg_cp_loss").unwrap_or(0.0),
            "white_classifications": r.try_get::<JsonValue, _>("white_classifications").unwrap_or(JsonValue::Object(serde_json::Map::new())),
            "black_classifications": r.try_get::<JsonValue, _>("black_classifications").unwrap_or(JsonValue::Object(serde_json::Map::new())),
            "moves": r.try_get::<JsonValue, _>("moves").unwrap_or(JsonValue::Array(vec![])),
            "isComplete": true,
        })
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
