use crate::analyzer_trait::{GameAnalyzer, MoveContext};
use chess_core::game_data::GameData;
use shakmaty::{Role, Position, Move, Square};

/// Detects knight forks: user's knight attacks the king AND at least one other
/// high-value piece (queen, rook) simultaneously. Only in wins.
pub struct KnightForkAnalyzer {
    username: String,
    matched_links: Vec<String>,
    current_link: Option<String>,
    user_is_white: bool,
    matched: bool,
}

impl KnightForkAnalyzer {
    pub fn new(username: &str) -> Self {
        Self {
            username: username.to_lowercase(),
            matched_links: Vec::new(),
            current_link: None,
            user_is_white: true,
            matched: false,
        }
    }

    fn knight_attacks(sq: Square) -> shakmaty::Bitboard {
        shakmaty::attacks::knight_attacks(sq)
    }
}

impl GameAnalyzer for KnightForkAnalyzer {
    fn name(&self) -> &'static str {
        "knight_fork"
    }

    fn start_game(&mut self, game_data: &GameData, user_is_white: bool) {
        self.current_link = game_data.metadata.link.clone();
        self.user_is_white = user_is_white;
        self.matched = false;
    }

    fn process_move(&mut self, ctx: &MoveContext) {
        if self.matched || !ctx.is_user_move {
            return;
        }

        // Check if user moved a knight
        let to_sq = match ctx.mv {
            Move::Normal { role: Role::Knight, to, .. } => *to,
            _ => return,
        };

        // Apply the move to get the resulting position
        let mut pos_after = ctx.board.clone();
        pos_after.play_unchecked(ctx.mv);
        let board_after = pos_after.board();

        let opponent_color = !ctx.user_color;
        let attacks = Self::knight_attacks(to_sq);

        // Check if knight attacks the opponent king
        let attacks_king = if let Some(king_sq) = pos_after.board().king_of(opponent_color) {
            attacks.contains(king_sq)
        } else {
            false
        };

        if !attacks_king {
            return;
        }

        // Check if knight also attacks another high-value piece (queen or rook)
        for sq in attacks {
            if let Some(piece) = board_after.piece_at(sq) {
                if piece.color == opponent_color
                    && (piece.role == Role::Queen || piece.role == Role::Rook)
                {
                    self.matched = true;
                    return;
                }
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
