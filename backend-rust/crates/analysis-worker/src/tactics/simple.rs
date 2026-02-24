/// Simple tactical detectors — no utility dependencies
/// Port of cook.py: double_check, en_passant, castling,
/// promotion, under_promotion, mate_in, advanced_pawn, check_escape

use chess::{Piece, MoveGen, BoardStatus};

use crate::board_utils;
use crate::puzzle::{Puzzle, TagKind};

/// Is there a double check in any solver move?
pub fn double_check(puzzle: &Puzzle) -> bool {
    for node in puzzle.solver_moves() {
        if node.board_after.checkers().popcnt() > 1 {
            return true;
        }
    }
    false
}

/// Is there an en passant in any solver move?
pub fn en_passant(puzzle: &Puzzle) -> bool {
    for node in puzzle.solver_moves() {
        let piece = node.board_after.piece_on(node.chess_move.get_dest());
        if piece == Some(Piece::Pawn) {
            let from_file = node.chess_move.get_source().get_file().to_index();
            let to_file = node.chess_move.get_dest().get_file().to_index();
            // Pawn moved diagonally but target square was empty before
            if from_file != to_file && node.board_before.piece_on(node.chess_move.get_dest()).is_none() {
                return true;
            }
        }
    }
    false
}

/// Is there a castling move in any solver move?
pub fn castling(puzzle: &Puzzle) -> bool {
    for node in puzzle.solver_moves() {
        if board_utils::is_castling_move(&node.board_before, node.chess_move) {
            return true;
        }
    }
    false
}

/// Is there a promotion in any solver move?
pub fn promotion(puzzle: &Puzzle) -> bool {
    for node in puzzle.solver_moves() {
        if node.chess_move.get_promotion().is_some() {
            return true;
        }
    }
    false
}

/// Is there an under-promotion in any solver move?
pub fn under_promotion(puzzle: &Puzzle) -> bool {
    for node in puzzle.solver_moves() {
        if let Some(promo) = node.chess_move.get_promotion() {
            if node.board_after.status() == BoardStatus::Checkmate {
                return promo != Piece::Queen;
            }
            if promo != Piece::Queen {
                return true;
            }
        }
    }
    false
}

/// Detect mate-in-N and return the appropriate tag
pub fn mate_in(puzzle: &Puzzle) -> Option<TagKind> {
    let end = puzzle.end_board();
    let moves_to_mate;

    if end.status() == BoardStatus::Checkmate {
        moves_to_mate = puzzle.mainline.len() / 2;
    } else if puzzle.cp >= 9900 {
        // Eval indicates forced mate even though mainline is truncated
        moves_to_mate = ((10000 - puzzle.cp) / 10) as usize;
    } else {
        return None;
    }

    match moves_to_mate {
        0 => None,
        1 => Some(TagKind::MateIn1),
        2 => Some(TagKind::MateIn2),
        3 => Some(TagKind::MateIn3),
        4 => Some(TagKind::MateIn4),
        _ => Some(TagKind::MateIn5),
    }
}

/// Advanced pawn: solver pushes a pawn to rank 7 or 8 (or rank 1/2 for black)
pub fn advanced_pawn(puzzle: &Puzzle) -> bool {
    for node in puzzle.solver_moves() {
        if board_utils::is_very_advanced_pawn_move(
            &node.board_after,
            node.chess_move,
            node.board_after.side_to_move(),
        ) {
            return true;
        }
    }
    false
}

/// Check escape: solver escapes check with a non-capturing, non-checking move
pub fn check_escape(puzzle: &Puzzle) -> bool {
    for node in puzzle.solver_moves() {
        // After solver's move, no check given
        if node.board_after.checkers().popcnt() > 0 {
            return false;
        }
        // Solver's move is not a capture
        if node.board_before.piece_on(node.chess_move.get_dest()).is_some() {
            return false;
        }
        // Must have had at least 3 legal moves
        if MoveGen::new_legal(&node.board_before).len() < 3 {
            return false;
        }
        // Was in check before solver's move
        if node.board_before.checkers().popcnt() > 0 {
            return true;
        }
    }
    false
}
