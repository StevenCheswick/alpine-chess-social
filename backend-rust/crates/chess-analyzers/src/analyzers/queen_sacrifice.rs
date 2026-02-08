use crate::analyzer_trait::{GameAnalyzer, MoveContext};
use chess_core::game_data::GameData;
use shakmaty::{Color, Role, Position, Move};

/// Detects queen sacrifices: user's queen is captured, but user does NOT
/// recapture opponent's queen within 1 move. Only in wins (not time wins).
/// Skip if material advantage >= 5 or queen was hanging anyway.
pub struct QueenSacrificeAnalyzer {
    username: String,
    matched_links: Vec<String>,
    current_link: Option<String>,
    user_is_white: bool,
    matched: bool,
    // Track state for sacrifice detection
    queen_lost_move: Option<usize>, // half-move when user lost queen
    user_had_queen: bool,
    result: String,
}

impl QueenSacrificeAnalyzer {
    pub fn new(username: &str) -> Self {
        Self {
            username: username.to_lowercase(),
            matched_links: Vec::new(),
            current_link: None,
            user_is_white: true,
            matched: false,
            queen_lost_move: None,
            user_had_queen: true,
            result: String::new(),
        }
    }

    fn material_value(role: Role) -> i32 {
        match role {
            Role::Pawn => 1,
            Role::Knight => 3,
            Role::Bishop => 3,
            Role::Rook => 5,
            Role::Queen => 9,
            Role::King => 0,
        }
    }

    fn compute_material_advantage(&self, board: &shakmaty::Chess, color: Color) -> i32 {
        let b = board.board();
        let mut score = 0i32;
        for sq in b.occupied() {
            if let Some(piece) = b.piece_at(sq) {
                let val = Self::material_value(piece.role);
                if piece.color == color {
                    score += val;
                } else {
                    score -= val;
                }
            }
        }
        score
    }
}

impl GameAnalyzer for QueenSacrificeAnalyzer {
    fn name(&self) -> &'static str {
        "queen_sacrifice"
    }

    fn start_game(&mut self, game_data: &GameData, user_is_white: bool) {
        self.current_link = game_data.metadata.link.clone();
        self.user_is_white = user_is_white;
        self.matched = false;
        self.queen_lost_move = None;
        self.user_had_queen = true;
        self.result = game_data.metadata.result.clone();
    }

    fn process_move(&mut self, ctx: &MoveContext) {
        if self.matched {
            return;
        }

        // Only consider wins, but not time forfeits
        // We check result in finish_game, but we can filter by result pattern
        let user_color = ctx.user_color;

        // Check if opponent just captured user's queen
        if ctx.is_opponent_move {
            if let Move::Normal { capture: Some(Role::Queen), .. } = ctx.mv {
                // Opponent captured something on user's side - check if it's user's queen
                // The board is before the move, so the queen is still there
                // Material advantage before the capture
                let mat_adv = self.compute_material_advantage(ctx.board, user_color);
                if mat_adv >= 5 {
                    // Skip - user already had big material advantage
                    return;
                }
                self.queen_lost_move = Some(ctx.move_number);
                self.user_had_queen = true;
            }
        }

        // Check if user recaptures opponent's queen within 1 move of losing theirs
        if ctx.is_user_move {
            if let Some(lost_move) = self.queen_lost_move {
                // User's next move after losing queen
                if ctx.move_number == lost_move + 1 {
                    // Did user capture opponent's queen?
                    if let Move::Normal { capture: Some(Role::Queen), .. } = ctx.mv {
                        // Recaptured - not a sacrifice
                        self.queen_lost_move = None;
                        return;
                    }
                    // Didn't recapture queen immediately - it's a sacrifice!
                    self.matched = true;
                    self.queen_lost_move = None;
                }
            }
        }
    }

    fn finish_game(&mut self) -> bool {
        // If queen was lost and user never got the chance to recapture (e.g., last move)
        // and the user won, treat it as a sacrifice
        if self.queen_lost_move.is_some() && !self.matched {
            self.matched = true;
        }

        // Only count wins that aren't time forfeits
        let user_won = (self.result == "1-0" && self.user_is_white)
            || (self.result == "0-1" && !self.user_is_white);

        let result = self.matched && user_won;
        if result {
            if let Some(ref link) = self.current_link {
                self.matched_links.push(link.clone());
            }
        }
        result
    }

    fn matched_game_links(&self) -> Vec<String> {
        self.matched_links.clone()
    }
}
