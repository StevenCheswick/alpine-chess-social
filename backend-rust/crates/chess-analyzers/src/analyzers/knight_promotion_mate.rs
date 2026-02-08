use crate::analyzer_trait::{GameAnalyzer, MoveContext};
use chess_core::game_data::GameData;
use shakmaty::{Role, Position, Move};

/// Detects knight promotion mate: checkmate delivered by a pawn promoting to
/// a knight (the promotion move itself is checkmate).
pub struct KnightPromotionMateAnalyzer {
    username: String,
    matched_links: Vec<String>,
    current_link: Option<String>,
    user_is_white: bool,
    matched: bool,
}

impl KnightPromotionMateAnalyzer {
    pub fn new(username: &str) -> Self {
        Self {
            username: username.to_lowercase(),
            matched_links: Vec::new(),
            current_link: None,
            user_is_white: true,
            matched: false,
        }
    }
}

impl GameAnalyzer for KnightPromotionMateAnalyzer {
    fn name(&self) -> &'static str {
        "knight_promotion_mate"
    }

    fn start_game(&mut self, game_data: &GameData, user_is_white: bool) {
        self.current_link = game_data.metadata.link.clone();
        self.user_is_white = user_is_white;
        self.matched = false;
    }

    fn process_move(&mut self, ctx: &MoveContext) {
        if !ctx.is_user_move || self.matched {
            return;
        }

        // Must be a pawn promoting to a knight
        match ctx.mv {
            Move::Normal { role: Role::Pawn, promotion: Some(Role::Knight), .. } => {}
            _ => return,
        }

        let mut pos_after = ctx.board.clone();
        pos_after.play_unchecked(ctx.mv);

        if pos_after.is_checkmate() {
            self.matched = true;
        }
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
