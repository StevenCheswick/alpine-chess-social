use sqlx::PgPool;

pub struct MaiaPosition {
    pub id: String,
    pub title: String,
    pub fen: String,
    pub user_side: String,
    pub notes: Option<String>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub async fn list_positions(pool: &PgPool) -> Result<Vec<MaiaPosition>, sqlx::Error> {
    let rows = sqlx::query_as::<
        _,
        (
            String,
            String,
            String,
            String,
            Option<String>,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        r#"
        SELECT id, title, fen, user_side, notes, updated_at
        FROM trainer_maia_positions
        ORDER BY title
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, title, fen, user_side, notes, updated_at)| MaiaPosition {
            id,
            title,
            fen,
            user_side,
            notes,
            updated_at,
        })
        .collect())
}

pub async fn get_position(
    pool: &PgPool,
    id: &str,
) -> Result<Option<MaiaPosition>, sqlx::Error> {
    let row = sqlx::query_as::<
        _,
        (
            String,
            String,
            String,
            String,
            Option<String>,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        r#"
        SELECT id, title, fen, user_side, notes, updated_at
        FROM trainer_maia_positions
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(id, title, fen, user_side, notes, updated_at)| MaiaPosition {
        id,
        title,
        fen,
        user_side,
        notes,
        updated_at,
    }))
}

pub async fn upsert_position(
    pool: &PgPool,
    id: &str,
    title: &str,
    fen: &str,
    user_side: &str,
    notes: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO trainer_maia_positions (id, title, fen, user_side, notes, updated_at)
        VALUES ($1, $2, $3, $4, $5, NOW())
        ON CONFLICT (id) DO UPDATE SET
            title = EXCLUDED.title,
            fen = EXCLUDED.fen,
            user_side = EXCLUDED.user_side,
            notes = EXCLUDED.notes,
            updated_at = NOW()
        "#,
    )
    .bind(id)
    .bind(title)
    .bind(fen)
    .bind(user_side)
    .bind(notes)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_position(pool: &PgPool, id: &str) -> Result<u64, sqlx::Error> {
    let res = sqlx::query("DELETE FROM trainer_maia_positions WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}
