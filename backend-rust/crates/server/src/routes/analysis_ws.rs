/// WebSocket analysis route — axum 0.8 port of chess_puzzler::server
///
/// Orchestrates game analysis by delegating Stockfish evaluation to the client
/// (browser WASM) while keeping puzzle extraction and cook() theme classification
/// server-side.

use anyhow::Result;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
};
use chess_puzzler::chess::{Board, Color, MoveGen, Piece};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};

use chess_puzzler::analysis;
use chess_puzzler::board_utils::piece_map_count;
use chess_puzzler::endgame::{EndgameSegment, EndgameTracker};
use chess_puzzler::puzzle::cook;
use chess_puzzler::puzzle::extraction::{
    parse_uci_move, BLUNDER_THRESHOLD, MIN_PUZZLE_CP, MIN_PUZZLE_LENGTH,
};
use chess_puzzler::puzzle::{Puzzle, PuzzleNode, TagKind};
use chess_puzzler::tactics::zugzwang::ZugzwangEval;

// ---- Message types ----

/// Server → Client messages
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerMessage {
    EvalBatch {
        positions: Vec<EvalPosition>,
    },
    EvalMultiPv {
        request_id: u32,
        fen: String,
        nodes: u32,
        multipv: u32,
    },
    ZugzwangTest {
        positions: Vec<ZugzwangPosition>,
    },
    Progress {
        phase: String,
        current: u32,
        total: u32,
    },
    AnalysisComplete {
        result: FullAnalysisResult,
    },
    Error {
        message: String,
    },
}

#[derive(Serialize)]
struct EvalPosition {
    id: u32,
    fen: String,
    nodes: u32,
}

#[derive(Serialize)]
struct ZugzwangPosition {
    /// Index into the puzzle list
    puzzle_idx: u32,
    /// Solver move index within the puzzle
    solver_idx: u32,
    /// FEN after the solver move (normal eval)
    fen: String,
    /// FEN with side-to-move flipped (null-move eval)
    null_fen: String,
    /// Nodes for the engine search
    nodes: u32,
}

/// Client → Server messages
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMessage {
    AnalyzeGame {
        game_id: String,
        moves: Vec<String>,
        nodes: Option<u32>,
    },
    EvalResults {
        results: Vec<EvalResult>,
    },
    MultiPvResult {
        request_id: u32,
        lines: Vec<PvLineResult>,
    },
    ZugzwangResults {
        results: Vec<ZugzwangResult>,
    },
}

#[derive(Deserialize)]
struct EvalResult {
    id: u32,
    cp: Option<i32>,
    mate: Option<i32>,
    best_move: String,
}

#[derive(Deserialize)]
struct PvLineResult {
    pv: Vec<String>,
    cp: Option<i32>,
    mate: Option<i32>,
}

#[derive(Deserialize)]
struct ZugzwangResult {
    puzzle_idx: u32,
    solver_idx: u32,
    cp: Option<i32>,
    null_cp: Option<i32>,
    mate: Option<i32>,
    null_mate: Option<i32>,
}

/// Full analysis result sent to client
#[derive(Serialize)]
struct FullAnalysisResult {
    moves: Vec<MoveOutput>,
    white_accuracy: f64,
    black_accuracy: f64,
    white_avg_cp_loss: f64,
    black_avg_cp_loss: f64,
    white_classifications: ClassificationsOutput,
    black_classifications: ClassificationsOutput,
    puzzles: Vec<PuzzleOutput>,
    endgame_segments: Vec<EndgameSegment>,
    #[serde(rename = "isComplete")]
    is_complete: bool,
}

#[derive(Serialize)]
struct MoveOutput {
    #[serde(rename = "move")]
    move_uci: String,
    move_eval: i32,
    best_move: String,
    best_eval: i32,
    cp_loss: i32,
    classification: String,
}

#[derive(Serialize, Default)]
struct ClassificationsOutput {
    best: u32,
    excellent: u32,
    good: u32,
    inaccuracy: u32,
    mistake: u32,
    blunder: u32,
    book: u32,
    forced: u32,
}

