//! Core game analysis logic — puzzle tags disabled pending found-flag fix
//!
//! Ported from server/src/routes/analysis_ws.rs but uses local Stockfish
//! instead of delegating to client-side WASM.

use crate::analysis;
use crate::board_utils::piece_map_count;
use crate::endgame::EndgameTracker;
use crate::puzzle::cook;
use crate::puzzle::extraction::{
    parse_uci_move, BLUNDER_THRESHOLD, MIN_PUZZLE_CP, MIN_PUZZLE_LENGTH,
};
use crate::puzzle::{Puzzle, PuzzleNode, TagKind};
use crate::tactics::zugzwang::ZugzwangEval;
use chess::{Board, ChessMove, Color, MoveGen, Piece};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::info;

use crate::book_cache;
use crate::config::WorkerConfig;
use crate::db;
use crate::error::WorkerError;
use crate::stockfish::StockfishEngine;

/// Move classification output
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClassificationsOutput {
    pub best: u32,
    pub excellent: u32,
    pub good: u32,
    pub inaccuracy: u32,
    pub mistake: u32,
    pub blunder: u32,
    pub book: u32,
    pub forced: u32,
}

/// Puzzle output for JSON serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PuzzleOutput {
    pub fen: String,
    pub moves: Vec<String>,
    pub cp: i32,
    pub themes: Vec<String>,
    pub solver_is_white: bool,
    pub found: bool,
    pub cp_before_blunder: i32,
}

/// Move output for JSON serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveOutput {
    #[serde(rename = "move")]
    pub move_uci: String,
    pub move_eval: i32,
    pub best_move: String,
    pub best_eval: i32,
    pub cp_loss: i32,
    pub classification: String,
}

