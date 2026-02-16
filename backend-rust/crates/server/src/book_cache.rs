//! In-memory opening book cache.
//!
//! The book is loaded from a binary file at startup for instant lookups.
//! Use `cargo run --bin export-book` to generate the binary from PostgreSQL.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::LazyLock;

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

/// Global in-memory book cache, loaded at first access.
pub static BOOK_CACHE: LazyLock<OpeningBook> = LazyLock::new(|| {
    match load_book(BOOK_FILE_PATH) {
        Ok(book) => {
            let total_moves: usize = book.values().map(|m| m.len()).sum();
            tracing::info!(
                "Loaded opening book: {} positions, {} moves",
                book.len(),
                total_moves
            );
            book
        }
        Err(e) => {
            tracing::warn!("Failed to load opening book from {}: {}", BOOK_FILE_PATH, e);
            tracing::warn!("Book lookups will fall back to database");
            HashMap::new()
        }
    }
});

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

/// Look up a move in the cached book.
/// Returns None if the position or move isn't in the book.
pub fn lookup(fen: &str, move_san: &str) -> Option<&'static BookMoveStats> {
    let normalized = normalize_fen(fen);
    BOOK_CACHE
        .get(&normalized)
        .and_then(|moves| moves.get(move_san))
}

/// Check if a move is in the book.
pub fn is_book_move(fen: &str, move_san: &str) -> bool {
    lookup(fen, move_san).is_some()
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
