use crate::analyzer_trait::{GameAnalyzer, MoveContext};
use chess_core::game_data::GameData;
use shakmaty::{Role, Position, Move, Square};

/// Detects smothered mate: knight delivers checkmate where the losing king
/// is completely surrounded by its own pieces (all adjacent squares occupied
/// by friendly pieces).
pub struct SmotheredMateAnalyzer {
    username: String,
    matched_links: Vec<String>,
    current_link: Option<String>,
    user_is_white: bool,
    matched: bool,
}

impl SmotheredMateAnalyzer {
    pub fn new(username: &str) -> Self {
        Self {
            username: username.to_lowercase(),
            matched_links: Vec::new(),
            current_link: None,
            user_is_white: true,
            matched: false,
        }
    }

    fn adjacent_squares(sq: Square) -> Vec<Square> {
        shakmaty::attacks::king_attacks(sq).into_iter().collect()
    }
}

impl GameAnalyzer for SmotheredMateAnalyzer {
    fn name(&self) -> &'static str {
        "smothered_mate"
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

        // Must be a knight move
        match ctx.mv {
            Move::Normal { role: Role::Knight, .. } => {}
            _ => return,
        }

        let mut pos_after = ctx.board.clone();
        pos_after.play_unchecked(ctx.mv);

        if !pos_after.is_checkmate() {
            return;
        }

        // Check that the opponent king is surrounded by friendly pieces
        let opponent_color = !ctx.user_color;
        if let Some(king_sq) = pos_after.board().king_of(opponent_color) {
            let adj = Self::adjacent_squares(king_sq);
            let board = pos_after.board();
            let all_smothered = adj.iter().all(|&sq| {
                if let Some(piece) = board.piece_at(sq) {
                    piece.color == opponent_color
                } else {
                    false
                }
            });
            if all_smothered && !adj.is_empty() {
                self.matched = true;
            }
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
