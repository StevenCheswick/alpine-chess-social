/// Checkmate pattern detectors (Lichess standard)
/// smothered_mate, back_rank_mate, anastasia_mate, arabian_mate,
/// hook_mate, boden_or_double_bishop_mate, dovetail_mate

use chess::{BoardStatus, Color, Piece, Square, Rank, File, EMPTY};

use crate::board_utils::{king_square, square_distance, king_adjacent_squares, attackers, attacker_pieces};
use crate::puzzle::{Puzzle, TagKind};

/// Smothered mate: knight checkmate with king surrounded by own pieces
pub fn smothered_mate(puzzle: &Puzzle) -> bool {
    let board = puzzle.end_board();
    if board.status() != BoardStatus::Checkmate {
        return false;
    }

    let king_sq = king_square(board, !puzzle.pov);

    for checker_sq in *board.checkers() {
        if let Some(piece) = board.piece_on(checker_sq) {
            if piece == Piece::Knight {
                // All adjacent squares must be blocked by friendly pieces
                for escape_sq in king_adjacent_squares(king_sq) {
                    match board.piece_on(escape_sq) {
                        Some(_) => {
                            // Check if it's the mated side's own piece
                            if board.color_on(escape_sq) == Some(puzzle.pov) {
                                return false;
                            }
                        }
                        None => return false,
                    }
                }
                return true;
            }
        }
    }
    false
}

/// Back rank mate: checkmate on the back rank with the king trapped by own pieces
pub fn back_rank_mate(puzzle: &Puzzle) -> bool {
    let board = puzzle.end_board();
    if board.status() != BoardStatus::Checkmate {
        return false;
    }

    let king_sq = king_square(board, !puzzle.pov);
    let back_rank = if puzzle.pov == Color::White { 7 } else { 0 };

    if king_sq.get_rank().to_index() != back_rank {
        return false;
    }

    // Squares in front of king (one rank toward the center)
    let front_rank = if puzzle.pov == Color::White { back_rank - 1 } else { back_rank + 1 };
    let king_file = king_sq.get_file().to_index();

    // Check the 2-3 squares in front of the king
    let _files_to_check: Vec<usize> = {
        let mut f = vec![king_file];
        if king_file > 0 { f.push(king_file - 1); }
        if king_file < 7 { f.push(king_file + 1); }
        f
    };

    // Front squares must be blocked by defender's own pieces (not attacked by attacker)
    let front_sq = Square::make_square(Rank::from_index(front_rank), File::from_index(king_file));
    let mut front_squares = vec![front_sq];
    if king_file > 0 {
        let sq = if puzzle.pov == Color::White {
            Square::make_square(Rank::from_index(back_rank - 1), File::from_index(king_file - 1))
        } else {
            Square::make_square(Rank::from_index(back_rank + 1), File::from_index(king_file - 1))
        };
        front_squares.push(sq);
    }
    if king_file < 7 {
        let sq = if puzzle.pov == Color::White {
            Square::make_square(Rank::from_index(back_rank - 1), File::from_index(king_file + 1))
        } else {
            Square::make_square(Rank::from_index(back_rank + 1), File::from_index(king_file + 1))
        };
        front_squares.push(sq);
    }

    for &sq in &front_squares {
        let piece = board.piece_on(sq);
        if piece.is_none() || board.color_on(sq) == Some(puzzle.pov)
            || attackers(board, puzzle.pov, sq) != EMPTY
        {
            return false;
        }
    }

    // Checker must be on the back rank
    for checker_sq in *board.checkers() {
        if checker_sq.get_rank().to_index() == back_rank {
            return true;
        }
    }
    false
}

