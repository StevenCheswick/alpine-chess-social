use crate::analyzer_trait::{GameAnalyzer, MoveContext};
use chess_core::game_data::GameData;
use shakmaty::{Color, Role, Square, Position, Move};

/// Detects windmill pattern: alternating discovered check. A piece moves to
/// give check, then returns (or another piece does) to give another discovered
/// check, repeated 2+ times. We track discovered checks by the user and look
/// for a pattern of 2+ alternating discovered checks.
pub struct WindmillAnalyzer {
    username: String,
    matched_links: Vec<String>,
    current_link: Option<String>,
    user_is_white: bool,
    matched: bool,
    // Track consecutive discovered checks by user
    consecutive_discovered_checks: usize,
    last_user_move_was_check: bool,
}

impl WindmillAnalyzer {
    pub fn new(username: &str) -> Self {
        Self {
            username: username.to_lowercase(),
            matched_links: Vec::new(),
            current_link: None,
            user_is_white: true,
            matched: false,
            consecutive_discovered_checks: 0,
            last_user_move_was_check: false,
        }
    }

    /// Check if the move gives a discovered check (the moving piece is not the
    /// one delivering the check).
    fn is_discovered_check(board: &shakmaty::Chess, mv: &Move, user_color: Color) -> bool {
        let mut pos_after = board.clone();
        pos_after.play_unchecked(mv);

        // Check if the resulting position has the opponent in check
        if !pos_after.is_check() {
            return false;
        }

        // Get the square the piece moved to
        let to_sq = match mv {
            Move::Normal { to, .. } => *to,
            Move::EnPassant { to, .. } => *to,
            Move::Castle { .. } | Move::Put { .. } => return false,
        };

        let moving_role = match mv {
            Move::Normal { role, promotion, .. } => promotion.unwrap_or(*role),
            Move::EnPassant { .. } => Role::Pawn,
            _ => return false,
        };

        // Check if the piece that moved is directly attacking the opponent king
        let opponent_color = !user_color;
        if let Some(king_sq) = pos_after.board().king_of(opponent_color) {
            let occupied = pos_after.board().occupied();
            // Get attacks from the moved piece's destination
            let attacks_from_piece = match moving_role {
                Role::Pawn => shakmaty::attacks::pawn_attacks(user_color, to_sq),
                Role::Knight => shakmaty::attacks::knight_attacks(to_sq),
                Role::Bishop => shakmaty::attacks::bishop_attacks(to_sq, occupied),
                Role::Rook => shakmaty::attacks::rook_attacks(to_sq, occupied),
                Role::Queen => shakmaty::attacks::queen_attacks(to_sq, occupied),
                Role::King => shakmaty::attacks::king_attacks(to_sq),
            };

            // If the moved piece does NOT attack the king, it's a discovered check
            if !attacks_from_piece.contains(king_sq) {
                return true;
            }
        }

        false
    }
}

impl GameAnalyzer for WindmillAnalyzer {
    fn name(&self) -> &'static str {
        "windmill"
    }

    fn start_game(&mut self, game_data: &GameData, user_is_white: bool) {
        self.current_link = game_data.metadata.link.clone();
        self.user_is_white = user_is_white;
        self.matched = false;
        self.consecutive_discovered_checks = 0;
        self.last_user_move_was_check = false;
    }

    fn process_move(&mut self, ctx: &MoveContext) {
        if self.matched {
            return;
        }

        if ctx.is_user_move {
            let is_disc_check = Self::is_discovered_check(ctx.board, ctx.mv, ctx.user_color);

            if is_disc_check {
                self.consecutive_discovered_checks += 1;
                if self.consecutive_discovered_checks >= 2 {
                    self.matched = true;
                }
            } else {
                // Also count direct checks that alternate with discovered checks
                let mut pos_after = ctx.board.clone();
                pos_after.play_unchecked(ctx.mv);
                if pos_after.is_check() {
                    // It's a direct check - keep the sequence going
                    // but don't increment discovered check counter
                } else {
                    // No check at all - reset
                    self.consecutive_discovered_checks = 0;
                }
            }
        }
        // Opponent moves between user moves are expected (opponent must deal with check)
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
