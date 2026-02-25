/// Queen sacrifice detection for game tagging.
/// Ported from feature-testing/games-page-tags/queen-sac-rs/src/detect.rs
/// but adapted to use pre-computed data from the analyzer (boards, evals, best_moves).

use chess::{BitBoard, Board, ChessMove, Color, Piece, Square, EMPTY};

use crate::board_utils;

// Eval filter constants (match standalone exactly)
const MATE_THRESHOLD: i32 = 9000;
const MAX_CP_DIFF: i32 = 400;
const SAC_CP_FLOOR: i32 = -100;
const MIN_PIECES: u32 = 8;
const ENDGAME_PIECES: u32 = 12;

/// A raw candidate before eval filtering
struct Candidate {
    move_idx: usize,
    board_before: Board,
    #[allow(dead_code)]
    captured_type: Option<Piece>,
}

/// Detect whether a game contains a queen sacrifice by the user.
/// Returns true if at least one eval-validated queen sacrifice is found.
///
/// All inputs are pre-computed by the analyzer — no additional Stockfish calls needed.
/// - `boards_before[i]` is the board state before move i
/// - `chess_moves[i]` is the parsed ChessMove for move i
/// - `evals[i]` is the white-POV eval BEFORE move i (len = moves + 1)
/// - `best_moves[i]` is the SF best move UCI string for position i
/// - `positions_uci[i]` is the UCI string of the move actually played at index i
pub fn detect_queen_sacrifice(
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

/// Check if the opponent has a queen on the board
fn opp_has_queen(board: &Board, user_color: Color) -> bool {
    let opp_color = !user_color;
    (*board.pieces(Piece::Queen) & *board.color_combined(opp_color)) != EMPTY
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

/// The dual state machine: find raw candidates (capture sacrifice + check sacrifice).
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
    let mut potential_captured_type: Option<Piece> = None;

    // Check sacrifice state
    let mut potential_check_idx: Option<usize> = None;
    let mut check_recaptured = false;
    let mut check_recapture_idx: Option<usize> = None;
    let mut check_square: Option<Square> = None;

    for i in 0..n {
        let board = &boards_before[i];
        let m = chess_moves[i];
        let is_user = board.side_to_move() == user_color;
        let is_opp = !is_user;

        // ===== CAPTURE SACRIFICE: user queen captures a piece =====
        if is_user && is_capture(board, m) {
            let piece = board.piece_on(m.get_source());
            if piece == Some(Piece::Queen) {
                let captured = board.piece_on(m.get_dest());

                // Queen-takes-queen is a trade, not a sacrifice
                if captured == Some(Piece::Queen) {
                    continue;
                }

                // Pinned queen — not voluntary
                if is_pinned(board, user_color, m.get_source()) {
                    continue;
                }

                // Forked queen — enemy attacks both queen and king
                let king_sq = board_utils::king_square(board, user_color);
                let queen_sq = m.get_source();
                let opp_color = !user_color;
                let queen_attackers = board_utils::attackers(board, opp_color, queen_sq);
                let king_attackers = board_utils::attackers(board, opp_color, king_sq);
                if (queen_attackers & king_attackers) != EMPTY {
                    continue;
                }

                // Opponent queen must be on the board
                if !opp_has_queen(board, user_color) {
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
                });
                potential_square = Some(m.get_dest());
                potential_captured_type = captured;
                continue;
            }
        }

        // Opponent recaptures on the sacrifice square
        if is_opp && potential.is_some() {
            let pot = potential.as_ref().unwrap();
            if i == pot.move_idx + 1 {
                if Some(m.get_dest()) == potential_square {
                    // Bishop-pin Qxr recapture filter
                    let recapture_piece = board.piece_on(m.get_source());
                    if potential_captured_type == Some(Piece::Rook)
                        && recapture_piece == Some(Piece::Bishop)
                        && is_pinned(board, user_color, potential_square.unwrap())
                    {
                        potential = None;
                        potential_square = None;
                        potential_captured_type = None;
                        continue;
                    }

                    recaptured = true;
                    recapture_idx = Some(i);
                    continue;
                } else {
                    // Opponent didn't recapture — queen wasn't taken
                    potential = None;
                    potential_square = None;
                    potential_captured_type = None;
                    continue;
                }
            }
        }

        // User's move after recapture
        if is_user && recaptured && recapture_idx.is_some() {
            if i == recapture_idx.unwrap() + 1 {
                let mut took_queen_back = false;
                let mut queen_for_two_rooks = false;

                if is_capture(board, m) {
                    let cap = board.piece_on(m.get_dest());
                    if cap == Some(Piece::Queen) {
                        took_queen_back = true;
                    }
                    if cap == Some(Piece::Rook)
                        && potential_captured_type == Some(Piece::Rook)
                    {
                        queen_for_two_rooks = true;
                    }
                }

                let immediate_repromotion = m.get_promotion() == Some(Piece::Queen);

                if !took_queen_back
                    && !queen_for_two_rooks
                    && !immediate_repromotion
                    && potential.is_some()
                {
                    candidates.push(potential.take().unwrap());
                }

                potential = None;
                potential_square = None;
                potential_captured_type = None;
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
                potential_captured_type = None;
                recaptured = false;
                recapture_idx = None;
            }
        }

        // ===== CHECK SACRIFICE: queen gives check (non-capture), gets captured =====
        if is_user && !is_capture(board, m) {
            let piece = board.piece_on(m.get_source());
            if piece == Some(Piece::Queen) && gives_check(board, m) {
                // Pinned queen
                if is_pinned(board, user_color, m.get_source()) {
                    continue;
                }

                // Forked queen
                let king_sq = board_utils::king_square(board, user_color);
                let queen_sq = m.get_source();
                let opp_color = !user_color;
                let queen_attackers = board_utils::attackers(board, opp_color, queen_sq);
                let king_attackers = board_utils::attackers(board, opp_color, king_sq);
                if (queen_attackers & king_attackers) != EMPTY {
                    continue;
                }

                // Opponent queen must be on the board
                if !opp_has_queen(board, user_color) {
                    continue;
                }

                // Minimum piece count
                if board_utils::piece_map_count(board) <= MIN_PIECES {
                    continue;
                }

                potential_check_idx = Some(i);
                check_square = Some(m.get_dest());
                continue;
            }
        }

        // Opponent captures queen after check
        if is_opp && potential_check_idx.is_some() {
            let pot_idx = potential_check_idx.unwrap();
            if i == pot_idx + 1 {
                if Some(m.get_dest()) == check_square && is_capture(board, m) {
                    check_recaptured = true;
                    check_recapture_idx = Some(i);
                    continue;
                } else {
                    // Opponent didn't capture the queen
                    potential_check_idx = None;
                    check_square = None;
                    continue;
                }
            }
        }

        // User's move after check-sacrifice capture
        if is_user && check_recaptured && check_recapture_idx.is_some() {
            if i == check_recapture_idx.unwrap() + 1 {
                let mut took_queen_back = false;

                if is_capture(board, m) {
                    let cap = board.piece_on(m.get_dest());
                    if cap == Some(Piece::Queen) {
                        took_queen_back = true;
                    }
                }

                let immediate_repromotion = m.get_promotion() == Some(Piece::Queen);

                if !took_queen_back && !immediate_repromotion && potential_check_idx.is_some() {
                    candidates.push(Candidate {
                        move_idx: potential_check_idx.unwrap(),
                        board_before: boards_before[potential_check_idx.unwrap()],
                        captured_type: None,
                    });
                }

                potential_check_idx = None;
                check_square = None;
                check_recaptured = false;
                check_recapture_idx = None;
                continue;
            }
        }

        // Clear stale check-sacrifice state
        if let Some(pot_idx) = potential_check_idx {
            if i > pot_idx + 2 {
                potential_check_idx = None;
                check_square = None;
                check_recaptured = false;
                check_recapture_idx = None;
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

    true
}
