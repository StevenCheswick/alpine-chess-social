use crate::analyzer_trait::{GameAnalyzer, MoveContext};
use chess_core::game_data::GameData;
use shakmaty::{Color, Rank, Position};

/// Detects back rank mate: checkmate delivered on the opponent's back rank
/// (rank 1 for black's king, rank 8 for white's king). User must deliver.
pub struct BackRankMateAnalyzer {
    username: String,
    matched_links: Vec<String>,
    current_link: Option<String>,
    user_is_white: bool,
    matched: bool,
    last_move_ctx: Option<LastMoveInfo>,
}

struct LastMoveInfo {
    is_user_move: bool,
    user_color: Color,
}

impl BackRankMateAnalyzer {
    pub fn new(username: &str) -> Self {
        Self {
            username: username.to_lowercase(),
            matched_links: Vec::new(),
            current_link: None,
            user_is_white: true,
            matched: false,
            last_move_ctx: None,
        }
    }
}

impl GameAnalyzer for BackRankMateAnalyzer {
    fn name(&self) -> &'static str {
        "back_rank_mate"
    }

    fn start_game(&mut self, game_data: &GameData, user_is_white: bool) {
        self.current_link = game_data.metadata.link.clone();
        self.user_is_white = user_is_white;
        self.matched = false;
        self.last_move_ctx = None;
    }

    fn process_move(&mut self, ctx: &MoveContext) {
        // We need to check if the LAST move results in checkmate on the back rank.
        // Board is BEFORE the move, so we apply it and check.
        if !ctx.is_user_move {
            return;
        }

        let mut pos_after = ctx.board.clone();
        pos_after.play_unchecked(ctx.mv);

        if !pos_after.is_checkmate() {
            return;
        }

        // Check if the opponent's king is on their back rank
        let opponent_color = !ctx.user_color;
        if let Some(king_sq) = pos_after.board().king_of(opponent_color) {
            let back_rank = if opponent_color == Color::White {
                Rank::First
            } else {
                Rank::Eighth
            };
            if king_sq.rank() == back_rank {
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
