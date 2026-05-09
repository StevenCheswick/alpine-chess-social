use axum::{extract::Path, http::HeaderMap, Extension, Json};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use sqlx::PgPool;

use crate::auth::middleware::AuthUser;
use crate::config::Config;
use crate::db::trainer_trees;
use crate::error::AppError;
use crate::routes::trainer::check_admin_secret;

/// GET /api/trainer/trees
/// List all available opening trees (id, name, color, sizes — no full tree body).
pub async fn list_trees(
    Extension(pool): Extension<PgPool>,
    _user: AuthUser,
) -> Result<Json<JsonValue>, AppError> {
    let trees = trainer_trees::list_trees(&pool).await?;
    let result: Vec<JsonValue> = trees
        .into_iter()
        .map(|t| {
            serde_json::json!({
                "id": t.id,
                "name": t.name,
                "color": t.color,
                "start_moves": t.start_moves,
                "start_fen": t.start_fen,
                "nodes_count": t.nodes_count,
                "lines_count": t.lines_count,
                "updated_at": t.updated_at,
            })
        })
        .collect();
    Ok(Json(serde_json::json!(result)))
}

/// GET /api/trainer/trees/:id
/// Full tree JSON for a single dataset.
pub async fn get_tree(
    Extension(pool): Extension<PgPool>,
    Path(id): Path<String>,
    _user: AuthUser,
) -> Result<Json<JsonValue>, AppError> {
    let tree = trainer_trees::get_tree(&pool, &id).await?;
    match tree {
        Some(t) => Ok(Json(t)),
        None => Err(AppError::NotFound("tree not found".into())),
    }
}

#[derive(Deserialize)]
pub struct UploadBody {
    pub id: String,
    pub name: String,
    pub color: String,
    pub start_moves: Option<String>,
    pub start_fen: String,
    pub nodes_count: i32,
    #[serde(default)]
    pub lines_count: i32,
    pub tree: JsonValue,
    pub opening_name: Option<String>,
}

/// POST /api/admin/trainer/trees/upload
pub async fn upload_tree(
    headers: HeaderMap,
    Extension(pool): Extension<PgPool>,
    Extension(config): Extension<Config>,
    Json(body): Json<UploadBody>,
) -> Result<Json<JsonValue>, AppError> {
    check_admin_secret(&headers, &config)?;
    let start_moves = body.start_moves.unwrap_or_default();
    let opening_name = body.opening_name.as_deref();
    trainer_trees::upsert_tree(
        &pool,
        &body.id,
        &body.name,
        &body.color,
        &start_moves,
        &body.start_fen,
        body.nodes_count,
        body.lines_count,
        &body.tree,
        opening_name,
    )
    .await?;
    Ok(Json(serde_json::json!({
        "ok": true,
        "id": body.id,
    })))
}

#[derive(Deserialize)]
pub struct DeleteBody {
    pub id: String,
}

/// POST /api/admin/trainer/trees/delete
pub async fn delete_tree(
    headers: HeaderMap,
    Extension(pool): Extension<PgPool>,
    Extension(config): Extension<Config>,
    Json(body): Json<DeleteBody>,
) -> Result<Json<JsonValue>, AppError> {
    check_admin_secret(&headers, &config)?;
    let deleted = trainer_trees::delete_tree(&pool, &body.id).await?;
    Ok(Json(serde_json::json!({
        "deleted": deleted,
        "id": body.id,
    })))
}

/// GET /api/trainer/trees/:id/progress
/// Returns this user's learned (fen, move_san) pairs for the given tree.
pub async fn get_progress(
    Extension(pool): Extension<PgPool>,
    Path(id): Path<String>,
    user: AuthUser,
) -> Result<Json<JsonValue>, AppError> {
    let pairs = trainer_trees::get_user_progress(&pool, user.id, &id).await?;
    let arr: Vec<JsonValue> = pairs
        .into_iter()
        .map(|(fen, san)| serde_json::json!([fen, san]))
        .collect();
    Ok(Json(serde_json::json!({ "learned": arr })))
}

#[derive(Deserialize)]
pub struct ProgressBody {
    pub moves: Vec<(String, String)>, // [[fen, san], ...]
}

/// POST /api/trainer/trees/:id/progress
/// Batch-mark moves as learned for the current user.
pub async fn post_progress(
    Extension(pool): Extension<PgPool>,
    Path(id): Path<String>,
    user: AuthUser,
    Json(body): Json<ProgressBody>,
) -> Result<Json<JsonValue>, AppError> {
    let inserted = trainer_trees::mark_learned(&pool, user.id, &id, &body.moves).await?;
    Ok(Json(serde_json::json!({ "ok": true, "inserted": inserted })))
}
