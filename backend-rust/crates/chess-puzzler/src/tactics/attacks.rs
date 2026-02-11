/// Attack-based detectors: fork, hanging_piece, trapped_piece
/// Port of cook.py

use chess::{BitBoard, Color, Piece, EMPTY};

use crate::board_utils::{self, attackers, attacked_opponent_squares, is_hanging, is_in_bad_spot, is_trapped, king_value, piece_value};
use crate::puzzle::Puzzle;

/// Fork: a piece attacks two or more higher-value or hanging pieces
pub fn fork(puzzle: &Puzzle) -> bool {
    let mut solver_moves: Vec<&_> = puzzle.solver_moves();

    // Lichess excludes the last solver move from fork detection —
    // a fork on the final move is irrelevant (game is already decided)
    solver_moves.pop();

    for node in solver_moves {
        let moved = node.board_after.piece_on(node.chess_move.get_dest());
        if moved == Some(Piece::King) {
            continue;
        }
        let moved_piece = match moved {
            Some(p) => p,
            None => continue,
        };

        let board = &node.board_after;
        let to_sq = node.chess_move.get_dest();

        // Don't count forks from a bad square
        if is_in_bad_spot(board, to_sq) {
            continue;
        }

        let mut fork_count = 0;
        for (piece, _color, square) in attacked_opponent_squares(board, to_sq, puzzle.pov) {
            if piece == Piece::Pawn {
                continue;
            }
            // Fork if: attacked piece is worth more, OR it's hanging and can't recapture
            if king_value(piece) > king_value(moved_piece)
                || (is_hanging(board, !puzzle.pov, square)
                    && (attackers(board, !puzzle.pov, to_sq) & BitBoard::from_square(square)) == EMPTY)
            {
                fork_count += 1;
            }
        }
        if fork_count > 1 {
            return true;
        }
    }
    false
}

/// Hanging piece: the first solver move captures a hanging piece
pub fn hanging_piece(puzzle: &Puzzle) -> bool {
    let first_solver = &puzzle.mainline[1]; // index 1 = first solver move
    let to_sq = first_solver.chess_move.get_dest();
    let board_before = &puzzle.mainline[0].board_after; // board after opponent's mistake

    // Check if we're in check (but no piece to capture)
    let captured = board_before.piece_on(to_sq);
    if board_before.checkers().popcnt() > 0
        && (captured.is_none() || captured == Some(Piece::Pawn))
    {
        return false;
    }

    if let Some(cap_piece) = captured {
        if cap_piece == Piece::Pawn {
            return false;
        }

        let cap_color = board_before.color_on(to_sq).unwrap_or(Color::White);
        if is_hanging(board_before, cap_color, to_sq) {
            // Check if the opponent just made an equal trade on this square
            let op_move = &puzzle.mainline[0]; // opponent's move
            let game_board = &op_move.board_before; // board before opponent moved
            let op_capture = game_board.piece_on(op_move.chess_move.get_dest());

            if let Some(op_cap) = op_capture {
                if piece_value(op_cap) >= piece_value(cap_piece)
                    && op_move.chess_move.get_dest() == to_sq
                {
                    return false;
                }
            }

            // For short puzzles (< 4 moves), the hanging piece is the tactic
            if puzzle.mainline.len() < 4 {
                return true;
            }

            // For longer puzzles, verify we end up with material advantage
            if puzzle.mainline.len() >= 4 {
                let mat_after_capture = board_utils::material_diff(&first_solver.board_after, puzzle.pov);
                let mat_later = board_utils::material_diff(&puzzle.mainline[3].board_after, puzzle.pov);
                if mat_later >= mat_after_capture {
                    return true;
                }
            }
        }
    }
    false
}

/// Trapped piece: a piece cannot escape and is eventually captured
pub fn trapped_piece(puzzle: &Puzzle) -> bool {
    let solver = puzzle.solver_moves();
    // Skip first solver move, check subsequent ones
    for node in solver.iter().skip(1) {
        let square = node.chess_move.get_dest();
        let captured = node.board_before.piece_on(square);

        if let Some(cap_piece) = captured {
            if cap_piece == Piece::Pawn {
                continue;
            }

            // If the opponent just moved to this square, check the previous position
            let prev_node = &puzzle.mainline[node.ply - 1]; // opponent's previous move
            let check_square = if prev_node.chess_move.get_dest() == square {
                prev_node.chess_move.get_source()
            } else {
                square
            };

            // Check if the piece was trapped in the position before the opponent moved
            if node.ply >= 2 {
                let check_board = &puzzle.mainline[node.ply - 2].board_after;
                if is_trapped(check_board, check_square) {
                    return true;
                }
            }
        }
    }
    false
}