/// Anastasia mate: rook/queen on the edge file, knight two squares away, piece in between
pub fn anastasia_mate(puzzle: &Puzzle) -> bool {
    let node = puzzle.mainline.last().unwrap();
    let board = &node.board_after;
    if board.status() != BoardStatus::Checkmate {
        return false;
    }

    let king_sq = king_square(board, !puzzle.pov);
    let king_file = king_sq.get_file().to_index();
    let king_rank = king_sq.get_rank().to_index();

    // King on a or h file, not in corner
    if (king_file != 0 && king_file != 7) || king_rank == 0 || king_rank == 7 {
        return false;
    }

    let moved_piece = board.piece_on(node.chess_move.get_dest());
    let checker_file = node.chess_move.get_dest().get_file().to_index();

    // Checker must be rook or queen on the same file as king
    if !matches!(moved_piece, Some(Piece::Rook) | Some(Piece::Queen)) || checker_file != king_file {
        return false;
    }

    // Normalize: if king on h-file, mirror mentally (check squares to the right of king)
    let inner_file = if king_file == 0 { 1 } else { 6 }; // file adjacent to king toward center

    let blocker_sq = Square::make_square(Rank::from_index(king_rank), File::from_index(inner_file));
    let blocker = board.piece_on(blocker_sq);
    if blocker.is_none() || board.color_on(blocker_sq) == Some(puzzle.pov) {
        return false;
    }

    // Knight should be 2 files in from king (or check for knight attacking king area)
    let knight_file = if king_file == 0 { 3 } else { 4 };
    let knight_sq = Square::make_square(Rank::from_index(king_rank), File::from_index(knight_file));
    if let Some(piece) = board.piece_on(knight_sq) {
        if piece == Piece::Knight && board.color_on(knight_sq) == Some(puzzle.pov) {
            return true;
        }
    }

    false
}

