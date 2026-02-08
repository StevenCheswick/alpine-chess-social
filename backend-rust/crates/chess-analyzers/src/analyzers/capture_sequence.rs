use crate::analyzer_trait::{GameAnalyzer, MoveContext};
use chess_core::game_data::GameData;
use shakmaty::{Color, Role, Position, Move};

/// Detects when user makes 3+ consecutive captures.
pub struct CaptureSequenceAnalyzer {
    username: String,
    matched_links: Vec<String>,
    current_link: Option<String>,
    user_is_white: bool,
    matched: bool,
    consecutive_captures: usize,
}

impl CaptureSequenceAnalyzer {
    pub fn new(username: &str) -> Self {
        Self {
            username: username.to_lowercase(),
            matched_links: Vec::new(),
            current_link: None,
            user_is_white: true,
            matched: false,
            consecutive_captures: 0,
        }
    }

    fn is_capture(mv: &Move) -> bool {
        match mv {
            Move::Normal { capture: Some(_), .. } => true,
            Move::EnPassant { .. } => true,
            _ => false,
        }
    }
}

impl GameAnalyzer for CaptureSequenceAnalyzer {
    fn name(&self) -> &'static str {
        "capture_sequence"
    }

    fn start_game(&mut self, game_data: &GameData, user_is_white: bool) {
        self.current_link = game_data.metadata.link.clone();
        self.user_is_white = user_is_white;
        self.matched = false;
        self.consecutive_captures = 0;
    }

    fn process_move(&mut self, ctx: &MoveContext) {
        if self.matched {
            return;
        }

        if ctx.is_user_move {
            if Self::is_capture(ctx.mv) {
                self.consecutive_captures += 1;
                if self.consecutive_captures >= 3 {
                    self.matched = true;
                }
            } else {
                self.consecutive_captures = 0;
            }
        }
        // Opponent moves don't reset the counter - we only count user's consecutive captures
    }

    fn finish_game(&mut self) -> bool {
        if self.matched {
            if let Some(ref link) = self.current_link {
                self.matched_links.push(link.clone());
            }
        }
        self.matched
    }

    fn matched_game_links(&self) -> Vec<String> {
        self.matched_links.clone()
    }
}
