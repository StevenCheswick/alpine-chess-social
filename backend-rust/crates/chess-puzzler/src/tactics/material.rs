/// Material-based detectors: sacrifice, exposed_king, endgame types
/// Port of cook.py

use chess::{Color, Piece, Square, Rank, File};

use crate::board_utils::{material_diff, king_square};
use crate::puzzle::Puzzle;

/// Sacrifice: solver goes down in material during the puzzle.
pub fn sacrifice(puzzle: &Puzzle) -> bool {
    let initial = material_diff(&puzzle.mainline[0].board_after, puzzle.pov);

    // Get material diffs after each solver move
    let solver_diffs: Vec<i32> = puzzle.solver_moves()
        .iter()
        .map(|n| material_diff(&n.board_after, puzzle.pov))
        .collect();

    // For multi-move puzzles, skip the first solver move
    let check_diffs = if solver_diffs.len() > 1 {
        &solver_diffs[1..]
    } else {
        &solver_diffs[..]
    };

    for &d in check_diffs {
        if d - initial <= -2 {
            // Not a sacrifice if it involves a promotion (check opponent moves)
            // Python: puzzle.mainline[::2][1:] = opponent moves, skipping first
            let has_promotion = puzzle.mainline.iter()
                .enumerate()
                .filter(|(i, _)| i % 2 == 0 && *i > 0)
                .any(|(_, n)| n.chess_move.get_promotion().is_some());
            if !has_promotion {
                return true;
            }
            break;
        }
    }

    false
}

/// Exposed king: opponent's king has no pawn cover and gets checked
pub fn exposed_king(puzzle: &Puzzle) -> bool {
    let board = &puzzle.mainline[0].board_after;
    let (check_board, check_pov) = if puzzle.pov == Color::White {
        (board, puzzle.pov)
    } else {
        // For black pov we'd need to mirror, but since we're checking from the board directly
        // we can just work with the actual position
        (board, puzzle.pov)
    };

    let king_sq = king_square(check_board, !check_pov);
    let king_rank = king_sq.get_rank().to_index();

    // Python logic: if pov is White, check black king rank < 5 (return False if so).
    // If pov is Black, mirror board, then check white king (mirrored) rank < 5.
    // Simplified: the enemy king must be on rank >= 5 from the attacker's perspective.
    let effective_rank = if check_pov == Color::White {
        king_rank
    } else {
        7 - king_rank
    };
    if effective_rank < 5 {
        return false;
    }


    // Check surrounding squares for pawn cover
    let front_rank = if puzzle.pov == Color::White {
        // Enemy is black, check ranks below king (toward white)
        if king_rank == 0 { return false; }
        king_rank - 1
    } else {
        if king_rank == 7 { return false; }
        king_rank + 1
    };

    // Check 2-3 squares in front of king for enemy pawns
    let king_file = king_sq.get_file().to_index();
    let front_sq = Square::make_square(Rank::from_index(front_rank), File::from_index(king_file));
    if board.piece_on(front_sq) == Some(Piece::Pawn) && board.color_on(front_sq) == Some(!puzzle.pov) {
        return false;
    }
    if king_file > 0 {
        let sq = Square::make_square(Rank::from_index(front_rank), File::from_index(king_file - 1));
        if board.piece_on(sq) == Some(Piece::Pawn) && board.color_on(sq) == Some(!puzzle.pov) {
            return false;
        }
    }
    if king_file < 7 {
        let sq = Square::make_square(Rank::from_index(front_rank), File::from_index(king_file + 1));
        if board.piece_on(sq) == Some(Piece::Pawn) && board.color_on(sq) == Some(!puzzle.pov) {
            return false;
        }
    }

    // Same-rank adjacent squares (king-1, king+1 in Python's square arithmetic)
    if king_file > 0 {
        let sq = Square::make_square(Rank::from_index(king_rank), File::from_index(king_file - 1));
        if board.piece_on(sq) == Some(Piece::Pawn) && board.color_on(sq) == Some(!puzzle.pov) {
            return false;
        }
    }
    if king_file < 7 {
        let sq = Square::make_square(Rank::from_index(king_rank), File::from_index(king_file + 1));
        if board.piece_on(sq) == Some(Piece::Pawn) && board.color_on(sq) == Some(!puzzle.pov) {
            return false;
        }
    }

    // Must have a check in solver moves (excluding first and last)
    let solver = puzzle.solver_moves();
    if solver.len() < 3 {
        return false;
    }
    for node in &solver[1..solver.len()-1] {
        if node.board_after.checkers().popcnt() > 0 {
            return true;
        }
    }
    false
}

/// Piece endgame: only kings, pawns, and the specified piece type
pub fn piece_endgame(puzzle: &Puzzle, piece_type: Piece) -> bool {
    // Check both the initial and first solver move positions
    for i in 0..2.min(puzzle.mainline.len()) {
        let board = &puzzle.mainline[i].board_after;

        // Must have at least one piece of this type
        let white_has = (*board.pieces(piece_type) & *board.color_combined(Color::White)).popcnt() > 0;
        let black_has = (*board.pieces(piece_type) & *board.color_combined(Color::Black)).popcnt() > 0;
        if !white_has && !black_has {
            return false;
        }

        // All pieces must be King, Pawn, or the specified type
        for sq in *board.combined() {
            if let Some(piece) = board.piece_on(sq) {
                if piece != Piece::King && piece != Piece::Pawn && piece != piece_type {
                    return false;
                }
            }
        }
    }
    true
}

/// Queen + rook endgame
pub fn queen_rook_endgame(puzzle: &Puzzle) -> bool {
    for i in 0..2.min(puzzle.mainline.len()) {
        let board = &puzzle.mainline[i].board_after;

        let mut queen_count = 0;
        let mut has_rook = false;

        for sq in *board.combined() {
            if let Some(piece) = board.piece_on(sq) {
                match piece {
                    Piece::Queen => queen_count += 1,
                    Piece::Rook => has_rook = true,
                    Piece::King | Piece::Pawn => {}
                    _ => return false, // bishops/knights present
                }
            }
        }

        if queen_count != 1 || !has_rook {
            return false;
        }
    }
    true
}