#[derive(Serialize)]
struct PuzzleOutput {
    fen: String,
    moves: Vec<String>,
    cp: i32,
    themes: Vec<String>,
}

// ---- WebSocket handler ----

pub async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(socket: WebSocket) {
    let (mut sender, mut receiver) = socket.split();

    while let Some(Ok(msg)) = receiver.next().await {
        let text = match msg {
            Message::Text(t) => t.to_string(),
            Message::Close(_) => break,
            _ => continue,
        };

        let client_msg: ClientMessage = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(e) => {
                let _ = sender
                    .send(Message::Text(
                        serde_json::to_string(&ServerMessage::Error {
                            message: format!("Invalid message: {}", e),
                        })
                        .unwrap()
                        .into(),
                    ))
                    .await;
                continue;
            }
        };

        match client_msg {
            ClientMessage::AnalyzeGame {
                game_id,
                moves,
                nodes,
            } => {
                let nodes = nodes.unwrap_or(100_000);
                match orchestrate_analysis(
                    &mut sender,
                    &mut receiver,
                    &game_id,
                    &moves,
                    nodes,
                )
                .await
                {
                    Ok(()) => {}
                    Err(e) => {
                        let _ = sender
                            .send(Message::Text(
                                serde_json::to_string(&ServerMessage::Error {
                                    message: format!("Analysis failed: {}", e),
                                })
                                .unwrap()
                                .into(),
                            ))
                            .await;
                    }
                }
            }
            _ => {
                let _ = sender
                    .send(Message::Text(
                        serde_json::to_string(&ServerMessage::Error {
                            message: "Expected analyze_game message".into(),
                        })
                        .unwrap()
                        .into(),
                    ))
                    .await;
            }
        }
    }
}

// ---- Helper: convert eval result to centipawns from white's perspective ----

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

// ---- Helper: send message ----

async fn send_msg(
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
    msg: &ServerMessage,
) -> Result<()> {
    let json = serde_json::to_string(msg)?;
    sender.send(Message::Text(json.into())).await?;
    Ok(())
}

// ---- Helper: receive next client message ----

async fn recv_msg(
    receiver: &mut futures::stream::SplitStream<WebSocket>,
) -> Result<ClientMessage> {
    loop {
        match receiver.next().await {
            Some(Ok(Message::Text(t))) => {
                return Ok(serde_json::from_str(&t.to_string())?);
            }
            Some(Ok(Message::Close(_))) | None => {
                anyhow::bail!("WebSocket closed");
            }
            _ => continue,
        }
    }
}

// ---- Helper: construct null-move FEN (flip side to move, clear en passant) ----

