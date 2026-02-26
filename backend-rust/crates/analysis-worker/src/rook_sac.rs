/// Rook sacrifice (exchange sacrifice) detection for game tagging.
/// Ported from feature-testing/games-page-tags/rook-sac-rs/src/detect.rs
/// but adapted to use pre-computed data from the analyzer (boards, evals, best_moves).

use chess::{BitBoard, Board, ChessMove, Color, Piece, Square, EMPTY};

use crate::board_utils;

// Eval filter constants (match standalone exactly)
const MATE_THRESHOLD: i32 = 9000;
const MAX_CP_DIFF: i32 = 100;
const SAC_CP_FLOOR: i32 = -100;
const MIN_PIECES: u32 = 8;
const ENDGAME_PIECES: u32 = 12;
const MAX_NON_MATE_EVAL: i32 = 300;
const ROOK_VALUE: i32 = 5;

/// Sacrifice pattern type
#[derive(Debug, Clone, Copy, PartialEq)]
enum Pattern {
    Capture,
    Check,
    Hanging,
}

/// A raw candidate before eval filtering
struct Candidate {
    move_idx: usize,
    board_before: Board,
    #[allow(dead_code)]
    captured_type: Option<Piece>,
    pattern: Pattern,
}

/// Tracks user's non-rook move for hanging sacrifice detection
struct HangingState {
    move_idx: usize,
    rook_squares: BitBoard,
    captured_value: i32,
}

/// Detect whether a game contains a rook sacrifice by the user.
/// Returns true if at least one eval-validated rook sacrifice is found.
///
/// All inputs are pre-computed by the analyzer — no additional Stockfish calls needed.
/// - `boards_before[i]` is the board state before move i (len = moves + 1)
/// - `chess_moves[i]` is the parsed ChessMove for move i
/// - `evals[i]` is the white-POV eval BEFORE move i (len = moves + 1)
/// - `best_moves[i]` is the SF best move UCI string for position i
/// - `positions_uci[i]` is the UCI string of the move actually played at index i
pub fn detect_rook_sacrifice(
    boards_before: &[Board],
    chess_moves: &[ChessMove],
    user_color: Color,
    evals: &[i32],
    best_moves: &[String],
    positions_uci: &[String],
) -> bool {
    let candidates = find_candidates(boards_before, chess_moves, user_color);

    for c in &candidates {
        if eval_filter(c, user_color, evals, best_moves, positions_uci) {
            return true;
        }
    }
    false
}

/// Check if a piece is pinned to its king.
/// Uses board_utils::pin_direction under the hood.
fn is_pinned(board: &Board, color: Color, square: Square) -> bool {
    let full_board = BitBoard::new(0xFFFF_FFFF_FFFF_FFFF);
    board_utils::pin_direction(board, color, square) != full_board
}

/// Check if a move is a capture
fn is_capture(board: &Board, m: ChessMove) -> bool {
    board.piece_on(m.get_dest()).is_some() || is_en_passant(board, m)
}

/// Check if a move is en passant
fn is_en_passant(board: &Board, m: ChessMove) -> bool {
    if board.piece_on(m.get_source()) == Some(Piece::Pawn) {
        if let Some(ep_sq) = board.en_passant() {
            return m.get_dest() == ep_sq;
        }
    }
    false
}

/// Check if a move gives check
fn gives_check(board: &Board, m: ChessMove) -> bool {
    let new_board = board.make_move_new(m);
    *new_board.checkers() != EMPTY
}

