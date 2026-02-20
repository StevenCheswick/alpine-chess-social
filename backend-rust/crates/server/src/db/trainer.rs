use serde_json::Value as JsonValue;
use sqlx::PgPool;

pub struct TrainerOpening {
    pub opening_name: String,
    pub eco_codes: Vec<String>,
    pub puzzle_count: i64,
    pub sample_fen: String,
}

pub async fn list_openings(pool: &PgPool) -> Result<Vec<TrainerOpening>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, Vec<String>, i64, String)>(
        r#"
        SELECT
            opening_name,
            array_agg(DISTINCT eco ORDER BY eco) AS eco_codes,
            COUNT(*) AS puzzle_count,
            MIN(pre_mistake_fen) AS sample_fen
        FROM trainer_puzzles
        GROUP BY opening_name
        ORDER BY opening_name
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(opening_name, eco_codes, puzzle_count, sample_fen)| TrainerOpening {
            opening_name,
            eco_codes,
            puzzle_count,
            sample_fen,
        })
        .collect())
}

pub async fn get_puzzles_by_opening(
    pool: &PgPool,
    opening_name: &str,
) -> Result<Vec<JsonValue>, sqlx::Error> {
    let rows = sqlx::query_scalar::<_, JsonValue>(
        r#"
        SELECT json_build_object(
            'id', id,
            'eco', eco,
            'opening_name', opening_name,
            'mistake_san', mistake_san,
            'mistake_uci', mistake_uci,
            'pre_mistake_fen', pre_mistake_fen,
            'solver_color', solver_color,
            'root_eval', root_eval,
            'cp_loss', cp_loss,
            'games', games,
            'tree', tree
        )
        FROM trainer_puzzles
        WHERE opening_name = $1
        ORDER BY games DESC
        "#,
    )
    .bind(opening_name)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn mark_puzzle_complete(
    pool: &PgPool,
    user_id: i64,
    puzzle_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO trainer_progress (user_id, puzzle_id)
        VALUES ($1, $2)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(user_id)
    .bind(puzzle_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_user_progress(
    pool: &PgPool,
    user_id: i64,
) -> Result<Vec<(String, i64)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, i64)>(
        r#"
        SELECT tp.opening_name, COUNT(*) AS completed_count
        FROM trainer_progress p
        JOIN trainer_puzzles tp ON tp.id = p.puzzle_id
        WHERE p.user_id = $1
        GROUP BY tp.opening_name
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_completed_puzzle_ids(
    pool: &PgPool,
    user_id: i64,
    opening_name: &str,
) -> Result<Vec<String>, sqlx::Error> {
    let rows = sqlx::query_scalar::<_, String>(
        r#"
        SELECT p.puzzle_id
        FROM trainer_progress p
        JOIN trainer_puzzles tp ON tp.id = p.puzzle_id
        WHERE p.user_id = $1 AND tp.opening_name = $2
        "#,
    )
    .bind(user_id)
    .bind(opening_name)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn upsert_puzzles(
    pool: &PgPool,
    opening_name: &str,
    puzzles: &[JsonValue],
) -> Result<usize, sqlx::Error> {
    let mut count = 0;

    for puzzle in puzzles {
        let id = puzzle["id"].as_str().unwrap_or_default();
        let eco = puzzle["eco"].as_str().unwrap_or_default();
        let mistake_san = puzzle["mistake_san"].as_str().unwrap_or_default();
        let mistake_uci = puzzle["mistake_uci"].as_str().unwrap_or_default();
        let pre_mistake_fen = puzzle["pre_mistake_fen"].as_str().unwrap_or_default();
        let solver_color = puzzle["solver_color"].as_str().unwrap_or("w");
        let root_eval = puzzle["root_eval"].as_i64().unwrap_or(0) as i32;
        let cp_loss = puzzle["cp_loss"].as_i64().unwrap_or(0) as i32;
        let games = puzzle["games"].as_i64().unwrap_or(0) as i32;
        let tree = &puzzle["tree"];

        sqlx::query(
            r#"
            INSERT INTO trainer_puzzles (id, eco, opening_name, mistake_san, mistake_uci, pre_mistake_fen, solver_color, root_eval, cp_loss, games, tree)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (id) DO UPDATE SET
                eco = EXCLUDED.eco,
                opening_name = EXCLUDED.opening_name,
                mistake_san = EXCLUDED.mistake_san,
                mistake_uci = EXCLUDED.mistake_uci,
                pre_mistake_fen = EXCLUDED.pre_mistake_fen,
                solver_color = EXCLUDED.solver_color,
                root_eval = EXCLUDED.root_eval,
                cp_loss = EXCLUDED.cp_loss,
                games = EXCLUDED.games,
                tree = EXCLUDED.tree
            "#,
        )
        .bind(id)
        .bind(eco)
        .bind(opening_name)
        .bind(mistake_san)
        .bind(mistake_uci)
        .bind(pre_mistake_fen)
        .bind(solver_color)
        .bind(root_eval)
        .bind(cp_loss)
        .bind(games)
        .bind(tree)
        .execute(pool)
        .await?;

        count += 1;
    }

    Ok(count)
}
