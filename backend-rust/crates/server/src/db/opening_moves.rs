use shakmaty::{Chess, Position, fen::Fen, san::San, EnPassantMode};
use sqlx::PgPool;
use std::collections::HashMap;

use crate::error::AppError;

const MAX_DEPTH: usize = 15;
const STARTING_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

/// Aggregated stats for a single position+move, keyed by (color, parent_fen, move_san).
struct AggEntry {
    result_fen: String,
    depth: i16,
    games: i32,
    wins: i32,
    losses: i32,
    draws: i32,
    eval_cp: Option<i32>,
    total_cp_loss: i64,
    cp_loss_count: i32,
}

/// Process all unprocessed games for a user and upsert per-position opening stats.
/// Pre-aggregates all positions in memory, then bulk-upserts in a single query.
pub async fn populate_opening_stats(pool: &PgPool, user_id: i64) -> Result<(), AppError> {
    use sqlx::Row;

    // Fetch unprocessed games with optional analysis evals
    let rows = sqlx::query(
        r#"SELECT ug.id, ug.tcn, ug.result, ug.user_color,
                  ga.moves AS analysis_moves
           FROM user_games ug
           LEFT JOIN game_analysis ga ON ug.id = ga.game_id
           WHERE ug.user_id = $1 AND ug.opening_stats_at IS NULL AND ug.tcn IS NOT NULL"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Sqlx)?;

    if rows.is_empty() {
        return Ok(());
    }

    // Pre-aggregate all positions in memory: (color, parent_fen, move_san) → AggEntry
    let mut agg: HashMap<(String, String, String), AggEntry> = HashMap::new();
    let mut processed_ids: Vec<i64> = Vec::new();

    for row in &rows {
        let game_id: i64 = row.try_get("id").unwrap_or(0);
        let tcn: String = match row.try_get::<Option<String>, _>("tcn").unwrap_or(None) {
            Some(t) if !t.is_empty() => t,
            _ => {
                processed_ids.push(game_id);
                continue;
            }
        };
        let result: String = row.try_get("result").unwrap_or_default();
        let user_color: String = row.try_get("user_color").unwrap_or_default();
        let color = user_color.to_lowercase();
        let analysis_moves: Option<serde_json::Value> =
            row.try_get("analysis_moves").unwrap_or(None);

        // Truncate TCN to MAX_DEPTH * 2 plies (2 chars per ply)
        let max_tcn_len = MAX_DEPTH * 2 * 2;
        let tcn_truncated = if tcn.len() > max_tcn_len {
            &tcn[..max_tcn_len]
        } else {
            &tcn
        };

        let moves = chess_core::tcn::decode_tcn(tcn_truncated);
        if moves.is_empty() {
            processed_ids.push(game_id);
            continue;
        }

        let mut pos = Chess::default();
        let mut parent_fen = STARTING_FEN.to_string();

        for (ply, mv) in moves.iter().enumerate() {
            let depth = (ply / 2) + 1;
            if depth > MAX_DEPTH {
                break;
            }

            let san = San::from_move(&pos, mv.clone()).to_string();
            pos.play_unchecked(mv.clone());
            let result_fen = Fen::from_position(&pos, EnPassantMode::Legal).to_string();

            let analysis_move = analysis_moves
                .as_ref()
                .and_then(|am| am.as_array())
                .and_then(|arr| arr.get(ply));

            let eval_cp: Option<i32> = analysis_move
                .and_then(|m| m.get("move_eval").or_else(|| m.get("eval")))
                .and_then(|v| v.as_i64())
                .map(|v| v as i32);

            let cp_loss: Option<f64> = analysis_move
                .and_then(|m| m.get("cp_loss"))
                .and_then(|v| v.as_f64());

            let key = (color.clone(), parent_fen.clone(), san);
            let entry = agg.entry(key).or_insert_with(|| AggEntry {
                result_fen: result_fen.clone(),
                depth: depth as i16,
                games: 0,
                wins: 0,
                losses: 0,
                draws: 0,
                eval_cp: None,
                total_cp_loss: 0,
                cp_loss_count: 0,
            });

            entry.games += 1;
            match result.as_str() {
                "W" => entry.wins += 1,
                "L" => entry.losses += 1,
                _ => entry.draws += 1,
            }
            if eval_cp.is_some() {
                entry.eval_cp = eval_cp;
            }
            if let Some(loss) = cp_loss {
                entry.total_cp_loss += loss.round() as i64;
                entry.cp_loss_count += 1;
            }

            parent_fen = result_fen;
        }

        processed_ids.push(game_id);
    }

    // Bulk upsert using UNNEST arrays (one query instead of thousands)
    if !agg.is_empty() {
        let len = agg.len();
        let mut v_color: Vec<String> = Vec::with_capacity(len);
        let mut v_parent_fen: Vec<String> = Vec::with_capacity(len);
        let mut v_move_san: Vec<String> = Vec::with_capacity(len);
        let mut v_result_fen: Vec<String> = Vec::with_capacity(len);
        let mut v_depth: Vec<i16> = Vec::with_capacity(len);
        let mut v_games: Vec<i32> = Vec::with_capacity(len);
        let mut v_wins: Vec<i32> = Vec::with_capacity(len);
        let mut v_losses: Vec<i32> = Vec::with_capacity(len);
        let mut v_draws: Vec<i32> = Vec::with_capacity(len);
        let mut v_eval_cp: Vec<Option<i32>> = Vec::with_capacity(len);
        let mut v_total_cp_loss: Vec<i64> = Vec::with_capacity(len);
        let mut v_cp_loss_count: Vec<i32> = Vec::with_capacity(len);

        for ((color, parent_fen, move_san), entry) in &agg {
            v_color.push(color.clone());
            v_parent_fen.push(parent_fen.clone());
            v_move_san.push(move_san.clone());
            v_result_fen.push(entry.result_fen.clone());
            v_depth.push(entry.depth);
            v_games.push(entry.games);
            v_wins.push(entry.wins);
            v_losses.push(entry.losses);
            v_draws.push(entry.draws);
            v_eval_cp.push(entry.eval_cp);
            v_total_cp_loss.push(entry.total_cp_loss);
            v_cp_loss_count.push(entry.cp_loss_count);
        }

        sqlx::query(
            r#"INSERT INTO user_opening_moves
                   (user_id, color, parent_fen, move_san, result_fen, depth, games, wins, losses, draws, eval_cp, total_cp_loss, cp_loss_count)
               SELECT $1, * FROM UNNEST(
                   $2::text[], $3::text[], $4::text[], $5::text[],
                   $6::smallint[], $7::int[], $8::int[], $9::int[], $10::int[], $11::int[],
                   $12::bigint[], $13::int[]
               ) AS t(color, parent_fen, move_san, result_fen, depth, games, wins, losses, draws, eval_cp, total_cp_loss, cp_loss_count)
               ON CONFLICT (user_id, color, parent_fen, move_san) DO UPDATE SET
                   games = user_opening_moves.games + EXCLUDED.games,
                   wins = user_opening_moves.wins + EXCLUDED.wins,
                   losses = user_opening_moves.losses + EXCLUDED.losses,
                   draws = user_opening_moves.draws + EXCLUDED.draws,
                   eval_cp = COALESCE(EXCLUDED.eval_cp, user_opening_moves.eval_cp),
                   total_cp_loss = user_opening_moves.total_cp_loss + EXCLUDED.total_cp_loss,
                   cp_loss_count = user_opening_moves.cp_loss_count + EXCLUDED.cp_loss_count"#,
        )
        .bind(user_id)
        .bind(&v_color)
        .bind(&v_parent_fen)
        .bind(&v_move_san)
        .bind(&v_result_fen)
        .bind(&v_depth)
        .bind(&v_games)
        .bind(&v_wins)
        .bind(&v_losses)
        .bind(&v_draws)
        .bind(&v_eval_cp)
        .bind(&v_total_cp_loss)
        .bind(&v_cp_loss_count)
        .execute(pool)
        .await
        .map_err(AppError::Sqlx)?;
    }

    // Mark processed games
    if !processed_ids.is_empty() {
        sqlx::query("UPDATE user_games SET opening_stats_at = NOW() WHERE id = ANY($1)")
            .bind(&processed_ids)
            .execute(pool)
            .await
            .map_err(AppError::Sqlx)?;
    }

    Ok(())
}

/// Update eval_cp values for opening positions when a game's analysis is saved.
pub async fn enrich_opening_evals(pool: &PgPool, game_id: i64) -> Result<(), AppError> {
    use sqlx::Row;

    // Get game info + analysis moves
    let row = sqlx::query(
        r#"SELECT ug.user_id, ug.tcn, ug.user_color, ug.opening_stats_at,
                  ga.moves AS analysis_moves
           FROM user_games ug
           INNER JOIN game_analysis ga ON ug.id = ga.game_id
           WHERE ug.id = $1"#,
    )
    .bind(game_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Sqlx)?;

    let row = match row {
        Some(r) => r,
        None => return Ok(()),
    };

    // Only enrich if the game has already been opening-processed
    let opening_stats_at: Option<chrono::DateTime<chrono::Utc>> =
        row.try_get("opening_stats_at").unwrap_or(None);
    if opening_stats_at.is_none() {
        return Ok(());
    }

    let user_id: i64 = row.try_get("user_id").unwrap_or(0);
    let tcn: String = match row.try_get::<Option<String>, _>("tcn").unwrap_or(None) {
        Some(t) if !t.is_empty() => t,
        _ => return Ok(()),
    };
    let user_color: String = row.try_get("user_color").unwrap_or_default();
    let color = user_color.to_lowercase();
    let analysis_moves: serde_json::Value =
        row.try_get("analysis_moves").unwrap_or(serde_json::Value::Null);

    let analysis_arr = match analysis_moves.as_array() {
        Some(arr) => arr,
        None => return Ok(()),
    };

    // Walk the opening to update evals
    let max_tcn_len = MAX_DEPTH * 2 * 2;
    let tcn_truncated = if tcn.len() > max_tcn_len {
        &tcn[..max_tcn_len]
    } else {
        &tcn
    };

    let moves = chess_core::tcn::decode_tcn(tcn_truncated);
    let mut pos = Chess::default();
    let mut parent_fen = STARTING_FEN.to_string();

    for (ply, mv) in moves.iter().enumerate() {
        let depth = (ply / 2) + 1;
        if depth > MAX_DEPTH {
            break;
        }

        let san = San::from_move(&pos, mv.clone()).to_string();
        pos.play_unchecked(mv.clone());

        let analysis_move = analysis_arr.get(ply);

        let eval_cp: Option<i32> = analysis_move
            .and_then(|m| m.get("move_eval").or_else(|| m.get("eval")))
            .and_then(|v| v.as_i64())
            .map(|v| v as i32);

        let cp_loss: Option<f64> = analysis_move
            .and_then(|m| m.get("cp_loss"))
            .and_then(|v| v.as_f64());

        if eval_cp.is_some() || cp_loss.is_some() {
            let cp_loss_rounded = cp_loss.map(|l| l.round() as i64).unwrap_or(0);
            let cp_loss_inc = if cp_loss.is_some() { 1i32 } else { 0 };

            sqlx::query(
                r#"UPDATE user_opening_moves
                   SET eval_cp = COALESCE($1, eval_cp),
                       total_cp_loss = total_cp_loss + $6,
                       cp_loss_count = cp_loss_count + $7
                   WHERE user_id = $2 AND color = $3 AND parent_fen = $4 AND move_san = $5"#,
            )
            .bind(eval_cp)
            .bind(user_id)
            .bind(&color)
            .bind(&parent_fen)
            .bind(&san)
            .bind(cp_loss_rounded)
            .bind(cp_loss_inc)
            .execute(pool)
            .await
            .map_err(AppError::Sqlx)?;
        }

        let result_fen = Fen::from_position(&pos, EnPassantMode::Legal).to_string();
        parent_fen = result_fen;
    }

    Ok(())
}

/// Get children of a single position (one level only).
pub async fn get_children(
    pool: &PgPool,
    user_id: i64,
    color: &str,
    parent_fen: &str,
) -> Result<Vec<OpeningMoveRow>, AppError> {
    let rows: Vec<OpeningMoveRow> = sqlx::query_as(
        r#"SELECT parent_fen, move_san, result_fen, depth, games, wins, losses, draws, eval_cp
           FROM user_opening_moves
           WHERE user_id = $1 AND color = $2 AND parent_fen = $3
           ORDER BY games DESC"#,
    )
    .bind(user_id)
    .bind(color)
    .bind(parent_fen)
    .fetch_all(pool)
    .await
    .map_err(AppError::Sqlx)?;

    Ok(rows)
}

/// Check if a user has any opening stats rows.
pub async fn has_opening_stats(pool: &PgPool, user_id: i64) -> Result<bool, AppError> {
    let row: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM user_opening_moves WHERE user_id = $1)",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::Sqlx)?;

    Ok(row.0)
}

