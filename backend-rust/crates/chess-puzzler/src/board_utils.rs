/// Board utility functions for tactical analysis
/// Port of tagger/util.py

use chess::{
    BitBoard, Board, ChessMove, Color, File, MoveGen, Piece, Rank, Square,
    EMPTY,
};

// Piece values for material calculation
pub const PAWN_VALUE: i32 = 1;
pub const KNIGHT_VALUE: i32 = 3;
pub const BISHOP_VALUE: i32 = 3;
pub const ROOK_VALUE: i32 = 5;
pub const QUEEN_VALUE: i32 = 9;
pub const KING_VALUE: i32 = 99;

/// Piece value (no king)
pub fn piece_value(piece: Piece) -> i32 {
    match piece {
        Piece::Pawn => PAWN_VALUE,
        Piece::Knight => KNIGHT_VALUE,
        Piece::Bishop => BISHOP_VALUE,
        Piece::Rook => ROOK_VALUE,
        Piece::Queen => QUEEN_VALUE,
        Piece::King => 0,
    }
}

/// Piece value including king (for fork detection etc)
pub fn king_value(piece: Piece) -> i32 {
    match piece {
        Piece::King => KING_VALUE,
        other => piece_value(other),
    }
}

/// Is this a ray (sliding) piece type?
pub fn is_ray_piece(piece: Piece) -> bool {
    matches!(piece, Piece::Queen | Piece::Rook | Piece::Bishop)
}

/// Get squares attacked by a piece on a given square
/// This is the equivalent of python-chess board.attacks(square)
pub fn attacks(board: &Board, square: Square) -> BitBoard {
    let piece = match board.piece_on(square) {
        Some(p) => p,
        None => return EMPTY,
    };

    match piece {
        Piece::Pawn => {
            let color = board.color_on(square).unwrap();
            pawn_attacks(square, color)
        }
        Piece::Knight => chess::get_knight_moves(square),
        Piece::King => chess::get_king_moves(square),
        Piece::Bishop => chess::get_bishop_moves(square, *board.combined()),
        Piece::Rook => chess::get_rook_moves(square, *board.combined()),
        Piece::Queen => {
            chess::get_bishop_moves(square, *board.combined())
                | chess::get_rook_moves(square, *board.combined())
        }
    }
}

/// Pawn attack squares (just the diagonal attacks, not pushes)
pub fn pawn_attacks(square: Square, color: Color) -> BitBoard {
    let file = square.get_file().to_index();
    let rank = square.get_rank().to_index();

    let mut result = EMPTY;

    match color {
        Color::White => {
            if rank < 7 {
                if file > 0 {
                    result |= BitBoard::from_square(Square::make_square(
                        Rank::from_index(rank + 1),
                        File::from_index(file - 1),
                    ));
                }
                if file < 7 {
                    result |= BitBoard::from_square(Square::make_square(
                        Rank::from_index(rank + 1),
                        File::from_index(file + 1),
                    ));
                }
            }
        }
        Color::Black => {
            if rank > 0 {
                if file > 0 {
                    result |= BitBoard::from_square(Square::make_square(
                        Rank::from_index(rank - 1),
                        File::from_index(file - 1),
                    ));
                }
                if file < 7 {
                    result |= BitBoard::from_square(Square::make_square(
                        Rank::from_index(rank - 1),
                        File::from_index(file + 1),
                    ));
                }
            }
        }
    }

    result
}

