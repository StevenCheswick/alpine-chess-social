use axum::{extract::Path, http::HeaderMap, Extension, Json};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use sqlx::PgPool;

use crate::config::Config;
use crate::db::trainer_maia;
use crate::error::AppError;
use crate::routes::trainer::check_admin_secret;

/// GET /api/trainer/maia-positions
/// List all play-vs-Maia positions (public — used to render the card grid).
pub async fn list_positions(
    Extension(pool): Extension<PgPool>,
) -> Result<Json<JsonValue>, AppError> {
    let positions = trainer_maia::list_positions(&pool).await?;
    let result: Vec<JsonValue> = positions
        .into_iter()
        .map(|p| {
            serde_json::json!({
                "id": p.id,
                "title": p.title,
                "fen": p.fen,
                "user_side": p.user_side,
                "notes": p.notes,
                "updated_at": p.updated_at,
            })
        })
        .collect();
    Ok(Json(serde_json::json!(result)))
}

/// GET /api/trainer/maia-positions/:id
pub async fn get_position(
    Extension(pool): Extension<PgPool>,
    Path(id): Path<String>,
) -> Result<Json<JsonValue>, AppError> {
    let pos = trainer_maia::get_position(&pool, &id).await?;
    match pos {
        Some(p) => Ok(Json(serde_json::json!({
            "id": p.id,
            "title": p.title,
            "fen": p.fen,
            "user_side": p.user_side,
            "notes": p.notes,
            "updated_at": p.updated_at,
        }))),
        None => Err(AppError::NotFound("maia position not found".into())),
    }
}

#[derive(Deserialize)]
pub struct UploadBody {
    pub id: String,
    pub title: String,
    pub fen: String,
    pub user_side: String,
    #[serde(default)]
    pub notes: Option<String>,
    pub opening_name: Option<String>,
}

/// POST /api/admin/trainer/maia-positions/upload
pub async fn upload_position(
    headers: HeaderMap,
    Extension(pool): Extension<PgPool>,
    Extension(config): Extension<Config>,
    Json(body): Json<UploadBody>,
) -> Result<Json<JsonValue>, AppError> {
    check_admin_secret(&headers, &config)?;
    if body.user_side != "white" && body.user_side != "black" {
        return Err(AppError::BadRequest(
            "user_side must be 'white' or 'black'".into(),
        ));
    }
    trainer_maia::upsert_position(
        &pool,
        &body.id,
        &body.title,
        &body.fen,
        &body.user_side,
        body.notes.as_deref(),
        body.opening_name.as_deref(),
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

/// POST /api/admin/trainer/maia-positions/delete
pub async fn delete_position(
    headers: HeaderMap,
    Extension(pool): Extension<PgPool>,
    Extension(config): Extension<Config>,
    Json(body): Json<DeleteBody>,
) -> Result<Json<JsonValue>, AppError> {
    check_admin_secret(&headers, &config)?;
    let deleted = trainer_maia::delete_position(&pool, &body.id).await?;
    Ok(Json(serde_json::json!({
        "deleted": deleted,
        "id": body.id,
    })))
}
