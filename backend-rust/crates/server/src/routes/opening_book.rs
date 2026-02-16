use axum::{extract::Query, Extension, Json};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::error::AppError;

/// Strips move counters from FEN, keeping only position + side + castling + ep.
fn normalize_fen(fen: &str) -> String {
    fen.split_whitespace().take(4).collect::<Vec<_>>().join(" ")
}

#[derive(Deserialize)]
pub struct BookCheckQuery {
    pub fen: String,
    pub san: String,
}

#[derive(Serialize)]
pub struct BookCheckResponse {
    pub is_book: bool,
    pub games: Option<i32>,
    pub white_wins: Option<i32>,
    pub draws: Option<i32>,
    pub black_wins: Option<i32>,
}

/// GET /api/opening-book/check?fen=...&san=...
/// Check if a move exists in the opening book.
pub async fn check_book_move(
    Extension(pool): Extension<PgPool>,
    Query(q): Query<BookCheckQuery>,
) -> Result<Json<BookCheckResponse>, AppError> {
    let normalized_fen = normalize_fen(&q.fen);

    let row: Option<(i32, i32, i32, i32)> = sqlx::query_as(
        r#"SELECT games, white_wins, draws, black_wins
           FROM opening_book
           WHERE parent_fen = $1 AND move_san = $2"#,
    )
    .bind(&normalized_fen)
    .bind(&q.san)
    .fetch_optional(&pool)
    .await
    .map_err(AppError::Sqlx)?;

    match row {
        Some((games, white_wins, draws, black_wins)) => Ok(Json(BookCheckResponse {
            is_book: true,
            games: Some(games),
            white_wins: Some(white_wins),
            draws: Some(draws),
            black_wins: Some(black_wins),
        })),
        None => Ok(Json(BookCheckResponse {
            is_book: false,
            games: None,
            white_wins: None,
            draws: None,
            black_wins: None,
        })),
    }
}
