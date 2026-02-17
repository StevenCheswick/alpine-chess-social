//! Build opening book from PGN files.
//!
//! Parses KingBase PGN files and outputs a bincode-serialized opening book.
//!
//! Usage: cargo run --release --bin build-book -- <pgn_dir> [--min-games 100] [--max-ply 60]
//!
//! Example:
//!   cargo run --release --bin build-book -- ../../../feature-testing/opening-book-test/

use pgn_reader::{Reader, RawTag, SanPlus, Visitor};
use server::book_cache::{BookMoveStats, OpeningBook, BOOK_FILE_PATH};
use shakmaty::{fen::Fen, CastlingMode, Chess, EnPassantMode, Position};
use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::BufReader;
use std::ops::ControlFlow;
use std::path::Path;
use std::time::Instant;

const DEFAULT_MIN_ELO: u16 = 2500;
const DEFAULT_MIN_GAMES: i32 = 100;
const DEFAULT_MAX_PLY: usize = 60;

/// Stats for a position+move during collection (mutable).
#[derive(Default)]
struct MoveStats {
    games: i32,
    white_wins: i32,
    draws: i32,
    black_wins: i32,
}

/// Tags collected during header parsing.
struct GameTags {
    white_elo: u16,
    black_elo: u16,
    result: i8, // 0=white, 1=draw, 2=black, -1=unknown
}

impl Default for GameTags {
    fn default() -> Self {
        Self {
            white_elo: 0,
            black_elo: 0,
            result: -1,
        }
    }
}

/// State during movetext parsing.
struct GameState {
    board: Chess,
    result: i8,
    ply: usize,
}

/// Visitor that collects opening moves from games.
struct BookBuilder {
    /// The global tree being built.
    tree: HashMap<String, HashMap<String, MoveStats>>,
    /// Max ply to record.
    max_ply: usize,
    /// Min Elo for both players.
    min_elo: u16,
    /// Count of games used.
    used_games: u64,
}

impl BookBuilder {
    fn new(max_ply: usize, min_elo: u16) -> Self {
        Self {
            tree: HashMap::new(),
            max_ply,
            min_elo,
            used_games: 0,
        }
    }

    /// Get normalized FEN (position + side + castling + ep, no move counters).
    fn normalized_fen(board: &Chess) -> String {
        let fen = Fen::from_position(board, EnPassantMode::Legal);
        let fen_str = fen.to_string();
        fen_str.split_whitespace().take(4).collect::<Vec<_>>().join(" ")
    }
}

impl Visitor for BookBuilder {
    type Tags = GameTags;
    type Movetext = GameState;
    type Output = ();

    fn begin_tags(&mut self) -> ControlFlow<(), GameTags> {
        ControlFlow::Continue(GameTags::default())
    }

    fn tag(&mut self, tags: &mut GameTags, name: &[u8], value: RawTag<'_>) -> ControlFlow<()> {
        match name {
            b"WhiteElo" => {
                if let Ok(elo) = value.decode_utf8_lossy().parse::<u16>() {
                    tags.white_elo = elo;
                }
            }
            b"BlackElo" => {
                if let Ok(elo) = value.decode_utf8_lossy().parse::<u16>() {
                    tags.black_elo = elo;
                }
            }
            b"Result" => {
                let val = value.decode_utf8_lossy();
                tags.result = match val.as_ref() {
                    "1-0" => 0,
                    "1/2-1/2" => 1,
                    "0-1" => 2,
                    _ => -1,
                };
            }
            _ => {}
        }
        ControlFlow::Continue(())
    }

    fn begin_movetext(&mut self, tags: GameTags) -> ControlFlow<(), GameState> {
        // Skip if either player below min elo or unknown result
        if tags.white_elo < self.min_elo || tags.black_elo < self.min_elo || tags.result < 0 {
            return ControlFlow::Break(());
        }

        self.used_games += 1;

        ControlFlow::Continue(GameState {
            board: Chess::default(),
            result: tags.result,
            ply: 0,
        })
    }

    fn san(&mut self, state: &mut GameState, san_plus: SanPlus) -> ControlFlow<()> {
        if state.ply >= self.max_ply {
            return ControlFlow::Continue(());
        }

        // Get FEN before the move
        let fen_before = Self::normalized_fen(&state.board);

        // Get SAN string
        let san_str = san_plus.san.to_string();

        // Try to make the move
        if let Ok(mv) = san_plus.san.to_move(&state.board) {
            // Record this position + move
            let position_entry = self.tree.entry(fen_before).or_default();
            let move_entry = position_entry.entry(san_str).or_default();

            move_entry.games += 1;
            match state.result {
                0 => move_entry.white_wins += 1,
                1 => move_entry.draws += 1,
                2 => move_entry.black_wins += 1,
                _ => {}
            }

            // Apply the move
            state.board = state.board.clone().play(mv).unwrap_or_else(|_| state.board.clone());
            state.ply += 1;
        }

        ControlFlow::Continue(())
    }

