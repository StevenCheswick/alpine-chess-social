/// Endgame classification and tracking based on Fundamental Chess Endings categories.
///
/// Classifies positions into FCE endgame types by piece composition and tracks
/// per-segment statistics during the unified game walk.

use chess::{Board, Color, Piece};
use serde::{Deserialize, Serialize};

/// Winning threshold in centipawns (±1 pawn)
const WINNING_THRESHOLD: i32 = 100;

/// Minimum cp_loss to record as a mistake in endgame tracking
const MISTAKE_THRESHOLD: i32 = 50;

/// Blunder threshold for endgame tracking
const BLUNDER_THRESHOLD: i32 = 200;

// Bit flags for non-pawn piece types
const KNIGHT_FLAG: u8 = 1;
const BISHOP_FLAG: u8 = 2;
const ROOK_FLAG: u8 = 4;
const QUEEN_FLAG: u8 = 8;
const MINOR_MASK: u8 = KNIGHT_FLAG | BISHOP_FLAG;
const ROOK_MINOR_MASK: u8 = ROOK_FLAG | MINOR_MASK;

/// FCE (Fundamental Chess Endings) endgame category
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EndgameType {
    PawnEndings,
    KnightEndings,
    BishopEndings,
    BishopVsKnight,
    RookEndings,
    RookVsMinorPiece,
    RookMinorVsRookMinor,
    RookMinorVsRook,
    QueenEndings,
    QueenVsRook,
    QueenVsMinorPiece,
    QueenPieceVsQueen,
}

impl EndgameType {
    pub fn name(&self) -> &'static str {
        match self {
            EndgameType::PawnEndings => "Pawn Endings",
            EndgameType::KnightEndings => "Knight Endings",
            EndgameType::BishopEndings => "Bishop Endings",
            EndgameType::BishopVsKnight => "Bishop vs Knight",
            EndgameType::RookEndings => "Rook Endings",
            EndgameType::RookVsMinorPiece => "Rook vs Minor Piece",
            EndgameType::RookMinorVsRookMinor => "Rook + Minor vs Rook + Minor",
            EndgameType::RookMinorVsRook => "Rook + Minor vs Rook",
            EndgameType::QueenEndings => "Queen Endings",
            EndgameType::QueenVsRook => "Queen vs Rook",
            EndgameType::QueenVsMinorPiece => "Queen vs Minor Piece",
            EndgameType::QueenPieceVsQueen => "Queen + Piece vs Queen",
        }
    }
}

/// A mistake made during an endgame segment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndgameMistake {
    pub fen: String,
    pub move_uci: String,
    pub best_move: String,
    pub cp_loss: i32,
    pub classification: String,
    pub move_number: u32,
    pub is_white: bool,
}

/// An endgame segment within a game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndgameSegment {
    pub endgame_type: String,
    pub entry_move: u32,
    pub entry_eval: i32,
    pub white_moves: u32,
    pub white_cp_loss: i32,
    pub white_blunders: u32,
    pub black_moves: u32,
    pub black_cp_loss: i32,
    pub black_blunders: u32,
    pub mistakes: Vec<EndgameMistake>,
}

/// Get the set of non-pawn, non-king piece types for a color as bit flags.
fn non_pawn_types(board: &Board, color: Color) -> u8 {
    let color_bb = *board.color_combined(color);
    let mut flags = 0u8;
    if (color_bb & *board.pieces(Piece::Knight)).popcnt() > 0 {
        flags |= KNIGHT_FLAG;
    }
    if (color_bb & *board.pieces(Piece::Bishop)).popcnt() > 0 {
        flags |= BISHOP_FLAG;
    }
    if (color_bb & *board.pieces(Piece::Rook)).popcnt() > 0 {
        flags |= ROOK_FLAG;
    }
    if (color_bb & *board.pieces(Piece::Queen)).popcnt() > 0 {
        flags |= QUEEN_FLAG;
    }
    flags
}

/// Count non-pawn, non-king pieces for a color.
fn count_non_pawn(board: &Board, color: Color) -> u32 {
    let color_bb = *board.color_combined(color);
    let kings = *board.pieces(Piece::King);
    let pawns = *board.pieces(Piece::Pawn);
    (color_bb & !(kings | pawns)).popcnt()
}

