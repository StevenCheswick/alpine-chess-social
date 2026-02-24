/// Engine-based zugzwang detection using null-move evaluation.
///
/// Port of Lichess's zugzwang detector: evaluate each solver-move position
/// normally and after a null move (pass). If win chances drop by >0.3 when
/// forced to move, it's zugzwang.

use chess::MoveGen;

use crate::puzzle::Puzzle;

/// Convert centipawns to win chances using Lichess's sigmoid.
fn win_chances(cp: i32) -> f64 {
    2.0 / (1.0 + (-0.004 * cp as f64).exp()) - 1.0
}

/// Convert eval to win chances, handling mate scores.
fn eval_win_chances(cp: i32, mate: Option<i32>) -> f64 {
    if let Some(m) = mate {
        if m > 0 { 1.0 } else { -1.0 }
    } else {
        win_chances(cp)
    }
}

/// Determine if a puzzle exhibits zugzwang using pre-computed engine evals.
///
/// `evals` contains one entry per solver move's resulting position:
///   `(normal_cp, null_move_cp, normal_mate, null_move_mate)`
///
/// For each position, if the opponent's win chances are higher when they
/// DON'T have to move (null move) than when they DO, by more than 0.3,
/// it's zugzwang.
pub fn zugzwang(puzzle: &Puzzle, evals: &[ZugzwangEval]) -> bool {
    // Piece-count filtering is done by the WS orchestrator (which decides
    // which puzzles to send for engine eval). Here we just check the evals.

    for (i, node) in puzzle.mainline.iter().enumerate() {
        // Only solver moves (odd indices)
        if i % 2 == 0 {
            continue;
        }

        let board_after = &node.board_after;

        // Skip positions in check
        if board_after.checkers().popcnt() > 0 {
            continue;
        }

        // Skip positions with many legal moves (>15)
        if MoveGen::new_legal(board_after).len() > 15 {
            continue;
        }

        // Find matching eval for this solver move index
        let solver_idx = i / 2; // 0-based solver move index
        if solver_idx >= evals.len() {
            continue;
        }

        let eval = &evals[solver_idx];

        // Evals are from the side-to-move perspective:
        // - normal: opponent is side-to-move → normal_wc IS opponent's win chances
        // - null: side-to-move was flipped to solver → negate to get opponent's view
        //
        // Zugzwang: opponent is worse when forced to move than if they could pass
        let normal_wc = eval_win_chances(eval.cp, eval.mate);
        let null_wc = eval_win_chances(eval.null_cp, eval.null_mate);

        let opp_normal = normal_wc;   // opponent's win chances when they must move
        let opp_null = -null_wc;      // opponent's win chances if they could pass

        if opp_normal < opp_null - 0.3 {
            return true;
        }
    }

    false
}

/// Pre-computed engine evaluation for a single position.
#[derive(Debug, Clone)]
pub struct ZugzwangEval {
    /// Centipawns for the normal position (side-to-move perspective)
    pub cp: i32,
    /// Centipawns for the null-move position (flipped side-to-move perspective)
    pub null_cp: i32,
    /// Mate score for normal position (None if no mate)
    pub mate: Option<i32>,
    /// Mate score for null-move position (None if no mate)
    pub null_mate: Option<i32>,
}
