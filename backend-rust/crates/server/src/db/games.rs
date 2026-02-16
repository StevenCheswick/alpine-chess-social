use serde_json::Value as JsonValue;
use sqlx::PgPool;

use crate::error::AppError;

// Virtual tag mappings
pub const RESULT_TAGS: &[(&str, &str)] = &[("Win", "W"), ("Loss", "L"), ("Draw", "D")];
pub const PLATFORM_TAGS: &[(&str, &str)] = &[("Chess.com", "chess_com")];

fn result_to_tag(result: &str) -> Option<&'static str> {
    match result {
        "W" => Some("Win"),
        "L" => Some("Loss"),
        "D" => Some("Draw"),
        _ => None,
    }
}

fn source_to_tag(source: &str) -> Option<&'static str> {
    match source {
        "chess_com" => Some("Chess.com"),
        _ => None,
    }
}

/// Upsert a batch of games for a user. Returns number of games inserted/updated.
pub async fn upsert_games(
    pool: &PgPool,
    user_id: i64,
    games: &[serde_json::Value],
    source: &str,
) -> Result<i64, AppError> {
    let mut count = 0i64;

    for game in games {
        let game_id_str = game["id"].as_str().unwrap_or("");
        let opponent = game["opponent"].as_str().unwrap_or("");
        let opponent_rating = game["opponentRating"].as_i64().map(|v| v as i32);
        let user_rating = game["userRating"].as_i64().map(|v| v as i32);
        let result = game["result"].as_str().unwrap_or("");
        let user_color = game["userColor"].as_str().unwrap_or("");
        let time_control = game["timeControl"].as_str();
        let date = game["date"].as_str();
        let tcn = game["tcn"].as_str();
        let tags = &game["tags"];

        sqlx::query(
            r#"INSERT INTO user_games (
                user_id, chess_com_game_id, opponent, opponent_rating, user_rating,
                result, user_color, time_control, date, tags, source, tcn
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            ON CONFLICT (user_id, source, chess_com_game_id) DO UPDATE SET
                opponent = EXCLUDED.opponent,
                opponent_rating = EXCLUDED.opponent_rating,
                user_rating = EXCLUDED.user_rating,
                result = EXCLUDED.result,
                user_color = EXCLUDED.user_color,
                time_control = EXCLUDED.time_control,
                date = EXCLUDED.date,
                tags = EXCLUDED.tags,
                tcn = EXCLUDED.tcn,
                updated_at = NOW()"#,
        )
        .bind(user_id)
        .bind(game_id_str)
        .bind(opponent)
        .bind(opponent_rating)
        .bind(user_rating)
        .bind(result)
        .bind(user_color)
        .bind(time_control)
        .bind(date)
        .bind(tags)
        .bind(source)
        .bind(tcn)
        .execute(pool)
        .await
        .map_err(AppError::Sqlx)?;

        count += 1;
    }

    Ok(count)
}