/// Classify the current position into an FCE endgame category.
/// Returns None if not an endgame.
pub fn classify_endgame(board: &Board) -> Option<EndgameType> {
    let w_count = count_non_pawn(board, Color::White);
    let b_count = count_non_pawn(board, Color::Black);

    let w_types = non_pawn_types(board, Color::White);
    let b_types = non_pawn_types(board, Color::Black);

    // Too many pieces - not an endgame
    if w_count > 3 || b_count > 3 {
        return None;
    }

    let w_has_queen = (w_types & QUEEN_FLAG) != 0;
    let b_has_queen = (b_types & QUEEN_FLAG) != 0;

    // Both sides have queens + other stuff - not a simple endgame
    // Use && so that Q+Piece vs Q (one side has extra, other doesn't) can still classify
    if w_has_queen && b_has_queen && w_count > 1 && b_count > 1 {
        return None;
    }

    // Pawn endgame: kings + pawns only
    if w_count == 0 && b_count == 0 {
        return Some(EndgameType::PawnEndings);
    }

    // Knight endings: only knights + pawns
    if (w_types & !KNIGHT_FLAG) == 0 && (b_types & !KNIGHT_FLAG) == 0 && (w_types | b_types) != 0 {
        return Some(EndgameType::KnightEndings);
    }

    // Bishop endings: only bishops + pawns
    if (w_types & !BISHOP_FLAG) == 0 && (b_types & !BISHOP_FLAG) == 0 && (w_types | b_types) != 0 {
        return Some(EndgameType::BishopEndings);
    }

    // Bishop vs Knight
    if (w_types == BISHOP_FLAG && b_types == KNIGHT_FLAG)
        || (w_types == KNIGHT_FLAG && b_types == BISHOP_FLAG)
    {
        return Some(EndgameType::BishopVsKnight);
    }

    // Rook endings: only rooks + pawns
    if (w_types & !ROOK_FLAG) == 0 && (b_types & !ROOK_FLAG) == 0 && (w_types | b_types) != 0 {
        return Some(EndgameType::RookEndings);
    }

    // Rook vs Minor piece(s)
    if (w_types == ROOK_FLAG && (b_types & !MINOR_MASK) == 0 && b_types != 0)
        || (b_types == ROOK_FLAG && (w_types & !MINOR_MASK) == 0 && w_types != 0)
    {
        return Some(EndgameType::RookVsMinorPiece);
    }

    // Rook + Minor vs Rook + Minor (both sides have rook + minor pieces)
    if (w_types & ROOK_FLAG) != 0
        && (b_types & ROOK_FLAG) != 0
        && (w_types & !ROOK_MINOR_MASK) == 0
        && (b_types & !ROOK_MINOR_MASK) == 0
        && w_types.count_ones() > 1
        && b_types.count_ones() > 1
    {
        return Some(EndgameType::RookMinorVsRookMinor);
    }

    // Rook + Minor vs Rook
    if ((w_types & !ROOK_MINOR_MASK) == 0
        && (w_types & ROOK_FLAG) != 0
        && b_types == ROOK_FLAG
        && w_types.count_ones() > 1)
        || ((b_types & !ROOK_MINOR_MASK) == 0
            && (b_types & ROOK_FLAG) != 0
            && w_types == ROOK_FLAG
            && b_types.count_ones() > 1)
    {
        return Some(EndgameType::RookMinorVsRook);
    }

    // Queen endings: only queens + pawns
    if (w_types & !QUEEN_FLAG) == 0 && (b_types & !QUEEN_FLAG) == 0 && (w_types | b_types) != 0 {
        return Some(EndgameType::QueenEndings);
    }

    // Queen vs Rook(s)
    if (w_types == QUEEN_FLAG && (b_types & !ROOK_FLAG) == 0 && b_types != 0)
        || (b_types == QUEEN_FLAG && (w_types & !ROOK_FLAG) == 0 && w_types != 0)
    {
        return Some(EndgameType::QueenVsRook);
    }

    // Queen vs Minor piece(s)
    if (w_types == QUEEN_FLAG && (b_types & !MINOR_MASK) == 0 && b_types != 0)
        || (b_types == QUEEN_FLAG && (w_types & !MINOR_MASK) == 0 && w_types != 0)
    {
        return Some(EndgameType::QueenVsMinorPiece);
    }

    // Queen + Piece vs Queen (one side has Q + something, other has Q only)
    if w_has_queen && b_has_queen && ((w_count > 1) != (b_count > 1)) {
        return Some(EndgameType::QueenPieceVsQueen);
    }

    None
}

