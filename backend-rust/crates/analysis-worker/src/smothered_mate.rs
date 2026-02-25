//! Smothered mate detection for game-level tagging.
//!
//! A smothered mate is a checkmate delivered by a knight where every square
//! adjacent to the mated king is occupied by the king's own pieces.
//! Only tags games where the user delivered the mate.

use chess::{Board, Color, MoveGen, Piece};

use crate::board_utils::king_square;

/// Check if the final position is a smothered mate delivered by the user.
pub fn detect_smothered_mate(final_board: &Board, user_color: Color) -> bool {
    // Must be checkmate
    if MoveGen::new_legal(final_board).len() != 0 || final_board.checkers().popcnt() == 0 {
        return false;
    }

    // The mated side is the side to move
    let mated_color = final_board.side_to_move();

    // User must be the mating side (not the mated side)
    if mated_color == user_color {
        return false;
    }

    // Exactly one checker (the knight)
    let checkers = final_board.checkers();
    if checkers.popcnt() != 1 {
        return false;
    }

    let checker_sq = checkers.to_square();
    if final_board.piece_on(checker_sq) != Some(Piece::Knight) {
        return false;
    }

    // Every adjacent square of the mated king must be occupied by the king's own pieces
    let king_sq = king_square(final_board, mated_color);
    let king_moves = chess::get_king_moves(king_sq);
    for adj_sq in king_moves {
        match final_board.piece_on(adj_sq) {
            Some(_) => {
                if final_board.color_on(adj_sq) != Some(mated_color) {
                    return false;
                }
            }
            None => return false,
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    // All 14 smothered mate positions from brexwick's 16,125 games.
    // 7 delivered by user (white=Brexwick), 7 by opponent.

    // ── User delivered (should detect as user's color) ──

    #[test]
    fn test_smileysashi_nf7() {
        // Brexwick (white) vs smileysashi — Nf7#
        let board =
            Board::from_str("r5rk/1p3Npp/p7/3p4/1P6/P4N2/2q3PP/4R1K1 b - - 1 31").unwrap();
        assert!(detect_smothered_mate(&board, Color::White));
        assert!(!detect_smothered_mate(&board, Color::Black));
    }

    #[test]
    fn test_joeldblanco_nf2() {
        // Brexwick (black) vs joeldblanco — Nf2#
        let board =
            Board::from_str("r5k1/NQ2bpp1/3pp2p/8/4P3/P3qP2/5nPP/6RK w - - 1 33").unwrap();
        assert!(detect_smothered_mate(&board, Color::Black));
        assert!(!detect_smothered_mate(&board, Color::White));
    }

    #[test]
    fn test_obiwanchessnobi_nf2() {
        // Brexwick (black) vs ObiwanChessnobi — Nf2#
        let board = Board::from_str(
            "r4r1k/3bb1pp/p2p4/1p1N1P2/1Pp1P3/P5N1/Q1P2nPP/R1B3RK w - - 1 25",
        )
        .unwrap();
        assert!(detect_smothered_mate(&board, Color::Black));
        assert!(!detect_smothered_mate(&board, Color::White));
    }

    #[test]
    fn test_reyhane_ir_nc2() {
        // Brexwick (black) vs reyhane_ir — Nc2#
        let board =
            Board::from_str("2r1r3/5pk1/p7/6N1/6P1/4p2Q/PPn2P2/KR6 w - - 1 32").unwrap();
        assert!(detect_smothered_mate(&board, Color::Black));
        assert!(!detect_smothered_mate(&board, Color::White));
    }

    #[test]
    fn test_rrigolino_nf7() {
        // Brexwick (white) vs rrigolino — Nf7#
        let board =
            Board::from_str("6rk/5Npp/8/5P2/3PP3/8/2P1K3/6r1 b - - 1 42").unwrap();
        assert!(detect_smothered_mate(&board, Color::White));
        assert!(!detect_smothered_mate(&board, Color::Black));
    }

    #[test]
    fn test_ahammadirfan_na7() {
        // Brexwick (white) vs ahammadirfan — Na7#
        let board = Board::from_str(
            "1nkr3r/Nppqnpbp/3p2p1/3N4/Q3P3/5P2/PPP2BPP/2KR3R b - - 6 18",
        )
        .unwrap();
        assert!(detect_smothered_mate(&board, Color::White));
        assert!(!detect_smothered_mate(&board, Color::Black));
    }

    #[test]
    fn test_rico0712_nf2() {
        // Brexwick (black) vs rico0712 — Nf2#
        let board = Board::from_str(
            "4Q3/pp3Bpk/7p/8/3P4/3bq3/P4nPP/6RK w - - 9 31",
        )
        .unwrap();
        assert!(detect_smothered_mate(&board, Color::Black));
        assert!(!detect_smothered_mate(&board, Color::White));
    }

    // ── Opponent delivered (should NOT detect for user) ──

    #[test]
    fn test_danae777248_nb3() {
        // danae777248 (black) delivered Nb3# against Brexwick (white)
        let board = Board::from_str(
            "2r1k2r/1p2b1pp/p2p2P1/4p2P/4P3/1n2B3/1PPQ4/qNKR3R w k - 3 22",
        )
        .unwrap();
        // User is white, got mated — should NOT tag
        assert!(!detect_smothered_mate(&board, Color::White));
        // But black did deliver it
        assert!(detect_smothered_mate(&board, Color::Black));
    }

    #[test]
    fn test_moslavac_nf7() {
        // moslavac (white) delivered Nf7# against Brexwick (black)
        let board = Board::from_str(
            "6rk/1p1n1Nrp/5Q2/p1p1pP2/Pn1pP1P1/1P1P3P/2P5/2R3K1 b - - 2 34",
        )
        .unwrap();
        assert!(!detect_smothered_mate(&board, Color::Black));
        assert!(detect_smothered_mate(&board, Color::White));
    }

    #[test]
    fn test_mushfiq70_nf7() {
        // mushfiq70 (white) delivered Nf7# against Brexwick (black)
        let board = Board::from_str(
            "r5rk/5Npp/1p6/1B3b2/pq1P3P/4p3/6P1/K1RR4 b - - 1 34",
        )
        .unwrap();
        assert!(!detect_smothered_mate(&board, Color::Black));
        assert!(detect_smothered_mate(&board, Color::White));
    }

    #[test]
    fn test_ip0j_nf2() {
        // IP0J (black) delivered Nf2# against Brexwick (white)
        let board = Board::from_str(
            "4rr1k/p5p1/7p/5p2/1Q3B2/2P5/P4nPP/1R4RK w - - 1 32",
        )
        .unwrap();
        assert!(!detect_smothered_mate(&board, Color::White));
        assert!(detect_smothered_mate(&board, Color::Black));
    }

    #[test]
    fn test_vokitojunior_nf7() {
        // Vokitojunior (white) delivered Nf7# against Brexwick (black)
        let board = Board::from_str(
            "4r1rk/p1qb1Nbp/1pnp2p1/2p1p3/8/2PPB1P1/PP3PB1/R3R1K1 b - - 1 22",
        )
        .unwrap();
        assert!(!detect_smothered_mate(&board, Color::Black));
        assert!(detect_smothered_mate(&board, Color::White));
    }

    #[test]
    fn test_jungle_kid_nxd6() {
        // jungle_kid (white) delivered Nxd6# against Brexwick (black)
        let board = Board::from_str(
            "r2qkb1r/1p1bnppp/p2N4/3Pp3/Q1P5/8/PP3PPP/R1B1KB1R b KQkq - 0 11",
        )
        .unwrap();
        assert!(!detect_smothered_mate(&board, Color::Black));
        assert!(detect_smothered_mate(&board, Color::White));
    }

    #[test]
    fn test_erikcocom21_nxd6() {
        // ErikCocom21 (white) delivered Nxd6# against Brexwick (black)
        let board = Board::from_str(
            "r2qkb1r/pp1bnppp/3N4/3Pp1B1/8/8/PPP2PPP/R2QKB1R b KQkq - 0 10",
        )
        .unwrap();
        assert!(!detect_smothered_mate(&board, Color::Black));
        assert!(detect_smothered_mate(&board, Color::White));
    }

    // ── Negative cases ──

    #[test]
    fn test_normal_checkmate_not_smothered() {
        // Scholar's mate — queen delivers, not knight
        let board = Board::from_str(
            "r1bqkb1r/pppp1Qpp/2n2n2/4p3/2B1P3/8/PPPP1PPP/RNB1K1NR b KQkq - 0 4",
        )
        .unwrap();
        assert!(!detect_smothered_mate(&board, Color::White));
    }

    #[test]
    fn test_not_checkmate() {
        let board = Board::default();
        assert!(!detect_smothered_mate(&board, Color::White));
        assert!(!detect_smothered_mate(&board, Color::Black));
    }
}
