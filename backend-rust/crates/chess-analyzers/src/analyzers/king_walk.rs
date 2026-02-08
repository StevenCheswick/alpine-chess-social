use crate::analyzer_trait::{GameAnalyzer, MoveContext};
use chess_core::game_data::GameData;
use shakmaty::{Role, Square, Position, Move};

/// Detects king walk: user's king moves 5+ squares from its starting position
/// during the game and user wins.
pub struct KingWalkAnalyzer {
    username: String,
    matched_links: Vec<String>,
    current_link: Option<String>,
    user_is_white: bool,
    matched: bool,
    king_start: Option<Square>,
    max_distance: i32,
}

impl KingWalkAnalyzer {
    pub fn new(username: &str) -> Self {
        Self {
            username: username.to_lowercase(),
            matched_links: Vec::new(),
            current_link: None,
            user_is_white: true,
            matched: false,
            king_start: None,
            max_distance: 0,
        }
    }

    fn chebyshev_distance(a: Square, b: Square) -> i32 {
        let file_diff = (a.file() as i32 - b.file() as i32).abs();
        let rank_diff = (a.rank() as i32 - b.rank() as i32).abs();
        file_diff.max(rank_diff)
    }
}

impl GameAnalyzer for KingWalkAnalyzer {
    fn name(&self) -> &'static str {
        "king_walk"
    }

    fn start_game(&mut self, game_data: &GameData, user_is_white: bool) {
        self.current_link = game_data.metadata.link.clone();
        self.user_is_white = user_is_white;
        self.matched = false;
        self.max_distance = 0;
        // Starting square of user's king
        self.king_start = if user_is_white {
            Some(Square::E1)
        } else {
            Some(Square::E8)
        };
    }

    fn process_move(&mut self, ctx: &MoveContext) {
        if !ctx.is_user_move {
            return;
        }

        // Track king moves
        let to_sq = match ctx.mv {
            Move::Normal { role: Role::King, to, .. } => *to,
            Move::Castle { king, .. } => {
                // After castling, king ends up at a specific square
                // For king-side: g1/g8, queen-side: c1/c8
                // We just track the king square from the board after the move
                let mut pos_after = ctx.board.clone();
                pos_after.play_unchecked(ctx.mv);
                if let Some(sq) = pos_after.board().king_of(ctx.user_color) {
                    sq
                } else {
                    return;
                }
            }
            _ => return,
        };

        if let Some(start) = self.king_start {
            let dist = Self::chebyshev_distance(start, to_sq);
            if dist > self.max_distance {
                self.max_distance = dist;
            }
        }
    }

    fn finish_game(&mut self) -> bool {
        self.matched = self.max_distance >= 5;

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
