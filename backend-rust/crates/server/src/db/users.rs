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

fn sync_column(platform: &str) -> &'static str {
    match platform {
        "lichess" => "lichess_last_synced_at",
        _ => "chess_com_last_synced_at",
    }
}
