//! Export the opening book from PostgreSQL to a binary file.
//!
//! Usage: cargo run --bin export-book
//!
//! Requires DATABASE_URL environment variable to be set.

use server::book_cache::{BookMoveStats, OpeningBook, BOOK_FILE_PATH};
use sqlx::postgres::PgPoolOptions;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    println!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await?;

    println!("Querying opening_book table...");
    let rows: Vec<(String, String, i32, i32, i32, i32)> = sqlx::query_as(
        "SELECT parent_fen, move_san, games, white_wins, draws, black_wins FROM opening_book"
    )
    .fetch_all(&pool)
    .await?;

    println!("Found {} rows", rows.len());

    // Build the nested HashMap
    let mut book: OpeningBook = HashMap::new();
    for (parent_fen, move_san, games, white_wins, draws, black_wins) in rows {
        let stats = BookMoveStats {
            games,
            white_wins,
            draws,
            black_wins,
        };
        book.entry(parent_fen)
            .or_insert_with(HashMap::new)
            .insert(move_san, stats);
    }

    let total_positions = book.len();
    let total_moves: usize = book.values().map(|m| m.len()).sum();

    // Ensure data directory exists
    let book_path = Path::new(BOOK_FILE_PATH);
    if let Some(parent) = book_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Serialize and write
    println!("Writing to {}...", BOOK_FILE_PATH);
    let file = fs::File::create(BOOK_FILE_PATH)?;
    bincode::serialize_into(&file, &book)?;

    let file_size = fs::metadata(BOOK_FILE_PATH)?.len();

    println!();
    println!("Export complete!");
    println!("  Positions: {}", total_positions);
    println!("  Moves:     {}", total_moves);
    println!("  File size: {} KB", file_size / 1024);
    println!("  Path:      {}", BOOK_FILE_PATH);

    Ok(())
}
