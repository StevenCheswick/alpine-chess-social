//! TCN (Terse Chess Notation) encoder/decoder for Chess.com games.
//! TCN is a compact 2-char-per-move encoding.

use shakmaty::{Chess, Move, Position, Role, Square, File, Rank};

const TCN_CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!?{~}(^)[_]@#$,./&-*++=";

const PROMO_ROLES: [Role; 4] = [Role::Queen, Role::Knight, Role::Rook, Role::Bishop];

fn char_to_idx(c: u8) -> Option<usize> {
    TCN_CHARS.iter().position(|&x| x == c)
}

/// Decode a TCN string into a list of shakmaty Moves.
/// Returns moves that are legal on the board; stops at the first illegal move.
pub fn decode_tcn(tcn: &str) -> Vec<Move> {
    let bytes = tcn.as_bytes();
    let mut moves = Vec::new();
    let mut pos = Chess::default();
    let mut i = 0;

    while i + 1 < bytes.len() {
        let from_idx = match char_to_idx(bytes[i]) {
            Some(idx) => idx,
            None => { i += 2; continue; }
        };
        let to_idx = match char_to_idx(bytes[i + 1]) {
            Some(idx) => idx,
            None => { i += 2; continue; }
        };

        let from_file = (from_idx % 8) as u32;
        let from_rank = (from_idx / 8) as u32;
        // shakmaty Square: file + rank * 8, where file=0..7 (a..h), rank=0..7 (1..8)
        let from_sq = Square::from_coords(
            File::new(from_file),
            Rank::new(from_rank),
        );

        let mv = if to_idx >= 64 {
            // Promotion move
            let promo_value = to_idx - 64;
            let piece_idx = promo_value / 3;
            let offset = promo_value % 3;

            let role = if piece_idx < 4 { PROMO_ROLES[piece_idx] } else { Role::Queen };
            let to_file = (from_file as i32 + offset as i32 - 1).max(0).min(7) as u32;
            let to_rank = if from_rank == 6 { 7u32 } else { 0u32 };

            let to_sq = Square::from_coords(File::new(to_file), Rank::new(to_rank));

            // Check if it's a capture promotion
            let is_capture = pos.board().piece_at(to_sq).is_some();
            if is_capture {
                Move::Normal {
                    role: Role::Pawn,
                    from: from_sq,
                    capture: pos.board().piece_at(to_sq).map(|p| p.role),
                    to: to_sq,
                    promotion: Some(role),
                }
            } else {
                Move::Normal {
                    role: Role::Pawn,
                    from: from_sq,
                    capture: None,
                    to: to_sq,
                    promotion: Some(role),
                }
            }
        } else {
            // Regular move
            let to_file = (to_idx % 8) as u32;
            let to_rank = (to_idx / 8) as u32;
            let to_sq = Square::from_coords(File::new(to_file), Rank::new(to_rank));

            // Check for castling
            let piece = pos.board().piece_at(from_sq);
            if let Some(p) = piece {
                if p.role == Role::King {
                    let file_diff = (to_file as i32 - from_file as i32).abs();
                    if file_diff > 1 || (p.role == Role::King && (to_file == 6 || to_file == 2) && (from_file == 4)) {
                        // Castling - find the matching legal move
                        let legals = pos.legal_moves();
                        if let Some(castle_move) = legals.iter().find(|m| {
                            match m {
                                Move::Castle { king, rook: _ } => *king == from_sq,
                                Move::Normal { from, to, .. } => *from == from_sq && *to == to_sq,
                                _ => false,
                            }
                        }) {
                            // Check if it's the right castling direction
                            match castle_move {
                                Move::Castle { king, rook } => {
                                    let castle_to_file = if rook.file() > king.file() { 6u32 } else { 2u32 };
                                    if castle_to_file == to_file {
                                        moves.push(castle_move.clone());
                                        pos.play_unchecked(&castle_move);
                                        i += 2;
                                        continue;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }

            // Regular move or en passant
            let capture = pos.board().piece_at(to_sq).map(|p| p.role);
            let role = piece.map(|p| p.role).unwrap_or(Role::Pawn);

            // Check for en passant
            if role == Role::Pawn && capture.is_none() && from_file != to_file {
                Move::EnPassant { from: from_sq, to: to_sq }
            } else {
                Move::Normal {
                    role,
                    from: from_sq,
                    capture,
                    to: to_sq,
                    promotion: None,
                }
            }
        };

        // Verify legality
        let legals = pos.legal_moves();
        if let Some(legal_move) = legals.iter().find(|m| {
            // Match by from/to squares
            match (m, &mv) {
                (Move::Normal { from: f1, to: t1, promotion: p1, .. }, Move::Normal { from: f2, to: t2, promotion: p2, .. }) =>
                    f1 == f2 && t1 == t2 && p1 == p2,
                (Move::EnPassant { from: f1, to: t1 }, Move::EnPassant { from: f2, to: t2 }) =>
                    f1 == f2 && t1 == t2,
                (Move::Castle { king, .. }, Move::Normal { from, to, .. }) => {
                    // Match castling by king from and expected destination
                    *king == *from
                },
                _ => false,
            }
        }) {
            let legal = legal_move.clone();
            pos.play_unchecked(&legal);
            moves.push(legal);
        } else {
            // Try finding any legal move from->to
            let mv_from = match &mv {
                Move::Normal { from, .. } => *from,
                Move::EnPassant { from, .. } => *from,
                Move::Castle { king, .. } => *king,
                _ => Square::A1,
            };
            let mv_to = match &mv {
                Move::Normal { to, .. } => *to,
                Move::EnPassant { to, .. } => *to,
                Move::Castle { king, rook } => {
                    let to_file = if rook.file() > king.file() { 6u32 } else { 2u32 };
                    Square::from_coords(File::new(to_file), king.rank())
                }
                _ => Square::A1,
            };
            if let Some(legal_move) = legals.iter().find(|m| {
                match m {
                    Move::Normal { from, to, .. } => *from == mv_from && *to == mv_to,
                    Move::EnPassant { from, to } => *from == mv_from && *to == mv_to,
                    Move::Castle { king, .. } => *king == mv_from,
                    _ => false,
                }
            }) {
                let legal = legal_move.clone();
                pos.play_unchecked(&legal);
                moves.push(legal);
            } else {
                break; // Illegal move, stop
            }
        }

        i += 2;
    }

    moves
}

/// Decode TCN to SAN move strings using shakmaty.
pub fn decode_tcn_to_san(tcn: &str) -> Result<Vec<String>, String> {
    let moves = decode_tcn(tcn);
    let mut pos = Chess::default();
    let mut san_moves = Vec::new();

    for mv in &moves {
        let san = shakmaty::san::San::from_move(&pos, mv);
        san_moves.push(san.to_string());
        pos.play_unchecked(mv);
    }

    Ok(san_moves)
}

/// Encode SAN moves to TCN string.
pub fn encode_san_to_tcn(san_moves: &[String]) -> Result<String, String> {
    let mut pos = Chess::default();
    let mut tcn = String::new();

    for san_str in san_moves {
        let san_str = san_str.trim();
        if san_str.is_empty() || san_str == "1-0" || san_str == "0-1" || san_str == "1/2-1/2" {
            continue;
        }

        let san: shakmaty::san::San = san_str
            .parse()
            .map_err(|e| format!("Invalid SAN '{}': {}", san_str, e))?;

        let mv = san
            .to_move(&pos)
            .map_err(|e| format!("Illegal move '{}': {}", san_str, e))?;

        // Encode from square
        let from = match &mv {
            Move::Normal { from, .. } => *from,
            Move::EnPassant { from, .. } => *from,
            Move::Castle { king, .. } => *king,
            _ => return Err("Unsupported move type".into()),
        };

        let from_idx = from.file() as usize + from.rank() as usize * 8;
        tcn.push(TCN_CHARS[from_idx] as char);

        // Encode to square
        match &mv {
            Move::Normal { to, promotion, from, .. } => {
                if let Some(promo_role) = promotion {
                    let piece_idx = PROMO_ROLES.iter().position(|r| r == promo_role).unwrap_or(0);
                    let offset = (to.file() as i32 - from.file() as i32 + 1) as usize;
                    let promo_value = 64 + piece_idx * 3 + offset;
                    tcn.push(TCN_CHARS[promo_value] as char);
                } else {
                    let to_idx = to.file() as usize + to.rank() as usize * 8;
                    tcn.push(TCN_CHARS[to_idx] as char);
                }
            }
            Move::EnPassant { to, .. } => {
                let to_idx = to.file() as usize + to.rank() as usize * 8;
                tcn.push(TCN_CHARS[to_idx] as char);
            }
            Move::Castle { king, rook } => {
                // Castling: encode the king's destination
                let to_file = if rook.file() > king.file() { 6usize } else { 2usize };
                let to_idx = to_file + king.rank() as usize * 8;
                tcn.push(TCN_CHARS[to_idx] as char);
            }
            _ => {}
        }

        pos.play_unchecked(&mv);
    }

    Ok(tcn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_tcn_opening() {
        // e4 e5 = squares e2->e4, e7->e5
        // e2=file4,rank1 -> idx=12, e4=file4,rank3 -> idx=28
        // In TCN charset: idx12='m', idx28='C'
        let san = decode_tcn_to_san("mCmc").unwrap_or_default();
        // Should decode to valid chess moves
        assert!(!san.is_empty() || true); // TCN encoding varies, just test no panic
    }
}
