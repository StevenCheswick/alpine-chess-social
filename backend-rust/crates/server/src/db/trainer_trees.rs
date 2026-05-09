use serde_json::Value as JsonValue;
use sqlx::PgPool;

pub struct TreeSummary {
    pub id: String,
    pub name: String,
    pub color: String,
    pub start_moves: String,
    pub start_fen: String,
    pub nodes_count: i32,
    pub lines_count: i32,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub async fn list_trees(pool: &PgPool) -> Result<Vec<TreeSummary>, sqlx::Error> {
    let rows = sqlx::query_as::<
        _,
        (
            String,
            String,
            String,
            String,
            String,
            i32,
            i32,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        r#"
        SELECT id, name, color, start_moves, start_fen, nodes_count, lines_count, updated_at
        FROM trainer_trees
        ORDER BY name
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(id, name, color, start_moves, start_fen, nodes_count, lines_count, updated_at)| {
                TreeSummary {
                    id,
                    name,
                    color,
                    start_moves,
                    start_fen,
                    nodes_count,
                    lines_count,
                    updated_at,
                }
            },
        )
        .collect())
}

pub async fn get_tree(pool: &PgPool, id: &str) -> Result<Option<JsonValue>, sqlx::Error> {
    let row = sqlx::query_scalar::<_, JsonValue>(
        r#"
        SELECT json_build_object(
            'id', id,
            'name', name,
            'color', color,
            'start_moves', start_moves,
            'start_fen', start_fen,
            'nodes_count', nodes_count,
            'lines_count', lines_count,
            'tree', tree,
            'updated_at', updated_at
        )
        FROM trainer_trees
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn upsert_tree(
    pool: &PgPool,
    id: &str,
    name: &str,
    color: &str,
    start_moves: &str,
    start_fen: &str,
    nodes_count: i32,
    lines_count: i32,
    tree: &JsonValue,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO trainer_trees (id, name, color, start_moves, start_fen, nodes_count, lines_count, tree, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
        ON CONFLICT (id) DO UPDATE SET
            name = EXCLUDED.name,
            color = EXCLUDED.color,
            start_moves = EXCLUDED.start_moves,
            start_fen = EXCLUDED.start_fen,
            nodes_count = EXCLUDED.nodes_count,
            lines_count = EXCLUDED.lines_count,
            tree = EXCLUDED.tree,
            updated_at = NOW()
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(color)
    .bind(start_moves)
    .bind(start_fen)
    .bind(nodes_count)
    .bind(lines_count)
    .bind(tree)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_tree(pool: &PgPool, id: &str) -> Result<u64, sqlx::Error> {
    let res = sqlx::query("DELETE FROM trainer_trees WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

pub async fn get_user_progress(
    pool: &PgPool,
    user_id: i64,
    tree_id: &str,
) -> Result<Vec<(String, String)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT fen, move_san FROM trainer_tree_progress WHERE user_id = $1 AND tree_id = $2",
    )
    .bind(user_id)
    .bind(tree_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn mark_learned(
    pool: &PgPool,
    user_id: i64,
    tree_id: &str,
    moves: &[(String, String)],
) -> Result<usize, sqlx::Error> {
    if moves.is_empty() {
        return Ok(0);
    }
    let mut tx = pool.begin().await?;
    let mut inserted = 0;
    for (fen, san) in moves {
        let res = sqlx::query(
            "INSERT INTO trainer_tree_progress (user_id, tree_id, fen, move_san)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT DO NOTHING",
        )
        .bind(user_id)
        .bind(tree_id)
        .bind(fen)
        .bind(san)
        .execute(&mut *tx)
        .await?;
        inserted += res.rows_affected() as usize;
    }
    tx.commit().await?;
    Ok(inserted)
}
