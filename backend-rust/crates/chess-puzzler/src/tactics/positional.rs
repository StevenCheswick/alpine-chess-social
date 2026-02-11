/// Positional detectors: quiet_move, attraction, deflection, intermezzo, clearance,
/// self_interference, interference, defensive_move, zugzwang
/// Port of cook.py

use chess::{BitBoard, MoveGen, Piece, EMPTY};

use crate::board_utils::{attackers, attacks, attacked_opponent_pieces, between, is_advanced_pawn_move, is_hanging, is_in_bad_spot, is_ray_piece, king_value};

use crate::puzzle::Puzzle;

/// Quiet move: non-capturing, non-checking move that doesn't attack pieces
pub fn quiet_move(puzzle: &Puzzle) -> bool {
    for (i, node) in puzzle.mainline.iter().enumerate() {
        // Must be solver's move (odd indices), not the last move
        if i % 2 == 0 || i == puzzle.mainline.len() - 1 {
            continue;
        }

        let board_after = &node.board_after;
        let board_before = &node.board_before;

        // No check given or escaped
        if board_after.checkers().popcnt() > 0 || board_before.checkers().popcnt() > 0 {
            continue;
        }

        // No capture
        if board_before.piece_on(node.chess_move.get_dest()).is_some() {
            continue;
        }

        // Doesn't threaten any opponent pieces
        if !attacked_opponent_pieces(board_after, node.chess_move.get_dest(), puzzle.pov).is_empty() {
            continue;
        }

        // Not an advanced pawn push
        if is_advanced_pawn_move(board_after, node.chess_move, board_after.side_to_move()) {
            continue;
        }

        // Not a king move
        if board_after.piece_on(node.chess_move.get_dest()) == Some(Piece::King) {
            continue;
        }

        return true;
    }
    false
}

/// Defensive move: quiet last move that defends
pub fn defensive_move(puzzle: &Puzzle) -> bool {
    if puzzle.mainline.len() < 2 {
        return false;
    }

    // At least 3 legal moves before the last move
    let second_to_last = &puzzle.mainline[puzzle.mainline.len() - 2];
    if MoveGen::new_legal(&second_to_last.board_after).len() < 3 {
        return false;
    }

    let last_node = puzzle.mainline.last().unwrap();
    let board = &last_node.board_after;

    // No check given, no capture
    if board.checkers().popcnt() > 0 {
        return false;
    }
    if last_node.board_before.piece_on(last_node.chess_move.get_dest()).is_some() {
        return false;
    }

    // No piece attacked
    if !attacked_opponent_pieces(board, last_node.chess_move.get_dest(), puzzle.pov).is_empty() {
        return false;
    }

    // Not an advanced pawn push
    !is_advanced_pawn_move(board, last_node.chess_move, board.side_to_move())
}

/// Attraction: lure a piece to a square where it can be exploited
pub fn attraction(puzzle: &Puzzle) -> bool {
    for (i, node) in puzzle.mainline.iter().enumerate().skip(1) {
        // Only solver's moves (odd indices)
        if i % 2 == 0 {
            continue;
        }

        let first_move_to = node.chess_move.get_dest();

        // Next move: opponent captures on that square
        if i + 1 >= puzzle.mainline.len() {
            continue;
        }
        let opponent_reply = &puzzle.mainline[i + 1];
        if opponent_reply.chess_move.get_dest() != first_move_to {
            continue;
        }

        let attracted_piece = opponent_reply.board_after.piece_on(opponent_reply.chess_move.get_dest());
        if !matches!(attracted_piece, Some(Piece::King) | Some(Piece::Queen) | Some(Piece::Rook)) {
            continue;
        }

        let attracted_to_square = opponent_reply.chess_move.get_dest();

        // Next solver move attacks that square
        if i + 2 >= puzzle.mainline.len() {
            continue;
        }
        let next_node = &puzzle.mainline[i + 2];
        let next_attackers = attackers(&next_node.board_after, puzzle.pov, attracted_to_square);

        if (next_attackers & BitBoard::from_square(next_node.chess_move.get_dest())) != EMPTY {
            if attracted_piece == Some(Piece::King) {
                return true;
            }
            // Or player later captures on that square
            if i + 4 < puzzle.mainline.len() {
                let n3 = &puzzle.mainline[i + 4];
                if n3.chess_move.get_dest() == attracted_to_square {
                    return true;
                }
            }
        }
    }
    false
}

