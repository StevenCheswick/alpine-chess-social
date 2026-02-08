use crate::analyzer_trait::{GameAnalyzer, MoveContext};
use chess_core::game_data::GameData;
use shakmaty;

/// Detects clutch win: user wins in a time scramble. We approximate this by
/// checking if the game was a long game (many moves) and user won on time or
/// had very low time. Since we don't have clock data in MoveContext, we
/// heuristically detect time scrambles from the game metadata and move count.
pub struct ClutchWinAnalyzer {
    username: String,
    matched_links: Vec<String>,
    current_link: Option<String>,
    user_is_white: bool,
    result: String,
    time_control: Option<String>,
    total_moves: usize,
}

impl ClutchWinAnalyzer {
    pub fn new(username: &str) -> Self {
        Self {
            username: username.to_lowercase(),
            matched_links: Vec::new(),
            current_link: None,
            user_is_white: true,
            result: String::new(),
            time_control: None,
            total_moves: 0,
        }
    }

    /// Parse time control "base+inc" and get base time in seconds
    fn parse_base_time(tc: &str) -> Option<f64> {
        tc.split('+').next()?.parse::<f64>().ok()
    }

    /// Estimate expected moves for a time control
    fn expected_moves(base_time: f64) -> usize {
        if base_time <= 180.0 {
            60 // bullet/blitz: ~30 full moves
        } else if base_time <= 600.0 {
            80 // rapid: ~40 full moves
        } else {
            100 // classical: ~50 full moves
        }
    }
}

impl GameAnalyzer for ClutchWinAnalyzer {
    fn name(&self) -> &'static str {
        "clutch_win"
    }

    fn start_game(&mut self, game_data: &GameData, user_is_white: bool) {
        self.current_link = game_data.metadata.link.clone();
        self.user_is_white = user_is_white;
        self.result = game_data.metadata.result.clone();
        self.time_control = game_data.metadata.time_control.clone();
        self.total_moves = game_data.moves.len();
    }

    fn process_move(&mut self, _ctx: &MoveContext) {
        // No per-move processing needed
    }

    fn finish_game(&mut self) -> bool {
        let user_won = (self.result == "1-0" && self.user_is_white)
            || (self.result == "0-1" && !self.user_is_white);

        if !user_won {
            return false;
        }

        // Check if this was a long game relative to the time control (time scramble)
        let is_clutch = if let Some(ref tc) = self.time_control {
            if let Some(base) = Self::parse_base_time(tc) {
                let expected = Self::expected_moves(base);
                // Game went significantly longer than expected - likely a time scramble
                self.total_moves > expected
            } else {
                false
            }
        } else {
            false
        };

        if is_clutch {
            if let Some(ref link) = self.current_link {
                self.matched_links.push(link.clone());
            }
        }
        is_clutch
    }

    fn matched_game_links(&self) -> Vec<String> {
        self.matched_links.clone()
    }
}
