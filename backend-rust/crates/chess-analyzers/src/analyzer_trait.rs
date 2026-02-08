//! Base trait and types for game analyzers.

use chess_core::game_data::GameData;
use shakmaty::{Chess, Color, Move};

/// Context available to analyzers at each move.
pub struct MoveContext<'a> {
    pub mv: &'a Move,
    pub move_number: usize, // 1-indexed half-move number
    pub board: &'a Chess,   // Board state BEFORE the move
    pub is_user_move: bool,
    pub is_opponent_move: bool,
    pub user_color: Color,
    pub game_data: &'a GameData,
}

/// Trait that all analyzers implement.
pub trait GameAnalyzer {
    /// Name used to look up the display tag.
    fn name(&self) -> &'static str;

    /// Initialize state for a new game.
    fn start_game(&mut self, game_data: &GameData, user_is_white: bool);

    /// Process a single move.
    fn process_move(&mut self, ctx: &MoveContext);

    /// Finalize analysis after all moves. Returns true if this game matched.
    fn finish_game(&mut self) -> bool;

    /// Get game links that matched this pattern across all games processed.
    fn matched_game_links(&self) -> Vec<String>;
}