/// Deflection: force a defender away from a key square
pub fn deflection(puzzle: &Puzzle) -> bool {
    let solver = puzzle.solver_moves();
    for (_idx, node) in solver.iter().enumerate().skip(1) {
        let captured_piece = node.board_before.piece_on(node.chess_move.get_dest());
        if captured_piece.is_none() && node.chess_move.get_promotion().is_none() {
            continue;
        }

        let capturing_piece = node.board_after.piece_on(node.chess_move.get_dest());
        if let (Some(cap), Some(mover)) = (captured_piece, capturing_piece) {
            if king_value(cap) > king_value(mover) {
                continue;
            }
        }

        let square = node.chess_move.get_dest();

        // Get previous opponent and solver moves
        let prev_op = &puzzle.mainline[node.ply - 1];
        let prev_op_move = prev_op.chess_move;

        if node.ply < 2 {
            continue;
        }
        let grandpa = &puzzle.mainline[node.ply - 2];
        let prev_player_move = grandpa.chess_move;

        // Check grandpa capture
        let prev_player_capture = grandpa.board_before.piece_on(prev_player_move.get_dest());
        let grandpa_moved = grandpa.board_after.piece_on(grandpa.chess_move.get_dest());

        let capture_check = match (prev_player_capture, grandpa_moved) {
            (None, _) => true,
            (Some(cap), Some(moved)) => king_value(cap) < king_value(moved),
            _ => false,
        };

        if !capture_check {
            continue;
        }

        if square == prev_op_move.get_dest() || square == prev_player_move.get_dest() {
            continue;
        }

        // Opponent responded to previous player move (recapture or escape check)
        let was_recapture = prev_op_move.get_dest() == prev_player_move.get_dest();
        let was_check_escape = grandpa.board_after.checkers().popcnt() > 0;
        if !was_recapture && !was_check_escape {
            continue;
        }

        // The captured square was defended by the opponent piece that moved
        let op_from = prev_op_move.get_source();
        let defended_from_original = {
            let atk = attacks(&grandpa.board_after, op_from);
            (atk & BitBoard::from_square(square)) != EMPTY
        };

        let promotion_file_match = node.chess_move.get_promotion().is_some()
            && node.chess_move.get_dest().get_file() == prev_op_move.get_source().get_file()
            && {
                let atk = attacks(&grandpa.board_after, op_from);
                (atk & BitBoard::from_square(node.chess_move.get_source())) != EMPTY
            };

        if !defended_from_original && !promotion_file_match {
            continue;
        }

        // The square is no longer defended from the new position
        let still_defended = {
            let atk = attacks(&node.board_before, prev_op_move.get_dest());
            (atk & BitBoard::from_square(square)) != EMPTY
        };
        if still_defended {
            continue;
        }

        return true;
    }
    false
}

