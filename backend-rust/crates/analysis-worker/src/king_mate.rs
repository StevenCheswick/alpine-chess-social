//! Checkmate delivered by a king move — the final move is a king move that
//! results in checkmate (typically a discovered checkmate).
//! Only tags games where the user delivered the mate.

use chess::{Board, ChessMove, Color, MoveGen, Piece, Square};

/// Check if the game ends with a king move that delivers checkmate.
pub fn detect_king_mate(
    final_board: &Board,
    board_before_last: &Board,
    last_move: ChessMove,
    user_color: Color,
) -> bool {
    // Must be checkmate
    if MoveGen::new_legal(final_board).len() != 0 || final_board.checkers().popcnt() == 0 {
        return false;
    }

    // User must be the mating side
    if final_board.side_to_move() == user_color {
        return false;
    }

    // The last move must be a king move (but not castling)
    if board_before_last.piece_on(last_move.get_source()) != Some(Piece::King) {
        return false;
    }

    // Exclude castling (king moves 2+ squares)
    !is_castling(last_move)
}

fn is_castling(m: ChessMove) -> bool {
    let src = m.get_source();
    let dst = m.get_dest();
    (src == Square::E1 && (dst == Square::G1 || dst == Square::C1))
        || (src == Square::E8 && (dst == Square::G8 || dst == Square::C8))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_king_mate_kg2() {
        // KNVB (black) plays Kg2# — king move delivers discovered mate from queen
        let final_board = Board::from_str(
            "8/8/8/2bp4/2p5/8/p4rk1/2K3q1 w - - 2 67",
        )
        .unwrap();
        let board_before = Board::from_str(
            "8/8/8/2bp4/2p5/8/p4r2/2K2kq1 b - - 1 66",
        )
        .unwrap();
        let m = ChessMove::new(Square::F1, Square::G2, None);
        assert!(detect_king_mate(&final_board, &board_before, m, Color::Black));
        assert!(!detect_king_mate(&final_board, &board_before, m, Color::White));
    }

    #[test]
    fn test_not_king_mate() {
        // Scholar's mate — queen delivers, not a king move
        let final_board = Board::from_str(
            "r1bqkb1r/pppp1Qpp/2n2n2/4p3/2B1P3/8/PPPP1PPP/RNB1K1NR b KQkq - 0 4",
        )
        .unwrap();
        let board_before = Board::from_str(
            "r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 4 4",
        )
        .unwrap();
        let m = ChessMove::new(Square::H5, Square::F7, None);
        assert!(!detect_king_mate(&final_board, &board_before, m, Color::White));
    }
}
