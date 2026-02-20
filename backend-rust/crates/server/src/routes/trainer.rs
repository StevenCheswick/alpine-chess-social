use axum::{extract::Query, Extension, Json};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use sqlx::PgPool;
use std::collections::HashMap;

use crate::auth::middleware::AuthUser;
use crate::db::trainer;
use crate::error::AppError;

/// GET /api/trainer/openings
/// List available openings with puzzle counts + per-user progress.
pub async fn list_openings(
    Extension(pool): Extension<PgPool>,
    user: AuthUser,
) -> Result<Json<JsonValue>, AppError> {
    let openings = trainer::list_openings(&pool).await?;
    let progress = trainer::get_user_progress(&pool, user.id).await?;

    let progress_map: HashMap<String, i64> = progress.into_iter().collect();

    let result: Vec<JsonValue> = openings
        .into_iter()
        .map(|o| {
            let completed_count = progress_map.get(&o.opening_name).copied().unwrap_or(0);
            serde_json::json!({
                "opening_name": o.opening_name,
                "eco_codes": o.eco_codes,
                "puzzle_count": o.puzzle_count,
                "completed_count": completed_count,
                "sample_fen": o.sample_fen,
            })
        })
        .collect();

    Ok(Json(serde_json::json!(result)))
}

#[derive(Deserialize)]
pub struct PuzzlesQuery {
    pub opening: String,
}

/// GET /api/trainer/puzzles?opening=Evans+Gambit
/// All puzzles for an opening (tree JSONB included) + completed IDs.
pub async fn get_puzzles(
    Extension(pool): Extension<PgPool>,
    Query(q): Query<PuzzlesQuery>,
    user: AuthUser,
) -> Result<Json<JsonValue>, AppError> {
    let puzzles = trainer::get_puzzles_by_opening(&pool, &q.opening).await?;
    let completed_ids = trainer::get_completed_puzzle_ids(&pool, user.id, &q.opening).await?;
    Ok(Json(serde_json::json!({
        "puzzles": puzzles,
        "completed_ids": completed_ids,
    })))
}

#[derive(Deserialize)]
pub struct MarkCompleteBody {
    pub puzzle_id: String,
}

/// POST /api/trainer/progress
/// Mark a puzzle as completed for the current user.
pub async fn mark_complete(
    Extension(pool): Extension<PgPool>,
    user: AuthUser,
    Json(body): Json<MarkCompleteBody>,
) -> Result<Json<JsonValue>, AppError> {
    trainer::mark_puzzle_complete(&pool, user.id, &body.puzzle_id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct UploadBody {
    pub opening_name: String,
    pub puzzles: Vec<JsonValue>,
}

/// POST /api/admin/trainer/upload
/// Upsert puzzle batch.
pub async fn upload_puzzles(
    Extension(pool): Extension<PgPool>,
    Json(body): Json<UploadBody>,
) -> Result<Json<JsonValue>, AppError> {
    let count = trainer::upsert_puzzles(&pool, &body.opening_name, &body.puzzles).await?;
    Ok(Json(serde_json::json!({
        "uploaded": count,
        "opening_name": body.opening_name,
    })))
}