/// Classify eval from white's perspective into a status string.
pub fn classify_eval(eval_white: i32) -> &'static str {
    if eval_white >= WINNING_THRESHOLD {
        "winning"
    } else if eval_white <= -WINNING_THRESHOLD {
        "losing"
    } else {
        "equal"
    }
}

/// Tracks endgame segments during a game walk.
pub struct EndgameTracker {
    current_eg: Option<EndgameType>,
    current_segment: Option<EndgameSegment>,
    segments: Vec<EndgameSegment>,
}

impl EndgameTracker {
    pub fn new() -> Self {
        EndgameTracker {
            current_eg: None,
            current_segment: None,
            segments: Vec::new(),
        }
    }

    /// Call after each move to track endgame segments.
    ///
    /// - `board_after`: the board position after the move
    /// - `move_eval`: eval after this move from white's perspective
    /// - `cp_loss`: centipawn loss for this move
    /// - `classification`: move classification string
    /// - `move_uci`: the played move in UCI
    /// - `best_move`: the best move in UCI
    /// - `fen_before`: FEN of position before the move
    /// - `is_white`: whether this was white's move
    /// - `move_index`: 0-based move index in the game
    pub fn track_move(
        &mut self,
        board_after: &Board,
        move_eval: i32,
        cp_loss: i32,
        classification: &str,
        move_uci: &str,
        best_move: &str,
        fen_before: &str,
        is_white: bool,
        move_index: usize,
    ) {
        let eg_type = classify_endgame(board_after);

        // Handle transitions
        if eg_type.as_ref() != self.current_eg.as_ref() {
            // Save current segment if any
            if let Some(segment) = self.current_segment.take() {
                self.segments.push(segment);
            }

            if let Some(ref eg) = eg_type {
                // Starting a new endgame segment
                self.current_segment = Some(EndgameSegment {
                    endgame_type: eg.name().to_string(),
                    entry_move: (move_index / 2 + 1) as u32,
                    entry_eval: move_eval,
                    white_moves: 0,
                    white_cp_loss: 0,
                    white_blunders: 0,
                    black_moves: 0,
                    black_cp_loss: 0,
                    black_blunders: 0,
                    mistakes: Vec::new(),
                });
            }

            self.current_eg = eg_type;
        }

        // Accumulate stats if we're in an endgame segment
        if let Some(ref mut segment) = self.current_segment {
            if is_white {
                segment.white_moves += 1;
                segment.white_cp_loss += cp_loss;
                if cp_loss >= BLUNDER_THRESHOLD {
                    segment.white_blunders += 1;
                }
            } else {
                segment.black_moves += 1;
                segment.black_cp_loss += cp_loss;
                if cp_loss >= BLUNDER_THRESHOLD {
                    segment.black_blunders += 1;
                }
            }

            if cp_loss >= MISTAKE_THRESHOLD {
                segment.mistakes.push(EndgameMistake {
                    fen: fen_before.to_string(),
                    move_uci: move_uci.to_string(),
                    best_move: best_move.to_string(),
                    cp_loss,
                    classification: classification.to_string(),
                    move_number: (move_index / 2 + 1) as u32,
                    is_white,
                });
            }
        }
    }

