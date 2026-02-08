use crate::analyzer_trait::{GameAnalyzer, MoveContext};
use chess_core::game_data::GameData;
use shakmaty::{Color, Position};

/// Detects games that end in stalemate.
pub struct StalemateAnalyzer {
    username: String,
    matched_links: Vec<String>,
    current_link: Option<String>,
    user_is_white: bool,
    matched: bool,
    last_position: Option<shakmaty::Chess>,
}

impl StalemateAnalyzer {
    pub fn new(username: &str) -> Self {
        Self {
            username: username.to_lowercase(),
            matched_links: Vec::new(),
            current_link: None,
            user_is_white: true,
            matched: false,
            last_position: None,
        }
    }
}

impl GameAnalyzer for StalemateAnalyzer {
    fn name(&self) -> &'static str {
        "stalemate"
    }

    fn start_game(&mut self, game_data: &GameData, user_is_white: bool) {
        self.current_link = game_data.metadata.link.clone();
        self.user_is_white = user_is_white;
        self.matched = false;
        self.last_position = None;
    }

    fn process_move(&mut self, ctx: &MoveContext) {
        // Apply the move and keep track of the final position
        let mut pos_after = ctx.board.clone();
        pos_after.play_unchecked(ctx.mv);
        self.last_position = Some(pos_after);
    }

    fn finish_game(&mut self) -> bool {
        if let Some(ref pos) = self.last_position {
            if pos.is_stalemate() {
                self.matched = true;
            }
        }

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