/// Get all pieces of a given color that attack a square
/// Equivalent of python-chess board.attackers(color, square)
pub fn attackers(board: &Board, color: Color, square: Square) -> BitBoard {
    let occupied = *board.combined();
    let color_pieces = *board.color_combined(color);

    let mut result = EMPTY;

    // Pawns: reverse lookup — pawn attacks FROM the target square
    // with the OPPOSITE color, then intersect with actual pawns
    let pawn_atk = pawn_attacks(square, !color);
    result |= pawn_atk & *board.pieces(Piece::Pawn) & color_pieces;

    // Knights
    let knight_atk = chess::get_knight_moves(square);
    result |= knight_atk & *board.pieces(Piece::Knight) & color_pieces;

    // King
    let king_atk = chess::get_king_moves(square);
    result |= king_atk & *board.pieces(Piece::King) & color_pieces;

    // Bishops (and queen diagonals)
    let bishop_atk = chess::get_bishop_moves(square, occupied);
    result |= bishop_atk & (*board.pieces(Piece::Bishop) | *board.pieces(Piece::Queen)) & color_pieces;

    // Rooks (and queen ranks/files)
    let rook_atk = chess::get_rook_moves(square, occupied);
    result |= rook_atk & (*board.pieces(Piece::Rook) | *board.pieces(Piece::Queen)) & color_pieces;

    result
}

/// Get the pin direction for a piece. Returns the ray mask if pinned,
/// or BitBoard::new(0xFFFFFFFFFFFFFFFF) if not pinned.
/// This is the equivalent of python-chess board.pin(color, square)
pub fn pin_direction(board: &Board, color: Color, square: Square) -> BitBoard {
    let king_sq = king_square(board, color);
    let bb_all = BitBoard::new(0xFFFF_FFFF_FFFF_FFFF);

    // If the piece is not on a line with the king, it can't be pinned
    let _between_king = chess::between(king_sq, square);
    let line = chess::line(king_sq, square);
    if line == EMPTY {
        return bb_all; // not on same ray, not pinned
    }

    // Check if there's exactly one piece between king and a sliding attacker on this ray
    let enemy_color = !color;
    let occupied = *board.combined();

    // Determine if this is a diagonal or orthogonal ray
    let king_file = king_sq.get_file().to_index() as i32;
    let king_rank = king_sq.get_rank().to_index() as i32;
    let sq_file = square.get_file().to_index() as i32;
    let sq_rank = square.get_rank().to_index() as i32;

    let is_diagonal = (king_file - sq_file).abs() == (king_rank - sq_rank).abs();

    // Find potential pinners (sliding pieces on the ray beyond the square)
    let pinners = if is_diagonal {
        (*board.pieces(Piece::Bishop) | *board.pieces(Piece::Queen)) & *board.color_combined(enemy_color)
    } else {
        (*board.pieces(Piece::Rook) | *board.pieces(Piece::Queen)) & *board.color_combined(enemy_color)
    };

    // Check each potential pinner
    for pinner_sq in pinners {
        let pinner_line = chess::line(king_sq, pinner_sq);
        if pinner_line == EMPTY || pinner_line != line {
            continue;
        }

        // Count pieces between king and pinner
        let between_pinner = chess::between(king_sq, pinner_sq);
        let blockers = between_pinner & occupied;

        if blockers.popcnt() == 1 {
            // Only one piece between king and pinner — check if it's our square
            if (BitBoard::from_square(square) & blockers).popcnt() == 1 {
                return line;
            }
        }
    }

    bb_all // not pinned
}

/// Find the king square for a color
pub fn king_square(board: &Board, color: Color) -> Square {
    let king_bb = *board.pieces(Piece::King) & *board.color_combined(color);
    // There should always be exactly one king
    debug_assert_eq!(king_bb.popcnt(), 1);
    // BitBoard implements IntoIterator
    king_bb.to_square()
}

/// What piece type moved? (look at destination after the move)
pub fn moved_piece_type(board_after: &Board, m: ChessMove) -> Option<Piece> {
    board_after.piece_on(m.get_dest())
}

/// Is this move an advanced pawn move?
pub fn is_advanced_pawn_move(board_after: &Board, m: ChessMove, side_to_move_after: Color) -> bool {
    if m.get_promotion().is_some() {
        return true;
    }
    if board_after.piece_on(m.get_dest()) != Some(Piece::Pawn) {
        return false;
    }
    let to_rank = m.get_dest().get_rank().to_index();
    // side_to_move_after is the color AFTER the move was made (i.e., opponent's turn)
    // So the mover is !side_to_move_after
    let mover = !side_to_move_after;
    match mover {
        Color::White => to_rank >= 5, // rank 6, 7, 8 (index 5, 6, 7)
        Color::Black => to_rank <= 2, // rank 1, 2, 3 (index 0, 1, 2)
    }
}

