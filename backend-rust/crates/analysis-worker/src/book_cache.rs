//! In-memory opening book cache for the analysis worker.
//!
//! Loads the same binary format as the server's book_cache module.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::LazyLock;

/// Stats for a single book move (must match server's BookMoveStats).
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
            tracing::warn!("Book move detection will be disabled");
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

/// Check if a move is in the book.
/// Returns true if the (fen, move_san) pair exists in the opening book.
pub fn is_book_move(fen: &str, move_san: &str) -> bool {
    if BOOK_CACHE.is_empty() {
        return false;
    }
    let normalized = normalize_fen(fen);
    BOOK_CACHE
        .get(&normalized)
        .map(|moves| moves.contains_key(move_san))
        .unwrap_or(false)
}

/// Strips move counters from FEN, keeping only position + side + castling + ep.
pub fn normalize_fen(fen: &str) -> String {
    fen.split_whitespace().take(4).collect::<Vec<_>>().join(" ")
}
