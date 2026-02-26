use sqlx::PgPool;

use crate::error::AppError;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct Account {
    pub id: i64,
    pub username: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub display_name: Option<String>,
    pub chess_com_username: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn create_account(
    pool: &PgPool,
    username: &str,
    email: &str,
    password_hash: &str,
    chess_com_username: &str,
) -> Result<i64, AppError> {
    let row: (i64,) = sqlx::query_as(
        r#"INSERT INTO accounts (username, email, password_hash, chess_com_username, display_name)
           VALUES ($1, $2, $3, $4, $1)
           RETURNING id"#,
    )
    .bind(username)
    .bind(email)
    .bind(password_hash)
    .bind(chess_com_username)
    .fetch_one(pool)
    .await
    .map_err(AppError::Sqlx)?;

    Ok(row.0)
}

pub async fn get_account_by_id(pool: &PgPool, id: i64) -> Result<Option<Account>, AppError> {
    sqlx::query_as::<_, Account>(
        "SELECT id, username, email, password_hash, display_name, chess_com_username, bio, avatar_url, created_at FROM accounts WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Sqlx)
}

pub async fn get_account_by_username(pool: &PgPool, username: &str) -> Result<Option<Account>, AppError> {
    sqlx::query_as::<_, Account>(
        "SELECT id, username, email, password_hash, display_name, chess_com_username, bio, avatar_url, created_at FROM accounts WHERE LOWER(username) = LOWER($1)",
    )
    .bind(username)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Sqlx)
}

pub async fn get_account_by_email(pool: &PgPool, email: &str) -> Result<Option<Account>, AppError> {
    sqlx::query_as::<_, Account>(
        "SELECT id, username, email, password_hash, display_name, chess_com_username, bio, avatar_url, created_at FROM accounts WHERE LOWER(email) = LOWER($1)",
    )
    .bind(email)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Sqlx)
}

pub async fn email_exists(pool: &PgPool, email: &str) -> Result<bool, AppError> {
    let row: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM accounts WHERE LOWER(email) = LOWER($1))",
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .map_err(AppError::Sqlx)?;

    Ok(row.0)
}

pub async fn username_exists(pool: &PgPool, username: &str) -> Result<bool, AppError> {
    let row: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM accounts WHERE LOWER(username) = LOWER($1))",
    )
    .bind(username)
    .fetch_one(pool)
    .await
    .map_err(AppError::Sqlx)?;

    Ok(row.0)
}

pub async fn update_account(
    pool: &PgPool,
    account_id: i64,
    display_name: Option<&str>,
    bio: Option<&str>,
    chess_com_username: Option<&str>,
) -> Result<Account, AppError> {
    sqlx::query_as::<_, Account>(
        r#"UPDATE accounts SET
            display_name = COALESCE($2, display_name),
            bio = COALESCE($3, bio),
            chess_com_username = COALESCE($4, chess_com_username)
        WHERE id = $1
        RETURNING id, username, email, password_hash, display_name, chess_com_username, bio, avatar_url, created_at"#,
    )
    .bind(account_id)
    .bind(display_name)
    .bind(bio)
    .bind(chess_com_username)
    .fetch_one(pool)
    .await
    .map_err(AppError::Sqlx)
}

pub async fn update_password_hash(
    pool: &PgPool,
    account_id: i64,
    new_hash: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE accounts SET password_hash = $2 WHERE id = $1",
    )
    .bind(account_id)
    .bind(new_hash)
    .execute(pool)
    .await
    .map_err(AppError::Sqlx)?;
    Ok(())
}

pub async fn delete_account(pool: &PgPool, account_id: i64) -> Result<(), AppError> {
    let mut tx = pool.begin().await.map_err(AppError::Sqlx)?;

    sqlx::query("DELETE FROM user_opening_moves WHERE user_id = $1")
        .bind(account_id)
        .execute(&mut *tx)
        .await
        .map_err(AppError::Sqlx)?;

    sqlx::query("DELETE FROM trainer_progress WHERE user_id = $1")
        .bind(account_id)
        .execute(&mut *tx)
        .await
        .map_err(AppError::Sqlx)?;

    // user_games -> game_tags, game_analysis cascade, but user_games itself doesn't cascade from accounts
    sqlx::query("DELETE FROM user_games WHERE user_id = $1")
        .bind(account_id)
        .execute(&mut *tx)
        .await
        .map_err(AppError::Sqlx)?;

    sqlx::query("DELETE FROM accounts WHERE id = $1")
        .bind(account_id)
        .execute(&mut *tx)
        .await
        .map_err(AppError::Sqlx)?;

    tx.commit().await.map_err(AppError::Sqlx)?;
    Ok(())
}

pub async fn get_public_profile(
    pool: &PgPool,
    username: &str,
) -> Result<Option<Account>, AppError> {
    sqlx::query_as::<_, Account>(
        "SELECT id, username, email, password_hash, display_name, chess_com_username, bio, avatar_url, created_at FROM accounts WHERE LOWER(username) = LOWER($1)",
    )
    .bind(username)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Sqlx)
}