#[derive(sqlx::FromRow)]
pub struct OpeningMoveRow {
    pub parent_fen: String,
    pub move_san: String,
    pub result_fen: String,
    pub depth: i16,
    pub games: i32,
    pub wins: i32,
    pub losses: i32,
    pub draws: i32,
    pub eval_cp: Option<i32>,
}

#[derive(sqlx::FromRow)]
pub struct OpeningBlunderRow {
    pub move_san: String,
    pub ply: i32,
    pub color: String,
    pub line: String,
    pub best_move: Option<String>,
    pub mistake_count: i64,
    pub avg_cp_loss: f64,
    pub sample_game_id: i64,
}

#[derive(sqlx::FromRow)]
pub struct CleanLineRow {
    pub line: String,
    pub color: String,
    pub clean_depth: i32,
    pub game_count: i64,
    pub avg_cp_loss: f64,
    pub sample_game_id: i64,
}

/// Get the user's most repeated opening mistakes by querying raw analysis data.
/// Groups by the full move sequence (line) so the same move in different openings
/// is counted separately. Returns the most frequently repeated line-specific mistakes.
pub async fn get_opening_blunders(
    pool: &PgPool,
    user_id: i64,
    min_cp_loss: f64,
    limit: i32,
) -> Result<Vec<OpeningBlunderRow>, AppError> {
    let rows: Vec<OpeningBlunderRow> = sqlx::query_as(
        r#"WITH opening_mistakes AS (
             SELECT
               ug.id AS game_id,
               (m->>'move')::text AS move_san,
               (ord - 1)::int AS ply,
               (m->>'cp_loss')::float8 AS cp_loss,
               (m->>'best_move')::text AS best_move,
               LOWER(ug.user_color) AS color,
               (SELECT string_agg(elem->>'move', ' ' ORDER BY o)
                FROM jsonb_array_elements(ga.moves) WITH ORDINALITY AS y(elem, o)
                WHERE o <= x.ord) AS line
             FROM user_games ug
             INNER JOIN game_analysis ga ON ug.id = ga.game_id,
             LATERAL jsonb_array_elements(ga.moves) WITH ORDINALITY AS x(m, ord)
             WHERE ug.user_id = $1
               AND (ord - 1) < 30
               AND (m->>'cp_loss')::float8 >= $2
               AND (
                 (LOWER(ug.user_color) = 'white' AND (ord - 1) % 2 = 0)
                 OR (LOWER(ug.user_color) = 'black' AND (ord - 1) % 2 = 1)
               )
           )
           SELECT
             move_san,
             ply,
             color,
             line,
             COUNT(*) AS mistake_count,
             ROUND(AVG(cp_loss)::numeric, 1)::float8 AS avg_cp_loss,
             MIN(best_move) AS best_move,
             MIN(game_id) AS sample_game_id
           FROM opening_mistakes
           GROUP BY line, move_san, ply, color
           ORDER BY mistake_count DESC
           LIMIT $3"#,
    )
    .bind(user_id)
    .bind(min_cp_loss)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(AppError::Sqlx)?;

    Ok(rows)
}

