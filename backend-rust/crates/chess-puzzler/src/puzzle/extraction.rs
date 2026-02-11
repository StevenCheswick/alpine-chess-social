/// Puzzle extraction constants and UCI move parsing
/// (Trimmed: no Stockfish/Engine dependencies)

use chess::{Board, ChessMove, File, Piece, Rank, Square};

/// Minimum CP loss to consider a move a blunder (puzzle candidate)
pub const BLUNDER_THRESHOLD: i32 = 200;

/// Minimum CP for the puzzle position to be interesting
pub const MIN_PUZZLE_CP: i32 = 100;

/// Maximum puzzle line length (in half-moves)
pub const MAX_PUZZLE_LENGTH: usize = 20;

/// Minimum puzzle line length
pub const MIN_PUZZLE_LENGTH: usize = 2;

/// Parse a UCI move string against a board position
pub fn parse_uci_move(_board: &Board, uci: &str) -> Option<ChessMove> {
    if uci.len() < 4 {
        return None;
    }

    let bytes = uci.as_bytes();
    let from = Square::make_square(
        Rank::from_index(bytes[1] as usize - b'1' as usize),
        File::from_index(bytes[0] as usize - b'a' as usize),
    );
    let to = Square::make_square(
        Rank::from_index(bytes[3] as usize - b'1' as usize),
        File::from_index(bytes[2] as usize - b'a' as usize),
    );

    let promotion = if uci.len() > 4 {
        match uci.as_bytes()[4] {
            b'q' | b'Q' => Some(Piece::Queen),
            b'r' | b'R' => Some(Piece::Rook),
            b'b' | b'B' => Some(Piece::Bishop),
            b'n' | b'N' => Some(Piece::Knight),
            _ => None,
        }
    } else {
        None
    };

    Some(ChessMove::new(from, to, promotion))
}