pub async fn get_user_games_count(
    pool: &PgPool,
    user_id: i64,
    source: Option<&str>,
) -> Result<i64, AppError> {
    let count: (i64,) = if let Some(src) = source {
        sqlx::query_as(
            "SELECT COUNT(*) FROM user_games WHERE user_id = $1 AND source = $2",
        )
        .bind(user_id)
        .bind(src)
        .fetch_one(pool)
        .await
        .map_err(AppError::Sqlx)?
    } else {
        sqlx::query_as(
            "SELECT COUNT(*) FROM user_games WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(AppError::Sqlx)?
    };

    Ok(count.0)
}

/// Get paginated games with optional tag filters.
/// Handles virtual tags (Win/Loss/Draw, Chess.com/Lichess) and regular game_tags.
pub async fn get_user_games_paginated(
    pool: &PgPool,
    user_id: i64,
    limit: i64,
    offset: i64,
    tag_filters: Option<&[String]>,
    source: Option<&str>,
    analyzed: Option<bool>,
) -> Result<Vec<serde_json::Value>, AppError> {
    // Build dynamic query
    let mut conditions = vec!["ug.user_id = $1".to_string()];
    let mut params_i64: Vec<i64> = vec![user_id];
    let mut params_str: Vec<String> = Vec::new();

    if let Some(src) = source {
        params_str.push(src.to_string());
        conditions.push(format!("ug.source = ${}", params_i64.len() + params_str.len() + 1 - 1));
    }

    match analyzed {
        Some(true) => conditions.push("ga.id IS NOT NULL".to_string()),
        Some(false) => conditions.push("ga.id IS NULL".to_string()),
        None => {}
    }

    if let Some(filters) = tag_filters {
        // Separate virtual tags from regular tags
        for f in filters {
            if let Some(result_code) = RESULT_TAGS.iter().find(|(tag, _)| *tag == f.as_str()).map(|(_, code)| *code) {
                params_str.push(result_code.to_string());
                conditions.push(format!("ug.result = ${}", params_i64.len() + params_str.len()));
            } else if let Some(platform_code) = PLATFORM_TAGS.iter().find(|(tag, _)| *tag == f.as_str()).map(|(_, code)| *code) {
                params_str.push(platform_code.to_string());
                conditions.push(format!("ug.source = ${}", params_i64.len() + params_str.len()));
            }
        }

        let regular_tags: Vec<&String> = filters
            .iter()
            .filter(|t| {
                !RESULT_TAGS.iter().any(|(tag, _)| tag == &t.as_str())
                    && !PLATFORM_TAGS.iter().any(|(tag, _)| tag == &t.as_str())
            })
            .collect();

        if !regular_tags.is_empty() {
            let tag_values: Vec<String> = regular_tags.iter().map(|t| t.to_string()).collect();
            // Use a subquery for ALL-tag matching
            let placeholders: Vec<String> = tag_values
                .iter()
                .enumerate()
                .map(|(i, _)| format!("${}", params_i64.len() + params_str.len() + i + 1))
                .collect();

            let tag_count = tag_values.len();
            params_str.extend(tag_values);

            conditions.push(format!(
                "ug.id IN (SELECT game_id FROM game_tags WHERE tag IN ({}) GROUP BY game_id HAVING COUNT(DISTINCT tag) = {})",
                placeholders.join(","),
                tag_count
            ));
        }
    }

    // For now, use a simpler approach with raw SQL that handles the dynamic parts
    // We'll use the basic paginated query without dynamic tag filtering for complex cases
    let where_clause = conditions.join(" AND ");

    let query = format!(
        r#"SELECT ug.id, ug.chess_com_game_id, ug.opponent, ug.opponent_rating, ug.user_rating,
                  ug.result, ug.user_color, ug.time_control, ug.date, ug.tcn, ug.source,
                  COALESCE(
                      (SELECT json_agg(gt.tag) FROM game_tags gt WHERE gt.game_id = ug.id),
                      '[]'::json
                  ) as tags,
                  CASE WHEN ga.id IS NOT NULL THEN true ELSE false END as has_analysis,
                  ga.white_accuracy, ga.black_accuracy
           FROM user_games ug
           LEFT JOIN game_analysis ga ON ug.id = ga.game_id
           WHERE {}
           ORDER BY ug.date DESC
           LIMIT {} OFFSET {}"#,
        where_clause, limit, offset
    );

    let mut q = sqlx::query(&query).bind(user_id);
    for s in &params_str {
        q = q.bind(s.clone());
    }
    let rows = q.fetch_all(pool).await.map_err(AppError::Sqlx)?;

    let games: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            use sqlx::Row;
            let tags_json: JsonValue = row.try_get("tags").unwrap_or(JsonValue::Array(vec![]));
            let tags: Vec<String> = match tags_json {
                JsonValue::Array(arr) => arr
                    .into_iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect(),
                _ => vec![],
            };

            let has_analysis: bool = row.try_get("has_analysis").unwrap_or(false);

            // Decode TCN to SAN moves
            let tcn: Option<String> = row.try_get("tcn").unwrap_or(None);
            let moves: Vec<String> = tcn
                .as_deref()
                .and_then(|t| chess_core::tcn::decode_tcn_to_san(t).ok())
                .unwrap_or_default();

            let mut game = serde_json::json!({
                "id": row.try_get::<i64, _>("id").unwrap_or(0),
                "chessComGameId": row.try_get::<String, _>("chess_com_game_id").unwrap_or_default(),
                "opponent": row.try_get::<String, _>("opponent").unwrap_or_default(),
                "opponentRating": row.try_get::<Option<i32>, _>("opponent_rating").unwrap_or(None),
                "userRating": row.try_get::<Option<i32>, _>("user_rating").unwrap_or(None),
                "result": row.try_get::<String, _>("result").unwrap_or_default(),
                "userColor": row.try_get::<String, _>("user_color").unwrap_or_default(),
                "timeControl": row.try_get::<Option<String>, _>("time_control").unwrap_or(None),
                "date": row.try_get::<Option<String>, _>("date").unwrap_or(None),
                "moves": moves,
                "tags": tags,
                "source": row.try_get::<String, _>("source").unwrap_or_default(),
                "hasAnalysis": has_analysis,
            });

            if has_analysis {
                game["whiteAccuracy"] = serde_json::json!(row.try_get::<Option<f64>, _>("white_accuracy").unwrap_or(None));
                game["blackAccuracy"] = serde_json::json!(row.try_get::<Option<f64>, _>("black_accuracy").unwrap_or(None));
            }

            game
        })
        .collect();

    Ok(games)
}