/// Self-interference: opponent blocks their own defender
pub fn self_interference(puzzle: &Puzzle) -> bool {
    let solver = puzzle.solver_moves();
    for (_idx, node) in solver.iter().enumerate().skip(1) {
        let prev_board = &node.board_before;
        let square = node.chess_move.get_dest();
        let capture = prev_board.piece_on(square);

        if let Some(_cap_piece) = capture {
            let cap_color = prev_board.color_on(square).unwrap();
            if is_hanging(prev_board, cap_color, square) {
                // Check the board before opponent's last move
                if node.ply >= 2 {
                    let init_board = &puzzle.mainline[node.ply - 2].board_after;
                    let defenders = attackers(init_board, cap_color, square);

                    // Python uses defenders.pop() — only checks the first (lowest-index) defender
                    if let Some(def_sq) = defenders.into_iter().next() {
                        if let Some(def_piece) = init_board.piece_on(def_sq) {
                            if is_ray_piece(def_piece) {
                                // Did opponent's move block the defense ray?
                                let prev_op = &puzzle.mainline[node.ply - 1];
                                let between_bb = between(square, def_sq);
                                if (between_bb & BitBoard::from_square(prev_op.chess_move.get_dest())) != EMPTY {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

/// Interference: player piece blocks opponent's defense
pub fn interference(puzzle: &Puzzle) -> bool {
    let solver = puzzle.solver_moves();
    for (_idx, node) in solver.iter().enumerate().skip(1) {
        let prev_board = &node.board_before;
        let square = node.chess_move.get_dest();
        let capture = prev_board.piece_on(square);

        let prev_op = &puzzle.mainline[node.ply - 1];
        if capture.is_none() || square == prev_op.chess_move.get_dest() {
            continue;
        }

        if let Some(_cap_piece) = capture {
            let cap_color = prev_board.color_on(square).unwrap();
            if !is_hanging(prev_board, cap_color, square) {
                continue;
            }

            // Check the board 3 plies back
            if node.ply >= 3 {
                let init_board = &puzzle.mainline[node.ply - 3].board_after;
                let defenders = attackers(init_board, cap_color, square);

                // Python uses defenders.pop() — only checks the first (lowest-index) defender
                if let Some(def_sq) = defenders.into_iter().next() {
                    if let Some(def_piece) = init_board.piece_on(def_sq) {
                        if is_ray_piece(def_piece) {
                            // Did the previous solver move block the defense ray?
                            let interfering = &puzzle.mainline[node.ply - 2];
                            let between_bb = between(square, def_sq);
                            if (between_bb & BitBoard::from_square(interfering.chess_move.get_dest())) != EMPTY {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

/// Intermezzo: an in-between move before recapturing
pub fn intermezzo(puzzle: &Puzzle) -> bool {
    let solver = puzzle.solver_moves();
    for (_idx, node) in solver.iter().enumerate().skip(1) {
        // Must be a capture
        if node.board_before.piece_on(node.chess_move.get_dest()).is_none() {
            continue;
        }

        let capture_square = node.chess_move.get_dest();

        // Previous opponent move
        let op_node = &puzzle.mainline[node.ply - 1];

        // Previous solver move
        if node.ply < 2 {
            continue;
        }
        let prev_pov_node = &puzzle.mainline[node.ply - 2];

        // The opponent's from-square shouldn't have been attacking the capture square
        // from the previous position
        let could_attack_before = {
            let atk = attackers(&prev_pov_node.board_after, !puzzle.pov, capture_square);
            (atk & BitBoard::from_square(op_node.chess_move.get_source())) != EMPTY
        };
        if could_attack_before {
            continue;
        }

        // Previous solver move wasn't to the capture square
        if prev_pov_node.chess_move.get_dest() == capture_square {
            continue;
        }

        // Python returns immediately here — first capture passing filters
        // determines the result (True or False), never checks subsequent captures
        if node.ply >= 3 {
            let prev_op_node = &puzzle.mainline[node.ply - 3];
            return prev_op_node.chess_move.get_dest() == capture_square
                && prev_op_node.board_before.piece_on(capture_square).is_some()
                && MoveGen::new_legal(&prev_op_node.board_after)
                    .into_iter()
                    .any(|m| m == node.chess_move);
        }
        return false;
    }
    false
}

/// Clearance: moving a piece to open a line for another
pub fn clearance(puzzle: &Puzzle) -> bool {
    let solver = puzzle.solver_moves();
    for (_idx, node) in solver.iter().enumerate().skip(1) {
        let board = &node.board_after;

        // The target square must have been empty (not a capture)
        if node.board_before.piece_on(node.chess_move.get_dest()).is_some() {
            continue;
        }

        // The piece that landed must be a ray piece
        let piece = board.piece_on(node.chess_move.get_dest());
        if !matches!(piece, Some(p) if is_ray_piece(p)) {
            continue;
        }

        // Previous solver move (the clearing move)
        if node.ply < 2 {
            continue;
        }
        let prev = &puzzle.mainline[node.ply - 2];

        // No promotion on the clearing move
        if prev.chess_move.get_promotion().is_some() {
            continue;
        }

        // Clearing move wasn't to our from or to square
        if prev.chess_move.get_dest() == node.chess_move.get_source()
            || prev.chess_move.get_dest() == node.chess_move.get_dest()
        {
            continue;
        }

        // No check after opponent's response
        let prev_op = &puzzle.mainline[node.ply - 1];
        if prev_op.board_after.checkers().popcnt() > 0 {
            continue;
        }

        // If we give check, opponent didn't have to move their king
        if board.checkers().popcnt() > 0 {
            let op_moved = prev_op.board_after.piece_on(prev_op.chess_move.get_dest());
            if op_moved == Some(Piece::King) {
                continue;
            }
        }

        // The clearing piece was on the line between our from and to
        let between_bb = between(node.chess_move.get_source(), node.chess_move.get_dest());
        if prev.chess_move.get_source() == node.chess_move.get_dest()
            || (between_bb & BitBoard::from_square(prev.chess_move.get_source())) != EMPTY
        {
            // The clearing move destination should be bad or the original position was empty
            if node.ply >= 3 {
                let before_clear = &puzzle.mainline[node.ply - 3];
                let dest_was_empty = before_clear.board_after.piece_on(prev.chess_move.get_dest()).is_none();
                let dest_is_bad = is_in_bad_spot(&prev.board_after, prev.chess_move.get_dest());
                if dest_was_empty || dest_is_bad {
                    return true;
                }
            }
        }
    }
    false
}

