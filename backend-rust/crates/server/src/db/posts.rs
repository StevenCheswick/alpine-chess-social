use serde_json::Value as JsonValue;
use sqlx::PgPool;

use crate::error::AppError;

pub async fn create_post(
    pool: &PgPool,
    account_id: i64,
    post_type: &str,
    content: &str,
    game_id: Option<i64>,
    key_position_index: i32,
) -> Result<i64, AppError> {
    let row: (i64,) = sqlx::query_as(
        r#"INSERT INTO posts (account_id, post_type, content, game_id, key_position_index)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING id"#,
    )
    .bind(account_id)
    .bind(post_type)
    .bind(content)
    .bind(game_id)
    .bind(key_position_index)
    .fetch_one(pool)
    .await
    .map_err(AppError::Sqlx)?;

    Ok(row.0)
}

fn post_row_to_json(r: &sqlx::postgres::PgRow) -> JsonValue {
    use sqlx::Row;

    let game_id: Option<i64> = r.try_get("game_id").unwrap_or(None);
    let chess_com_game_id: Option<String> = r.try_get("chess_com_game_id").unwrap_or(None);

    let game_data = if game_id.is_some() && chess_com_game_id.is_some() {
        let game_tags: Option<JsonValue> = r.try_get("game_tags").unwrap_or(None);
        Some(serde_json::json!({
            "id": chess_com_game_id,
            "opponent": r.try_get::<Option<String>, _>("opponent").unwrap_or(None),
            "opponentRating": r.try_get::<Option<i32>, _>("opponent_rating").unwrap_or(None),
            "userRating": r.try_get::<Option<i32>, _>("user_rating").unwrap_or(None),
            "result": r.try_get::<Option<String>, _>("game_result").unwrap_or(None),
            "userColor": r.try_get::<Option<String>, _>("user_color").unwrap_or(None),
            "timeControl": r.try_get::<Option<String>, _>("time_control").unwrap_or(None),
            "date": r.try_get::<Option<String>, _>("game_date").unwrap_or(None),
            "moves": [],
            "tags": game_tags.unwrap_or(JsonValue::Array(vec![])),
            "keyPositionIndex": r.try_get::<Option<i32>, _>("key_position_index").unwrap_or(Some(0)),
        }))
    } else {
        None
    };

    let author_display_name: Option<String> = r.try_get("author_display_name").unwrap_or(None);
    let author_username: String = r.try_get("author_username").unwrap_or_default();
    let created_at: chrono::DateTime<chrono::Utc> = r.try_get("created_at").unwrap_or_default();

    serde_json::json!({
        "id": r.try_get::<i64, _>("id").unwrap_or(0),
        "postType": r.try_get::<String, _>("post_type").unwrap_or_default(),
        "content": r.try_get::<String, _>("content").unwrap_or_default(),
        "createdAt": created_at.to_rfc3339(),
        "author": {
            "id": r.try_get::<i64, _>("author_id").unwrap_or(0),
            "username": &author_username,
            "displayName": author_display_name.as_deref().unwrap_or(&author_username),
            "avatarUrl": r.try_get::<Option<String>, _>("author_avatar_url").unwrap_or(None),
        },
        "gameData": game_data,
    })
}

const POST_QUERY: &str = r#"SELECT
    p.id,
    p.post_type,
    p.content,
    p.game_id,
    p.key_position_index,
    p.created_at,
    a.id as author_id,
    a.username as author_username,
    a.display_name as author_display_name,
    a.avatar_url as author_avatar_url,
    g.chess_com_game_id,
    g.opponent,
    g.opponent_rating,
    g.user_rating,
    g.result as game_result,
    g.user_color,
    g.time_control,
    g.date as game_date,
    g.tcn,
    g.tags as game_tags
FROM posts p
JOIN accounts a ON p.account_id = a.id
LEFT JOIN user_games g ON p.game_id = g.id"#;

pub async fn get_post_by_id(pool: &PgPool, post_id: i64) -> Result<Option<JsonValue>, AppError> {
    let query = format!("{} WHERE p.id = $1", POST_QUERY);
    let row = sqlx::query(&query)
        .bind(post_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Sqlx)?;

    Ok(row.as_ref().map(post_row_to_json))
}

pub async fn get_posts(
    pool: &PgPool,
    limit: i64,
    offset: i64,
) -> Result<Vec<JsonValue>, AppError> {
    let query = format!(
        "{} ORDER BY p.created_at DESC LIMIT $1 OFFSET $2",
        POST_QUERY
    );
    let rows = sqlx::query(&query)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(AppError::Sqlx)?;

    Ok(rows.iter().map(post_row_to_json).collect())
}

pub async fn get_posts_count(pool: &PgPool) -> Result<i64, AppError> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts")
        .fetch_one(pool)
        .await
        .map_err(AppError::Sqlx)?;
    Ok(count.0)
}

pub async fn get_posts_by_username(
    pool: &PgPool,
    username: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<JsonValue>, AppError> {
    let query = format!(
        "{} WHERE LOWER(a.username) = LOWER($1) ORDER BY p.created_at DESC LIMIT $2 OFFSET $3",
        POST_QUERY
    );
    let rows = sqlx::query(&query)
        .bind(username)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(AppError::Sqlx)?;

    Ok(rows.iter().map(post_row_to_json).collect())
}

pub async fn get_posts_count_by_username(pool: &PgPool, username: &str) -> Result<i64, AppError> {
    let count: (i64,) = sqlx::query_as(
        r#"SELECT COUNT(*) FROM posts p
           JOIN accounts a ON p.account_id = a.id
           WHERE LOWER(a.username) = LOWER($1)"#,
    )
    .bind(username)
    .fetch_one(pool)
    .await
    .map_err(AppError::Sqlx)?;
    Ok(count.0)
}
