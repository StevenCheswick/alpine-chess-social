/// Line geometry detectors: discovered_attack, x_ray, skewer
/// Port of cook.py

use chess::{BitBoard, Piece, EMPTY};

use crate::board_utils::{between, is_in_bad_spot, is_ray_piece, king_value, is_castling_move};
use crate::puzzle::Puzzle;

/// Discovered check: checker is not the piece that moved (private helper)
fn discovered_check(puzzle: &Puzzle) -> bool {
    for node in puzzle.solver_moves() {
        let board = &node.board_after;
        let checkers = *board.checkers();
        if checkers != EMPTY
            && (checkers & BitBoard::from_square(node.chess_move.get_dest())) == EMPTY
        {
            return true;
        }
    }
    false
}

/// Discovered attack (includes discovered check)
pub fn discovered_attack(puzzle: &Puzzle) -> bool {
    if discovered_check(puzzle) {
        return true;
    }

    let solver = puzzle.solver_moves();
    // Skip first solver move
    for (_idx, node) in solver.iter().enumerate().skip(1) {
        // Must be a capture
        if node.board_before.piece_on(node.chess_move.get_dest()).is_none() {
            continue;
        }

        let between_bb = between(node.chess_move.get_source(), node.chess_move.get_dest());

        // Check that the opponent didn't just recapture on the same square
        let prev_node = &puzzle.mainline[node.ply - 1];
        if prev_node.chess_move.get_dest() == node.chess_move.get_dest() {
            return false;
        }

        // The previous solver move must have been on the between line
        if node.ply >= 2 {
            let prev_solver = &puzzle.mainline[node.ply - 2];
            if (between_bb & BitBoard::from_square(prev_solver.chess_move.get_source())) != EMPTY
                && node.chess_move.get_dest() != prev_solver.chess_move.get_dest()
                && node.chess_move.get_source() != prev_solver.chess_move.get_dest()
                && !is_castling_move(&prev_solver.board_before, prev_solver.chess_move)
            {
                return true;
            }
        }
    }
    false
}

/// X-ray: capture through a piece that was in between
pub fn x_ray(puzzle: &Puzzle) -> bool {
    let solver = puzzle.solver_moves();
    // Skip first solver move
    for (_idx, node) in solver.iter().enumerate().skip(1) {
        // Must be a capture
        if node.board_before.piece_on(node.chess_move.get_dest()).is_none() {
            continue;
        }

        // Previous opponent move
        let prev_op = &puzzle.mainline[node.ply - 1];
        if prev_op.chess_move.get_dest() != node.chess_move.get_dest() {
            continue;
        }
        if node.board_after.piece_on(node.chess_move.get_dest()) == Some(Piece::King) {
            // Check moved_piece of opponent, not current piece
            let op_moved = prev_op.board_after.piece_on(prev_op.chess_move.get_dest());
            if op_moved == Some(Piece::King) {
                continue;
            }
        }

        // Previous solver move
        if node.ply < 2 {
            continue;
        }
        let prev_solver = &puzzle.mainline[node.ply - 2];
        if prev_solver.chess_move.get_dest() != prev_op.chess_move.get_dest() {
            continue;
        }

        // The opponent's from-square must be between our from-square and the target
        let between_bb = between(node.chess_move.get_source(), node.chess_move.get_dest());
        if (between_bb & BitBoard::from_square(prev_op.chess_move.get_source())) != EMPTY {
            return true;
        }
    }
    false
}

/// Skewer: a sliding piece attacks through a higher-value piece
pub fn skewer(puzzle: &Puzzle) -> bool {
    let solver = puzzle.solver_moves();
    // Skip first solver move
    for (_idx, node) in solver.iter().enumerate().skip(1) {
        let capture = node.board_before.piece_on(node.chess_move.get_dest());
        let moved = node.board_after.piece_on(node.chess_move.get_dest());

        if let (Some(cap_piece), Some(mover)) = (capture, moved) {
            if !is_ray_piece(mover) {
                continue;
            }
            if node.board_after.status() == chess::BoardStatus::Checkmate {
                continue;
            }

            let between_bb = between(node.chess_move.get_source(), node.chess_move.get_dest());

            // Previous opponent move
            let prev_op = &puzzle.mainline[node.ply - 1];
            if prev_op.chess_move.get_dest() == node.chess_move.get_dest() {
                continue;
            }
            if (between_bb & BitBoard::from_square(prev_op.chess_move.get_source())) == EMPTY {
                continue;
            }

            // The piece that moved away must be worth more than what we captured
            let op_moved = prev_op.board_after.piece_on(prev_op.chess_move.get_dest());
            if let Some(op_piece) = op_moved {
                if king_value(op_piece) > king_value(cap_piece)
                    && is_in_bad_spot(&prev_op.board_after, node.chess_move.get_dest())
                {
                    return true;
                }
            }
        }
    }
    false
}
