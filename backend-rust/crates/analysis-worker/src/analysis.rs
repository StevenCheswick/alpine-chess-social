/// Move analysis and classification — pure functions only
/// (No Board/Cache/Engine/Game dependencies)

use serde::{Deserialize, Serialize};

/// Classification thresholds (centipawn loss)
const THRESHOLD_BEST: i32 = 0;
const THRESHOLD_EXCELLENT: i32 = 10;
const THRESHOLD_GOOD: i32 = 50;
const THRESHOLD_INACCURACY: i32 = 100;
const THRESHOLD_MISTAKE: i32 = 200;

/// Mate detection threshold
const MATE_THRESHOLD: i32 = 9000;

/// Maximum CP loss to cap at
const MAX_CP_LOSS: i32 = 500;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveAnalysis {
    #[serde(rename = "move")]
    pub move_uci: String,
    pub move_eval: i32,
    pub best_move: String,
    pub best_eval: i32,
    pub cp_loss: i32,
    pub classification: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Classifications {
    pub best: u32,
    pub excellent: u32,
    pub good: u32,
    pub inaccuracy: u32,
    pub mistake: u32,
    pub blunder: u32,
    pub forced: u32,
    pub book: u32,
}

fn is_mate_position(eval: i32) -> bool {
    eval.abs() > MATE_THRESHOLD
}

pub fn is_mate_blunder(
    best_eval: i32,
    after_eval: i32,
    is_white: bool,
    is_checkmate: bool,
) -> bool {
    if is_checkmate {
        return false;
    }

    let best_is_mate = is_mate_position(best_eval);
    let after_is_mate = is_mate_position(after_eval);

    if best_is_mate && !after_is_mate {
        return true;
    }

    if !best_is_mate && after_is_mate {
        let allowed_bad_mate = if is_white {
            after_eval < 0
        } else {
            after_eval > 0
        };
        return allowed_bad_mate;
    }

    false
}

pub fn calculate_cp_loss(
    best_eval: i32,
    after_eval: i32,
    is_white: bool,
    is_checkmate: bool,
) -> i32 {
    if is_checkmate {
        return 0;
    }

    let best_is_mate = is_mate_position(best_eval);
    let after_is_mate = is_mate_position(after_eval);

    if best_is_mate && after_is_mate {
        if (best_eval > 0) == (after_eval > 0) {
            return 0;
        } else {
            return MAX_CP_LOSS;
        }
    }

    let cp_loss = if is_white {
        best_eval - after_eval
    } else {
        after_eval - best_eval
    };

    cp_loss.max(0).min(MAX_CP_LOSS)
}

pub fn classify_move(cp_loss: i32, is_mate_blunder: bool) -> &'static str {
    if is_mate_blunder {
        return "blunder";
    }
    if cp_loss <= THRESHOLD_BEST {
        "best"
    } else if cp_loss < THRESHOLD_EXCELLENT {
        "excellent"
    } else if cp_loss < THRESHOLD_GOOD {
        "good"
    } else if cp_loss < THRESHOLD_INACCURACY {
        "inaccuracy"
    } else if cp_loss < THRESHOLD_MISTAKE {
        "mistake"
    } else {
        "blunder"
    }
}

pub fn calculate_accuracy(total_cp_loss: i32, move_count: u32) -> f64 {
    if move_count == 0 {
        return 100.0;
    }
    let acpl = total_cp_loss as f64 / move_count as f64;
    let accuracy = 100.0 * (1.0 / (1.0 + acpl / 100.0)).sqrt();
    accuracy.max(0.0).min(100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_move() {
        assert_eq!(classify_move(0, false), "best");
        assert_eq!(classify_move(5, false), "excellent");
        assert_eq!(classify_move(25, false), "good");
        assert_eq!(classify_move(75, false), "inaccuracy");
        assert_eq!(classify_move(150, false), "mistake");
        assert_eq!(classify_move(250, false), "blunder");
        assert_eq!(classify_move(0, true), "blunder");
    }

    #[test]
    fn test_calculate_accuracy() {
        assert!((calculate_accuracy(0, 20) - 100.0).abs() < 0.1);
        assert!((calculate_accuracy(500, 20) - 89.4).abs() < 1.0);
        assert!((calculate_accuracy(2000, 20) - 70.7).abs() < 1.0);
    }

    #[test]
    fn test_cp_loss_calculation() {
        assert_eq!(calculate_cp_loss(100, 80, true, false), 20);
        assert_eq!(calculate_cp_loss(100, 120, false, false), 20);
        assert_eq!(calculate_cp_loss(100, 9990, true, true), 0);
        assert_eq!(calculate_cp_loss(9990, 9980, true, false), 0);
        assert_eq!(calculate_cp_loss(9990, -9990, true, false), 500);
    }

    #[test]
    fn test_mate_blunder_detection() {
        assert!(!is_mate_blunder(9990, 9990, true, true));
        assert!(is_mate_blunder(9990, 100, true, false));
        assert!(is_mate_blunder(100, -9990, true, false));
        assert!(!is_mate_blunder(100, 80, true, false));
    }
}
