/// Side attack detectors: kingside_attack, queenside_attack
/// Port of cook.py

use chess::{Color, Square, Rank, File};

use crate::board_utils::{king_square, square_distance};
use crate::puzzle::Puzzle;

/// Kingside attack
pub fn kingside_attack(puzzle: &Puzzle) -> bool {
    side_attack(puzzle, 7, &[6, 7], 20)
}

/// Queenside attack
pub fn queenside_attack(puzzle: &Puzzle) -> bool {
    side_attack(puzzle, 0, &[0, 1, 2], 18)
}

fn side_attack(puzzle: &Puzzle, corner_file: usize, king_files: &[usize], nb_pieces: u32) -> bool {
    let back_rank = if puzzle.pov == Color::White { 7 } else { 0 };
    let init_board = &puzzle.mainline[0].board_after;
    let king_sq = king_square(init_board, !puzzle.pov);

    if king_sq.get_rank().to_index() != back_rank {
        return false;
    }
    if !king_files.contains(&king_sq.get_file().to_index()) {
        return false;
    }
    if crate::board_utils::piece_map_count(init_board) < nb_pieces {
        return false;
    }

    // Must have at least one check among solver moves
    if !puzzle.solver_moves().iter().any(|n| n.board_after.checkers().popcnt() > 0) {
        return false;
    }

    let corner = Square::make_square(Rank::from_index(back_rank), File::from_index(corner_file));
    let mut score: i32 = 0;

    for node in puzzle.solver_moves() {
        let corner_dist = square_distance(corner, node.chess_move.get_dest());
        if node.board_after.checkers().popcnt() > 0 {
            score += 1;
        }
        if node.board_before.piece_on(node.chess_move.get_dest()).is_some() && corner_dist <= 3 {
            score += 1;
        } else if corner_dist >= 5 {
            score -= 1;
        }
    }

    score >= 2
}