fn null_move_fen(fen: &str) -> String {
    let parts: Vec<&str> = fen.split(' ').collect();
    if parts.len() < 4 {
        return fen.to_string();
    }
    let side = if parts[1] == "w" { "b" } else { "w" };
    // Reconstruct with flipped side and cleared en passant
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

// ---- Core orchestrator ----

async fn orchestrate_analysis(
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
    receiver: &mut futures::stream::SplitStream<WebSocket>,
    game_id: &str,
    san_moves: &[String],
    nodes: u32,
) -> Result<()> {
    // Step 1: Parse SAN moves to get FENs and UCI moves
    let mut board = Board::default();
    let mut positions: Vec<(String, String, Board)> = Vec::new();
    let mut boards_before: Vec<Board> = vec![board];
    let mut legal_counts: Vec<usize> = Vec::new();

    let start_fen = board.to_string();

    for san in san_moves {
        let fen_before = board.to_string();
        let legal_count = MoveGen::new_legal(&board).len();
        legal_counts.push(legal_count);

        let chess_move = find_san_move(&board, san)?;
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

        board = board.make_move_new(chess_move);
        positions.push((fen_before, uci, board));
        boards_before.push(board);
    }

    let total_moves = positions.len() as u32;

    // Step 2: Build eval batch — starting position + position after each move
    let mut eval_positions = Vec::new();
    eval_positions.push(EvalPosition {
        id: 0,
        fen: start_fen.clone(),
        nodes,
    });
    for (i, (_, _, board_after)) in positions.iter().enumerate() {
        eval_positions.push(EvalPosition {
            id: (i + 1) as u32,
            fen: board_after.to_string(),
            nodes,
        });
    }

    send_msg(
        sender,
        &ServerMessage::EvalBatch {
            positions: eval_positions,
        },
    )
    .await?;
    send_msg(
        sender,
        &ServerMessage::Progress {
            phase: "eval".into(),
            current: 0,
            total: total_moves,
        },
    )
    .await?;

    // Wait for eval results
    let eval_results = match recv_msg(receiver).await? {
        ClientMessage::EvalResults { results } => results,
        _ => anyhow::bail!("Expected eval_results"),
    };

    // Build eval map: id → (cp_white_perspective)
    let mut evals: Vec<i32> = vec![0; positions.len() + 1];
    for r in &eval_results {
        let id = r.id as usize;
        if id < evals.len() {
            let is_white = if id == 0 {
                true
            } else {
                id % 2 == 0
            };
            evals[id] = eval_to_white_cp(r.cp, r.mate, is_white);
        }
    }

    // Step 3: Classify each move
    let mut move_outputs = Vec::new();
    let mut puzzles: Vec<PuzzleOutput> = Vec::new();
    let mut eg_tracker = EndgameTracker::new();

    let mut white_cp_loss = 0;
    let mut black_cp_loss = 0;
    let mut white_move_count = 0u32;
    let mut black_move_count = 0u32;
    let mut white_class = ClassificationsOutput::default();
    let mut black_class = ClassificationsOutput::default();

    let mut best_moves: Vec<String> = vec![String::new(); positions.len() + 1];
    for r in &eval_results {
        let id = r.id as usize;
        if id < best_moves.len() {
            best_moves[id] = r.best_move.clone();
        }
    }

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
                board_after, eval_after, 0, "forced", uci_move, uci_move, fen_before, is_white, i,
            );
            continue;
        }

        let cp_loss =
            analysis::calculate_cp_loss(eval_before, eval_after, is_white, is_checkmate);
        let mate_blunder =
            analysis::is_mate_blunder(eval_before, eval_after, is_white, is_checkmate);
        let classification = analysis::classify_move(cp_loss, mate_blunder);

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

    send_msg(
        sender,
        &ServerMessage::Progress {
            phase: "eval".into(),
            current: total_moves,
            total: total_moves,
        },
    )
    .await?;

    // Step 4: Puzzle extraction for blunders
    let total_blunders = blunder_indices.len() as u32;
    let mut request_id_counter: u32 = 0;

    // Store puzzle objects alongside outputs for zugzwang post-processing
    let mut puzzle_objects: Vec<Puzzle> = Vec::new();

    for (puzzle_idx, &blunder_i) in blunder_indices.iter().enumerate() {
        send_msg(
            sender,
            &ServerMessage::Progress {
                phase: "puzzles".into(),
                current: puzzle_idx as u32,
                total: total_blunders,
            },
        )
        .await?;

        let is_white_blunder = blunder_i % 2 == 0;
        let solver_color = if is_white_blunder {
            Color::Black
        } else {
            Color::White
        };

        let board_before = boards_before[blunder_i];
        let board_after = boards_before[blunder_i + 1];
        let uci_move = &positions[blunder_i].1;

        let puzzle_result = extend_puzzle_line_ws(
            sender,
            receiver,
            &board_after,
            nodes,
            solver_color,
            &mut request_id_counter,
        )
        .await;

        if let Ok(Some((mainline_moves, cp))) = puzzle_result {
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
                });

                puzzle_objects.push(puzzle);
            }
        }
    }

    send_msg(
        sender,
        &ServerMessage::Progress {
            phase: "puzzles".into(),
            current: total_blunders,
            total: total_blunders,
        },
    )
    .await?;

    // Step 4b: Zugzwang detection via engine null-move analysis
    // Collect positions for puzzles that might have zugzwang (endgame, ≤8 pieces)
    let mut zug_positions: Vec<ZugzwangPosition> = Vec::new();

    for (pidx, puzzle) in puzzle_objects.iter().enumerate() {
        // Only endgame-like positions (pawns + kings can be up to 18 pieces
        // but still zugzwang-prone; 16 filters out crowded middlegames)
        if piece_map_count(puzzle.end_board()) > 16 {
            continue;
        }
        // Don't bother if already checkmate
        if MoveGen::new_legal(puzzle.end_board()).len() == 0
            && puzzle.end_board().checkers().popcnt() > 0
        {
            continue;
        }

        for (i, node) in puzzle.mainline.iter().enumerate() {
            // Only solver moves (odd indices)
            if i % 2 == 0 {
                continue;
            }

            let board_after = &node.board_after;

            // Skip positions in check
            if board_after.checkers().popcnt() > 0 {
                continue;
            }

            // Skip positions with >15 legal moves
            if MoveGen::new_legal(board_after).len() > 15 {
                continue;
            }

            let fen = board_after.to_string();
            let nfen = null_move_fen(&fen);
            let solver_idx = (i / 2) as u32;

            zug_positions.push(ZugzwangPosition {
                puzzle_idx: pidx as u32,
                solver_idx,
                fen,
                null_fen: nfen,
                nodes: nodes.min(50_000), // lower node count for zugzwang test
            });
        }
    }

    if !zug_positions.is_empty() {
        send_msg(
            sender,
            &ServerMessage::ZugzwangTest {
                positions: zug_positions,
            },
        )
        .await?;

        // Wait for results
        if let Ok(ClientMessage::ZugzwangResults { results }) = recv_msg(receiver).await {
            // Group results by puzzle_idx
            let mut evals_by_puzzle: std::collections::HashMap<u32, Vec<(u32, ZugzwangEval)>> =
                std::collections::HashMap::new();

            for r in &results {
                let eval = ZugzwangEval {
                    cp: r.cp.unwrap_or(0),
                    null_cp: r.null_cp.unwrap_or(0),
                    mate: r.mate,
                    null_mate: r.null_mate,
                };
                evals_by_puzzle
                    .entry(r.puzzle_idx)
                    .or_default()
                    .push((r.solver_idx, eval));
            }

            for (pidx, mut eval_pairs) in evals_by_puzzle {
                let pidx = pidx as usize;
                if pidx >= puzzle_objects.len() || pidx >= puzzles.len() {
                    continue;
                }

                // Sort by solver_idx to build ordered eval vec
                eval_pairs.sort_by_key(|(idx, _)| *idx);

                // Build evals array with correct length
                let max_solver_idx = eval_pairs.last().map(|(idx, _)| *idx).unwrap_or(0);
                let mut zug_evals = Vec::with_capacity((max_solver_idx + 1) as usize);
                let mut pair_iter = eval_pairs.into_iter().peekable();

                for si in 0..=max_solver_idx {
                    if pair_iter.peek().map(|(idx, _)| *idx) == Some(si) {
                        let (_, eval) = pair_iter.next().unwrap();
                        zug_evals.push(eval);
                    } else {
                        // No eval for this solver move — use neutral placeholder
                        zug_evals.push(ZugzwangEval {
                            cp: 0,
                            null_cp: 0,
                            mate: None,
                            null_mate: None,
                        });
                    }
                }

                if cook::cook_zugzwang(&puzzle_objects[pidx], &zug_evals) {
                    let tag_str = serde_json::to_value(TagKind::Zugzwang)
                        .unwrap()
                        .as_str()
                        .unwrap_or("zugzwang")
                        .to_string();
                    if !puzzles[pidx].themes.contains(&tag_str) {
                        puzzles[pidx].themes.push(tag_str);
                    }
                }
            }
        }
    }

    // Step 5: Compute final stats
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

    let result = FullAnalysisResult {
        moves: move_outputs,
        white_accuracy,
        black_accuracy,
        white_avg_cp_loss: white_avg,
        black_avg_cp_loss: black_avg,
        white_classifications: white_class,
        black_classifications: black_class,
        puzzles,
        endgame_segments,
        is_complete: true,
    };

    send_msg(sender, &ServerMessage::AnalysisComplete { result }).await?;

    Ok(())
}