/// Check if an opponent bishop/queen is on the same diagonal as queen_sq and rook_sq,
/// attacking through the queen to the rook (skewer).
fn is_bishop_skewer(board: &Board, user_color: Color, queen_sq: Square, rook_sq: Square) -> bool {
    let opp_color = !user_color;

    let q_file = queen_sq.get_file().to_index() as i32;
    let q_rank = queen_sq.get_rank().to_index() as i32;
    let r_file = rook_sq.get_file().to_index() as i32;
    let r_rank = rook_sq.get_rank().to_index() as i32;

    let file_diff = (q_file - r_file).abs();
    let rank_diff = (q_rank - r_rank).abs();
    if file_diff != rank_diff || file_diff == 0 {
        return false;
    }

    // Direction from rook through queen (outward to find the bishop)
    let d_file = if q_file > r_file { 1 } else { -1 };
    let d_rank = if q_rank > r_rank { 1 } else { -1 };

    let mut f = q_file + d_file;
    let mut r = q_rank + d_rank;
    while (0..=7).contains(&f) && (0..=7).contains(&r) {
        let sq = Square::make_square(
            chess::Rank::from_index(r as usize),
            chess::File::from_index(f as usize),
        );
        if let Some(piece) = board.piece_on(sq) {
            let piece_color =
                if (*board.color_combined(Color::White) & BitBoard::from_square(sq)) != EMPTY {
                    Color::White
                } else {
                    Color::Black
                };
            if piece_color == opp_color && (piece == Piece::Bishop || piece == Piece::Queen) {
                return true;
            }
            break; // blocked by another piece
        }
        f += d_file;
        r += d_rank;
    }

    false
}

/// Check if an opponent knight or bishop forks the rook with another user R/Q/K.
fn is_piece_fork(board: &Board, user_color: Color, rook_sq: Square) -> bool {
    let opp_color = !user_color;
    let opp_attackers = board_utils::attackers(board, opp_color, rook_sq);

    for attacker_sq in opp_attackers {
        let attacker_piece = board.piece_on(attacker_sq);
        if attacker_piece != Some(Piece::Knight) && attacker_piece != Some(Piece::Bishop) {
            continue;
        }

        // Get all squares this piece attacks
        let occupied = *board.combined();
        let attack_bb = match attacker_piece {
            Some(Piece::Knight) => chess::get_knight_moves(attacker_sq),
            Some(Piece::Bishop) => chess::get_bishop_moves(attacker_sq, occupied),
            _ => EMPTY,
        };

        // Check if any attacked square has a user R/Q/K (other than our rook)
        let user_pieces = *board.color_combined(user_color);
        let valuable = (*board.pieces(Piece::Rook)
            | *board.pieces(Piece::Queen)
            | *board.pieces(Piece::King))
            & user_pieces;

        let forked = attack_bb & valuable & !BitBoard::from_square(rook_sq);
        if forked != EMPTY {
            return true;
        }
    }

    false
}

/// After the user plays a check (at check_move_idx), look one ply deeper:
/// opponent responds to check, then user's next move — does it capture a R/Q?
fn check_deeper_recovery(
    boards_before: &[Board],
    chess_moves: &[ChessMove],
    check_move_idx: usize,
) -> bool {
    let user_follow_idx = check_move_idx + 2;
    if user_follow_idx >= chess_moves.len() {
        return false;
    }

    // boards_before[user_follow_idx] is the board after opponent's reply (before user's follow-up)
    if let Some(cap) = boards_before[user_follow_idx].piece_on(chess_moves[user_follow_idx].get_dest())
    {
        if cap == Piece::Rook || cap == Piece::Queen {
            return true;
        }
    }

    false
}