/// Find the user's deepest, most consistently clean opening lines.
/// For each game, finds the deepest user-move ply where ALL user moves up to
/// that point have cp_loss below the threshold. Groups by that line, counts
/// frequency, and orders by depth then frequency.
pub async fn get_cleanest_lines(
    pool: &PgPool,
    user_id: i64,
    max_cp_loss: f64,
    min_depth: i32,
    limit: i32,
) -> Result<Vec<CleanLineRow>, AppError> {
    let rows: Vec<CleanLineRow> = sqlx::query_as(
        r#"WITH game_first_mistake AS (
             -- For each game, find the first user-move ply where cp_loss >= threshold
             SELECT
               ug.id AS game_id,
               LOWER(ug.user_color) AS color,
               MIN(CASE
                 WHEN (m->>'cp_loss')::float8 >= $2
                      AND (
                        (LOWER(ug.user_color) = 'white' AND (ord - 1) % 2 = 0)
                        OR (LOWER(ug.user_color) = 'black' AND (ord - 1) % 2 = 1)
                      )
                 THEN (ord - 1)::int
                 ELSE NULL
               END) AS first_mistake_ply
             FROM user_games ug
             INNER JOIN game_analysis ga ON ug.id = ga.game_id,
             LATERAL jsonb_array_elements(ga.moves) WITH ORDINALITY AS x(m, ord)
             WHERE ug.user_id = $1 AND (ord - 1) < 30
             GROUP BY ug.id, ug.user_color
           ),
           clean_lines AS (
             SELECT
               gfm.game_id,
               gfm.color,
               -- End line on user's last clean move:
               -- With a mistake: mistake ply is always a user ply, so -1 ends on prior user move
               -- No mistake: default cap is 29 for white (odd→last ply even), 30 for black (even→last ply odd)
               CASE
                 WHEN gfm.first_mistake_ply IS NOT NULL THEN gfm.first_mistake_ply - 1
                 WHEN gfm.color = 'white' THEN 29
                 ELSE 30
               END AS clean_up_to,
               (SELECT string_agg(elem->>'move', ' ' ORDER BY o)
                FROM jsonb_array_elements(ga.moves) WITH ORDINALITY AS y(elem, o)
                WHERE (o - 1) < CASE
                  WHEN gfm.first_mistake_ply IS NOT NULL THEN gfm.first_mistake_ply - 1
                  WHEN gfm.color = 'white' THEN 29
                  ELSE 30
                END
               ) AS line,
               (SELECT COALESCE(AVG((elem->>'cp_loss')::float8), 0)
                FROM jsonb_array_elements(ga.moves) WITH ORDINALITY AS y(elem, o)
                WHERE (o - 1) < CASE
                  WHEN gfm.first_mistake_ply IS NOT NULL THEN gfm.first_mistake_ply - 1
                  WHEN gfm.color = 'white' THEN 29
                  ELSE 30
                END
                  AND (
                    (gfm.color = 'white' AND (o - 1) % 2 = 0)
                    OR (gfm.color = 'black' AND (o - 1) % 2 = 1)
                  )
               ) AS avg_cp_loss
             FROM game_first_mistake gfm
             INNER JOIN game_analysis ga ON gfm.game_id = ga.game_id
           )
           SELECT
             line,
             color,
             (clean_up_to / 2 + clean_up_to % 2)::int AS clean_depth,
             COUNT(*) AS game_count,
             ROUND(AVG(avg_cp_loss)::numeric, 1)::float8 AS avg_cp_loss,
             MIN(game_id) AS sample_game_id
           FROM clean_lines
           WHERE line IS NOT NULL
             AND (clean_up_to / 2 + clean_up_to % 2) >= $3
           GROUP BY line, color, clean_up_to
           HAVING COUNT(*) >= 2
           ORDER BY clean_depth DESC, game_count DESC
           LIMIT $4"#,
    )
    .bind(user_id)
    .bind(max_cp_loss)
    .bind(min_depth)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(AppError::Sqlx)?;

    Ok(rows)
}
