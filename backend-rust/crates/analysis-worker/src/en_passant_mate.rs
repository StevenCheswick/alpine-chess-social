//! Checkmate delivered via en passant capture.
//! The final move must be an en passant capture that results in checkmate.
//! Only tags games where the user delivered the mate.

use chess::{Board, ChessMove, Color, MoveGen, Piece};

/// Check if the game ends with an en passant capture that delivers checkmate.
pub fn detect_en_passant_mate(
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

    // The last move must be en passant
    is_en_passant(board_before_last, last_move)
}

fn is_en_passant(board_before: &Board, m: ChessMove) -> bool {
    // Must be a pawn
    if board_before.piece_on(m.get_source()) != Some(Piece::Pawn) {
        return false;
    }

    // en_passant() returns the square of the pawn to be captured (not the destination).
    // The capturing pawn lands on the same file as ep_sq, one rank behind it.
    if let Some(ep_sq) = board_before.en_passant() {
        // Destination must be on the same file as the captured pawn
        if m.get_dest().get_file() != ep_sq.get_file() {
            return false;
        }
        // And the destination square must be empty (pawn captures "through" to empty square)
        return board_before.piece_on(m.get_dest()).is_none();
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_not_en_passant_mate() {
        // Scholar's mate — not en passant
        let board = Board::from_str(
            "r1bqkb1r/pppp1Qpp/2n2n2/4p3/2B1P3/8/PPPP1PPP/RNB1K1NR b KQkq - 0 4",
        )
        .unwrap();
        let board_before = Board::from_str(
            "r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 4 4",
        )
        .unwrap();
        let m = ChessMove::new(chess::Square::H5, chess::Square::F7, None);
        assert!(!detect_en_passant_mate(&board, &board_before, m, Color::White));
    }

    #[test]
    fn test_starting_position() {
        let board = Board::default();
        let m = ChessMove::new(chess::Square::E2, chess::Square::E4, None);
        assert!(!detect_en_passant_mate(&board, &Board::default(), m, Color::White));
    }
}
