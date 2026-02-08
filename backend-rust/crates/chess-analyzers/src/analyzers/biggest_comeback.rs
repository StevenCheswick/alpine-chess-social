use crate::analyzer_trait::{GameAnalyzer, MoveContext};
use chess_core::game_data::GameData;
use shakmaty::{Color, Role, Position};

/// Detects biggest comeback: user wins by checkmate while having had less
/// material at some point (tracks the biggest material deficit during the game).
/// Reports the single game with the biggest deficit overcome.
pub struct BiggestComebackAnalyzer {
    username: String,
    matched_links: Vec<String>,
    current_link: Option<String>,
    user_is_white: bool,
    // Track the biggest comeback across all games
    biggest_deficit: i32,
    biggest_link: Option<String>,
    // Current game tracking
    current_max_deficit: i32,
    result: String,
}

impl BiggestComebackAnalyzer {
    pub fn new(username: &str) -> Self {
        Self {
            username: username.to_lowercase(),
            matched_links: Vec::new(),
            current_link: None,
            user_is_white: true,
            biggest_deficit: 0,
            biggest_link: None,
            current_max_deficit: 0,
            result: String::new(),
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

    fn compute_material_balance(board: &shakmaty::Chess, user_color: Color) -> i32 {
        let b = board.board();
        let mut balance = 0i32;
        for sq in b.occupied() {
            if let Some(piece) = b.piece_at(sq) {
                let val = Self::piece_value(piece.role);
                if piece.color == user_color {
                    balance += val;
                } else {
                    balance -= val;
                }
            }
        }
        balance
    }
}

impl GameAnalyzer for BiggestComebackAnalyzer {
    fn name(&self) -> &'static str {
        "biggest_comeback"
    }

    fn start_game(&mut self, game_data: &GameData, user_is_white: bool) {
        self.current_link = game_data.metadata.link.clone();
        self.user_is_white = user_is_white;
        self.current_max_deficit = 0;
        self.result = game_data.metadata.result.clone();
    }

    fn process_move(&mut self, ctx: &MoveContext) {
        // Compute material balance at each position
        let balance = Self::compute_material_balance(ctx.board, ctx.user_color);
        // Deficit = how much material the user is down (negative balance)
        if balance < self.current_max_deficit {
            self.current_max_deficit = balance;
        }
    }

    fn finish_game(&mut self) -> bool {
        let user_won = (self.result == "1-0" && self.user_is_white)
            || (self.result == "0-1" && !self.user_is_white);

        if !user_won {
            return false;
        }

        // Only count if user was actually down material at some point
        if self.current_max_deficit >= 0 {
            return false;
        }

        let deficit = self.current_max_deficit.abs();
        if deficit > self.biggest_deficit {
            self.biggest_deficit = deficit;
            self.biggest_link = self.current_link.clone();
        }

        false // We only report in matched_game_links
    }

    fn matched_game_links(&self) -> Vec<String> {
        if let Some(ref link) = self.biggest_link {
            vec![link.clone()]
        } else {
            Vec::new()
        }
    }
}