/// Is this a very advanced pawn move? (rank 7/8 for white, rank 1/2 for black)
pub fn is_very_advanced_pawn_move(board_after: &Board, m: ChessMove, side_to_move_after: Color) -> bool {
    if !is_advanced_pawn_move(board_after, m, side_to_move_after) {
        return false;
    }
    let to_rank = m.get_dest().get_rank().to_index();
    let mover = !side_to_move_after;
    match mover {
        Color::White => to_rank >= 6,
        Color::Black => to_rank <= 1,
    }
}

/// Is a piece defended?
/// Checks direct defenders AND ray defense (x-ray through an attacker)
pub fn is_defended(board: &Board, color: Color, square: Square) -> bool {
    // Direct defenders
    if attackers(board, color, square) != EMPTY {
        return true;
    }

    // Ray defense: remove each enemy attacker and check if we then have a defender
    let enemy_attackers = attackers(board, !color, square);
    for att_sq in enemy_attackers {
        if let Some(att_piece) = board.piece_on(att_sq) {
            if is_ray_piece(att_piece) {
                // Remove the attacker and check for defenders behind it
                // We do this by checking if there's a same-color ray piece
                // on the ray beyond the attacker
                let behind = chess::between(att_sq, square);
                let _ = behind; // for ray defense we need to simulate removal
                // Simpler approach: make a board without the attacker
                let new_board = *board;
                // chess crate Board doesn't have remove_piece_at, so we create
                // a position via FEN manipulation or use combined bitboard tricks.
                // For now, use a simplified check: look for defenders along the ray
                // behind the enemy attacker
                let ray_from_sq = chess::line(square, att_sq);
                if ray_from_sq != EMPTY {
                    // Check for friendly ray pieces on this line beyond the attacker
                    let beyond_attacker = ray_from_sq & !chess::between(square, att_sq) & !BitBoard::from_square(square) & !BitBoard::from_square(att_sq);
                    let friendly_on_ray = beyond_attacker & *board.color_combined(color);
                    for friend_sq in friendly_on_ray {
                        if let Some(friend_piece) = board.piece_on(friend_sq) {
                            if is_ray_piece(friend_piece) {
                                // Check the piece can actually move along this ray
                                let is_diag = {
                                    let df = (square.get_file().to_index() as i32 - att_sq.get_file().to_index() as i32).abs();
                                    let dr = (square.get_rank().to_index() as i32 - att_sq.get_rank().to_index() as i32).abs();
                                    df == dr
                                };
                                let can_slide = match friend_piece {
                                    Piece::Bishop => is_diag,
                                    Piece::Rook => !is_diag,
                                    Piece::Queen => true,
                                    _ => false,
                                };
                                if can_slide {
                                    // Check no pieces between friend and attacker
                                    let between_friend_att = chess::between(friend_sq, att_sq);
                                    if (between_friend_att & *board.combined()).popcnt() == 0 {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
                let _ = new_board;
            }
        }
    }

    false
}

/// Is a piece hanging (not defended)?
pub fn is_hanging(board: &Board, color: Color, square: Square) -> bool {
    !is_defended(board, color, square)
}

/// Can a piece be taken by a lower-value piece?
pub fn can_be_taken_by_lower_piece(board: &Board, piece: Piece, color: Color, square: Square) -> bool {
    let enemy_attackers = attackers(board, !color, square);
    for att_sq in enemy_attackers {
        if let Some(att_piece) = board.piece_on(att_sq) {
            if att_piece != Piece::King && piece_value(att_piece) < piece_value(piece) {
                return true;
            }
        }
    }
    false
}

/// Is a piece in a bad spot (hanging or takeable by lower piece)?
pub fn is_in_bad_spot(board: &Board, square: Square) -> bool {
    let piece = match board.piece_on(square) {
        Some(p) => p,
        None => return false,
    };
    let color = match board.color_on(square) {
        Some(c) => c,
        None => return false,
    };

    let enemy_attacks = attackers(board, !color, square);
    if enemy_attacks == EMPTY {
        return false;
    }

    is_hanging(board, color, square) || can_be_taken_by_lower_piece(board, piece, color, square)
}

/// Is a piece trapped? (in a bad spot and all escape squares are also bad)
pub fn is_trapped(board: &Board, square: Square) -> bool {
    // Can't be trapped if in check or pinned
    if board.checkers().popcnt() > 0 {
        return false;
    }
    if (*board.pinned() & BitBoard::from_square(square)).popcnt() > 0 {
        return false;
    }

    let piece = match board.piece_on(square) {
        Some(p) => p,
        None => return false,
    };

    // Pawns and kings can't be "trapped" in the tactical sense
    if piece == Piece::Pawn || piece == Piece::King {
        return false;
    }

    if !is_in_bad_spot(board, square) {
        return false;
    }

    // Check all legal moves from this square
    let legal = MoveGen::new_legal(board);
    for m in legal {
        if m.get_source() == square {
            // Can capture a piece of equal or greater value — not trapped
            if let Some(captured) = board.piece_on(m.get_dest()) {
                if piece_value(captured) >= piece_value(piece) {
                    return false;
                }
            }
            // Check if the destination is safe
            let new_board = board.make_move_new(m);
            if !is_in_bad_spot(&new_board, m.get_dest()) {
                return false;
            }
        }
    }

    true
}

/// Count material for one side
pub fn material_count(board: &Board, color: Color) -> i32 {
    let color_bb = *board.color_combined(color);
    let pawns = (*board.pieces(Piece::Pawn) & color_bb).popcnt() as i32;
    let knights = (*board.pieces(Piece::Knight) & color_bb).popcnt() as i32;
    let bishops = (*board.pieces(Piece::Bishop) & color_bb).popcnt() as i32;
    let rooks = (*board.pieces(Piece::Rook) & color_bb).popcnt() as i32;
    let queens = (*board.pieces(Piece::Queen) & color_bb).popcnt() as i32;

    pawns * PAWN_VALUE
        + knights * KNIGHT_VALUE
        + bishops * BISHOP_VALUE
        + rooks * ROOK_VALUE
        + queens * QUEEN_VALUE
}

/// Material difference (positive = side has more)
pub fn material_diff(board: &Board, side: Color) -> i32 {
    material_count(board, side) - material_count(board, !side)
}

/// Get opponent pieces attacked from a square
pub fn attacked_opponent_squares(
    board: &Board,
    from_square: Square,
    pov: Color,
) -> Vec<(Piece, Color, Square)> {
    let mut result = Vec::new();
    let atk = attacks(board, from_square);

    for sq in atk {
        if let (Some(piece), Some(color)) = (board.piece_on(sq), board.color_on(sq)) {
            if color != pov {
                result.push((piece, color, sq));
            }
        }
    }

    result
}

/// Get opponent pieces attacked from a square (just the pieces, no squares)
pub fn attacked_opponent_pieces(
    board: &Board,
    from_square: Square,
    pov: Color,
) -> Vec<Piece> {
    attacked_opponent_squares(board, from_square, pov)
        .into_iter()
        .map(|(piece, _, _)| piece)
        .collect()
}

/// Get all pieces of a color that attack a square (as Piece list)
pub fn attacker_pieces(board: &Board, color: Color, square: Square) -> Vec<Piece> {
    let atk_bb = attackers(board, color, square);
    let mut result = Vec::new();
    for sq in atk_bb {
        if let Some(piece) = board.piece_on(sq) {
            result.push(piece);
        }
    }
    result
}

/// Distance between two squares (Chebyshev distance)
pub fn square_distance(s1: Square, s2: Square) -> u32 {
    let r1 = s1.get_rank().to_index() as i32;
    let r2 = s2.get_rank().to_index() as i32;
    let f1 = s1.get_file().to_index() as i32;
    let f2 = s2.get_file().to_index() as i32;
    (r1 - r2).unsigned_abs().max((f1 - f2).unsigned_abs())
}

/// Is a move castling?
pub fn is_castling_move(board: &Board, m: ChessMove) -> bool {
    if let Some(piece) = board.piece_on(m.get_source()) {
        if piece == Piece::King {
            let from_file = m.get_source().get_file().to_index() as i32;
            let to_file = m.get_dest().get_file().to_index() as i32;
            return (from_file - to_file).abs() > 1;
        }
    }
    false
}

/// Squares adjacent to a king (distance == 1)
pub fn king_adjacent_squares(king: Square) -> Vec<Square> {
    let mut result = Vec::new();
    let king_moves = chess::get_king_moves(king);
    for sq in king_moves {
        result.push(sq);
    }
    result
}

/// Piece map: all (square, piece, color) tuples
pub fn piece_map(board: &Board) -> Vec<(Square, Piece, Color)> {
    let mut result = Vec::new();
    for sq in *board.combined() {
        if let (Some(piece), Some(color)) = (board.piece_on(sq), board.color_on(sq)) {
            result.push((sq, piece, color));
        }
    }
    result
}

/// Count of all pieces on the board
pub fn piece_map_count(board: &Board) -> u32 {
    board.combined().popcnt()
}

/// Helper: get between squares (re-export from chess crate)
pub fn between(s1: Square, s2: Square) -> BitBoard {
    chess::between(s1, s2)
}

/// Helper: get full line through two squares
pub fn line(s1: Square, s2: Square) -> BitBoard {
    chess::line(s1, s2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_king_square() {
        let board = Board::default();
        assert_eq!(king_square(&board, Color::White), Square::make_square(Rank::First, File::E));
        assert_eq!(king_square(&board, Color::Black), Square::make_square(Rank::Eighth, File::E));
    }

    #[test]
    fn test_material_count_starting() {
        let board = Board::default();
        // 8 pawns + 2 knights + 2 bishops + 2 rooks + 1 queen = 8+6+6+10+9 = 39
        assert_eq!(material_count(&board, Color::White), 39);
        assert_eq!(material_count(&board, Color::Black), 39);
        assert_eq!(material_diff(&board, Color::White), 0);
    }

    #[test]
    fn test_attackers_starting_position() {
        let board = Board::default();
        // e2 pawn attacks d3 and f3
        let e2 = Square::make_square(Rank::Second, File::E);
        let d3 = Square::make_square(Rank::Third, File::D);
        let f3 = Square::make_square(Rank::Third, File::F);

        let atk = attacks(&board, e2);
        assert!((atk & BitBoard::from_square(d3)).popcnt() > 0);
        assert!((atk & BitBoard::from_square(f3)).popcnt() > 0);
    }

    #[test]
    fn test_pawn_attacks() {
        let e4 = Square::make_square(Rank::Fourth, File::E);
        let white_atk = pawn_attacks(e4, Color::White);
        let d5 = Square::make_square(Rank::Fifth, File::D);
        let f5 = Square::make_square(Rank::Fifth, File::F);
        assert!((white_atk & BitBoard::from_square(d5)).popcnt() > 0);
        assert!((white_atk & BitBoard::from_square(f5)).popcnt() > 0);
        assert_eq!(white_atk.popcnt(), 2);
    }

    #[test]
    fn test_square_distance() {
        let e1 = Square::make_square(Rank::First, File::E);
        let e4 = Square::make_square(Rank::Fourth, File::E);
        assert_eq!(square_distance(e1, e4), 3);

        let a1 = Square::make_square(Rank::First, File::A);
        let h8 = Square::make_square(Rank::Eighth, File::H);
        assert_eq!(square_distance(a1, h8), 7);
    }

    #[test]
    fn test_attackers_reverse_lookup() {
        // Position where white knight on f3 attacks e5
        let board = Board::from_str("rnbqkbnr/pppppppp/8/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 1 2").unwrap();
        let e5 = Square::make_square(Rank::Fifth, File::E);
        let white_attackers = attackers(&board, Color::White, e5);
        let f3 = Square::make_square(Rank::Third, File::F);
        assert!((white_attackers & BitBoard::from_square(f3)).popcnt() > 0);
    }
}