/// The triple state machine: find raw candidates (capture + check + hanging sacrifice).
/// Takes pre-built boards_before and chess_moves instead of replaying from PGN.
fn find_candidates(
    boards_before: &[Board],
    chess_moves: &[ChessMove],
    user_color: Color,
) -> Vec<Candidate> {
    let n = chess_moves.len();
    let mut candidates = Vec::new();

    // Capture sacrifice state
    let mut potential: Option<Candidate> = None;
    let mut recaptured = false;
    let mut recapture_idx: Option<usize> = None;
    let mut potential_square: Option<Square> = None;

    // Check sacrifice state
    let mut potential_check: Option<Candidate> = None;
    let mut check_recaptured = false;
    let mut check_recapture_idx: Option<usize> = None;
    let mut check_square: Option<Square> = None;

    // Hanging sacrifice state
    let mut pending_user_move: Option<HangingState> = None;
    let mut hanging_captured: Option<(usize, Square)> = None;

    for i in 0..n {
        let board = &boards_before[i];
        let m = chess_moves[i];
        let is_user = board.side_to_move() == user_color;
        let is_opp = !is_user;

        // ===== HANGING SACRIFICE: user's recovery move after opponent took hanging rook =====
        if is_user {
            if let Some((hang_idx, _hang_sq)) = hanging_captured {
                if let Some(ref pum) = pending_user_move {
                    if i == hang_idx + 1 {
                        let mut got_enough_back = false;

                        let orig_val = pum.captured_value;

                        // Original move already captured enough (e.g. took a queen)
                        if orig_val >= ROOK_VALUE {
                            got_enough_back = true;
                        }

                        if !got_enough_back && is_capture(board, m) {
                            if let Some(cap) = board.piece_on(m.get_dest()) {
                                let follow_val = board_utils::piece_value(cap);
                                if orig_val + follow_val >= ROOK_VALUE {
                                    got_enough_back = true;
                                }
                                if cap == Piece::Rook || cap == Piece::Queen {
                                    got_enough_back = true;
                                }
                            }
                        }

                        let immediate_rook_promo = m.get_promotion() == Some(Piece::Rook);

                        // Check-deeper recovery
                        if !got_enough_back && !immediate_rook_promo && gives_check(board, m) {
                            if check_deeper_recovery(boards_before, chess_moves, i) {
                                got_enough_back = true;
                            }
                        }

                        if !got_enough_back && !immediate_rook_promo {
                            candidates.push(Candidate {
                                move_idx: pum.move_idx,
                                board_before: boards_before[pum.move_idx],
                                captured_type: None,
                                pattern: Pattern::Hanging,
                            });
                        }

                        pending_user_move = None;
                        hanging_captured = None;
                        continue;
                    }
                }
            }
        }

        // Clear stale hanging state
        if let Some((hang_idx, _)) = hanging_captured {
            if i > hang_idx + 1 {
                pending_user_move = None;
                hanging_captured = None;
            }
        }
        if hanging_captured.is_none() {
            if let Some(ref pum) = pending_user_move {
                if i > pum.move_idx + 1 {
                    pending_user_move = None;
                }
            }
        }

        // ===== CAPTURE SACRIFICE: user rook captures a piece =====
        if is_user && is_capture(board, m) {
            let piece = board.piece_on(m.get_source());
            if piece == Some(Piece::Rook) {
                let captured = board.piece_on(m.get_dest());

                // RxR is a trade, not a sacrifice
                if captured == Some(Piece::Rook) {
                    continue;
                }

                // RxQ is winning material, not a sacrifice
                if captured == Some(Piece::Queen) {
                    continue;
                }

                // Pinned rook — not voluntary
                if is_pinned(board, user_color, m.get_source()) {
                    continue;
                }

                // Forked rook — enemy attacks both rook and king
                let king_sq = board_utils::king_square(board, user_color);
                let rook_sq = m.get_source();
                let opp_color = !user_color;
                let rook_attackers = board_utils::attackers(board, opp_color, rook_sq);
                let king_attackers = board_utils::attackers(board, opp_color, king_sq);
                if (rook_attackers & king_attackers) != EMPTY {
                    continue;
                }

                // Minimum piece count
                if board_utils::piece_map_count(board) <= MIN_PIECES {
                    continue;
                }

                potential = Some(Candidate {
                    move_idx: i,
                    board_before: *board,
                    captured_type: captured,
                    pattern: Pattern::Capture,
                });
                potential_square = Some(m.get_dest());
                continue;
            }
        }

        // Opponent recaptures on the sacrifice square
        if is_opp && potential.is_some() {
            let pot = potential.as_ref().unwrap();
            if i == pot.move_idx + 1 {
                if Some(m.get_dest()) == potential_square {
                    recaptured = true;
                    recapture_idx = Some(i);
                    continue;
                } else {
                    potential = None;
                    potential_square = None;
                    continue;
                }
            }
        }

        // User's move after recapture — material balance check
        if is_user && recaptured && recapture_idx.is_some() {
            if i == recapture_idx.unwrap() + 1 {
                let mut got_enough_back = false;
                let mut immediate_rook_promo = false;

                if is_capture(board, m) {
                    if let Some(cap) = board.piece_on(m.get_dest()) {
                        // Total material recovered: original captured piece + this capture
                        let orig_val = potential
                            .as_ref()
                            .and_then(|p| p.captured_type)
                            .map(board_utils::piece_value)
                            .unwrap_or(0);
                        let new_val = board_utils::piece_value(cap);
                        if orig_val + new_val >= ROOK_VALUE {
                            got_enough_back = true;
                        }
                    }
                }

                if m.get_promotion() == Some(Piece::Rook) {
                    immediate_rook_promo = true;
                }

                // Check-deeper recovery
                if !got_enough_back && !immediate_rook_promo && gives_check(board, m) {
                    if check_deeper_recovery(boards_before, chess_moves, i) {
                        got_enough_back = true;
                    }
                }

                if !got_enough_back && !immediate_rook_promo && potential.is_some() {
                    candidates.push(potential.take().unwrap());
                }

                potential = None;
                potential_square = None;
                recaptured = false;
                recapture_idx = None;
                continue;
            }
        }

        // Clear stale capture-sacrifice state
        if let Some(ref pot) = potential {
            if i > pot.move_idx + 2 {
                potential = None;
                potential_square = None;
                recaptured = false;
                recapture_idx = None;
            }
        }

        // ===== CHECK SACRIFICE: rook gives check (non-capture), gets captured =====
        if is_user && !is_capture(board, m) {
            let piece = board.piece_on(m.get_source());
            if piece == Some(Piece::Rook) && gives_check(board, m) {
                // Pinned rook
                if is_pinned(board, user_color, m.get_source()) {
                    continue;
                }

                // Forked rook
                let king_sq = board_utils::king_square(board, user_color);
                let rook_sq = m.get_source();
                let opp_color = !user_color;
                let rook_attackers = board_utils::attackers(board, opp_color, rook_sq);
                let king_attackers = board_utils::attackers(board, opp_color, king_sq);
                if (rook_attackers & king_attackers) != EMPTY {
                    continue;
                }

                // Minimum piece count
                if board_utils::piece_map_count(board) <= MIN_PIECES {
                    continue;
                }

                potential_check = Some(Candidate {
                    move_idx: i,
                    board_before: *board,
                    captured_type: None,
                    pattern: Pattern::Check,
                });
                check_square = Some(m.get_dest());
                continue;
            }
        }

        // Opponent captures rook after check
        if is_opp && potential_check.is_some() {
            let pot = potential_check.as_ref().unwrap();
            if i == pot.move_idx + 1 {
                if Some(m.get_dest()) == check_square && is_capture(board, m) {
                    check_recaptured = true;
                    check_recapture_idx = Some(i);
                    continue;
                } else {
                    potential_check = None;
                    check_square = None;
                    continue;
                }
            }
        }

        // User's move after check-sacrifice capture
        if is_user && check_recaptured && check_recapture_idx.is_some() {
            if i == check_recapture_idx.unwrap() + 1 {
                let mut took_rook_or_queen_back = false;

                if is_capture(board, m) {
                    if let Some(cap) = board.piece_on(m.get_dest()) {
                        if cap == Piece::Rook || cap == Piece::Queen {
                            took_rook_or_queen_back = true;
                        }
                    }
                }

                let immediate_rook_promo = m.get_promotion() == Some(Piece::Rook);

                // Check-deeper recovery
                if !took_rook_or_queen_back && !immediate_rook_promo && gives_check(board, m) {
                    if check_deeper_recovery(boards_before, chess_moves, i) {
                        took_rook_or_queen_back = true;
                    }
                }

                if !took_rook_or_queen_back
                    && !immediate_rook_promo
                    && potential_check.is_some()
                {
                    candidates.push(potential_check.take().unwrap());
                }

                potential_check = None;
                check_square = None;
                check_recaptured = false;
                check_recapture_idx = None;
                continue;
            }
        }

        // Clear stale check-sacrifice state
        if let Some(ref pot) = potential_check {
            if i > pot.move_idx + 2 {
                potential_check = None;
                check_square = None;
                check_recaptured = false;
                check_recapture_idx = None;
            }
        }

        // ===== HANGING SACRIFICE: opponent captures a stationary user rook =====
        if is_opp {
            if let Some(ref pum) = pending_user_move {
                if i == pum.move_idx + 1 && hanging_captured.is_none() {
                    if is_capture(board, m) {
                        let victim = board.piece_on(m.get_dest());
                        let is_user_piece = (*board.color_combined(user_color)
                            & BitBoard::from_square(m.get_dest()))
                            != EMPTY;

                        if victim == Some(Piece::Rook) && is_user_piece {
                            // Make sure this rook was stationary (not just moved)
                            if (pum.rook_squares & BitBoard::from_square(m.get_dest())) != EMPTY {
                                let board_before_user = &boards_before[pum.move_idx];

                                // Bishop skewer filter: user moved queen, opp bishop takes rook behind it
                                let user_piece =
                                    board_before_user.piece_on(chess_moves[pum.move_idx].get_source());
                                if user_piece == Some(Piece::Queen) {
                                    if is_bishop_skewer(
                                        board_before_user,
                                        user_color,
                                        chess_moves[pum.move_idx].get_source(),
                                        m.get_dest(),
                                    ) {
                                        pending_user_move = None;
                                        continue;
                                    }
                                }

                                // Pin filter (on board before user's move)
                                if is_pinned(board_before_user, user_color, m.get_dest()) {
                                    pending_user_move = None;
                                    continue;
                                }

                                // Piece fork filter (on board before user's move)
                                if is_piece_fork(board_before_user, user_color, m.get_dest()) {
                                    pending_user_move = None;
                                    continue;
                                }

                                hanging_captured = Some((i, m.get_dest()));
                                continue;
                            }
                        }
                    }
                }
            }
        }

        // ===== Record user's non-rook move for hanging detection =====
        if is_user && hanging_captured.is_none() {
            let piece = board.piece_on(m.get_source());
            if let Some(p) = piece {
                if p != Piece::Rook
                    && *board.checkers() == EMPTY
                    && board_utils::piece_map_count(board) > MIN_PIECES
                {
                    let rook_squares =
                        *board.pieces(Piece::Rook) & *board.color_combined(user_color);
                    if rook_squares != EMPTY {
                        let captured_value = if is_capture(board, m) {
                            board
                                .piece_on(m.get_dest())
                                .map(board_utils::piece_value)
                                .unwrap_or(0)
                        } else {
                            0
                        };

                        pending_user_move = Some(HangingState {
                            move_idx: i,
                            rook_squares,
                            captured_value,
                        });
                    }
                }
            }
        }
    }

    candidates
}