    fn end_game(&mut self, _state: GameState) {}
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <pgn_dir> [--min-games N] [--max-ply N] [--min-elo N]", args[0]);
        eprintln!();
        eprintln!("Example:");
        eprintln!("  cargo run --release --bin build-book -- ../../../feature-testing/opening-book-test/");
        std::process::exit(1);
    }

    let pgn_dir = &args[1];

    // Parse optional args
    let mut min_games = DEFAULT_MIN_GAMES;
    let mut max_ply = DEFAULT_MAX_PLY;
    let mut min_elo = DEFAULT_MIN_ELO;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--min-games" => {
                min_games = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(DEFAULT_MIN_GAMES);
                i += 2;
            }
            "--max-ply" => {
                max_ply = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(DEFAULT_MAX_PLY);
                i += 2;
            }
            "--min-elo" => {
                min_elo = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(DEFAULT_MIN_ELO);
                i += 2;
            }
            _ => i += 1,
        }
    }

    println!("Building opening book:");
    println!("  PGN directory: {}", pgn_dir);
    println!("  Min Elo: {}", min_elo);
    println!("  Max ply: {} ({} moves per side)", max_ply, max_ply / 2);
    println!("  Min games: {}", min_games);
    println!();

    // Find all PGN files
    let pattern = format!("{}/*.pgn", pgn_dir);
    let pgn_files: Vec<_> = glob::glob(&pattern)?
        .filter_map(|p| p.ok())
        .collect();

    if pgn_files.is_empty() {
        eprintln!("No PGN files found in {}", pgn_dir);
        std::process::exit(1);
    }

    println!("Found {} PGN files", pgn_files.len());

    // Build the tree
    let mut builder = BookBuilder::new(max_ply, min_elo);
    let mut total_games = 0u64;
    let start = Instant::now();

    for pgn_path in &pgn_files {
        println!("Processing {}...", pgn_path.display());

        let file = File::open(pgn_path)?;
        let reader = BufReader::new(file);
        let mut pgn_reader = Reader::new(reader);

        let file_start_games = total_games;
        while pgn_reader.read_game(&mut builder)?.is_some() {
            total_games += 1;

            if total_games % 50_000 == 0 {
                let elapsed = start.elapsed().as_secs();
                let rate = if elapsed > 0 { total_games / elapsed } else { 0 };
                println!(
                    "  {:>8} games scanned, {:>8} used, {:>8} positions ({:>3}s, {} games/s)",
                    total_games, builder.used_games, builder.tree.len(), elapsed, rate
                );
            }
        }

        let file_games = total_games - file_start_games;
        println!("  Processed {} games from {}", file_games, pgn_path.file_name().unwrap().to_string_lossy());
    }

    let elapsed = start.elapsed();
    println!();
    println!("Parsing complete in {:.1}s", elapsed.as_secs_f64());
    println!("  Total scanned: {}", total_games);
    println!("  Used ({}+ Elo): {}", min_elo, builder.used_games);
    println!("  Unique positions: {}", builder.tree.len());

    // Filter to min_games and convert to OpeningBook format
    println!();
    println!("Filtering to {}+ games...", min_games);

    let mut book: OpeningBook = HashMap::new();
    let mut total_moves = 0usize;

    for (fen, moves) in builder.tree {
        let filtered_moves: HashMap<String, BookMoveStats> = moves
            .into_iter()
            .filter(|(_, stats)| stats.games >= min_games)
            .map(|(san, stats)| {
                (san, BookMoveStats {
                    games: stats.games,
                    white_wins: stats.white_wins,
                    draws: stats.draws,
                    black_wins: stats.black_wins,
                })
            })
            .collect();

        if !filtered_moves.is_empty() {
            total_moves += filtered_moves.len();
            book.insert(fen, filtered_moves);
        }
    }

    println!("After filtering:");
    println!("  Positions: {}", book.len());
    println!("  Moves: {}", total_moves);

    // Ensure output directory exists
    let book_path = Path::new(BOOK_FILE_PATH);
    if let Some(parent) = book_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Serialize and write
    println!();
    println!("Writing to {}...", BOOK_FILE_PATH);
    let file = File::create(BOOK_FILE_PATH)?;
    bincode::serialize_into(&file, &book)?;

    let file_size = fs::metadata(BOOK_FILE_PATH)?.len();

    println!();
    println!("Done!");
    println!("  Output: {}", BOOK_FILE_PATH);
    println!("  Size: {} KB", file_size / 1024);
    println!("  Positions: {}", book.len());
    println!("  Moves: {}", total_moves);

    // Analyze most one-sided positions
    println!();
    println!("=== Most One-Sided Positions ===");
    println!();

    let mut all_moves: Vec<(&String, &String, f64, i32, &BookMoveStats)> = Vec::new();

    for (fen, moves) in &book {
        for (san, stats) in moves {
            let total = stats.white_wins + stats.draws + stats.black_wins;
            if total >= 100 {
                let white_pct = stats.white_wins as f64 / total as f64 * 100.0;
                let black_pct = stats.black_wins as f64 / total as f64 * 100.0;
                let most_sided = white_pct.max(black_pct);
                all_moves.push((fen, san, most_sided, total, stats));
            }
        }
    }

    all_moves.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());

    // Build a reverse lookup: result_fen -> (parent_fen, move_san)
    // This lets us trace back from any position to the start
    let mut parent_map: HashMap<String, (String, String)> = HashMap::new();
    for (parent_fen, moves) in &book {
        for (san, _) in moves {
            // Replay to get result FEN
            let fen_str = format!("{} 0 1", parent_fen); // Add move counters
            if let Ok(fen) = fen_str.parse::<Fen>() {
                if let Ok(pos) = fen.into_position::<Chess>(shakmaty::CastlingMode::Standard) {
                    if let Ok(san_parsed) = san.parse::<shakmaty::san::San>() {
                        if let Ok(mv) = san_parsed.to_move(&pos) {
                            if let Ok(new_pos) = pos.play(mv) {
                                let result_fen = BookBuilder::normalized_fen(&new_pos);
                                // Only store if not already present (prefer shorter paths)
                                parent_map.entry(result_fen).or_insert((parent_fen.clone(), san.clone()));
                            }
                        }
                    }
                }
            }
        }
    }

    // Function to trace back to starting position
    let starting_fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq -";

    let trace_path = |target_fen: &str| -> Vec<String> {
        let mut path = Vec::new();
        let mut current = target_fen.to_string();

        while current != starting_fen {
            if let Some((parent, mv)) = parent_map.get(&current) {
                path.push(mv.clone());
                current = parent.clone();
            } else {
                break;
            }
            if path.len() > 60 {
                break; // Safety limit
            }
        }
        path.reverse();
        path
    };

    // Find deepest lines - only count paths that actually start from the initial position
    println!("=== Deepest Lines ===\n");

    let mut depths: Vec<(String, Vec<String>)> = Vec::new();
    for (fen, _) in &book {
        let path = trace_path(fen);
        // Only count if we actually reached the starting position (path is valid)
        if !path.is_empty() {
            // Verify by replaying the path
            let mut pos = Chess::default();
            let mut valid = true;
            for mv_san in &path {
                if let Ok(san) = mv_san.parse::<shakmaty::san::San>() {
                    if let Ok(mv) = san.to_move(&pos) {
                        if let Ok(new_pos) = pos.clone().play(mv) {
                            pos = new_pos;
                        } else { valid = false; break; }
                    } else { valid = false; break; }
                } else { valid = false; break; }
            }
            if valid && path.len() >= 20 {  // Only show lines >= 10 full moves
                depths.push((fen.clone(), path));
            }
        }
    }
    depths.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

    for (i, (fen, path)) in depths.iter().take(5).enumerate() {
        let mut pgn = String::new();
        for (j, mv) in path.iter().enumerate() {
            if j % 2 == 0 {
                if !pgn.is_empty() { pgn.push(' '); }
                pgn.push_str(&format!("{}.", j / 2 + 1));
            }
            pgn.push(' ');
            pgn.push_str(mv);
        }

        // Get available moves at this position
        let moves_at_pos = book.get(fen).map(|m| m.len()).unwrap_or(0);

        println!("{}. Depth: {} ply ({} moves), {} continuations", i + 1, path.len(), path.len() / 2, moves_at_pos);
        println!("   {}", pgn);
        println!();
    }

    println!("=== Most One-Sided Positions ===\n");

    for (i, (fen, san, _, total, stats)) in all_moves.iter().take(10).enumerate() {
        let w = stats.white_wins as f64 / *total as f64 * 100.0;
        let d = stats.draws as f64 / *total as f64 * 100.0;
        let b = stats.black_wins as f64 / *total as f64 * 100.0;

        // Get path to this position
        let path = trace_path(fen);

        // Format as PGN
        let mut pgn = String::new();
        for (j, mv) in path.iter().enumerate() {
            if j % 2 == 0 {
                if !pgn.is_empty() { pgn.push(' '); }
                pgn.push_str(&format!("{}.", j / 2 + 1));
            }
            pgn.push(' ');
            pgn.push_str(mv);
        }
        // Add the final move
        if !pgn.is_empty() { pgn.push(' '); }
        if path.len() % 2 == 0 {
            pgn.push_str(&format!("{}.", path.len() / 2 + 1));
            pgn.push(' ');
        }
        pgn.push_str(san);

        println!("{}. W:{:.1}% D:{:.1}% B:{:.1}% ({} games)", i + 1, w, d, b, total);
        println!("   {}", pgn);
        println!();
    }

    Ok(())
}
