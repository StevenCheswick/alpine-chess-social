use sqlx::PgPool;

use crate::error::AppError;

/// Get the last synced timestamp for a specific platform.
pub async fn get_last_synced(
    pool: &PgPool,
    account_id: i64,
    platform: &str,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, AppError> {
    let col = sync_column(platform);
    let query = format!("SELECT {} FROM accounts WHERE id = $1", col);
    let row: Option<(Option<chrono::DateTime<chrono::Utc>>,)> =
        sqlx::query_as(&query).bind(account_id).fetch_optional(pool).await.map_err(AppError::Sqlx)?;
    Ok(row.and_then(|r| r.0))
}

/// Update the last synced timestamp for a specific platform.
pub async fn update_last_synced(
    pool: &PgPool,
    account_id: i64,
    platform: &str,
) -> Result<(), AppError> {
    let col = sync_column(platform);
    let query = format!("UPDATE accounts SET {} = NOW() WHERE id = $1", col);
    sqlx::query(&query).bind(account_id).execute(pool).await.map_err(AppError::Sqlx)?;
    Ok(())
}

fn sync_column(_platform: &str) -> &'static str {
    // Only Chess.com is supported
    "chess_com_last_synced_at"
}

/// Get the oldest synced month cursor for Chess.com backfill.
/// Returns None (never synced), Some("YYYY-MM"), or Some("complete").
pub async fn get_oldest_synced_month(
    pool: &PgPool,
    account_id: i64,
) -> Result<Option<String>, AppError> {
    let row: Option<(Option<String>,)> = sqlx::query_as(
        "SELECT chess_com_oldest_synced_month FROM accounts WHERE id = $1",
    )
    .bind(account_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Sqlx)?;
    Ok(row.and_then(|r| r.0))
}

/// Update the oldest synced month cursor for Chess.com backfill.
pub async fn update_oldest_synced_month(
    pool: &PgPool,
    account_id: i64,
    value: &str,
) -> Result<(), AppError> {
    sqlx::query("UPDATE accounts SET chess_com_oldest_synced_month = $1 WHERE id = $2")
        .bind(value)
        .bind(account_id)
        .execute(pool)
        .await
        .map_err(AppError::Sqlx)?;
    Ok(())
}