/// Hook mate: rook adjacent to king, defended by knight, knight defended by pawn
pub fn hook_mate(puzzle: &Puzzle) -> bool {
    let node = puzzle.mainline.last().unwrap();
    let board = &node.board_after;
    if board.status() != BoardStatus::Checkmate {
        return false;
    }

    let king_sq = king_square(board, !puzzle.pov);
    let moved_piece = board.piece_on(node.chess_move.get_dest());

    if moved_piece != Some(Piece::Rook) {
        return false;
    }
    if square_distance(node.chess_move.get_dest(), king_sq) != 1 {
        return false;
    }

    // Rook must be defended by a knight
    let rook_defenders = attackers(board, puzzle.pov, node.chess_move.get_dest());
    for def_sq in rook_defenders {
        if let Some(piece) = board.piece_on(def_sq) {
            if piece == Piece::Knight && square_distance(def_sq, king_sq) == 1 {
                // Knight must be defended by a pawn
                let knight_defenders = attackers(board, puzzle.pov, def_sq);
                for kd_sq in knight_defenders {
                    if board.piece_on(kd_sq) == Some(Piece::Pawn) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Arabian mate: rook adjacent to king in corner, knight covers escape
pub fn arabian_mate(puzzle: &Puzzle) -> bool {
    let node = puzzle.mainline.last().unwrap();
    let board = &node.board_after;
    if board.status() != BoardStatus::Checkmate {
        return false;
    }

    let king_sq = king_square(board, !puzzle.pov);
    let king_file = king_sq.get_file().to_index();
    let king_rank = king_sq.get_rank().to_index();

    // King must be in a corner
    if (king_file != 0 && king_file != 7) || (king_rank != 0 && king_rank != 7) {
        return false;
    }

    let moved_piece = board.piece_on(node.chess_move.get_dest());
    if moved_piece != Some(Piece::Rook) {
        return false;
    }
    if square_distance(node.chess_move.get_dest(), king_sq) != 1 {
        return false;
    }

    // A knight must be 2 ranks and 2 files from the king (the typical L-shape position)
    let rook_defenders = attackers(board, puzzle.pov, node.chess_move.get_dest());
    for knight_sq in rook_defenders {
        if let Some(piece) = board.piece_on(knight_sq) {
            if piece == Piece::Knight {
                let rank_diff = (knight_sq.get_rank().to_index() as i32 - king_rank as i32).abs();
                let file_diff = (knight_sq.get_file().to_index() as i32 - king_file as i32).abs();
                if rank_diff == 2 && file_diff == 2 {
                    return true;
                }
            }
        }
    }
    false
}

/// Boden's mate or double bishop mate
pub fn boden_or_double_bishop_mate(puzzle: &Puzzle) -> Option<TagKind> {
    let board = puzzle.end_board();
    if board.status() != BoardStatus::Checkmate {
        return None;
    }

    let king_sq = king_square(board, !puzzle.pov);

    // Need at least 2 bishops of the attacker's color
    let bishop_bb = *board.pieces(Piece::Bishop) & *board.color_combined(puzzle.pov);
    if bishop_bb.popcnt() < 2 {
        return None;
    }

    // All attackers of the king square AND adjacent squares must be bishops only
    let mut check_squares = king_adjacent_squares(king_sq);
    check_squares.push(king_sq); // Include king square itself (Python uses distance < 2 which includes distance 0)
    for &sq in &check_squares {
        let atk_pieces = attacker_pieces(board, puzzle.pov, sq);
        for piece in &atk_pieces {
            if *piece != Piece::Bishop {
                return None;
            }
        }
    }

    // Collect bishop squares
    let mut bishop_squares: Vec<Square> = Vec::new();
    for sq in bishop_bb {
        bishop_squares.push(sq);
    }

    if bishop_squares.len() >= 2 {
        let king_file = king_sq.get_file().to_index() as i32;
        let b0_file = bishop_squares[0].get_file().to_index() as i32;
        let b1_file = bishop_squares[1].get_file().to_index() as i32;

        // Boden: bishops on opposite sides of the king
        if (b0_file < king_file) == (b1_file > king_file) {
            return Some(TagKind::BodenMate);
        } else {
            return Some(TagKind::DoubleBishopMate);
        }
    }

    None
}

/// Dovetail mate: queen adjacent diagonally to king, all other escape squares blocked
pub fn dovetail_mate(puzzle: &Puzzle) -> bool {
    let node = puzzle.mainline.last().unwrap();
    let board = &node.board_after;
    if board.status() != BoardStatus::Checkmate {
        return false;
    }

    let king_sq = king_square(board, !puzzle.pov);
    let king_file = king_sq.get_file().to_index();
    let king_rank = king_sq.get_rank().to_index();

    // King must not be on an edge
    if king_file == 0 || king_file == 7 || king_rank == 0 || king_rank == 7 {
        return false;
    }

    let queen_sq = node.chess_move.get_dest();
    let moved_piece = board.piece_on(queen_sq);

    if moved_piece != Some(Piece::Queen) {
        return false;
    }
    // Queen must be diagonally adjacent
    if queen_sq.get_file().to_index() == king_file || queen_sq.get_rank().to_index() == king_rank {
        return false;
    }
    if square_distance(queen_sq, king_sq) > 1 {
        return false;
    }

    // Check all adjacent squares — must match Python logic exactly:
    // attackers == [queen_sq] and piece present → return false
    // attackers == [queen_sq] and empty → OK (queen covers escape)
    // attackers non-empty but not just queen → return false
    // no attackers → OK (own piece blocks)
    for adj in king_adjacent_squares(king_sq) {
        if adj == queen_sq {
            continue;
        }
        let adj_attackers: Vec<Square> = attackers(board, puzzle.pov, adj)
            .into_iter()
            .collect();

        if adj_attackers == vec![queen_sq] {
            // Only the queen attacks this square
            if board.piece_on(adj).is_some() {
                return false;
            }
            // Empty + only queen attacks → OK
        } else if !adj_attackers.is_empty() {
            // Other pieces attack this square → not dovetail
            return false;
        }
        // No attackers → OK (own piece blocks, or square otherwise inaccessible)
    }
    true
}