/// Extend a puzzle line using WebSocket multi-PV requests.
async fn extend_puzzle_line_ws(
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
    receiver: &mut futures::stream::SplitStream<WebSocket>,
    board: &Board,
    nodes: u32,
    solver_color: Color,
    request_id_counter: &mut u32,
) -> Result<Option<(Vec<String>, i32)>> {
    let max_puzzle_length = 20;
    let mut current_board = *board;
    let mut line_moves = Vec::new();
    let mut final_cp = 0;

    for _ in 0..max_puzzle_length / 2 {
        let fen = current_board.to_string();
        let is_solver_turn = current_board.side_to_move() == solver_color;

        if is_solver_turn {
            *request_id_counter += 1;
            let req_id = *request_id_counter;

            send_msg(
                sender,
                &ServerMessage::EvalMultiPv {
                    request_id: req_id,
                    fen: fen.clone(),
                    nodes,
                    multipv: 2,
                },
            )
            .await?;

            let lines = match recv_msg(receiver).await? {
                ClientMessage::MultiPvResult { request_id, lines } if request_id == req_id => {
                    lines
                }
                _ => break,
            };

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
            *request_id_counter += 1;
            let req_id = *request_id_counter;

            send_msg(
                sender,
                &ServerMessage::EvalMultiPv {
                    request_id: req_id,
                    fen: fen.clone(),
                    nodes,
                    multipv: 1,
                },
            )
            .await?;

            let lines = match recv_msg(receiver).await? {
                ClientMessage::MultiPvResult { request_id, lines } if request_id == req_id => {
                    lines
                }
                _ => break,
            };

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

/// Find the chess_puzzler::chess::ChessMove matching a SAN string on the given board.
fn find_san_move(board: &Board, san: &str) -> Result<chess_puzzler::chess::ChessMove> {
    let clean = san.trim_end_matches(|c: char| c == '+' || c == '#' || c == '!' || c == '?');

    let legal_moves: Vec<chess_puzzler::chess::ChessMove> = MoveGen::new_legal(board).collect();

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
        anyhow::bail!("No kingside castling move found for: {}", san);
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
        anyhow::bail!("No queenside castling move found for: {}", san);
    }

    // Parse piece, disambiguation, capture, destination, promotion
    let bytes = clean.as_bytes();
    if bytes.is_empty() {
        anyhow::bail!("Empty SAN move");
    }

    let (piece, rest) = if bytes[0].is_ascii_uppercase() {
        let p = match bytes[0] {
            b'K' => Piece::King,
            b'Q' => Piece::Queen,
            b'R' => Piece::Rook,
            b'B' => Piece::Bishop,
            b'N' => Piece::Knight,
            _ => anyhow::bail!("Unknown piece: {}", bytes[0] as char),
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
        anyhow::bail!("SAN too short: {}", san);
    }

    let dest_file = rest_bytes[rest_bytes.len() - 2];
    let dest_rank = rest_bytes[rest_bytes.len() - 1];

    if !(b'a'..=b'h').contains(&dest_file) || !(b'1'..=b'8').contains(&dest_rank) {
        anyhow::bail!("Invalid destination in SAN: {}", san);
    }

    let dest = chess_puzzler::chess::Square::make_square(
        chess_puzzler::chess::Rank::from_index((dest_rank - b'1') as usize),
        chess_puzzler::chess::File::from_index((dest_file - b'a') as usize),
    );

    // Disambiguation (whatever is left before destination)
    let disambig = &rest[..rest.len() - 2];

    let mut candidates: Vec<chess_puzzler::chess::ChessMove> = legal_moves
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
        0 => anyhow::bail!("No legal move matches SAN: {}", san),
        _ => anyhow::bail!("Ambiguous SAN: {} ({} candidates)", san, candidates.len()),
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