/// Analyze a game and save results to database
pub async fn analyze_game(
    engine: &mut StockfishEngine,
    pool: &PgPool,
    config: &WorkerConfig,
    game_id: i64,
) -> Result<(), WorkerError> {
    info!(game_id, "Starting analysis");

    // Fetch game from database
    let game = db::fetch_game(pool, game_id)
        .await?
        .ok_or(WorkerError::GameNotFound(game_id))?;

    // Decode TCN to SAN moves
    let san_moves = chess_core::tcn::decode_tcn_to_san(&game.tcn)
        .map_err(|e| WorkerError::TcnDecode(e.to_string()))?;

    info!(game_id, move_count = san_moves.len(), "Decoded TCN");

    // Parse moves and build positions
    let mut board = Board::default();
    let mut positions: Vec<(String, String, Board)> = Vec::new();
    let mut boards_before: Vec<Board> = vec![board];
    let mut chess_moves: Vec<ChessMove> = Vec::new();
    let mut legal_counts: Vec<usize> = Vec::new();

    let start_fen = board.to_string();

    for san in &san_moves {
        let fen_before = board.to_string();
        let legal_count = MoveGen::new_legal(&board).len();
        legal_counts.push(legal_count);

        let chess_move = find_san_move(&board, san)
            .map_err(|e| WorkerError::Analysis(format!("Invalid move {san}: {e}")))?;
        let uci = format!(
            "{}{}{}",
            chess_move.get_source(),
            chess_move.get_dest(),
            chess_move
                .get_promotion()
                .map(|p| match p {
                    Piece::Queen => "q",
                    Piece::Rook => "r",
                    Piece::Bishop => "b",
                    Piece::Knight => "n",
                    _ => "",
                })
                .unwrap_or("")
        );

        chess_moves.push(chess_move);
        board = board.make_move_new(chess_move);
        positions.push((fen_before, uci, board));
        boards_before.push(board);
    }

    let nodes = config.nodes_per_position;

    // Evaluate all positions (start + after each move)
    info!(game_id, "Evaluating positions");
    let mut evals: Vec<i32> = Vec::with_capacity(positions.len() + 1);
    let mut best_moves: Vec<String> = Vec::with_capacity(positions.len() + 1);

    // Evaluate starting position
    let start_result = engine.evaluate(&start_fen, nodes).await?;
    evals.push(eval_to_white_cp(start_result.cp, start_result.mate, true));
    best_moves.push(start_result.best_move);

    // Evaluate position after each move
    for (i, (_, _, board_after)) in positions.iter().enumerate() {
        let fen = board_after.to_string();
        let is_white = (i + 1) % 2 == 0; // After move i, it's the other side's turn
        let result = engine.evaluate(&fen, nodes).await?;
        evals.push(eval_to_white_cp(result.cp, result.mate, is_white));
        best_moves.push(result.best_move);
    }

    // Classify moves
    info!(game_id, "Classifying moves");
    let mut move_outputs = Vec::new();
    let mut eg_tracker = EndgameTracker::new();

    let mut white_cp_loss = 0;
    let mut black_cp_loss = 0;
    let mut white_move_count = 0u32;
    let mut black_move_count = 0u32;
    let mut white_class = ClassificationsOutput::default();
    let mut black_class = ClassificationsOutput::default();

    let mut blunder_indices: Vec<usize> = Vec::new();

    for (i, (fen_before, uci_move, board_after)) in positions.iter().enumerate() {
        let is_white = i % 2 == 0;
        let is_forced = legal_counts[i] == 1;
        let is_checkmate =
            MoveGen::new_legal(board_after).len() == 0 && board_after.checkers().popcnt() > 0;

        let eval_before = evals[i];
        let eval_after = evals[i + 1];

        if is_forced {
            move_outputs.push(MoveOutput {
                move_uci: uci_move.clone(),
                move_eval: eval_after,
                best_move: uci_move.clone(),
                best_eval: eval_before,
                cp_loss: 0,
                classification: "forced".to_string(),
            });
            if is_white {
                white_class.forced += 1;
                white_move_count += 1;
            } else {
                black_class.forced += 1;
                black_move_count += 1;
            }
            eg_tracker.track_move(
                board_after,
                eval_after,
                0,
                "forced",
                uci_move,
                uci_move,
                fen_before,
                is_white,
                i,
            );
            continue;
        }

        let cp_loss =
            analysis::calculate_cp_loss(eval_before, eval_after, is_white, is_checkmate);

        // Check if this is a book move (instant in-memory lookup)
        let is_book = book_cache::is_book_move(fen_before, &san_moves[i]);

        let classification = if is_book {
            "book"
        } else {
            let mate_blunder =
                analysis::is_mate_blunder(eval_before, eval_after, is_white, is_checkmate);
            analysis::classify_move(cp_loss, mate_blunder)
        };

        let best = &best_moves[i];

        move_outputs.push(MoveOutput {
            move_uci: uci_move.clone(),
            move_eval: eval_after,
            best_move: best.clone(),
            best_eval: eval_before,
            cp_loss,
            classification: classification.to_string(),
        });

        if classification != "book" && classification != "forced" {
            if is_white {
                white_cp_loss += cp_loss;
                white_move_count += 1;
                update_class(&mut white_class, classification);
            } else {
                black_cp_loss += cp_loss;
                black_move_count += 1;
                update_class(&mut black_class, classification);
            }
        } else if is_white {
            update_class(&mut white_class, classification);
            white_move_count += 1;
        } else {
            update_class(&mut black_class, classification);
            black_move_count += 1;
        }

        eg_tracker.track_move(
            board_after,
            eval_after,
            cp_loss,
            classification,
            uci_move,
            best,
            fen_before,
            is_white,
            i,
        );

        if cp_loss >= BLUNDER_THRESHOLD && classification != "forced" && classification != "book" {
            blunder_indices.push(i);
        }
    }

    // Extract puzzles from blunders
    info!(
        game_id,
        blunder_count = blunder_indices.len(),
        "Extracting puzzles"
    );
    let mut puzzles: Vec<PuzzleOutput> = Vec::new();
    let mut puzzle_objects: Vec<Puzzle> = Vec::new();

    for &blunder_i in &blunder_indices {
        let is_white_blunder = blunder_i % 2 == 0;
        let solver_color = if is_white_blunder {
            Color::Black
        } else {
            Color::White
        };

        let board_before = boards_before[blunder_i];
        let board_after = boards_before[blunder_i + 1];
        let uci_move = &positions[blunder_i].1;

        let puzzle_result =
            extend_puzzle_line(engine, &board_after, nodes, solver_color).await?;

        if let Some((mainline_moves, cp)) = puzzle_result {
            if mainline_moves.len() < MIN_PUZZLE_LENGTH || cp.abs() < MIN_PUZZLE_CP {
                continue;
            }

            let blunder_move = parse_uci_move(&board_before, uci_move);
            if blunder_move.is_none() {
                continue;
            }
            let blunder_move = blunder_move.unwrap();

            let mut mainline = Vec::new();
            mainline.push(PuzzleNode {
                board_before,
                board_after,
                chess_move: blunder_move,
                ply: 0,
            });

            let mut current_board = board_after;
            for (j, uci) in mainline_moves.iter().enumerate() {
                if let Some(m) = parse_uci_move(&current_board, uci) {
                    if current_board.legal(m) {
                        let next_board = current_board.make_move_new(m);
                        mainline.push(PuzzleNode {
                            board_before: current_board,
                            board_after: next_board,
                            chess_move: m,
                            ply: j + 1,
                        });
                        current_board = next_board;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }

            if mainline.len() >= MIN_PUZZLE_LENGTH {
                let puzzle_cp = cp.abs();
                let puzzle = Puzzle {
                    id: format!("{}_m{}", game_id, blunder_i),
                    mainline,
                    pov: solver_color,
                    cp: puzzle_cp,
                };

                let tags = cook::cook(&puzzle);

                // Check if solver actually found the puzzle
                let solver_is_white = solver_color == Color::White;
                let found = if !mainline_moves.is_empty() && blunder_i + 1 < positions.len() {
                    let actual_move = &positions[blunder_i + 1].1;
                    let solution_move = &mainline_moves[0];
                    actual_move == solution_move
                } else {
                    false
                };

                // Get eval before blunder from solver's perspective
                let eval_before_raw = evals[blunder_i];
                let cp_before_blunder = if solver_is_white {
                    eval_before_raw
                } else {
                    -eval_before_raw
                };

                puzzles.push(PuzzleOutput {
                    fen: puzzle.initial_board().to_string(),
                    moves: puzzle
                        .mainline
                        .iter()
                        .map(|n| {
                            format!(
                                "{}{}{}",
                                n.chess_move.get_source(),
                                n.chess_move.get_dest(),
                                n.chess_move
                                    .get_promotion()
                                    .map(|p| match p {
                                        Piece::Queen => "q",
                                        Piece::Rook => "r",
                                        Piece::Bishop => "b",
                                        Piece::Knight => "n",
                                        _ => "",
                                    })
                                    .unwrap_or("")
                            )
                        })
                        .collect(),
                    cp: puzzle_cp,
                    themes: tags
                        .iter()
                        .map(|t| {
                            serde_json::to_value(t)
                                .unwrap()
                                .as_str()
                                .unwrap_or("")
                                .to_string()
                        })
                        .collect(),
                    solver_is_white,
                    found,
                    cp_before_blunder,
                });

                puzzle_objects.push(puzzle);
            }
        }
    }

    // Zugzwang detection via null-move analysis
    info!(game_id, "Detecting zugzwang");
    for (pidx, puzzle) in puzzle_objects.iter().enumerate() {
        // Only endgame-like positions
        if piece_map_count(puzzle.end_board()) > 16 {
            continue;
        }
        // Skip if already checkmate
        if MoveGen::new_legal(puzzle.end_board()).len() == 0
            && puzzle.end_board().checkers().popcnt() > 0
        {
            continue;
        }

        let mut zug_evals: Vec<ZugzwangEval> = Vec::new();

        for (i, node) in puzzle.mainline.iter().enumerate() {
            // Only solver moves (odd indices)
            if i % 2 == 0 {
                continue;
            }

            let board_after = &node.board_after;

            // Skip positions in check (null-move would be illegal)
            if board_after.checkers().popcnt() > 0 {
                zug_evals.push(ZugzwangEval {
                    cp: 0,
                    null_cp: 0,
                    mate: None,
                    null_mate: None,
                });
                continue;
            }

            // Skip positions with >15 legal moves
            if MoveGen::new_legal(board_after).len() > 15 {
                zug_evals.push(ZugzwangEval {
                    cp: 0,
                    null_cp: 0,
                    mate: None,
                    null_mate: None,
                });
                continue;
            }

            let fen = board_after.to_string();
            let null_fen = null_move_fen(&fen);

            // Evaluate normal position
            let normal_result = engine.evaluate(&fen, nodes.min(50_000)).await?;

            // Evaluate null-move position
            let null_result = engine.evaluate(&null_fen, nodes.min(50_000)).await?;

            zug_evals.push(ZugzwangEval {
                cp: normal_result.cp.unwrap_or(0),
                null_cp: null_result.cp.unwrap_or(0),
                mate: normal_result.mate,
                null_mate: null_result.mate,
            });
        }

        if cook::cook_zugzwang(puzzle, &zug_evals) {
            let tag_str = serde_json::to_value(TagKind::Zugzwang)
                .unwrap()
                .as_str()
                .unwrap_or("zugzwang")
                .to_string();
            if pidx < puzzles.len() && !puzzles[pidx].themes.contains(&tag_str) {
                puzzles[pidx].themes.push(tag_str);
            }
        }
    }

    // Compute final stats
    let white_accuracy = analysis::calculate_accuracy(white_cp_loss, white_move_count);
    let black_accuracy = analysis::calculate_accuracy(black_cp_loss, black_move_count);
    let white_avg = if white_move_count > 0 {
        white_cp_loss as f64 / white_move_count as f64
    } else {
        0.0
    };
    let black_avg = if black_move_count > 0 {
        black_cp_loss as f64 / black_move_count as f64
    } else {
        0.0
    };

    let endgame_segments = eg_tracker.finish();

    // Game-level tags (queen sacrifice, etc.)
    // Puzzle tags are disabled pending found-flag fix, but game-level tags
    // are computed from the actual moves played and don't need puzzle verification.
    let mut all_tags: Vec<String> = Vec::new();

    // Queen sacrifice detection (uses pre-computed data, no extra SF calls)
    let user_color = if game.user_color == "white" { Color::White } else { Color::Black };
    let positions_uci: Vec<String> = positions.iter().map(|(_, uci, _)| uci.clone()).collect();
    let has_queen_sac = crate::queen_sac::detect_queen_sacrifice(
        &boards_before,
        &chess_moves,
        user_color,
        &evals,
        &best_moves,
        &positions_uci,
    );
    if has_queen_sac {
        all_tags.push("queen_sacrifice".to_string());
    }

    // Rook sacrifice detection
    if crate::rook_sac::detect_rook_sacrifice(
        &boards_before, &chess_moves, user_color,
        &evals, &best_moves, &positions_uci,
    ) {
        all_tags.push("rook_sacrifice".to_string());
    }

    // Final-position detectors (smothered mate, king mate, castling mate, en passant mate)
    let final_board = boards_before.last().copied().unwrap_or_default();
    if crate::smothered_mate::detect_smothered_mate(&final_board, user_color) {
        all_tags.push("smothered_mate".to_string());
    }
    if let Some(&last_move) = chess_moves.last() {
        let board_before_last = boards_before.get(chess_moves.len() - 1).copied().unwrap_or_default();
        if crate::king_mate::detect_king_mate(&final_board, &board_before_last, last_move, user_color) {
            all_tags.push("king_mate".to_string());
        }
        if crate::castling_mate::detect_castling_mate(&final_board, &board_before_last, last_move, user_color) {
            all_tags.push("castling_mate".to_string());
        }
        if crate::en_passant_mate::detect_en_passant_mate(&final_board, &board_before_last, last_move, user_color) {
            all_tags.push("en_passant_mate".to_string());
        }
    }

    // Build final analysis JSON
    let analysis = serde_json::json!({
        "moves": move_outputs,
        "white_accuracy": white_accuracy,
        "black_accuracy": black_accuracy,
        "white_avg_cp_loss": white_avg,
        "black_avg_cp_loss": black_avg,
        "white_classifications": white_class,
        "black_classifications": black_class,
        "puzzles": puzzles,
        "endgame_segments": endgame_segments,
        "tags": all_tags,
        "isComplete": true,
    });

    // Save to database
    info!(game_id, "Saving analysis");
    db::save_game_analysis(pool, game_id, &analysis).await?;

    info!(
        game_id,
        puzzles = puzzles.len(),
        white_accuracy,
        black_accuracy,
        "Analysis complete"
    );

    Ok(())
}

/// Convert eval result to centipawns from white's perspective
fn eval_to_white_cp(cp: Option<i32>, mate: Option<i32>, is_white_to_move: bool) -> i32 {
    if let Some(m) = mate {
        let mate_score = if m > 0 {
            10000 - m * 10
        } else {
            -10000 - m * 10
        };
        if is_white_to_move {
            mate_score
        } else {
            -mate_score
        }
    } else if let Some(c) = cp {
        if is_white_to_move {
            c
        } else {
            -c
        }
    } else {
        0
    }
}

/// Construct null-move FEN (flip side to move, clear en passant)
fn null_move_fen(fen: &str) -> String {
    let parts: Vec<&str> = fen.split(' ').collect();
    if parts.len() < 4 {
        return fen.to_string();
    }
    let side = if parts[1] == "w" { "b" } else { "w" };
    let mut result = String::new();
    result.push_str(parts[0]);
    result.push(' ');
    result.push_str(side);
    result.push(' ');
    result.push_str(parts[2]); // castling
    result.push_str(" - "); // clear en passant
    if parts.len() > 4 {
        result.push_str(parts[4]); // halfmove clock
    } else {
        result.push('0');
    }
    if parts.len() > 5 {
        result.push(' ');
        result.push_str(parts[5]); // fullmove number
    } else {
        result.push_str(" 1");
    }
    result
}

/// Extend a puzzle line using multi-PV analysis
async fn extend_puzzle_line(
    engine: &mut StockfishEngine,
    board: &Board,
    nodes: u32,
    solver_color: Color,
) -> Result<Option<(Vec<String>, i32)>, WorkerError> {
    let max_puzzle_length = 20;
    let mut current_board = *board;
    let mut line_moves = Vec::new();
    let mut final_cp = 0;

    for _ in 0..max_puzzle_length / 2 {
        let fen = current_board.to_string();
        let is_solver_turn = current_board.side_to_move() == solver_color;

        if is_solver_turn {
            let lines = engine.evaluate_multipv(&fen, nodes, 2).await?;

            if lines.is_empty() || lines[0].pv.is_empty() {
                break;
            }

            if lines.len() >= 2 {
                let is_white = current_board.side_to_move() == Color::White;
                let best_cp = eval_to_white_cp(lines[0].cp, lines[0].mate, is_white);
                let second_cp = eval_to_white_cp(lines[1].cp, lines[1].mate, is_white);

                let advantage = if solver_color == Color::White {
                    best_cp - second_cp
                } else {
                    second_cp - best_cp
                };

                if advantage < 100 && lines[0].mate.is_none() {
                    break;
                }
            }

            let best_move_uci = &lines[0].pv[0];
            line_moves.push(best_move_uci.clone());

            if let Some(m) = parse_uci_move(&current_board, best_move_uci) {
                if current_board.legal(m) {
                    current_board = current_board.make_move_new(m);
                    let is_white = current_board.side_to_move() == Color::White;
                    final_cp = eval_to_white_cp(lines[0].cp, lines[0].mate, !is_white);

                    if MoveGen::new_legal(&current_board).len() == 0 {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        } else {
            let lines = engine.evaluate_multipv(&fen, nodes, 1).await?;

            if lines.is_empty() || lines[0].pv.is_empty() {
                break;
            }

            let best_move_uci = &lines[0].pv[0];
            line_moves.push(best_move_uci.clone());

            if let Some(m) = parse_uci_move(&current_board, best_move_uci) {
                if current_board.legal(m) {
                    current_board = current_board.make_move_new(m);
                    if MoveGen::new_legal(&current_board).len() == 0 {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    if line_moves.is_empty() {
        return Ok(None);
    }

    Ok(Some((line_moves, final_cp)))
}

/// Find the chess move matching a SAN string
fn find_san_move(
    board: &Board,
    san: &str,
) -> Result<chess::ChessMove, WorkerError> {
    let clean = san.trim_end_matches(|c: char| c == '+' || c == '#' || c == '!' || c == '?');

    let legal_moves: Vec<chess::ChessMove> = MoveGen::new_legal(board).collect();

    // Handle castling
    if clean == "O-O" || clean == "0-0" {
        for m in &legal_moves {
            let src = m.get_source();
            let dst = m.get_dest();
            if board.piece_on(src) == Some(Piece::King) {
                let src_file = src.get_file().to_index();
                let dst_file = dst.get_file().to_index();
                if dst_file > src_file && (dst_file - src_file) == 2 {
                    return Ok(*m);
                }
            }
        }
        return Err(WorkerError::Analysis(format!(
            "No kingside castling move found for: {}",
            san
        )));
    }
    if clean == "O-O-O" || clean == "0-0-0" {
        for m in &legal_moves {
            let src = m.get_source();
            let dst = m.get_dest();
            if board.piece_on(src) == Some(Piece::King) {
                let src_file = src.get_file().to_index();
                let dst_file = dst.get_file().to_index();
                if src_file > dst_file && (src_file - dst_file) == 2 {
                    return Ok(*m);
                }
            }
        }
        return Err(WorkerError::Analysis(format!(
            "No queenside castling move found for: {}",
            san
        )));
    }

    // Parse piece, disambiguation, capture, destination, promotion
    let bytes = clean.as_bytes();
    if bytes.is_empty() {
        return Err(WorkerError::Analysis("Empty SAN move".to_string()));
    }

    let (piece, rest) = if bytes[0].is_ascii_uppercase() {
        let p = match bytes[0] {
            b'K' => Piece::King,
            b'Q' => Piece::Queen,
            b'R' => Piece::Rook,
            b'B' => Piece::Bishop,
            b'N' => Piece::Knight,
            _ => {
                return Err(WorkerError::Analysis(format!(
                    "Unknown piece: {}",
                    bytes[0] as char
                )))
            }
        };
        (p, &clean[1..])
    } else {
        (Piece::Pawn, clean)
    };

    // Extract promotion
    let (rest, promotion) = if let Some(eq_pos) = rest.find('=') {
        let promo_piece = match rest.as_bytes().get(eq_pos + 1) {
            Some(b'Q') => Some(Piece::Queen),
            Some(b'R') => Some(Piece::Rook),
            Some(b'B') => Some(Piece::Bishop),
            Some(b'N') => Some(Piece::Knight),
            _ => None,
        };
        (&rest[..eq_pos], promo_piece)
    } else {
        (rest, None)
    };

    // Remove captures marker
    let rest = rest.replace('x', "");

    // The last two characters should be the destination square
    let rest_bytes = rest.as_bytes();
    if rest_bytes.len() < 2 {
        return Err(WorkerError::Analysis(format!("SAN too short: {}", san)));
    }

    let dest_file = rest_bytes[rest_bytes.len() - 2];
    let dest_rank = rest_bytes[rest_bytes.len() - 1];

    if !(b'a'..=b'h').contains(&dest_file) || !(b'1'..=b'8').contains(&dest_rank) {
        return Err(WorkerError::Analysis(format!(
            "Invalid destination in SAN: {}",
            san
        )));
    }

    let dest = chess::Square::make_square(
        chess::Rank::from_index((dest_rank - b'1') as usize),
        chess::File::from_index((dest_file - b'a') as usize),
    );

    // Disambiguation
    let disambig = &rest[..rest.len() - 2];

    let mut candidates: Vec<chess::ChessMove> = legal_moves
        .into_iter()
        .filter(|m| {
            m.get_dest() == dest
                && board.piece_on(m.get_source()) == Some(piece)
                && m.get_promotion() == promotion
        })
        .collect();

    if candidates.len() == 1 {
        return Ok(candidates[0]);
    }

    if !disambig.is_empty() {
        let disambig_bytes = disambig.as_bytes();
        candidates.retain(|m| {
            let src = m.get_source();
            for &b in disambig_bytes {
                if (b'a'..=b'h').contains(&b) {
                    if src.get_file().to_index() != (b - b'a') as usize {
                        return false;
                    }
                } else if (b'1'..=b'8').contains(&b) {
                    if src.get_rank().to_index() != (b - b'1') as usize {
                        return false;
                    }
                }
            }
            true
        });
    }

    match candidates.len() {
        1 => Ok(candidates[0]),
        0 => Err(WorkerError::Analysis(format!(
            "No legal move matches SAN: {}",
            san
        ))),
        _ => Err(WorkerError::Analysis(format!(
            "Ambiguous SAN: {} ({} candidates)",
            san,
            candidates.len()
        ))),
    }
}

fn update_class(class: &mut ClassificationsOutput, classification: &str) {
    match classification {
        "best" => class.best += 1,
        "excellent" => class.excellent += 1,
        "good" => class.good += 1,
        "inaccuracy" => class.inaccuracy += 1,
        "mistake" => class.mistake += 1,
        "blunder" => class.blunder += 1,
        "forced" => class.forced += 1,
        "book" => class.book += 1,
        _ => {}
    }
}