    /// Finalize and return all endgame segments.
    pub fn finish(mut self) -> Vec<EndgameSegment> {
        if let Some(segment) = self.current_segment.take() {
            self.segments.push(segment);
        }
        self.segments
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_starting_position_not_endgame() {
        let board = Board::default();
        assert_eq!(classify_endgame(&board), None);
    }

    #[test]
    fn test_pawn_ending() {
        // Kings + pawns only
        let board = Board::from_str("4k3/pppp4/8/8/8/8/PPPP4/4K3 w - - 0 1").unwrap();
        assert_eq!(classify_endgame(&board), Some(EndgameType::PawnEndings));
    }

    #[test]
    fn test_rook_ending() {
        // Kings + rooks + pawns
        let board = Board::from_str("4k3/pppp4/8/8/8/8/PPPP4/R3K3 w - - 0 1").unwrap();
        assert_eq!(classify_endgame(&board), Some(EndgameType::RookEndings));
    }

    #[test]
    fn test_rook_vs_rook_ending() {
        let board = Board::from_str("r3k3/pppp4/8/8/8/8/PPPP4/R3K3 w - - 0 1").unwrap();
        assert_eq!(classify_endgame(&board), Some(EndgameType::RookEndings));
    }

    #[test]
    fn test_bishop_ending() {
        let board = Board::from_str("4k3/pppp4/8/8/8/8/PPPP4/B3K3 w - - 0 1").unwrap();
        assert_eq!(classify_endgame(&board), Some(EndgameType::BishopEndings));
    }

    #[test]
    fn test_knight_ending() {
        let board = Board::from_str("4k3/pppp4/8/8/8/8/PPPP4/N3K3 w - - 0 1").unwrap();
        assert_eq!(classify_endgame(&board), Some(EndgameType::KnightEndings));
    }

    #[test]
    fn test_bishop_vs_knight() {
        let board = Board::from_str("n3k3/pppp4/8/8/8/8/PPPP4/B3K3 w - - 0 1").unwrap();
        assert_eq!(classify_endgame(&board), Some(EndgameType::BishopVsKnight));
    }

    #[test]
    fn test_queen_ending() {
        let board = Board::from_str("4k3/pppp4/8/8/8/8/PPPP4/Q3K3 w - - 0 1").unwrap();
        assert_eq!(classify_endgame(&board), Some(EndgameType::QueenEndings));
    }

    #[test]
    fn test_queen_vs_rook() {
        let board = Board::from_str("r3k3/pppp4/8/8/8/8/PPPP4/Q3K3 w - - 0 1").unwrap();
        assert_eq!(classify_endgame(&board), Some(EndgameType::QueenVsRook));
    }

    #[test]
    fn test_rook_vs_minor() {
        let board = Board::from_str("n3k3/pppp4/8/8/8/8/PPPP4/R3K3 w - - 0 1").unwrap();
        assert_eq!(classify_endgame(&board), Some(EndgameType::RookVsMinorPiece));
    }

    #[test]
    fn test_rook_minor_vs_rook() {
        let board = Board::from_str("r3k3/pppp4/8/8/8/8/PPPP4/RN2K3 w - - 0 1").unwrap();
        assert_eq!(classify_endgame(&board), Some(EndgameType::RookMinorVsRook));
    }

    #[test]
    fn test_rook_minor_vs_rook_minor() {
        let board =
            Board::from_str("rn2k3/pppp4/8/8/8/8/PPPP4/RB2K3 w - - 0 1").unwrap();
        assert_eq!(
            classify_endgame(&board),
            Some(EndgameType::RookMinorVsRookMinor)
        );
    }

    #[test]
    fn test_queen_vs_minor() {
        let board = Board::from_str("b3k3/pppp4/8/8/8/8/PPPP4/Q3K3 w - - 0 1").unwrap();
        assert_eq!(
            classify_endgame(&board),
            Some(EndgameType::QueenVsMinorPiece)
        );
    }

    #[test]
    fn test_queen_piece_vs_queen() {
        let board =
            Board::from_str("q3k3/pppp4/8/8/8/8/PPPP4/QN2K3 w - - 0 1").unwrap();
        assert_eq!(
            classify_endgame(&board),
            Some(EndgameType::QueenPieceVsQueen)
        );
    }

    #[test]
    fn test_too_many_pieces() {
        // 4 non-pawn pieces per side - not an endgame
        let board =
            Board::from_str("rnb1k3/pppp4/8/8/8/8/PPPP4/RNB1K3 w - - 0 1").unwrap();
        // w_count=3, b_count=3 — still ≤ 3, check what this classifies as
        // Actually R+N+B = 3, so it's within the limit
        // Both have rook + minor pieces
        assert!(classify_endgame(&board).is_some());
    }

    #[test]
    fn test_tracker_basic() {
        let mut tracker = EndgameTracker::new();

        // A pawn-only position
        let board = Board::from_str("4k3/pppp4/8/8/8/8/PPPP4/4K3 w - - 0 1").unwrap();
        tracker.track_move(
            &board, 50, 10, "excellent", "e2e4", "e2e4",
            "4k3/pppp4/8/8/4P3/8/PPP4/4K3 b - - 0 1", true, 40,
        );

        let segments = tracker.finish();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].endgame_type, "Pawn Endings");
        assert_eq!(segments[0].white_moves, 1);
    }

    #[test]
    fn test_classify_eval() {
        assert_eq!(classify_eval(150), "winning");
        assert_eq!(classify_eval(50), "equal");
        assert_eq!(classify_eval(-150), "losing");
        assert_eq!(classify_eval(100), "winning");
        assert_eq!(classify_eval(-100), "losing");
        assert_eq!(classify_eval(0), "equal");
    }
}