pub async fn get_user_games_count_filtered(
    pool: &PgPool,
    user_id: i64,
    tag_filters: Option<&[String]>,
    source: Option<&str>,
    analyzed: Option<bool>,
) -> Result<i64, AppError> {
    // Simple case — no tag filters and no analyzed filter
    if analyzed.is_none()
        && (tag_filters.is_none() || tag_filters.map(|t| t.is_empty()).unwrap_or(true))
    {
        return get_user_games_count(pool, user_id, source).await;
    }

    let filters = tag_filters.unwrap_or(&[]);

    let mut conditions = vec!["ug.user_id = $1".to_string()];

    if let Some(src) = source {
        conditions.push(format!("ug.source = '{}'", src.replace('\'', "''")));
    }

    match analyzed {
        Some(true) => conditions.push("ga.id IS NOT NULL".to_string()),
        Some(false) => conditions.push("ga.id IS NULL".to_string()),
        None => {}
    }

    for f in filters {
        if let Some((_, code)) = RESULT_TAGS.iter().find(|(tag, _)| *tag == f.as_str()) {
            conditions.push(format!("ug.result = '{}'", code));
        } else if let Some((_, code)) = PLATFORM_TAGS.iter().find(|(tag, _)| *tag == f.as_str()) {
            conditions.push(format!("ug.source = '{}'", code));
        }
    }

    let regular_tags: Vec<&String> = filters
        .iter()
        .filter(|t| {
            !RESULT_TAGS.iter().any(|(tag, _)| tag == &t.as_str())
                && !PLATFORM_TAGS.iter().any(|(tag, _)| tag == &t.as_str())
        })
        .collect();

    if !regular_tags.is_empty() {
        let quoted: Vec<String> = regular_tags
            .iter()
            .map(|t| format!("'{}'", t.replace('\'', "''")))
            .collect();
        conditions.push(format!(
            "ug.id IN (SELECT game_id FROM game_tags WHERE tag IN ({}) GROUP BY game_id HAVING COUNT(DISTINCT tag) = {})",
            quoted.join(","),
            regular_tags.len()
        ));
    }

    let join = if analyzed.is_some() {
        "LEFT JOIN game_analysis ga ON ug.id = ga.game_id"
    } else {
        ""
    };

    let query = format!(
        "SELECT COUNT(*) as count FROM user_games ug {} WHERE {}",
        join,
        conditions.join(" AND ")
    );

    let row: (i64,) = sqlx::query_as(&query)
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(AppError::Sqlx)?;

    Ok(row.0)
}

