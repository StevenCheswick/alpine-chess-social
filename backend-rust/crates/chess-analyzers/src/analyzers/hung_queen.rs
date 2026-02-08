use crate::analyzer_trait::{GameAnalyzer, MoveContext};
use chess_core::game_data::GameData;
use shakmaty::{Color, Role, Position, Move, Square};

/// Detects when the opponent hangs their queen (queen can be captured for free -
/// not protected). Checks if user can capture an undefended opponent queen.
pub struct HungQueenAnalyzer {
    username: String,
    matched_links: Vec<String>,
    current_link: Option<String>,
    user_is_white: bool,
    matched: bool,
}

impl HungQueenAnalyzer {
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

impl GameAnalyzer for HungQueenAnalyzer {
    fn name(&self) -> &'static str {
        "hung_queen"
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

        // Check if user captures the opponent's queen
        let captures_queen = match ctx.mv {
            Move::Normal { capture: Some(Role::Queen), .. } => true,
            _ => false,
        };

        if !captures_queen {
            return;
        }

        // The queen is on the target square before the move
        let target_sq = match ctx.mv {
            Move::Normal { to, .. } => *to,
            _ => return,
        };

        // Check if the opponent's queen on that square was defended
        // We look at whether any opponent piece defends that square
        let board = ctx.board.board();
        let opponent_color = !ctx.user_color;
        let occupied = board.occupied();

        // Get all attacks to the target square from opponent pieces
        // If no opponent piece (other than the queen itself) attacks this square,
        // then the queen was hung (undefended)
        let defenders = board.attacks_to(target_sq, opponent_color, occupied);
        // Remove the queen itself from defenders (it can't defend itself)
        let defenders_without_queen = defenders & !shakmaty::Bitboard::from(target_sq);

        if defenders_without_queen.is_empty() {
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