/// Eval-filter a candidate using pre-computed evals and best_moves.
/// Returns true if the sacrifice passes all filters.
fn eval_filter(
    candidate: &Candidate,
    user_color: Color,
    evals: &[i32],
    best_moves: &[String],
    positions_uci: &[String],
) -> bool {
    let i = candidate.move_idx;

    // Safety: need evals[i] and evals[i+1]
    if i + 1 >= evals.len() {
        return false;
    }

    // Convert white-POV evals to user-POV
    let (best_cp, move_cp) = if user_color == Color::White {
        (evals[i], evals[i + 1])
    } else {
        (-evals[i], -evals[i + 1])
    };

    let is_best = positions_uci[i] == best_moves[i];
    let pieces = board_utils::piece_map_count(&candidate.board_before);

    // Mate was available but player sacced instead
    if best_cp >= MATE_THRESHOLD && move_cp < MATE_THRESHOLD {
        return false;
    }

    // Non-mate sac when already winning big — not impressive
    if move_cp < MATE_THRESHOLD && move_cp >= MAX_NON_MATE_EVAL {
        return false;
    }

    // Sac too far from best move
    let cp_diff = best_cp - move_cp;
    if !is_best && cp_diff > MAX_CP_DIFF {
        return false;
    }

    // Sac eval below floor
    if move_cp < SAC_CP_FLOOR {
        return false;
    }

    // Endgame: must be best move
    if pieces <= ENDGAME_PIECES && !is_best {
        return false;
    }

    // Hanging sacrifices must be best move
    if candidate.pattern == Pattern::Hanging && !is_best {
        return false;
    }

    true
}