/// Get tag counts for a user's games, including virtual tags.
pub async fn get_user_tag_counts(
    pool: &PgPool,
    user_id: i64,
) -> Result<serde_json::Map<String, JsonValue>, AppError> {
    let mut tag_counts = serde_json::Map::new();

    // Platform counts
    let platform_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT source, COUNT(*) FROM user_games WHERE user_id = $1 GROUP BY source",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Sqlx)?;

    for (source, count) in platform_rows {
        if let Some(tag) = source_to_tag(&source) {
            tag_counts.insert(tag.to_string(), JsonValue::Number(count.into()));
        }
    }

    // Result counts
    let result_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT result, COUNT(*) FROM user_games WHERE user_id = $1 GROUP BY result",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Sqlx)?;

    for (result, count) in result_rows {
        if let Some(tag) = result_to_tag(&result) {
            tag_counts.insert(tag.to_string(), JsonValue::Number(count.into()));
        }
    }

    // Regular tag counts
    let tag_rows: Vec<(String, i64)> = sqlx::query_as(
        r#"SELECT gt.tag, COUNT(*)
           FROM game_tags gt
           JOIN user_games ug ON gt.game_id = ug.id
           WHERE ug.user_id = $1
           GROUP BY gt.tag"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Sqlx)?;

    for (tag, count) in tag_rows {
        tag_counts.insert(tag, JsonValue::Number(count.into()));
    }

    Ok(tag_counts)
}

/// Get tag counts filtered by selected tags.
pub async fn get_user_tag_counts_filtered(
    pool: &PgPool,
    user_id: i64,
    selected_tags: &[String],
) -> Result<serde_json::Map<String, JsonValue>, AppError> {
    // Build WHERE conditions
    let mut conditions = vec!["ug.user_id = $1".to_string()];

    for tag in selected_tags {
        if let Some((_, code)) = RESULT_TAGS.iter().find(|(t, _)| *t == tag.as_str()) {
            conditions.push(format!("ug.result = '{}'", code));
        } else if let Some((_, code)) = PLATFORM_TAGS.iter().find(|(t, _)| *t == tag.as_str()) {
            conditions.push(format!("ug.source = '{}'", code));
        }
    }

    let regular_tags: Vec<&String> = selected_tags
        .iter()
        .filter(|t| {
            !RESULT_TAGS.iter().any(|(tag, _)| tag == &t.as_str())
                && !PLATFORM_TAGS.iter().any(|(tag, _)| tag == &t.as_str())
        })
        .collect();

    if !regular_tags.is_empty() {
        let quoted: Vec<String> = regular_tags
            .iter()
            .map(|t| format!("'{}'", t.replace('\'', "''")))
            .collect();
        conditions.push(format!(
            "ug.id IN (SELECT game_id FROM game_tags WHERE tag IN ({}) GROUP BY game_id HAVING COUNT(DISTINCT tag) = {})",
            quoted.join(","),
            regular_tags.len()
        ));
    }

    let where_clause = conditions.join(" AND ");
    let mut tag_counts = serde_json::Map::new();

    // Platform counts
    let query = format!(
        "SELECT ug.source, COUNT(*) as count FROM user_games ug WHERE {} GROUP BY ug.source",
        where_clause
    );
    let rows: Vec<(String, i64)> = sqlx::query_as(&query)
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(AppError::Sqlx)?;

    for (source, count) in rows {
        if let Some(tag) = source_to_tag(&source) {
            tag_counts.insert(tag.to_string(), JsonValue::Number(count.into()));
        }
    }

    // Result counts
    let query = format!(
        "SELECT ug.result, COUNT(*) as count FROM user_games ug WHERE {} GROUP BY ug.result",
        where_clause
    );
    let rows: Vec<(String, i64)> = sqlx::query_as(&query)
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(AppError::Sqlx)?;

    for (result, count) in rows {
        if let Some(tag) = result_to_tag(&result) {
            tag_counts.insert(tag.to_string(), JsonValue::Number(count.into()));
        }
    }

    // Regular tag counts
    let query = format!(
        r#"SELECT gt.tag, COUNT(*) as count
           FROM game_tags gt
           JOIN user_games ug ON gt.game_id = ug.id
           WHERE {}
           GROUP BY gt.tag"#,
        where_clause
    );
    let rows: Vec<(String, i64)> = sqlx::query_as(&query)
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(AppError::Sqlx)?;

    for (tag, count) in rows {
        tag_counts.insert(tag, JsonValue::Number(count.into()));
    }

    Ok(tag_counts)
}

pub async fn get_game_by_id(
    pool: &PgPool,
    user_id: i64,
    game_id: i64,
) -> Result<Option<serde_json::Value>, AppError> {
    use sqlx::Row;

    let row = sqlx::query(
        r#"SELECT ug.id, ug.chess_com_game_id, ug.opponent, ug.opponent_rating, ug.user_rating,
                  ug.result, ug.user_color, ug.time_control, ug.date, ug.tcn, ug.source,
                  COALESCE(
                      (SELECT json_agg(gt.tag) FROM game_tags gt WHERE gt.game_id = ug.id),
                      '[]'::json
                  ) as tags
           FROM user_games ug
           WHERE ug.user_id = $1 AND ug.id = $2"#,
    )
    .bind(user_id)
    .bind(game_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Sqlx)?;

    Ok(row.map(|r| {
        let tags_json: JsonValue = r.try_get("tags").unwrap_or(JsonValue::Array(vec![]));
        let tags: Vec<String> = match tags_json {
            JsonValue::Array(arr) => arr
                .into_iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
            _ => vec![],
        };

        // Decode TCN to SAN moves
        let tcn: Option<String> = r.try_get("tcn").unwrap_or(None);
        let moves: Vec<String> = tcn
            .as_deref()
            .and_then(|t| chess_core::tcn::decode_tcn_to_san(t).ok())
            .unwrap_or_default();

        serde_json::json!({
            "id": r.try_get::<i64, _>("id").unwrap_or(0),
            "chessComGameId": r.try_get::<String, _>("chess_com_game_id").unwrap_or_default(),
            "opponent": r.try_get::<String, _>("opponent").unwrap_or_default(),
            "opponentRating": r.try_get::<Option<i32>, _>("opponent_rating").unwrap_or(None),
            "userRating": r.try_get::<Option<i32>, _>("user_rating").unwrap_or(None),
            "result": r.try_get::<String, _>("result").unwrap_or_default(),
            "userColor": r.try_get::<String, _>("user_color").unwrap_or_default(),
            "timeControl": r.try_get::<Option<String>, _>("time_control").unwrap_or(None),
            "date": r.try_get::<Option<String>, _>("date").unwrap_or(None),
            "moves": moves,
            "tags": tags,
            "source": r.try_get::<String, _>("source").unwrap_or_default(),
        })
    }))
}

/// Get games by color for opening tree building.
pub async fn get_user_games_by_color(
    pool: &PgPool,
    user_id: i64,
    color: &str,
) -> Result<Vec<serde_json::Value>, AppError> {
    use sqlx::Row;

    let rows = sqlx::query(
        r#"SELECT chess_com_game_id, result, tcn
           FROM user_games
           WHERE user_id = $1 AND LOWER(user_color) = LOWER($2)
           ORDER BY date DESC"#,
    )
    .bind(user_id)
    .bind(color)
    .fetch_all(pool)
    .await
    .map_err(AppError::Sqlx)?;

    Ok(rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.try_get::<String, _>("chess_com_game_id").unwrap_or_default(),
                "result": r.try_get::<String, _>("result").unwrap_or_default(),
                "tcn": r.try_get::<Option<String>, _>("tcn").unwrap_or(None),
            })
        })
        .collect())
}

