use crate::analyzer_trait::{GameAnalyzer, MoveContext};
use chess_core::game_data::GameData;
use shakmaty::Position;

/// Tracks the fewest-move checkmate across all games. Excludes scholar's mate
/// (Qxf7# or Qf7#). Reports only the single quickest.
pub struct QuickestMateAnalyzer {
    username: String,
    matched_links: Vec<String>,
    current_link: Option<String>,
    user_is_white: bool,
    // Track the quickest mate found so far
    quickest_moves: Option<usize>,
    quickest_link: Option<String>,
    // Current game state
    current_mate_move: Option<usize>,
    is_scholars_mate: bool,
    total_moves: usize,
}

impl QuickestMateAnalyzer {
    pub fn new(username: &str) -> Self {
        Self {
            username: username.to_lowercase(),
            matched_links: Vec::new(),
            current_link: None,
            user_is_white: true,
            quickest_moves: None,
            quickest_link: None,
            current_mate_move: None,
            is_scholars_mate: false,
            total_moves: 0,
        }
    }
}

impl GameAnalyzer for QuickestMateAnalyzer {
    fn name(&self) -> &'static str {
        "quickest_mate"
    }

    fn start_game(&mut self, game_data: &GameData, user_is_white: bool) {
        self.current_link = game_data.metadata.link.clone();
        self.user_is_white = user_is_white;
        self.current_mate_move = None;
        self.is_scholars_mate = false;
        self.total_moves = 0;

        // Check for scholar's mate pattern in SAN moves
        // Scholar's mate typically ends with Qxf7# or Qf7# around move 4
        let moves = &game_data.moves;
        if let Some(last) = moves.last() {
            let last_upper = last.to_uppercase();
            if (last_upper.contains("QXF7") || last_upper.contains("QF7")
                || last_upper.contains("QXF2") || last_upper.contains("QF2"))
                && last.contains('#')
                && moves.len() <= 8
            {
                self.is_scholars_mate = true;
            }
        }
    }

    fn process_move(&mut self, ctx: &MoveContext) {
        self.total_moves = ctx.move_number;

        if !ctx.is_user_move {
            return;
        }

        let mut pos_after = ctx.board.clone();
        pos_after.play_unchecked(ctx.mv);

        if pos_after.is_checkmate() {
            self.current_mate_move = Some(ctx.move_number);
        }
    }

    fn finish_game(&mut self) -> bool {
        if self.is_scholars_mate {
            return false;
        }

        if let Some(mate_move) = self.current_mate_move {
            let dominated = match self.quickest_moves {
                Some(prev) => mate_move < prev,
                None => true,
            };

            if dominated {
                // Replace previous quickest
                self.quickest_moves = Some(mate_move);
                self.quickest_link = self.current_link.clone();
            }
        }

        false // We only report in matched_game_links
    }

    fn matched_game_links(&self) -> Vec<String> {
        if let Some(ref link) = self.quickest_link {
            vec![link.clone()]
        } else {
            Vec::new()
        }
    }
}
