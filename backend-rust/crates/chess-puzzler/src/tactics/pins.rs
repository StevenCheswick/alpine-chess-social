/// Pin detectors: pin_prevents_attack, pin_prevents_escape
/// Port of cook.py

use chess::{BitBoard, Color, Piece, Rank, File, Square, EMPTY};

use crate::board_utils::{attackers, attacks, pin_direction, piece_value, is_hanging};
use crate::puzzle::Puzzle;

/// Generate pseudo-legal move destinations for a piece (matching Python's board.pseudo_legal_moves).
/// For non-pawns: attack squares minus own-piece squares.
/// For pawns: diagonal captures only where enemy exists (or en passant) + forward pushes.
fn pseudo_legal_dests(board: &chess::Board, sq: Square, piece: Piece, color: Color) -> BitBoard {
    if piece != Piece::Pawn {
        return attacks(board, sq);
    }

    // Pawn: build move set manually to match python-chess pseudo_legal_moves
    let mut result = EMPTY;
    let rank = sq.get_rank().to_index();
    let file = sq.get_file().to_index();
    let occupied = *board.combined();
    let enemy = *board.color_combined(!color);
    let ep_square = board.en_passant();

    // Diagonal captures: only where enemy piece exists or en passant
    let diag_attacks = crate::board_utils::pawn_attacks(sq, color);
    for diag_sq in diag_attacks {
        if (enemy & BitBoard::from_square(diag_sq)) != EMPTY {
            result |= BitBoard::from_square(diag_sq);
        } else if let Some(ep) = ep_square {
            // En passant square: the ep square in the chess crate is the square
            // of the pawn that can be captured, but we check the target square
            let ep_target = match color {
                Color::White => Square::make_square(Rank::from_index(ep.get_rank().to_index() + 1), ep.get_file()),
                Color::Black => Square::make_square(Rank::from_index(ep.get_rank().to_index() - 1), ep.get_file()),
            };
            if diag_sq == ep_target {
                result |= BitBoard::from_square(diag_sq);
            }
        }
    }

    // Forward pushes
    match color {
        Color::White => {
            if rank < 7 {
                let one_ahead = Square::make_square(Rank::from_index(rank + 1), File::from_index(file));
                if (occupied & BitBoard::from_square(one_ahead)) == EMPTY {
                    result |= BitBoard::from_square(one_ahead);
                    if rank == 1 {
                        let two_ahead = Square::make_square(Rank::from_index(rank + 2), File::from_index(file));
                        if (occupied & BitBoard::from_square(two_ahead)) == EMPTY {
                            result |= BitBoard::from_square(two_ahead);
                        }
                    }
                }
            }
        }
        Color::Black => {
            if rank > 0 {
                let one_ahead = Square::make_square(Rank::from_index(rank - 1), File::from_index(file));
                if (occupied & BitBoard::from_square(one_ahead)) == EMPTY {
                    result |= BitBoard::from_square(one_ahead);
                    if rank == 6 {
                        let two_ahead = Square::make_square(Rank::from_index(rank - 2), File::from_index(file));
                        if (occupied & BitBoard::from_square(two_ahead)) == EMPTY {
                            result |= BitBoard::from_square(two_ahead);
                        }
                    }
                }
            }
        }
    }
    result
}

const BB_ALL: u64 = 0xFFFF_FFFF_FFFF_FFFF;

/// The pinned piece can't attack a player piece
pub fn pin_prevents_attack(puzzle: &Puzzle) -> bool {
    for node in puzzle.solver_moves() {
        let board = &node.board_after;

        for sq in *board.combined() {
            let piece = match board.piece_on(sq) {
                Some(p) => p,
                None => continue,
            };
            let color = match board.color_on(sq) {
                Some(c) => c,
                None => continue,
            };

            if color == puzzle.pov {
                continue;
            }

            let pin_dir = pin_direction(board, color, sq);
            if pin_dir == BitBoard::new(BB_ALL) {
                continue; // not pinned
            }

            // Check if the pinned piece attacks any of our pieces outside the pin line
            let piece_attacks = attacks(board, sq);
            for atk_sq in piece_attacks {
                if let (Some(attacked_piece), Some(attacked_color)) =
                    (board.piece_on(atk_sq), board.color_on(atk_sq))
                {
                    if attacked_color == puzzle.pov
                        && (pin_dir & BitBoard::from_square(atk_sq)) == EMPTY
                    {
                        // The attack is prevented by the pin
                        if piece_value(attacked_piece) > piece_value(piece)
                            || is_hanging(board, attacked_color, atk_sq)
                        {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

/// The pinned piece can't escape the attack
pub fn pin_prevents_escape(puzzle: &Puzzle) -> bool {
    for node in puzzle.solver_moves() {
        let board = &node.board_after;

        for sq in *board.combined() {
            let pinned_piece = match board.piece_on(sq) {
                Some(p) => p,
                None => continue,
            };
            let color = match board.color_on(sq) {
                Some(c) => c,
                None => continue,
            };

            if color == puzzle.pov {
                continue;
            }

            let pin_dir = pin_direction(board, color, sq);
            if pin_dir == BitBoard::new(BB_ALL) {
                continue; // not pinned
            }

            // Check if there's an attacker along the pin line
            let our_attackers = attackers(board, puzzle.pov, sq);
            for att_sq in our_attackers {
                if (pin_dir & BitBoard::from_square(att_sq)) == EMPTY {
                    continue; // attacker not on pin line
                }

                if let Some(att_piece) = board.piece_on(att_sq) {
                    // Pinned piece worth more than attacker
                    if piece_value(pinned_piece) > piece_value(att_piece) {
                        return true;
                    }

                    // Pinned piece is hanging and can't escape along the pin
                    if is_hanging(board, color, sq)
                        && (attackers(board, !puzzle.pov, att_sq) & BitBoard::from_square(sq)) == EMPTY
                    {
                        // Check if piece has pseudo-legal moves outside pin direction
                        // Use pseudo-legal destinations (ignores pins) to match Python's board.pseudo_legal_moves
                        let piece_dests = pseudo_legal_dests(board, sq, pinned_piece, color);
                        let has_escape = piece_dests.into_iter().any(|dest| {
                            (pin_dir & BitBoard::from_square(dest)) == EMPTY
                                // Exclude squares occupied by own pieces (pseudo-legal can't capture own)
                                && board.color_on(dest) != Some(color)
                        });
                        if has_escape {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}