/// Get internal DB IDs + opponent for a list of source game IDs.
pub async fn get_game_ids_and_opponents(
    pool: &PgPool,
    user_id: i64,
    source: &str,
    source_game_ids: &[String],
) -> Result<Vec<(i64, String, String)>, AppError> {
    if source_game_ids.is_empty() {
        return Ok(vec![]);
    }

    use sqlx::Row;
    let placeholders: Vec<String> = source_game_ids
        .iter()
        .enumerate()
        .map(|(i, _)| format!("${}", i + 3))
        .collect();
    let query = format!(
        "SELECT id, chess_com_game_id, opponent FROM user_games WHERE user_id = $1 AND source = $2 AND chess_com_game_id IN ({})",
        placeholders.join(",")
    );

    let mut q = sqlx::query(&query).bind(user_id).bind(source);
    for id in source_game_ids {
        q = q.bind(id);
    }

    let rows = q.fetch_all(pool).await.map_err(AppError::Sqlx)?;
    Ok(rows
        .into_iter()
        .map(|r| {
            (
                r.try_get::<i64, _>("id").unwrap_or(0),
                r.try_get::<String, _>("chess_com_game_id").unwrap_or_default(),
                r.try_get::<String, _>("opponent").unwrap_or_default(),
            )
        })
        .collect())
}

/// Get games count by Chess.com username (for profile).
pub async fn get_games_count_by_chess_com_username(
    pool: &PgPool,
    username: &str,
) -> Result<i64, AppError> {
    let user_id: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM platform_users WHERE LOWER(chess_com_username) = LOWER($1)",
    )
    .bind(username)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Sqlx)?;

    match user_id {
        Some((uid,)) => get_user_games_count(pool, uid, None).await,
        None => Ok(0),
    }
}

