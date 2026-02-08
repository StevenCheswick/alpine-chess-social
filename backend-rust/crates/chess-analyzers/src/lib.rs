//! Chess game pattern analyzers.
//!
//! Ports of the 21 Python analyzers. Each analyzer processes games move-by-move
//! using shakmaty for board state, and reports which games match each pattern.
//! The main entry point is `analyze_batch()` which runs all analyzers.

pub mod analyzer_trait;
pub mod unified;
pub mod analyzers;

use chess_core::game_data::GameData;
use std::collections::HashMap;

/// Tag names for each analyzer.
pub const ANALYZER_TAGS: &[(&str, &str)] = &[
    ("queen_sacrifice", "Queen Sacrifice"),
    ("knight_fork", "Knight Fork"),
    ("rook_sacrifice", "Rook Sacrifice"),
    ("back_rank_mate", "Back Rank Mate"),
    ("smothered_mate", "Smothered Mate"),
    ("king_mate", "King Mate"),
    ("castle_mate", "Castle Mate"),
    ("pawn_mate", "Pawn Mate"),
    ("knight_promotion_mate", "Knight Promotion Mate"),
    ("promotion_mate", "Promotion Mate"),
    ("quickest_mate", "Quickest Mate"),
    ("en_passant_mate", "En Passant Mate"),
    ("knight_bishop_mate", "Knight Bishop Mate"),
    ("king_walk", "King Walk"),
    ("biggest_comeback", "Biggest Comeback"),
    ("clutch_win", "Clutch Win"),
    ("best_game", "Best Game"),
    ("longest_game", "Longest Game"),
    ("hung_queen", "Hung Queen"),
    ("capture_sequence", "Capture Sequence"),
    ("stalemate", "Stalemate"),
    ("windmill", "Windmill"),
];

/// Analyze a batch of games and return a map of game_link -> Vec<tag_name>.
/// This is the main entry point used by the server during sync.
pub fn analyze_batch(
    username: &str,
    games: &[GameData],
) -> HashMap<String, Vec<String>> {
    unified::analyze_games(username, games)
}
