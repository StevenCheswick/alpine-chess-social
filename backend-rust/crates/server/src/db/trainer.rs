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

pub async fn delete_by_opening(
    pool: &PgPool,
    opening_name: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM trainer_puzzles WHERE opening_name = $1",
    )
    .bind(opening_name)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

// ── Hard Moves ──────────────────────────────────────────────────────

pub struct HardMoveOpening {
    pub opening_name: String,
    pub count: i64,
    pub sample_fen: String,
}

pub async fn list_hard_move_openings(pool: &PgPool) -> Result<Vec<HardMoveOpening>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, i64, String)>(
        r#"
        SELECT opening_name, COUNT(*) AS cnt, MIN(fen) AS sample_fen
        FROM trainer_hard_moves
        GROUP BY opening_name
        ORDER BY opening_name
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(opening_name, count, sample_fen)| HardMoveOpening {
            opening_name,
            count,
            sample_fen,
        })
        .collect())
}

pub async fn get_hard_moves_by_opening(
    pool: &PgPool,
    opening_name: &str,
) -> Result<Vec<JsonValue>, sqlx::Error> {
    let rows = sqlx::query_scalar::<_, JsonValue>(
        r#"
        SELECT json_build_object(
            'id', id,
            'fen', fen,
            'sequence', sequence,
            'ply', ply,
            'side', side,
            'games', games,
            'best_move', best_move,
            'best_eval_cp', best_eval_cp,
            'best_maia_pct', best_maia_pct,
            'second_move', second_move,
            'second_eval_cp', second_eval_cp,
            'gap_cp', gap_cp,
            'mistake_move', mistake_move,
            'mistake_eval_cp', mistake_eval_cp,
            'mistake_maia_pct', mistake_maia_pct,
            'eval_loss_cp', eval_loss_cp,
            'maia_top_3', maia_top_3
        )
        FROM trainer_hard_moves
        WHERE opening_name = $1
        ORDER BY games DESC
        "#,
    )
    .bind(opening_name)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn mark_hard_move_complete(
    pool: &PgPool,
    user_id: i64,
    hard_move_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO trainer_hard_move_progress (user_id, hard_move_id)
        VALUES ($1, $2)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(user_id)
    .bind(hard_move_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_hard_move_user_progress(
    pool: &PgPool,
    user_id: i64,
) -> Result<Vec<(String, i64)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, i64)>(
        r#"
        SELECT hm.opening_name, COUNT(*) AS completed_count
        FROM trainer_hard_move_progress p
        JOIN trainer_hard_moves hm ON hm.id = p.hard_move_id
        WHERE p.user_id = $1
        GROUP BY hm.opening_name
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_completed_hard_move_ids(
    pool: &PgPool,
    user_id: i64,
    opening_name: &str,
) -> Result<Vec<String>, sqlx::Error> {
    let rows = sqlx::query_scalar::<_, String>(
        r#"
        SELECT p.hard_move_id
        FROM trainer_hard_move_progress p
        JOIN trainer_hard_moves hm ON hm.id = p.hard_move_id
        WHERE p.user_id = $1 AND hm.opening_name = $2
        "#,
    )
    .bind(user_id)
    .bind(opening_name)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn delete_hard_moves_by_opening(
    pool: &PgPool,
    opening_name: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM trainer_hard_moves WHERE opening_name = $1",
    )
    .bind(opening_name)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn upsert_hard_moves(
    pool: &PgPool,
    opening_name: &str,
    moves: &[JsonValue],
) -> Result<usize, sqlx::Error> {
    let mut count = 0;

    for m in moves {
        let id = m["id"].as_str().unwrap_or_default();
        let fen = m["fen"].as_str().unwrap_or_default();
        let sequence = m["sequence"].as_str().unwrap_or_default();
        let ply = m["ply"].as_i64().unwrap_or(0) as i16;
        let side = m["side"].as_str().unwrap_or("black");
        let games = m["games"].as_i64().unwrap_or(0) as i32;
        let best_move = m["best_move"].as_str().unwrap_or_default();
        let best_eval_cp = m["best_eval_cp"].as_i64().unwrap_or(0) as i32;
        let best_maia_pct = m["best_maia_pct"].as_f64().map(|v| v as f32);
        let second_move = m["second_move"].as_str().map(|s| s.to_string());
        let second_eval_cp = m["second_eval_cp"].as_i64().map(|v| v as i32);
        let gap_cp = m["gap_cp"].as_i64().unwrap_or(0) as i32;
        let mistake_move = m["mistake_move"].as_str().unwrap_or_default();
        let mistake_eval_cp = m["mistake_eval_cp"].as_i64().unwrap_or(0) as i32;
        let mistake_maia_pct = m["mistake_maia_pct"].as_f64().map(|v| v as f32);
        let eval_loss_cp = m["eval_loss_cp"].as_i64().unwrap_or(0) as i32;
        let maia_top_3 = m.get("maia_top_3");

        sqlx::query(
            r#"
            INSERT INTO trainer_hard_moves
                (id, opening_name, fen, sequence, ply, side, games, best_move, best_eval_cp,
                 best_maia_pct, second_move, second_eval_cp, gap_cp, mistake_move,
                 mistake_eval_cp, mistake_maia_pct, eval_loss_cp, maia_top_3)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            ON CONFLICT (id) DO UPDATE SET
                opening_name = EXCLUDED.opening_name,
                fen = EXCLUDED.fen,
                sequence = EXCLUDED.sequence,
                ply = EXCLUDED.ply,
                side = EXCLUDED.side,
                games = EXCLUDED.games,
                best_move = EXCLUDED.best_move,
                best_eval_cp = EXCLUDED.best_eval_cp,
                best_maia_pct = EXCLUDED.best_maia_pct,
                second_move = EXCLUDED.second_move,
                second_eval_cp = EXCLUDED.second_eval_cp,
                gap_cp = EXCLUDED.gap_cp,
                mistake_move = EXCLUDED.mistake_move,
                mistake_eval_cp = EXCLUDED.mistake_eval_cp,
                mistake_maia_pct = EXCLUDED.mistake_maia_pct,
                eval_loss_cp = EXCLUDED.eval_loss_cp,
                maia_top_3 = EXCLUDED.maia_top_3
            "#,
        )
        .bind(id)
        .bind(opening_name)
        .bind(fen)
        .bind(sequence)
        .bind(ply)
        .bind(side)
        .bind(games)
        .bind(best_move)
        .bind(best_eval_cp)
        .bind(best_maia_pct)
        .bind(&second_move)
        .bind(second_eval_cp)
        .bind(gap_cp)
        .bind(mistake_move)
        .bind(mistake_eval_cp)
        .bind(mistake_maia_pct)
        .bind(eval_loss_cp)
        .bind(maia_top_3)
        .execute(pool)
        .await?;

        count += 1;
    }

    Ok(count)
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