/// Verify that a list of game IDs belong to a user.
/// Returns only the IDs that belong to the user.
pub async fn verify_game_ownership(
    pool: &PgPool,
    user_id: i64,
    game_ids: &[i64],
) -> Result<Vec<i64>, AppError> {
    if game_ids.is_empty() {
        return Ok(vec![]);
    }

    let placeholders: Vec<String> = game_ids
        .iter()
        .enumerate()
        .map(|(i, _)| format!("${}", i + 2))
        .collect();

    let query = format!(
        "SELECT id FROM user_games WHERE user_id = $1 AND id IN ({})",
        placeholders.join(",")
    );

    let mut q = sqlx::query_as::<_, (i64,)>(&query).bind(user_id);
    for id in game_ids {
        q = q.bind(*id);
    }

    let rows = q.fetch_all(pool).await.map_err(AppError::Sqlx)?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

/// Get IDs of games that haven't been analyzed yet.
pub async fn get_unanalyzed_game_ids(
    pool: &PgPool,
    user_id: i64,
    tag_filters: Option<&[String]>,
    source: Option<&str>,
) -> Result<Vec<i64>, AppError> {
    let mut conditions = vec!["ug.user_id = $1".to_string(), "ga.id IS NULL".to_string()];

    if let Some(src) = source {
        conditions.push(format!("ug.source = '{}'", src.replace('\'', "''")));
    }

    if let Some(filters) = tag_filters {
        for f in filters {
            if let Some((_, code)) = RESULT_TAGS.iter().find(|(tag, _)| *tag == f.as_str()) {
                conditions.push(format!("ug.result = '{}'", code));
            } else if let Some((_, code)) = PLATFORM_TAGS.iter().find(|(tag, _)| *tag == f.as_str()) {
                conditions.push(format!("ug.source = '{}'", code));
            }
        }

        let regular_tags: Vec<&String> = filters
            .iter()
            .filter(|t| {
                !RESULT_TAGS.iter().any(|(tag, _)| tag == &t.as_str())
                    && !PLATFORM_TAGS.iter().any(|(tag, _)| tag == &t.as_str())
            })
            .collect();

        if !regular_tags.is_empty() {
            let quoted: Vec<String> = regular_tags
                .iter()
                .map(|t| format!("'{}'", t.replace('\'', "''")))
                .collect();
            conditions.push(format!(
                "ug.id IN (SELECT game_id FROM game_tags WHERE tag IN ({}) GROUP BY game_id HAVING COUNT(DISTINCT tag) = {})",
                quoted.join(","),
                regular_tags.len()
            ));
        }
    }

    let query = format!(
        r#"SELECT ug.id
           FROM user_games ug
           LEFT JOIN game_analysis ga ON ug.id = ga.game_id
           WHERE {}
           ORDER BY ug.date DESC"#,
        conditions.join(" AND ")
    );

    let rows: Vec<(i64,)> = sqlx::query_as(&query)
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(AppError::Sqlx)?;

    Ok(rows.into_iter().map(|(id,)| id).collect())
}

/// Get games by Chess.com username (for user profile /users/me/games).
pub async fn get_games_by_chess_com_username(
    pool: &PgPool,
    username: &str,
    limit: i64,
) -> Result<Vec<serde_json::Value>, AppError> {
    use sqlx::Row;

    let user_id: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM platform_users WHERE LOWER(chess_com_username) = LOWER($1)",
    )
    .bind(username)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Sqlx)?;

    let uid = match user_id {
        Some((id,)) => id,
        None => return Ok(vec![]),
    };

    let rows = sqlx::query(
        r#"SELECT id, chess_com_game_id, opponent, opponent_rating, user_rating,
                  result, user_color, time_control, date, tcn, tags
           FROM user_games
           WHERE user_id = $1
           ORDER BY date DESC
           LIMIT $2"#,
    )
    .bind(uid)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(AppError::Sqlx)?;

    Ok(rows
        .into_iter()
        .map(|r| {
            let tags: JsonValue = r.try_get("tags").unwrap_or(JsonValue::Array(vec![]));
            serde_json::json!({
                "id": r.try_get::<i64, _>("id").unwrap_or(0),
                "chessComGameId": r.try_get::<String, _>("chess_com_game_id").unwrap_or_default(),
                "opponent": r.try_get::<String, _>("opponent").unwrap_or_default(),
                "opponentRating": r.try_get::<Option<i32>, _>("opponent_rating").unwrap_or(None),
                "userRating": r.try_get::<Option<i32>, _>("user_rating").unwrap_or(None),
                "result": r.try_get::<String, _>("result").unwrap_or_default(),
                "userColor": r.try_get::<String, _>("user_color").unwrap_or_default(),
                "timeControl": r.try_get::<Option<String>, _>("time_control").unwrap_or(None),
                "date": r.try_get::<Option<String>, _>("date").unwrap_or(None),
                "moves": [],
                "tags": tags,
            })
        })
        .collect())
}
