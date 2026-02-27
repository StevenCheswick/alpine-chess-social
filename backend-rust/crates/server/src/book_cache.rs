//! In-memory opening book cache.
//!
//! The book is loaded from the database at server startup via `load_from_db()`.
//! Use the `POST /api/admin/opening-book/reload` endpoint to hot-reload.
//! The `load_book`/`save_book` functions remain for the export-book binary.

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::{LazyLock, RwLock};

/// Stats for a single book move.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookMoveStats {
    pub games: i32,
    pub white_wins: i32,
    pub draws: i32,
    pub black_wins: i32,
}

/// The entire opening book: FEN -> (move_san -> stats)
pub type OpeningBook = HashMap<String, HashMap<String, BookMoveStats>>;

/// Default path to the binary book file.
pub const BOOK_FILE_PATH: &str = "data/opening_book.bin";

/// Global in-memory book cache, loaded from DB at startup via `load_from_db()`.
pub static BOOK_CACHE: LazyLock<RwLock<OpeningBook>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Load the book from a binary file.
pub fn load_book<P: AsRef<Path>>(path: P) -> Result<OpeningBook, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let book: OpeningBook = bincode::deserialize_from(reader)?;
    Ok(book)
}

/// Save the book to a binary file.
pub fn save_book<P: AsRef<Path>>(book: &OpeningBook, path: P) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(path)?;
    bincode::serialize_into(file, book)?;
    Ok(())
}

/// Load the opening book from the database into the in-memory cache.
/// Called at server startup and by the reload admin endpoint.
pub async fn load_from_db(pool: &PgPool) {
    let rows: Vec<(String, String, i32, i32, i32, i32)> = match sqlx::query_as(
        "SELECT parent_fen, move_san, games, white_wins, draws, black_wins FROM opening_book",
    )
    .fetch_all(pool)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Failed to load opening book from DB: {}", e);
            return;
        }
    };

    let mut book: OpeningBook = HashMap::new();
    for (fen, san, games, white_wins, draws, black_wins) in &rows {
        book.entry(fen.clone()).or_default().insert(
            san.clone(),
            BookMoveStats {
                games: *games,
                white_wins: *white_wins,
                draws: *draws,
                black_wins: *black_wins,
            },
        );
    }

    let positions = book.len();
    let total_moves = rows.len();

    {
        let mut cache = BOOK_CACHE.write().unwrap();
        *cache = book;
    }

    tracing::info!(
        "Loaded opening book from DB: {} positions, {} moves",
        positions,
        total_moves
    );
}

/// Look up a move in the cached book.
/// Returns None if the position or move isn't in the book.
pub fn lookup(fen: &str, move_san: &str) -> Option<BookMoveStats> {
    let normalized = normalize_fen(fen);
    let cache = BOOK_CACHE.read().unwrap();
    cache
        .get(&normalized)
        .and_then(|moves| moves.get(move_san).cloned())
}

/// Check if a move is in the book.
pub fn is_book_move(fen: &str, move_san: &str) -> bool {
    lookup(fen, move_san).is_some()
}

/// Check if the book cache is empty (no positions loaded).
pub fn is_empty() -> bool {
    let cache = BOOK_CACHE.read().unwrap();
    cache.is_empty()
}

/// Strips move counters from FEN, keeping only position + side + castling + ep.
pub fn normalize_fen(fen: &str) -> String {
    fen.split_whitespace().take(4).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_fen() {
        let fen = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";
        let normalized = normalize_fen(fen);
        assert_eq!(normalized, "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3");
    }
}
