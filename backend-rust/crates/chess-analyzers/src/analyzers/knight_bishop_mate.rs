use crate::analyzer_trait::{GameAnalyzer, MoveContext};
use chess_core::game_data::GameData;
use shakmaty::{Role, Position};

/// Detects checkmate where both a knight and bishop are involved in the attack
/// (the final position has both a knight and bishop from the user's side
/// participating in the mating pattern).
pub struct KnightBishopMateAnalyzer {
    username: String,
    matched_links: Vec<String>,
    current_link: Option<String>,
    user_is_white: bool,
    matched: bool,
}

impl KnightBishopMateAnalyzer {
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

impl GameAnalyzer for KnightBishopMateAnalyzer {
    fn name(&self) -> &'static str {
        "knight_bishop_mate"
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

        let mut pos_after = ctx.board.clone();
        pos_after.play_unchecked(ctx.mv);

        if !pos_after.is_checkmate() {
            return;
        }

        // Check that the user has both a knight and bishop on the board
        let board = pos_after.board();
        let user_color = ctx.user_color;

        let has_knight = !(board.by_role(Role::Knight) & board.by_color(user_color)).is_empty();
        let has_bishop = !(board.by_role(Role::Bishop) & board.by_color(user_color)).is_empty();

        if has_knight && has_bishop {
            // Verify at least one knight and one bishop are attacking squares
            // around the opponent king (participating in the mate)
            let opponent_color = !user_color;
            if let Some(king_sq) = pos_after.board().king_of(opponent_color) {
                let king_area = shakmaty::attacks::king_attacks(king_sq) | shakmaty::Bitboard::from(king_sq);
                let occupied = board.occupied();

                // Check if any user knight attacks the king or adjacent squares
                let user_knights = board.by_role(Role::Knight) & board.by_color(user_color);
                let mut knight_participates = false;
                for sq in user_knights {
                    let attacks = shakmaty::attacks::knight_attacks(sq);
                    if !(attacks & king_area).is_empty() {
                        knight_participates = true;
                        break;
                    }
                }

                // Check if any user bishop attacks the king or adjacent squares
                let user_bishops = board.by_role(Role::Bishop) & board.by_color(user_color);
                let mut bishop_participates = false;
                for sq in user_bishops {
                    let attacks = shakmaty::attacks::bishop_attacks(sq, occupied);
                    if !(attacks & king_area).is_empty() {
                        bishop_participates = true;
                        break;
                    }
                }

                if knight_participates && bishop_participates {
                    self.matched = true;
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
