use crate::analyzer_trait::{GameAnalyzer, MoveContext};
use chess_core::game_data::GameData;
use shakmaty;

/// Placeholder/composite analyzer: marks all games where user won.
pub struct BestGameAnalyzer {
    username: String,
    matched_links: Vec<String>,
    current_link: Option<String>,
    user_is_white: bool,
    result: String,
}

impl BestGameAnalyzer {
    pub fn new(username: &str) -> Self {
        Self {
            username: username.to_lowercase(),
            matched_links: Vec::new(),
            current_link: None,
            user_is_white: true,
            result: String::new(),
        }
    }
}

impl GameAnalyzer for BestGameAnalyzer {
    fn name(&self) -> &'static str {
        "best_game"
    }

    fn start_game(&mut self, game_data: &GameData, user_is_white: bool) {
        self.current_link = game_data.metadata.link.clone();
        self.user_is_white = user_is_white;
        self.result = game_data.metadata.result.clone();
    }

    fn process_move(&mut self, _ctx: &MoveContext) {
        // No per-move processing needed
    }

    fn finish_game(&mut self) -> bool {
        let user_won = (self.result == "1-0" && self.user_is_white)
            || (self.result == "0-1" && !self.user_is_white);

        if user_won {
            if let Some(ref link) = self.current_link {
                self.matched_links.push(link.clone());
            }
        }
        user_won
    }

    fn matched_game_links(&self) -> Vec<String> {
        self.matched_links.clone()
    }
}
