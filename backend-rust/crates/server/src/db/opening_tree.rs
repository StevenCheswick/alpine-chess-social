use serde_json::Value as JsonValue;
use sqlx::PgPool;

use crate::error::AppError;

pub async fn get_cached_opening_tree(
    pool: &PgPool,
    user_id: i64,
    color: &str,
) -> Result<Option<JsonValue>, AppError> {
    let row: Option<(JsonValue, Option<i32>, Option<chrono::DateTime<chrono::Utc>>)> = sqlx::query_as(
        r#"SELECT tree_json, total_games, updated_at
           FROM user_opening_trees
           WHERE user_id = $1 AND color = $2"#,
    )
    .bind(user_id)
    .bind(color)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Sqlx)?;

    Ok(row.map(|(tree_json, total_games, updated_at)| {
        serde_json::json!({
            "tree": tree_json,
            "totalGames": total_games,
            "updatedAt": updated_at.map(|t| t.to_rfc3339()),
        })
    }))
}

pub async fn save_opening_tree(
    pool: &PgPool,
    user_id: i64,
    color: &str,
    tree: &JsonValue,
    total_games: i32,
) -> Result<(), AppError> {
    sqlx::query(
        r#"INSERT INTO user_opening_trees (user_id, color, tree_json, total_games, updated_at)
           VALUES ($1, $2, $3, $4, NOW())
           ON CONFLICT (user_id, color) DO UPDATE SET
               tree_json = EXCLUDED.tree_json,
               total_games = EXCLUDED.total_games,
               updated_at = NOW()"#,
    )
    .bind(user_id)
    .bind(color)
    .bind(tree)
    .bind(total_games)
    .execute(pool)
    .await
    .map_err(AppError::Sqlx)?;

    Ok(())
}

pub async fn invalidate_opening_trees(pool: &PgPool, user_id: i64) -> Result<(), AppError> {
    sqlx::query("DELETE FROM user_opening_trees WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Sqlx)?;
    Ok(())
}
