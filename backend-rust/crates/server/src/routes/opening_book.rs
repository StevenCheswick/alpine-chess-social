use axum::{extract::Query, Extension, Json};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::book_cache::{self, BOOK_CACHE};
use crate::error::AppError;

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
/// Uses in-memory cache for instant lookups, falls back to DB if cache is empty.
pub async fn check_book_move(
    Extension(pool): Extension<PgPool>,
    Query(q): Query<BookCheckQuery>,
) -> Result<Json<BookCheckResponse>, AppError> {
    // Try in-memory cache first (instant lookup)
    if !BOOK_CACHE.is_empty() {
        if let Some(stats) = book_cache::lookup(&q.fen, &q.san) {
            return Ok(Json(BookCheckResponse {
                is_book: true,
                games: Some(stats.games),
                white_wins: Some(stats.white_wins),
                draws: Some(stats.draws),
                black_wins: Some(stats.black_wins),
            }));
        } else {
            // Cache is loaded but move not found - it's not a book move
            return Ok(Json(BookCheckResponse {
                is_book: false,
                games: None,
                white_wins: None,
                draws: None,
                black_wins: None,
            }));
        }
    }

    // Fallback to database query (cache not loaded)
    let normalized_fen = book_cache::normalize_fen(&q.fen);

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
