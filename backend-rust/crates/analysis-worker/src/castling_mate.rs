//! Checkmate delivered via castling.
//! The final move must be a castling move (O-O or O-O-O) that results in checkmate.
//! Only tags games where the user delivered the mate.

use chess::{Board, ChessMove, Color, MoveGen, Piece, Square};

/// Check if the game ends with a castling move that delivers checkmate.
pub fn detect_castling_mate(
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

    // The last move must be castling: king moves 2 squares from e1/e8
    is_castling(board_before_last, last_move)
}

fn is_castling(board_before: &Board, m: ChessMove) -> bool {
    let piece = board_before.piece_on(m.get_source());
    if piece != Some(Piece::King) {
        return false;
    }

    let src = m.get_source();
    let dst = m.get_dest();

    // White castling: e1->g1 or e1->c1
    if src == Square::E1 && (dst == Square::G1 || dst == Square::C1) {
        return true;
    }
    // Black castling: e8->g8 or e8->c8
    if src == Square::E8 && (dst == Square::G8 || dst == Square::C8) {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_not_castling_mate() {
        // Scholar's mate — not castling
        let board = Board::from_str(
            "r1bqkb1r/pppp1Qpp/2n2n2/4p3/2B1P3/8/PPPP1PPP/RNB1K1NR b KQkq - 0 4",
        )
        .unwrap();
        let board_before = Board::from_str(
            "r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 4 4",
        )
        .unwrap();
        let m = ChessMove::new(Square::H5, Square::F7, None);
        assert!(!detect_castling_mate(&board, &board_before, m, Color::White));
    }

    #[test]
    fn test_starting_position() {
        let board = Board::default();
        let m = ChessMove::new(Square::E2, Square::E4, None);
        assert!(!detect_castling_mate(&board, &Board::default(), m, Color::White));
    }
}
