use crate::analyzer_trait::{GameAnalyzer, MoveContext};
use chess_core::game_data::GameData;
use shakmaty;

/// Tracks the longest game (most half-moves). Reports the single longest.
pub struct LongestGameAnalyzer {
    username: String,
    matched_links: Vec<String>,
    current_link: Option<String>,
    user_is_white: bool,
    // Track the longest game across all games
    longest_moves: usize,
    longest_link: Option<String>,
    // Current game
    current_move_count: usize,
}

impl LongestGameAnalyzer {
    pub fn new(username: &str) -> Self {
        Self {
            username: username.to_lowercase(),
            matched_links: Vec::new(),
            current_link: None,
            user_is_white: true,
            longest_moves: 0,
            longest_link: None,
            current_move_count: 0,
        }
    }
}

impl GameAnalyzer for LongestGameAnalyzer {
    fn name(&self) -> &'static str {
        "longest_game"
    }

    fn start_game(&mut self, game_data: &GameData, user_is_white: bool) {
        self.current_link = game_data.metadata.link.clone();
        self.user_is_white = user_is_white;
        self.current_move_count = 0;
    }

    fn process_move(&mut self, ctx: &MoveContext) {
        self.current_move_count = ctx.move_number;
    }

    fn finish_game(&mut self) -> bool {
        if self.current_move_count > self.longest_moves {
            self.longest_moves = self.current_move_count;
            self.longest_link = self.current_link.clone();
        }
        false // Only report in matched_game_links
    }

    fn matched_game_links(&self) -> Vec<String> {
        if let Some(ref link) = self.longest_link {
            vec![link.clone()]
        } else {
            Vec::new()
        }
    }
}
