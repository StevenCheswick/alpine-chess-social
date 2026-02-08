use crate::analyzer_trait::{GameAnalyzer, MoveContext};
use chess_core::game_data::GameData;
use shakmaty::{Role, Move};

/// Detects rook sacrifices: user gives up a rook (captured by opponent)
/// without immediate recapture of equal or higher value material. Only in wins.
pub struct RookSacrificeAnalyzer {
    username: String,
    matched_links: Vec<String>,
    current_link: Option<String>,
    user_is_white: bool,
    matched: bool,
    rook_lost_move: Option<usize>,
}

impl RookSacrificeAnalyzer {
    pub fn new(username: &str) -> Self {
        Self {
            username: username.to_lowercase(),
            matched_links: Vec::new(),
            current_link: None,
            user_is_white: true,
            matched: false,
            rook_lost_move: None,
        }
    }

    fn piece_value(role: Role) -> i32 {
        match role {
            Role::Pawn => 1,
            Role::Knight => 3,
            Role::Bishop => 3,
            Role::Rook => 5,
            Role::Queen => 9,
            Role::King => 0,
        }
    }
}

impl GameAnalyzer for RookSacrificeAnalyzer {
    fn name(&self) -> &'static str {
        "rook_sacrifice"
    }

    fn start_game(&mut self, game_data: &GameData, user_is_white: bool) {
        self.current_link = game_data.metadata.link.clone();
        self.user_is_white = user_is_white;
        self.matched = false;
        self.rook_lost_move = None;
    }

    fn process_move(&mut self, ctx: &MoveContext) {
        if self.matched {
            return;
        }

        // Opponent captures user's rook
        if ctx.is_opponent_move {
            if let Move::Normal { capture: Some(Role::Rook), .. } = ctx.mv {
                self.rook_lost_move = Some(ctx.move_number);
            }
        }

        // User's next move after losing rook
        if ctx.is_user_move {
            if let Some(lost_move) = self.rook_lost_move {
                if ctx.move_number == lost_move + 1 {
                    // Check if user recaptures equal or higher value
                    let recaptured_value = match ctx.mv {
                        Move::Normal { capture: Some(role), .. } => Self::piece_value(*role),
                        _ => 0,
                    };
                    if recaptured_value < 5 {
                        // Didn't recapture equal/higher value - it's a sacrifice
                        self.matched = true;
                    }
                    self.rook_lost_move = None;
                }
            }
        }
    }

    fn finish_game(&mut self) -> bool {
        // If rook was lost on the last move and never recaptured
        if self.rook_lost_move.is_some() && !self.matched {
            self.matched = true;
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
