//! Integration tests: compare our puzzle classifier against Lichess puzzle themes.
//!
//! Each test builds puzzles from real Lichess data (FEN + UCI moves), runs cook(),
//! and checks that we produce the expected tags.
//!
//! Puzzle data sourced from: https://huggingface.co/datasets/Lichess/chess-puzzles

use chess::{Board, Color, MoveGen, Piece, Square, Rank, File};
use chess_puzzler::puzzle::cook::{cook, cook_zugzwang};
use chess_puzzler::puzzle::extraction::parse_uci_move;
use chess_puzzler::puzzle::{Puzzle, PuzzleNode, TagKind};
use chess_puzzler::tactics::zugzwang::ZugzwangEval;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::str::FromStr;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a Puzzle from Lichess CSV fields: FEN, space-separated UCI moves, and CP.
///
/// In the Lichess format:
///   - FEN = position before the first move
///   - Moves[0] = opponent's last move (setup/blunder)
///   - Moves[1..] = solver's moves interleaved with opponent responses
///   - Solver POV = opposite of whose turn it is in the FEN
fn build_puzzle(id: &str, fen: &str, moves: &str, cp: i32) -> Puzzle {
    let mut board = Board::from_str(fen).expect("invalid FEN");
    let move_strs: Vec<&str> = moves.split_whitespace().collect();
    let mut mainline = Vec::new();

    for (i, uci) in move_strs.iter().enumerate() {
        let chess_move =
            parse_uci_move(&board, uci).unwrap_or_else(|| panic!("invalid UCI move: {}", uci));
        let board_after = board.make_move_new(chess_move);
        mainline.push(PuzzleNode {
            board_before: board,
            board_after,
            chess_move,
            ply: i,
        });
        board = board_after;
    }

    let opponent_color = Board::from_str(fen).unwrap().side_to_move();
    let pov = !opponent_color;

    Puzzle {
        id: id.to_string(),
        mainline,
        pov,
        cp,
    }
}

/// Map a Lichess theme string to our TagKind (None for themes we don't classify).
#[allow(dead_code)]
fn lichess_theme_to_tag(theme: &str) -> Option<TagKind> {
    match theme {
        "advancedPawn" => Some(TagKind::AdvancedPawn),
        "advantage" => Some(TagKind::Advantage),
        "anastasiaMate" => Some(TagKind::AnastasiaMate),
        "arabianMate" => Some(TagKind::ArabianMate),
        "attraction" => Some(TagKind::Attraction),
        "backRankMate" => Some(TagKind::BackRankMate),
        "bishopEndgame" => Some(TagKind::BishopEndgame),
        "castling" => Some(TagKind::Castling),
        "clearance" => Some(TagKind::Clearance),
        "crushing" => Some(TagKind::Crushing),
        "defensiveMove" => Some(TagKind::DefensiveMove),
        "deflection" => Some(TagKind::Deflection),
        "discoveredAttack" => Some(TagKind::DiscoveredAttack),
        "doubleBishopMate" => Some(TagKind::DoubleBishopMate),
        "doubleCheck" => Some(TagKind::DoubleCheck),
        "enPassant" => Some(TagKind::EnPassant),
        "equality" => Some(TagKind::Equality),
        "exposedKing" => Some(TagKind::ExposedKing),
        "fork" => Some(TagKind::Fork),
        "hangingPiece" => Some(TagKind::HangingPiece),
        "hookMate" => Some(TagKind::HookMate),
        "interference" => Some(TagKind::Interference),
        "intermezzo" => Some(TagKind::Intermezzo),
        "kingsideAttack" => Some(TagKind::KingsideAttack),
        "knightEndgame" => Some(TagKind::KnightEndgame),
        "long" => Some(TagKind::Long),
        "mate" => Some(TagKind::Mate),
        "mateIn1" => Some(TagKind::MateIn1),
        "mateIn2" => Some(TagKind::MateIn2),
        "mateIn3" => Some(TagKind::MateIn3),
        "mateIn4" => Some(TagKind::MateIn4),
        "mateIn5" => Some(TagKind::MateIn5),
        "oneMove" => Some(TagKind::OneMove),
        "pawnEndgame" => Some(TagKind::PawnEndgame),
        "pin" => Some(TagKind::Pin),
        "promotion" => Some(TagKind::Promotion),
        "queenEndgame" => Some(TagKind::QueenEndgame),
        "queenRookEndgame" => Some(TagKind::QueenRookEndgame),
        "queensideAttack" => Some(TagKind::QueensideAttack),
        "quietMove" => Some(TagKind::QuietMove),
        "rookEndgame" => Some(TagKind::RookEndgame),
        "sacrifice" => Some(TagKind::Sacrifice),
        "short" => Some(TagKind::Short),
        "skewer" => Some(TagKind::Skewer),
        "smotheredMate" => Some(TagKind::SmotheredMate),
        "trappedPiece" => Some(TagKind::TrappedPiece),
        "underPromotion" => Some(TagKind::UnderPromotion),
        "veryLong" => Some(TagKind::VeryLong),
        "xRayAttack" => Some(TagKind::XRayAttack),
        "zugzwang" => Some(TagKind::Zugzwang),
        // Lichess themes we don't classify
        "middlegame" | "endgame" | "opening" | "master" | "masterVsMaster" | "superGM" => None,
        _ => None,
    }
}

/// A puzzle from the Lichess database with its expected themes.
struct LichessPuzzle {
    id: &'static str,
    fen: &'static str,
    moves: &'static str,
    themes: &'static [&'static str],
}

/// Evaluate the initial position (after opponent's blunder) to get real CP.
/// Returns CP from the solver's perspective (positive = solver winning).
fn evaluate_puzzle_cp(sf: &mut StockfishProcess, puzzle: &Puzzle, nodes: u32) -> i32 {
    let board_after_blunder = &puzzle.mainline[0].board_after;
    let fen = board_after_blunder.to_string();
    let (cp, mate) = sf.evaluate(&fen, nodes);

    // Convert from side-to-move perspective to solver's perspective
    let solver_is_white = puzzle.pov == Color::White;
    let stm_is_white = board_after_blunder.side_to_move() == Color::White;

    if solver_is_white == stm_is_white {
        // Solver IS side-to-move, eval is already from solver's perspective
        if let Some(m) = mate {
            if m > 0 { 10000 } else { -10000 }
        } else {
            cp
        }
    } else {
        // Solver is NOT side-to-move, flip
        if let Some(m) = mate {
            if m > 0 { -10000 } else { 10000 }
        } else {
            -cp
        }
    }
}

/// Full engine-integrated classification:
/// 1. Evaluate initial position for real CP
/// 2. Run cook() with real CP
/// 3. Run cook_zugzwang() if position qualifies (endgame, ≤16 pieces)
fn cook_with_engine(sf: &mut StockfishProcess, p: &LichessPuzzle, nodes: u32) -> Vec<TagKind> {
    // Build with temp CP to get the puzzle structure, then evaluate for real CP
    let puzzle = build_puzzle(p.id, p.fen, p.moves, 500);
    let real_cp = evaluate_puzzle_cp(sf, &puzzle, nodes);

    // Rebuild with real CP
    let puzzle = build_puzzle(p.id, p.fen, p.moves, real_cp);
    let mut tags = cook(&puzzle);

    // Zugzwang detection for qualifying positions (endgame, ≤16 pieces)
    // Pre-filter: only run expensive engine evals if at least one solver-move
    // position is not in check and has ≤15 legal moves (matching zugzwang.rs logic).
    let piece_count = puzzle.end_board().combined().popcnt();
    if piece_count <= 16 {
        let has_candidate = puzzle.mainline.iter().enumerate().any(|(i, node)| {
            if i % 2 == 0 { return false; }
            let board = &node.board_after;
            board.checkers().popcnt() == 0
                && chess::MoveGen::new_legal(board).len() <= 15
        });

        if has_candidate {
            let mut evals = Vec::new();
            for (i, node) in puzzle.mainline.iter().enumerate() {
                if i % 2 == 0 {
                    continue; // skip opponent moves
                }
                let board = &node.board_after;
                // Skip positions in check or with many legal moves —
                // zugzwang.rs will skip them anyway, and the null-move FEN
                // of an in-check position is illegal (king can be captured).
                if board.checkers().popcnt() > 0
                    || chess::MoveGen::new_legal(board).len() > 15
                {
                    evals.push(ZugzwangEval { cp: 0, null_cp: 0, mate: None, null_mate: None });
                    continue;
                }
                let fen = node.board_after.to_string();
                let nfen = null_move_fen(&fen);
                let (cp, mate) = sf.evaluate(&fen, 100_000);
                let (null_cp, null_mate) = sf.evaluate(&nfen, 100_000);
                evals.push(ZugzwangEval { cp, null_cp, mate, null_mate });
            }
            if cook_zugzwang(&puzzle, &evals) {
                tags.push(TagKind::Zugzwang);
            }
        }
    }

    tags
}

/// Assert that a specific tag is detected in every puzzle using real Stockfish evaluation.
/// No longer skips CP-dependent tags (Crushing/Advantage/Equality).
fn assert_theme_detected_with_engine(
    sf: &mut StockfishProcess,
    puzzles: &[LichessPuzzle],
    target: TagKind,
    nodes: u32,
) {
    let mut failures = Vec::new();
    for (idx, p) in puzzles.iter().enumerate() {
        let tags = cook_with_engine(sf, p, nodes);
        if tags.contains(&target) {
            eprintln!("  [{}/{}] {} ... ok", idx + 1, puzzles.len(), p.id);
        } else {
            eprintln!("  [{}/{}] {} ... MISS (expected {:?}, got {:?})", idx + 1, puzzles.len(), p.id, target, tags);
            failures.push(format!(
                "  MISS {}: expected {:?}, got {:?}",
                p.id, target, tags
            ));
        }
    }
    if !failures.is_empty() {
        panic!(
            "{:?}: {}/{} failed\n{}",
            target,
            failures.len(),
            puzzles.len(),
            failures.join("\n")
        );
    }
}

// ===========================================================================
// Fork
// ===========================================================================

#[test]
fn test_fork() {
    let sf_path = match find_stockfish() {
        Some(p) => p,
        None => { eprintln!("SKIPPING test_fork: Stockfish not found"); return; }
    };
    let mut sf = StockfishProcess::new(&sf_path);
    let nodes = 50_000u32;

    let puzzles = [
        LichessPuzzle { id: "2FoRk", fen: "8/8/6k1/1q3P2/4N1pp/8/5RK1/8 b - - 0 60", moves: "g6f7 e4d6 f7f6 d6b5", themes: &["crushing", "endgame", "fork", "short"] },
        LichessPuzzle { id: "r11X9", fen: "8/6N1/6b1/7p/7P/5k1K/8/8 w - - 3 56", moves: "g7e6 g6f5 h3h2 f5e6", themes: &["crushing", "endgame", "fork", "short"] },
        LichessPuzzle { id: "f67P5", fen: "r2k4/5R2/4p1p1/p1B5/2P1n1P1/7P/5K2/8 w - - 1 40", moves: "f2f3 e4g5 f3e3 g5f7", themes: &["advantage", "endgame", "fork", "short"] },
        LichessPuzzle { id: "j56l1", fen: "6k1/6p1/1P5p/2n1B3/p7/7P/6P1/6K1 w - - 1 40", moves: "g1f2 c5d3 f2e3 d3e5", themes: &["crushing", "endgame", "fork", "short"] },
        LichessPuzzle { id: "1OH2s", fen: "6k1/p4p1p/4p1p1/3n4/6P1/3R1Q2/5P1P/2q3K1 w - - 2 35", moves: "g1g2 d5f4 g2g3 f4d3", themes: &["advantage", "endgame", "fork", "short"] },
        LichessPuzzle { id: "W0Y4W", fen: "8/6k1/R7/1p1N4/1P2n1p1/P2r4/8/5K2 w - - 4 46", moves: "d5f4 d3f3 f1g1 f3f4", themes: &["advantage", "endgame", "fork", "short"] },
        LichessPuzzle { id: "b8G3I", fen: "3r2k1/5p1p/p1R3n1/1p1P4/3R1b2/8/P3B2P/6K1 w - - 0 32", moves: "c6a6 f4e3 g1f1 e3d4", themes: &["advantage", "endgame", "fork", "short"] },
        LichessPuzzle { id: "O1Q2R", fen: "6r1/1R6/1p1p4/p1p2k1p/P1P2n1R/8/4K2P/8 w - - 6 46", moves: "e2e3 f4g2 e3d3 g2h4", themes: &["crushing", "endgame", "fork", "short"] },
        LichessPuzzle { id: "C3X5r", fen: "4r3/8/3R4/2n3p1/2k3P1/4K2P/4N1P1/8 w - - 1 57", moves: "e3f2 c5e4 f2g1 e4d6", themes: &["crushing", "endgame", "fork", "short"] },
        LichessPuzzle { id: "MfIvc", fen: "8/6p1/p2P1k2/2n5/6p1/6P1/6K1/4R3 w - - 0 44", moves: "g2f2 c5d3 f2e3 d3e1", themes: &["crushing", "endgame", "fork", "short"] },
    ];
    assert_theme_detected_with_engine(&mut sf, &puzzles, TagKind::Fork, nodes);
}

// ===========================================================================
// Pin
// ===========================================================================

#[test]
fn test_pin() {
    let sf_path = match find_stockfish() {
        Some(p) => p,
        None => { eprintln!("SKIPPING test_pin: Stockfish not found"); return; }
    };
    let mut sf = StockfishProcess::new(&sf_path);
    let nodes = 50_000u32;

    let puzzles = [
        LichessPuzzle { id: "GiYGO", fen: "r4rk1/p1p3bp/p2p4/3P1pq1/5R2/P1N4P/1PPQ2P1/5RK1 b - - 3 21", moves: "g7e5 f4g4 g5g4 h3g4", themes: &["advantage", "middlegame", "pin", "short"] },
        LichessPuzzle { id: "NAqS7", fen: "k5r1/1p3p2/p1p5/4n3/P3q3/1P6/2K1B2Q/R6R w - - 16 41", moves: "c2b2 g8g2 h2h8 a8a7", themes: &["advantage", "middlegame", "pin", "short"] },
        LichessPuzzle { id: "4hnHv", fen: "r5k1/p4r1p/1p1p3B/2n1p2R/4P3/2Q2P2/q2R1K2/8 w - - 0 29", moves: "d2a2 c5e4 f2e3 e4c3", themes: &["advantage", "middlegame", "pin", "short"] },
        LichessPuzzle { id: "Is282", fen: "1r3b1k/2r2p2/6p1/p1q1p3/P1B1P3/1P1Q4/K1P3PP/3R3R w - - 3 26", moves: "a2b1 c5c4 d3c4 c7c4", themes: &["advantage", "middlegame", "pin", "short"] },
        LichessPuzzle { id: "G5K5U", fen: "5b2/n1r2k2/4p3/P1r1P3/1Q2BB1p/1K5P/1P6/8 w - - 5 44", moves: "f4e3 c5b5 b4b5 a7b5", themes: &["advantage", "middlegame", "pin", "short"] },
        LichessPuzzle { id: "hkedo", fen: "3r1n2/1p2k3/p1b1p3/5B2/1p6/P1b5/4Q1P1/2R2K2 w - - 1 35", moves: "a3b4 c6b5 e2b5 a6b5", themes: &["advantage", "middlegame", "pin", "short"] },
        LichessPuzzle { id: "aNl9H", fen: "r2r2k1/p4p1N/1p2p1p1/3q4/P2p1P2/2Q1P3/1P1K1P2/R6R w - - 0 26", moves: "h7f6 g8g7 f6d5 d4c3", themes: &["equality", "middlegame", "pin", "short"] },
        LichessPuzzle { id: "ZlHTS", fen: "r1r3k1/5p1p/p2p2n1/5N2/q7/3B2Q1/1P4P1/1K5R b - - 4 31", moves: "a4f4 f5e7 g8g7 h1h7", themes: &["crushing", "middlegame", "pin", "short"] },
        LichessPuzzle { id: "h4R9C", fen: "8/8/5r2/2pk2r1/3n4/R2B4/P2R2P1/6K1 w - - 7 42", moves: "a3a8 d4f3 g1f1 f3d2", themes: &["crushing", "doubleCheck", "endgame", "pin", "short"] },
        LichessPuzzle { id: "1S6V9", fen: "3b4/r4p2/5p2/P1R1p3/2P1P3/2K2k2/2B5/8 w - - 1 43", moves: "c3b4 d8e7 b4b5 e7c5 b5c5 a7a5", themes: &["attraction", "crushing", "endgame", "long", "pin"] },
    ];
    assert_theme_detected_with_engine(&mut sf, &puzzles, TagKind::Pin, nodes);
}

// ===========================================================================
// Skewer
// ===========================================================================

#[test]
fn test_skewer() {
    let sf_path = match find_stockfish() {
        Some(p) => p,
        None => { eprintln!("SKIPPING test_skewer: Stockfish not found"); return; }
    };
    let mut sf = StockfishProcess::new(&sf_path);
    let nodes = 50_000u32;

    let puzzles = [
        LichessPuzzle { id: "02a3l", fen: "1R2b3/4P3/R7/3p1p1k/p2P1P2/r1K5/8/8 w - - 4 56", moves: "c3b4 a3b3 b4c5 b3b8", themes: &["advantage", "endgame", "short", "skewer"] },
        LichessPuzzle { id: "03p9e", fen: "5k2/5r2/B7/5P1p/P6b/1P3K2/5P2/4R3 w - - 1 36", moves: "f3e4 f7e7 e4f3 e7e1", themes: &["crushing", "endgame", "short", "skewer"] },
        LichessPuzzle { id: "0559u", fen: "8/8/3k1p2/2p3b1/4p3/4P1K1/8/4R3 w - - 0 46", moves: "g3f2 g5h4 f2f1 h4e1", themes: &["crushing", "endgame", "short", "skewer"] },
        LichessPuzzle { id: "05as7", fen: "1Q6/8/7p/5p1P/8/K5P1/1P3k2/q7 w - - 5 54", moves: "a3b4 a1b2 b4c5 b2b8", themes: &["crushing", "endgame", "queenEndgame", "short", "skewer"] },
        LichessPuzzle { id: "4Q39F", fen: "5Q2/8/p2p2p1/1p1k2P1/3B4/P1P5/1P1n3P/4r1K1 w - - 1 34", moves: "g1f2 e1f1 f2e2 f1f8", themes: &["advantage", "endgame", "short", "skewer"] },
        LichessPuzzle { id: "G0dRG", fen: "8/8/6k1/3p2p1/p1p3P1/P1P3B1/1P4r1/1R2K3 w - - 13 57", moves: "g3e5 g2g1 e1f2 g1b1", themes: &["crushing", "endgame", "short", "skewer"] },
        LichessPuzzle { id: "G9591", fen: "8/4r3/5k2/3P1b2/B2K3p/4R3/8/8 w - - 2 47", moves: "e3f3 e7e4 d4c5 e4a4", themes: &["crushing", "endgame", "short", "skewer"] },
        LichessPuzzle { id: "6i13S", fen: "2k5/6p1/N1p3r1/1p2B3/1P6/3P3r/P3K3/2R5 w - - 1 37", moves: "d3d4 g6g2 e2f1 g2a2", themes: &["advantage", "endgame", "short", "skewer"] },
        LichessPuzzle { id: "3C2v2", fen: "6k1/5p1p/6p1/1P1B4/2P2n2/5P1P/P6r/1R3K2 w - - 2 38", moves: "b5b6 h2h1 f1f2 h1b1", themes: &["crushing", "endgame", "short", "skewer"] },
        LichessPuzzle { id: "8Z3IT", fen: "8/p2R4/1p5p/5p1k/1P3K1P/P1P5/6b1/8 w - - 0 36", moves: "f4f5 g2h3 f5f4 h3d7", themes: &["crushing", "endgame", "short", "skewer"] },
    ];
    assert_theme_detected_with_engine(&mut sf, &puzzles, TagKind::Skewer, nodes);
}

// ===========================================================================
// Back Rank Mate
// ===========================================================================

#[test]
fn test_back_rank_mate() {
    let sf_path = match find_stockfish() {
        Some(p) => p,
        None => { eprintln!("SKIPPING test_back_rank_mate: Stockfish not found"); return; }
    };
    let mut sf = StockfishProcess::new(&sf_path);
    let nodes = 50_000u32;

    let puzzles = [
        LichessPuzzle { id: "A8d4e", fen: "k3r3/1p4Q1/7p/p7/2P5/5N2/5PPP/4q1K1 w - - 5 31", moves: "f3e1 e8e1", themes: &["backRankMate", "endgame", "mate", "mateIn1", "oneMove"] },
        LichessPuzzle { id: "j4j7Y", fen: "1r4k1/5pp1/5b1p/p3p3/P2p4/5P2/3Q2PP/1q3R1K w - - 0 33", moves: "f1b1 b8b1 d2c1 b1c1", themes: &["backRankMate", "endgame", "mate", "mateIn2", "short"] },
        LichessPuzzle { id: "5d03f", fen: "3q3k/6p1/7p/1Q2P3/2P5/8/3r1PPP/1R4K1 w - - 5 37", moves: "e5e6 d2d1 b1d1 d8d1", themes: &["backRankMate", "endgame", "mate", "mateIn2", "short"] },
        LichessPuzzle { id: "S08O3", fen: "2r3k1/3q2p1/5n1p/R3p3/1n2P3/R7/1P3PPP/3Q2K1 w - - 0 30", moves: "d1d7 c8c1 d7d1 c1d1", themes: &["backRankMate", "endgame", "mate", "mateIn2", "short"] },
        LichessPuzzle { id: "b877u", fen: "6k1/4P2p/3B2p1/8/4N3/6R1/1r2r1PP/4R2K w - - 1 42", moves: "e1e2 b2b1 e2e1 b1e1", themes: &["backRankMate", "endgame", "mate", "mateIn2", "short"] },
        LichessPuzzle { id: "4Z3W8", fen: "6R1/p3k2p/8/4N3/1P1b4/P7/2r1r1PP/4R2K w - - 5 30", moves: "e1e2 c2c1 e2e1 c1e1", themes: &["backRankMate", "endgame", "mate", "mateIn2", "short"] },
        LichessPuzzle { id: "0T05w", fen: "5r1k/4b1p1/2Q5/2P2r2/8/P7/6PP/R4q1K w - - 2 32", moves: "a1f1 f5f1", themes: &["backRankMate", "endgame", "mate", "mateIn1", "oneMove"] },
        LichessPuzzle { id: "0Z78t", fen: "2r5/p5k1/6b1/3p4/1R6/8/P4PPP/2q1R1K1 w - - 0 45", moves: "e1c1 c8c1", themes: &["backRankMate", "endgame", "mate", "mateIn1", "oneMove"] },
        LichessPuzzle { id: "C7H8o", fen: "7k/R6p/2p3n1/8/3P4/4P2r/PP6/K4R2 w - - 1 43", moves: "f1f6 h3h1 f6f1 h1f1", themes: &["backRankMate", "endgame", "mate", "mateIn2", "short"] },
        LichessPuzzle { id: "f635e", fen: "8/2r3k1/2B1R1p1/7p/P7/8/1P3PPP/6K1 w - - 0 44", moves: "c6d5 c7c1 e6e1 c1e1", themes: &["backRankMate", "endgame", "mate", "mateIn2", "short"] },
    ];
    assert_theme_detected_with_engine(&mut sf, &puzzles, TagKind::BackRankMate, nodes);
}

// ===========================================================================
// Smothered Mate
// ===========================================================================

#[test]
fn test_smothered_mate() {
    let sf_path = match find_stockfish() {
        Some(p) => p,
        None => { eprintln!("SKIPPING test_smothered_mate: Stockfish not found"); return; }
    };
    let mut sf = StockfishProcess::new(&sf_path);
    let nodes = 50_000u32;

    let puzzles = [
        LichessPuzzle { id: "E9t7J", fen: "2r3k1/p3Q1p1/4p2p/3p4/5P2/7n/P5PP/1R3RqK w - - 7 32", moves: "f1g1 h3f2", themes: &["endgame", "mate", "mateIn1", "oneMove", "smotheredMate"] },
        LichessPuzzle { id: "e7Aqy", fen: "1r2r2k/1p1R2p1/7p/p2Q4/2P5/P6n/1P4PN/2R3qK w - - 2 36", moves: "c1g1 h3f2", themes: &["mate", "mateIn1", "middlegame", "oneMove", "smotheredMate"] },
        LichessPuzzle { id: "3S75t", fen: "r4r1k/p1p3pp/7N/1q3p2/3B4/P4P2/P2R1K2/6R1 b - - 3 26", moves: "f8g8 h6f7", themes: &["mate", "mateIn1", "middlegame", "oneMove", "smotheredMate"] },
        LichessPuzzle { id: "43h3J", fen: "8/1p3k1p/6p1/p2N1p2/1P6/P6n/6PP/2R1R1qK w - - 9 39", moves: "e1g1 h3f2", themes: &["endgame", "mate", "mateIn1", "oneMove", "smotheredMate"] },
        LichessPuzzle { id: "S7W9A", fen: "k2r4/2p5/6Q1/3p1P2/8/1P5n/P5PP/R3R1qK w - - 5 44", moves: "e1g1 h3f2", themes: &["endgame", "mate", "mateIn1", "oneMove", "smotheredMate"] },
        LichessPuzzle { id: "T4R6D", fen: "2b3k1/7p/p6r/1p1N2p1/3Pnp2/1P3N2/6P1/6QK w - - 4 44", moves: "f3h2 e4g3", themes: &["endgame", "mate", "mateIn1", "oneMove", "smotheredMate"] },
        LichessPuzzle { id: "bm2z9", fen: "r6k/1p6/2b1p1Q1/p3P2N/8/7n/1P4PP/5RqK w - - 7 30", moves: "f1g1 h3f2", themes: &["endgame", "mate", "mateIn1", "oneMove", "smotheredMate"] },
        LichessPuzzle { id: "cGr5w", fen: "3r2k1/5p2/Q2q1p2/1p2p2p/1n5P/2P3P1/PP3P2/KN5R w - - 1 30", moves: "a6d6 b4c2", themes: &["endgame", "mate", "mateIn1", "oneMove", "smotheredMate"] },
        LichessPuzzle { id: "01H4V", fen: "r5k1/6p1/2N3pp/3p4/P2P4/1P5n/6PP/5RqK w - - 7 34", moves: "f1g1 h3f2", themes: &["endgame", "mate", "mateIn1", "oneMove", "smotheredMate"] },
        LichessPuzzle { id: "ZItNZ", fen: "r3r1k1/1Q1R2b1/p6p/8/2p4P/n1N3P1/PP6/Kq5R w - - 9 32", moves: "c3b1 a3c2", themes: &["mate", "mateIn1", "middlegame", "oneMove", "smotheredMate"] },
    ];
    assert_theme_detected_with_engine(&mut sf, &puzzles, TagKind::SmotheredMate, nodes);
}

// ===========================================================================
// Hook Mate
// ===========================================================================

#[test]
fn test_hook_mate() {
    let sf_path = match find_stockfish() {
        Some(p) => p,
        None => { eprintln!("SKIPPING test_hook_mate: Stockfish not found"); return; }
    };
    let mut sf = StockfishProcess::new(&sf_path);
    let nodes = 50_000u32;

    let puzzles = [
        LichessPuzzle { id: "1uxXv", fen: "6r1/1R5p/3k4/3p1B2/4p3/1N3n2/5P1P/5K2 w - - 3 46", moves: "b7h7 g8g1 f1e2 g1e1", themes: &["endgame", "hookMate", "mate", "mateIn2", "short"] },
        LichessPuzzle { id: "no77n", fen: "8/3k1p2/5p1p/1p3P2/2n1B1PP/2R2P2/1r2K3/8 w - - 2 50", moves: "e2d3 b2d2", themes: &["endgame", "hookMate", "mate", "mateIn1", "oneMove"] },
        LichessPuzzle { id: "SnYMa", fen: "2r2k2/R7/7p/3N2p1/2P4n/1P3p1P/P4r1K/R7 w - - 0 37", moves: "h2g3 f2g2", themes: &["endgame", "hookMate", "mate", "mateIn1", "oneMove"] },
        LichessPuzzle { id: "Arwom", fen: "8/1N4k1/8/7p/1R4n1/6P1/P2r2K1/8 w - - 4 40", moves: "g2h3 d2h2", themes: &["endgame", "hookMate", "mate", "mateIn1", "oneMove"] },
        LichessPuzzle { id: "MtxLk", fen: "8/8/4p1k1/R1B5/1P1P2p1/7n/1r5P/7K w - - 4 43", moves: "a5a3 b2b1 h1g2 b1g1", themes: &["endgame", "hookMate", "mate", "mateIn2", "short"] },
        LichessPuzzle { id: "96w31", fen: "5r1k/R7/2P3p1/8/P3P2p/2N3nP/6P1/6K1 w - - 1 37", moves: "c3e2 f8f1 g1h2 f1h1", themes: &["endgame", "hookMate", "mate", "mateIn2", "short"] },
        LichessPuzzle { id: "995h1", fen: "1k4r1/1p3R2/p4p2/7p/2P1p3/1P2Bn1P/P4P2/5K2 w - - 0 28", moves: "f7f6 g8g1 f1e2 g1e1", themes: &["endgame", "hookMate", "mate", "mateIn2", "short"] },
        LichessPuzzle { id: "41k0u", fen: "6k1/6p1/7p/1N3p2/4n3/1P2PK1P/P2r2P1/R7 w - - 2 35", moves: "b5d4 d2f2", themes: &["endgame", "hookMate", "mate", "mateIn1", "oneMove"] },
        LichessPuzzle { id: "4We8P", fen: "6k1/2R5/4P3/6N1/5r1p/6n1/6P1/6K1 w - - 2 46", moves: "e6e7 f4f1 g1h2 f1h1", themes: &["endgame", "hookMate", "mate", "mateIn2", "short"] },
        LichessPuzzle { id: "Y0O4L", fen: "8/1p3p1k/p1p4p/6n1/2P2NPK/1P1r3r/P3Q3/8 w - - 0 41", moves: "f4h3 d3h3", themes: &["endgame", "hookMate", "mate", "mateIn1", "oneMove"] },
    ];
    assert_theme_detected_with_engine(&mut sf, &puzzles, TagKind::HookMate, nodes);
}

// ===========================================================================
// Anastasia Mate
// ===========================================================================

#[test]
fn test_anastasia_mate() {
    let sf_path = match find_stockfish() {
        Some(p) => p,
        None => { eprintln!("SKIPPING test_anastasia_mate: Stockfish not found"); return; }
    };
    let mut sf = StockfishProcess::new(&sf_path);
    let nodes = 50_000u32;

    let puzzles = [
        LichessPuzzle { id: "Auehj", fen: "6k1/R6p/4N1p1/8/P6P/4n1P1/6K1/2r5 w - - 3 34", moves: "g2h3 c1h1", themes: &["anastasiaMate", "endgame", "mate", "mateIn1", "oneMove"] },
        LichessPuzzle { id: "E411C", fen: "6Q1/1b5Q/p2k1r2/8/5n2/P7/1P4P1/6K1 w - - 1 40", moves: "h7b7 f4e2 g1h2 f6h6", themes: &["anastasiaMate", "endgame", "mate", "mateIn2", "short"] },
        LichessPuzzle { id: "gdiwl", fen: "5k2/6R1/5p1p/1p5N/2r1n3/1K4P1/1P5P/8 w - - 1 47", moves: "g7h7 e4d2 b3a2 c4a4", themes: &["anastasiaMate", "endgame", "mate", "mateIn2", "short"] },
        LichessPuzzle { id: "2piEW", fen: "6k1/8/1p5p/3p4/2p2r2/2P3n1/1P4P1/R2R2K1 w - - 2 37", moves: "d1d5 g3e2 g1h2 f4h4", themes: &["anastasiaMate", "endgame", "mate", "mateIn2", "short"] },
        LichessPuzzle { id: "JzhNq", fen: "8/2k5/p7/2p2r2/P2n4/3N4/6P1/3R2K1 w - - 0 37", moves: "d3c5 d4e2 g1h2 f5h5", themes: &["anastasiaMate", "endgame", "mate", "mateIn2", "short"] },
        LichessPuzzle { id: "PuIbR", fen: "1B6/8/7R/8/1P1n4/K4k1p/8/7r w - - 3 57", moves: "a3a4 h1a1", themes: &["anastasiaMate", "endgame", "mate", "mateIn1", "oneMove"] },
        LichessPuzzle { id: "dnDhA", fen: "6k1/1R1R1p2/3P2p1/4P3/5r2/6n1/6P1/6K1 w - - 1 46", moves: "d7c7 g3e2 g1h2 f4h4", themes: &["anastasiaMate", "endgame", "mate", "mateIn2", "short"] },
        LichessPuzzle { id: "MYtRS", fen: "7k/1R6/8/1N1p3p/P2P1r1P/2P3n1/6P1/6K1 w - - 1 37", moves: "b7d7 g3e2 g1h2 f4h4", themes: &["anastasiaMate", "endgame", "mate", "mateIn2", "short"] },
        LichessPuzzle { id: "aE14S", fen: "6k1/1p3p2/p2p2p1/3P1r2/1P1Q1n2/2P5/1P4P1/6K1 w - - 5 42", moves: "d4b6 f4e2 g1h2 f5h5", themes: &["anastasiaMate", "endgame", "mate", "mateIn2", "short"] },
        LichessPuzzle { id: "52F0r", fen: "6k1/p2N1r2/1pp3p1/3p4/1P1P4/P1P3n1/6P1/R5K1 w - - 2 29", moves: "d7e5 g3e2 g1h2 f7h7", themes: &["anastasiaMate", "endgame", "mate", "mateIn2", "short"] },
    ];
    assert_theme_detected_with_engine(&mut sf, &puzzles, TagKind::AnastasiaMate, nodes);
}

// ===========================================================================
// Arabian Mate
// ===========================================================================

#[test]
fn test_arabian_mate() {
    let sf_path = match find_stockfish() {
        Some(p) => p,
        None => { eprintln!("SKIPPING test_arabian_mate: Stockfish not found"); return; }
    };
    let mut sf = StockfishProcess::new(&sf_path);
    let nodes = 50_000u32;

    let puzzles = [
        LichessPuzzle { id: "0hE5M", fen: "1k4r1/7R/2R5/4P1n1/8/7P/7K/8 w - - 2 47", moves: "h7e7 g5f3 h2h1 g8g1", themes: &["arabianMate", "endgame", "mate", "mateIn2", "short"] },
        LichessPuzzle { id: "0y04h", fen: "Q7/5RP1/8/3p1P2/3n1k2/1P4rK/8/8 w - - 1 52", moves: "h3h2 d4f3 h2h1 g3g1", themes: &["arabianMate", "endgame", "mate", "mateIn2", "short"] },
        LichessPuzzle { id: "llG6y", fen: "B7/8/8/8/4R3/3k4/n4r2/1K6 w - - 1 62", moves: "e4e8 a2c3 b1a1 f2a2", themes: &["arabianMate", "endgame", "mate", "mateIn2", "short"] },
        LichessPuzzle { id: "lyR0P", fen: "7k/R7/5N1P/8/8/5K2/p4P2/3r4 b - - 6 57", moves: "a2a1q a7h7", themes: &["arabianMate", "endgame", "mate", "mateIn1", "oneMove"] },
        LichessPuzzle { id: "mLtDB", fen: "7k/8/p7/5N1p/1R5p/P4n2/1P2r3/7K w - - 4 40", moves: "f5h4 e2h2", themes: &["arabianMate", "endgame", "mate", "mateIn1", "oneMove"] },
        LichessPuzzle { id: "RFhKZ", fen: "7k/2q4p/p6r/8/1P6/P4n2/2R4P/3R3K w - - 0 34", moves: "c2c7 h6h2", themes: &["arabianMate", "endgame", "mate", "mateIn1", "oneMove"] },
        LichessPuzzle { id: "b9nTY", fen: "8/p5kp/1p4p1/2p3P1/P3N3/1P1P1n2/4r2P/3R3K w - - 3 44", moves: "d1f1 e2h2", themes: &["arabianMate", "endgame", "mate", "mateIn1", "oneMove"] },
        LichessPuzzle { id: "bY9QZ", fen: "5r1k/4R1p1/R7/7p/4N1r1/1P3n2/5P1K/8 w - - 3 39", moves: "h2h1 g4g1", themes: &["arabianMate", "endgame", "mate", "mateIn1", "oneMove"] },
        LichessPuzzle { id: "czx9b", fen: "2R1r2k/1P1R2p1/5p1p/p7/6P1/1B3n1P/P3r3/6K1 w - - 9 39", moves: "g1h1 e2h2", themes: &["arabianMate", "endgame", "mate", "mateIn1", "oneMove"] },
        LichessPuzzle { id: "ZCCZr", fen: "2k5/8/8/7R/3n3P/6P1/2r3K1/5B2 w - - 1 52", moves: "g2g1 d4f3 g1h1 c2h2", themes: &["arabianMate", "endgame", "mate", "mateIn2", "short"] },
    ];
    assert_theme_detected_with_engine(&mut sf, &puzzles, TagKind::ArabianMate, nodes);
}

// ===========================================================================
// Discovered Attack
// ===========================================================================

#[test]
fn test_discovered_attack() {
    let sf_path = match find_stockfish() {
        Some(p) => p,
        None => { eprintln!("SKIPPING test_discovered_attack: Stockfish not found"); return; }
    };
    let mut sf = StockfishProcess::new(&sf_path);
    let nodes = 50_000u32;

    let puzzles = [
        LichessPuzzle { id: "ObT3G", fen: "2R5/3n4/8/R1p1r1k1/1p4p1/1P2K3/5P2/8 w - - 5 60", moves: "e3d3 c5c4 c8c4 e5a5", themes: &["advantage", "discoveredAttack", "endgame", "short"] },
        LichessPuzzle { id: "c0N5A", fen: "7k/1p1B2p1/p6p/P1q5/8/4r2P/6P1/R3Q2K w - - 0 36", moves: "e1f2 e3h3 d7h3 c5f2", themes: &["crushing", "discoveredAttack", "endgame", "short"] },
        LichessPuzzle { id: "5hszZ", fen: "2k5/5R2/1p3n2/2p5/P2K3P/1P1P4/2P2r2/8 w - - 0 38", moves: "d4e3 f6g4 e3e4 f2f7", themes: &["crushing", "discoveredAttack", "endgame", "short"] },
        LichessPuzzle { id: "S3n5I", fen: "5r1k/1R4p1/p1p2q2/7p/2Q4P/P3P3/6P1/7K w - - 6 40", moves: "c4f4 f6a1 h1h2 f8f4", themes: &["crushing", "discoveredAttack", "endgame", "short"] },
        LichessPuzzle { id: "X3heR", fen: "R4b1r/6p1/1k6/1P2p2p/4p2P/8/2P3P1/R5K1 w - - 0 31", moves: "a1f1 f8c5 g1h2 h8a8", themes: &["crushing", "discoveredAttack", "endgame", "short"] },
        LichessPuzzle { id: "00T4i", fen: "2r4k/q2b1Q1p/2p2P2/1p2p3/4P3/p2P3R/4N1rK/R7 w - - 0 38", moves: "h2g2 d7h3 g2h3 a7f7", themes: &["crushing", "discoveredAttack", "middlegame", "short"] },
        LichessPuzzle { id: "0Krnx", fen: "4R3/8/6p1/3k1p1b/4p2P/4K3/8/8 w - - 10 67", moves: "e3f4 g6g5 h4g5 h5e8", themes: &["crushing", "discoveredAttack", "endgame", "short"] },
        LichessPuzzle { id: "0Q97Y", fen: "7R/8/p5p1/1p1p1k2/P3b3/2P1K2P/1P6/8 w - - 0 61", moves: "h8a8 d5d4 e3d4 e4a8", themes: &["crushing", "discoveredAttack", "endgame", "short"] },
        LichessPuzzle { id: "0S773", fen: "8/5k2/3P3B/6P1/6K1/8/r7/8 b - - 0 54", moves: "a2d2 g5g6 f7g6 h6d2", themes: &["crushing", "discoveredAttack", "endgame", "short"] },
        LichessPuzzle { id: "11V6l", fen: "2b5/1r2k2p/3Rp1p1/p4r2/P7/1B3P2/2P3P1/1K2R3 w - - 5 31", moves: "d6a6 b7b3 c2b3 c8a6", themes: &["crushing", "discoveredAttack", "endgame", "short"] },
    ];
    assert_theme_detected_with_engine(&mut sf, &puzzles, TagKind::DiscoveredAttack, nodes);
}

// ===========================================================================
// Double Check
// ===========================================================================

#[test]
fn test_double_check() {
    let sf_path = match find_stockfish() {
        Some(p) => p,
        None => { eprintln!("SKIPPING test_double_check: Stockfish not found"); return; }
    };
    let mut sf = StockfishProcess::new(&sf_path);
    let nodes = 50_000u32;

    let puzzles = [
        LichessPuzzle { id: "y5w7I", fen: "5k2/b4bpp/R4p2/1p3P2/1P1r2P1/7P/P2Q4/6K1 w - - 0 37", moves: "d2g2 d4d1 g1h2 a7b8", themes: &["crushing", "discoveredCheck", "doubleCheck", "endgame", "short"] },
        LichessPuzzle { id: "4WxBE", fen: "8/1p3p2/p2p2p1/4b1k1/3p1r2/1P2N2P/P2R3K/4R3 w - - 6 32", moves: "e3g4 f4f2 h2g1 f2d2 g4e5 d6e5", themes: &["crushing", "discoveredCheck", "doubleCheck", "endgame", "long"] },
        LichessPuzzle { id: "mHTWe", fen: "8/8/B6p/2b1k3/5R2/4r1P1/P6P/6K1 w - - 1 36", moves: "a2a4 e3g3 g1f1 g3g1", themes: &["crushing", "discoveredCheck", "doubleCheck", "endgame", "short"] },
        LichessPuzzle { id: "mXpJH", fen: "3r1r2/p1k4p/p2b4/2p3p1/4b1P1/7K/P2Q3P/3R3R w - - 1 32", moves: "h1e1 f8f3 h3g2 f3g3 g2f1 d8f8", themes: &["crushing", "discoveredCheck", "doubleCheck", "long", "middlegame"] },
        LichessPuzzle { id: "iODWl", fen: "5k2/q4p1p/P1Q3p1/8/3r4/5P1P/6P1/2R3K1 w - - 0 34", moves: "c6b7 d4d1 g1h2 a7g1 h2g3 g1e1", themes: &["crushing", "discoveredCheck", "doubleCheck", "endgame", "long"] },
        LichessPuzzle { id: "q8CpC", fen: "7B/7p/1R6/2p3k1/5p2/8/PPrn1K1P/8 w - - 1 47", moves: "b6c6 d2e4 f2f3 g5f5 c6f6 e4f6", themes: &["crushing", "doubleCheck", "endgame", "long"] },
        LichessPuzzle { id: "HSlOY", fen: "7k/3R4/4N2p/7K/5P1P/4n1r1/8/8 w - - 0 46", moves: "h5h6 e3g4 h6g6 g4e5 g6f5 e5d7", themes: &["advantage", "discoveredCheck", "doubleCheck", "endgame", "long"] },
        LichessPuzzle { id: "5CybR", fen: "5r1r/1P2k3/2p3p1/3p4/P6p/2N3B1/7K/R7 w - - 0 42", moves: "b7b8q h4g3 h2g3 f8b8", themes: &["crushing", "discoveredCheck", "doubleCheck", "endgame", "short"] },
        LichessPuzzle { id: "OLdNh", fen: "8/1p2r1k1/p7/6NB/2P2rbP/1P6/P7/1K4R1 b - - 0 32", moves: "g4h5 g5e6 g7h6 e6f4", themes: &["advantage", "doubleCheck", "endgame", "short"] },
        LichessPuzzle { id: "p133e", fen: "1q6/3P1k2/5p2/4r1p1/6P1/3R3P/3Q3K/8 w - - 9 59", moves: "d7d8q e5e2 h2g1 b8h2 g1f1 h2f2", themes: &["discoveredCheck", "doubleCheck", "endgame", "long", "mate", "mateIn3"] },
    ];
    assert_theme_detected_with_engine(&mut sf, &puzzles, TagKind::DoubleCheck, nodes);
}

// ===========================================================================
// Deflection
// ===========================================================================

#[test]
fn test_deflection() {
    let sf_path = match find_stockfish() {
        Some(p) => p,
        None => { eprintln!("SKIPPING test_deflection: Stockfish not found"); return; }
    };
    let mut sf = StockfishProcess::new(&sf_path);
    let nodes = 50_000u32;

    let puzzles = [
        LichessPuzzle { id: "1r27K", fen: "6k1/p7/1p4q1/1P6/P2P2B1/3K2P1/4Q2P/2r5 w - - 5 45", moves: "e2e4 c1c3 d3c3 g6e4", themes: &["crushing", "deflection", "endgame", "short"] },
        LichessPuzzle { id: "23D3u", fen: "8/2b5/2k5/p1p3p1/P1N1K3/1P3P2/7r/3R4 w - - 6 44", moves: "d1d5 h2h4 e4d3 c6d5", themes: &["crushing", "deflection", "endgame", "short"] },
        LichessPuzzle { id: "42S5X", fen: "8/3B4/3B4/1P4k1/2P3p1/1r5p/5K2/8 w - - 0 49", moves: "d6g3 b3f3 f2e2 f3g3", themes: &["crushing", "deflection", "endgame", "short"] },
        LichessPuzzle { id: "8o6n6", fen: "6k1/6p1/1Q3r2/3p3p/P2P4/2q3P1/1R5K/8 w - - 1 34", moves: "b6b4 f6f2 b2f2 c3b4", themes: &["crushing", "deflection", "endgame", "short"] },
        LichessPuzzle { id: "6l5b1", fen: "8/6p1/1p2k2p/1P1R4/6K1/1r4N1/8/8 w - - 4 51", moves: "d5f5 b3g3 g4g3 e6f5", themes: &["crushing", "deflection", "endgame", "short"] },
        LichessPuzzle { id: "G5Q6U", fen: "8/8/4k3/1p3R1K/p2B4/P1P2P2/2P1r1b1/8 w - - 3 43", moves: "h5g4 g2h3 g4h3 e6f5", themes: &["advantage", "deflection", "endgame", "short"] },
        LichessPuzzle { id: "R3G1B", fen: "6R1/8/4p3/3b1p1p/5K1k/5P2/6P1/8 w - - 0 47", moves: "g8g3 e6e5 f4e5 h4g3", themes: &["crushing", "deflection", "endgame", "short"] },
        LichessPuzzle { id: "R59R0", fen: "4r3/7k/p2B2R1/2p2K1p/2P3p1/8/P5P1/8 w - - 12 50", moves: "d6e5 e8e5 f5e5 h7g6", themes: &["crushing", "deflection", "endgame", "short"] },
        LichessPuzzle { id: "B8V6F", fen: "6k1/r6p/6q1/1B3R2/P3Q1K1/6P1/1P1r3P/8 w - - 3 39", moves: "f5g5 h7h5 g4h3 g6g5", themes: &["crushing", "deflection", "endgame", "short"] },
        LichessPuzzle { id: "W7l14", fen: "8/5b2/5P1p/1p1p4/p1k2P1p/P1P2B1K/1P1R4/5r2 w - - 2 48", moves: "h3g4 h6h5 g4h4 f1f3", themes: &["advantage", "deflection", "endgame", "short"] },
    ];
    assert_theme_detected_with_engine(&mut sf, &puzzles, TagKind::Deflection, nodes);
}

// ===========================================================================
// Attraction
// ===========================================================================

#[test]
fn test_attraction() {
    let sf_path = match find_stockfish() {
        Some(p) => p,
        None => { eprintln!("SKIPPING test_attraction: Stockfish not found"); return; }
    };
    let mut sf = StockfishProcess::new(&sf_path);
    let nodes = 50_000u32;

    let puzzles = [
        LichessPuzzle { id: "sdZGK", fen: "r7/5p1k/2q1n1p1/p6p/8/2P4R/2Q4P/4R2K w - - 4 37", moves: "c2g2 c6g2 h1g2 e6f4", themes: &["attraction", "crushing", "endgame", "short"] },
        LichessPuzzle { id: "i0K6l", fen: "1Q6/1b3R2/1k2p1p1/p2p2q1/2p3P1/PpP3R1/1P5P/4r1K1 w - - 7 38", moves: "f7f1 e1f1 g1f1 g5c1", themes: &["attraction", "crushing", "endgame", "short"] },
        LichessPuzzle { id: "6d8ez", fen: "8/8/1rPkp1p1/1P1P1p1p/1R3P1P/r1K1B1P1/8/8 w - - 1 59", moves: "b4b3 a3b3 c3b3 b6b5", themes: &["attraction", "crushing", "endgame", "short"] },
        LichessPuzzle { id: "RD36P", fen: "3r2k1/1Q1r4/6q1/2p1P2p/p3p3/8/P1P3P1/4RRK1 w - - 1 33", moves: "b7b1 d7d2 f1f2 d2f2 g1f2 d8d2", themes: &["attraction", "crushing", "endgame", "long"] },
        LichessPuzzle { id: "4AS3a", fen: "3r3k/r5p1/7p/2P2Q2/1P1p4/P2R2P1/q4P1P/3R2K1 w - - 1 32", moves: "d3d4 d8d4 d1d4 a2a1 g1g2 a1d4", themes: &["advantage", "attraction", "endgame", "fork", "long"] },
        LichessPuzzle { id: "r5DRr", fen: "8/6p1/1p3k2/3n1b1p/R2P1P1P/P2K2P1/8/8 w - - 2 41", moves: "d3c4 b6b5 c4b5 d5c3", themes: &["attraction", "crushing", "endgame", "short"] },
        LichessPuzzle { id: "b99sO", fen: "7k/3r2pp/p7/4n3/P4R2/1Q4P1/B6P/3q2K1 w - - 7 32", moves: "g1g2 d7d2 f4f2 d2f2 g2f2 e5g4 f2g2 d1e2 g2h3 g4f2", themes: &["attraction", "crushing", "endgame", "veryLong"] },
        LichessPuzzle { id: "9pGvJ", fen: "8/1k2b3/4p3/1P2p1p1/N1K1P1Pp/5P2/5n1B/8 w - - 2 45", moves: "a4c5 e7c5 c4c5 f2d3 c5d6 d3e1", themes: &["attraction", "crushing", "endgame", "long"] },
        LichessPuzzle { id: "Ygdvf", fen: "3Q4/p4k1p/2p1b1p1/1p3p2/8/2P1B2P/1P1K3q/1B6 w - - 2 36", moves: "d2d3 e6c4 d3d4 c6c5 d4c5 h2e5", themes: &["advantage", "attraction", "endgame", "long"] },
        LichessPuzzle { id: "aGx2n", fen: "3R1b1k/7p/5r2/2N5/2q1B3/p3Q3/7P/7K w - - 2 38", moves: "d8d7 c4f1 e3g1 f1g1 h1g1 f8c5", themes: &["advantage", "attraction", "long", "middlegame"] },
    ];
    assert_theme_detected_with_engine(&mut sf, &puzzles, TagKind::Attraction, nodes);
}

// ===========================================================================
// Hanging Piece
// ===========================================================================

#[test]
fn test_hanging_piece() {
    let sf_path = match find_stockfish() {
        Some(p) => p,
        None => { eprintln!("SKIPPING test_hanging_piece: Stockfish not found"); return; }
    };
    let mut sf = StockfishProcess::new(&sf_path);
    let nodes = 50_000u32;

    let puzzles = [
        LichessPuzzle { id: "E0s2k", fen: "r6R/1k6/N3p1p1/1P3p2/P2n1P2/1n1P2P1/5K1P/8 w - - 3 39", moves: "a6c5 b3c5 h8a8 b7a8", themes: &["crushing", "endgame", "hangingPiece", "short"] },
        LichessPuzzle { id: "Z6X7i", fen: "8/4B3/1K2p1p1/P2n1p1p/k4P1P/8/6P1/8 w - - 16 53", moves: "b6a6 d5e7 a6b6 e7d5", themes: &["crushing", "endgame", "hangingPiece", "short"] },
        LichessPuzzle { id: "v6F9R", fen: "7k/6p1/7p/2b5/7P/2P1N1P1/4R1K1/2r5 w - - 7 46", moves: "e2c2 c5e3 c2c1 e3c1", themes: &["advantage", "endgame", "hangingPiece", "short"] },
        LichessPuzzle { id: "mE7iT", fen: "8/8/7P/2p4N/p2k2r1/3b3R/8/4K3 w - - 6 58", moves: "h3d3 d4d3 h6h7 g4e4", themes: &["crushing", "endgame", "hangingPiece", "short"] },
        LichessPuzzle { id: "T9I2h", fen: "8/8/1B6/P2k2p1/4n1P1/3p1K1P/1P6/8 w - - 0 49", moves: "b6d4 d5d4 a5a6 e4d2 f3g2 d2c4", themes: &["crushing", "endgame", "hangingPiece", "long"] },
        LichessPuzzle { id: "p8e6s", fen: "4q3/4P1k1/2R2r2/p2Q3P/P6K/8/8/8 w - - 1 52", moves: "d5d4 e8c6 e7e8q c6e8", themes: &["crushing", "endgame", "hangingPiece", "short"] },
        LichessPuzzle { id: "S03J2", fen: "8/6p1/3K3p/8/8/3k3P/4p1P1/6N1 w - - 0 68", moves: "g1e2 d3e2 h3h4 e2f2 h4h5 f2g2", themes: &["crushing", "endgame", "hangingPiece", "long"] },
        LichessPuzzle { id: "r2D6r", fen: "1r5k/2p3p1/4q2p/p2R4/5b2/P2Q3P/K1R2P2/8 w - - 1 38", moves: "c2c6 e6c6 d5d8 b8d8 d3d8 h8h7", themes: &["crushing", "endgame", "hangingPiece", "long"] },
        LichessPuzzle { id: "w2403", fen: "2k5/2p5/1p2P3/1P6/P3n1p1/2B3P1/6K1/8 w - - 0 50", moves: "g2f1 e4c3 e6e7 c8d7 e7e8r d7e8", themes: &["crushing", "endgame", "hangingPiece", "long"] },
        LichessPuzzle { id: "U3R0c", fen: "6k1/5p2/1Q6/2P1P3/1P2b3/7P/3r2r1/R4K2 w - - 2 39", moves: "a1a8 e4a8 b6b8 g8g7 b8a8 d2f2", themes: &["crushing", "endgame", "hangingPiece", "long"] },
    ];
    assert_theme_detected_with_engine(&mut sf, &puzzles, TagKind::HangingPiece, nodes);
}

// ===========================================================================
// En Passant
// ===========================================================================

#[test]
fn test_en_passant() {
    let sf_path = match find_stockfish() {
        Some(p) => p,
        None => { eprintln!("SKIPPING test_en_passant: Stockfish not found"); return; }
    };
    let mut sf = StockfishProcess::new(&sf_path);
    let nodes = 50_000u32;

    let puzzles = [
        LichessPuzzle { id: "BdtMZ", fen: "8/8/5p2/4r2R/4k1p1/6P1/5K1P/8 w - - 4 44", moves: "h5e5 f6e5 h2h4 g4h3", themes: &["crushing", "enPassant", "endgame", "short"] },
        LichessPuzzle { id: "x3C3X", fen: "8/7p/6p1/2k5/3p3P/3PbK1N/2P5/8 w - - 0 40", moves: "f3g2 c5b4 c2c4 d4c3", themes: &["crushing", "enPassant", "endgame", "short"] },
        LichessPuzzle { id: "l2o7a", fen: "1R6/8/3p4/2k5/1r5p/7K/6P1/8 w - - 2 53", moves: "b8b4 c5b4 g2g4 h4g3", themes: &["crushing", "enPassant", "endgame", "short"] },
        LichessPuzzle { id: "7DKpp", fen: "2k5/1r3R2/5p2/6p1/7p/7P/6P1/5K2 w - - 2 50", moves: "f7b7 c8b7 g2g4 h4g3", themes: &["crushing", "enPassant", "endgame", "short"] },
        LichessPuzzle { id: "5U7X7", fen: "7k/7p/2R3p1/8/p7/P5P1/1P1r3P/6K1 w - - 0 33", moves: "b2b4 a4b3 c6c8 h8g7", themes: &["crushing", "enPassant", "endgame", "rookEndgame", "short"] },
        LichessPuzzle { id: "8z1I4", fen: "8/6p1/8/3k3p/2p2p1P/2K2P2/1P4P1/8 w - - 0 41", moves: "b2b4 c4b3 c3b3 d5d4 b3c2 d4e3", themes: &["crushing", "enPassant", "endgame", "long", "pawnEndgame"] },
        LichessPuzzle { id: "m5Y5r", fen: "4k3/4P1p1/2R4p/p2N4/P2Pp3/7b/1r3P1P/6K1 w - - 0 38", moves: "f2f4 e4f3 c6c8 h3c8 d5c7 e8e7", themes: &["crushing", "enPassant", "endgame", "long"] },
        LichessPuzzle { id: "j0r8A", fen: "8/8/7p/3R2nk/1p4p1/2b1K1P1/P4P1P/8 w - - 5 40", moves: "h2h4 g4h3 f2f3 c3e1 d5d1 e1g3", themes: &["crushing", "enPassant", "endgame", "long"] },
        LichessPuzzle { id: "2fb1E", fen: "8/8/6r1/5N2/5p2/5P1P/4k1P1/6K1 w - - 13 58", moves: "f5h4 g6g5 g2g4 f4g3 g1g2 e2e3", themes: &["crushing", "enPassant", "endgame", "long"] },
        LichessPuzzle { id: "Ghwue", fen: "8/7p/8/2p2N2/1p2K1k1/1P6/P5P1/6b1 w - - 6 46", moves: "f5e3 g1e3 e4e3 g4g3 a2a4 b4a3", themes: &["crushing", "enPassant", "endgame", "long"] },
    ];
    assert_theme_detected_with_engine(&mut sf, &puzzles, TagKind::EnPassant, nodes);
}

// ===========================================================================
// Zugzwang — integration test using native Stockfish engine.
// Evaluates each solver-move position normally and after a null move,
// then feeds real evals into cook_zugzwang().
// ===========================================================================

/// Find the Stockfish binary. Checks:
///   1. backend-rust/stockfish.exe (local)
///   2. "stockfish" on PATH
fn find_stockfish() -> Option<String> {
    let local = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("stockfish.exe");
    if local.exists() {
        return Some(local.to_string_lossy().into_owned());
    }
    // Try without .exe (Linux/Mac)
    let local_unix = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("stockfish");
    if local_unix.exists() {
        return Some(local_unix.to_string_lossy().into_owned());
    }
    // Try PATH
    if Command::new("stockfish").arg("quit").stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok() {
        return Some("stockfish".to_string());
    }
    None
}

/// Simple Stockfish UCI wrapper for tests.
struct StockfishProcess {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    reader: BufReader<std::process::ChildStdout>,
}

impl StockfishProcess {
    fn new(path: &str) -> Self {
        let mut child = Command::new(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to start Stockfish");

        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let reader = BufReader::new(stdout);

        let mut sf = Self { child, stdin, reader };
        sf.send("uci");
        sf.wait_for("uciok");
        // Limit hash to 64MB to prevent OOM over 10K puzzles
        sf.send("setoption name Hash value 64");
        sf.send("setoption name Threads value 1");
        sf.send("isready");
        sf.wait_for("readyok");
        sf
    }

    fn send(&mut self, cmd: &str) {
        writeln!(self.stdin, "{}", cmd).unwrap();
        self.stdin.flush().unwrap();
    }

    fn wait_for(&mut self, target: &str) {
        let mut line = String::new();
        loop {
            line.clear();
            let bytes = self.reader.read_line(&mut line).unwrap();
            if bytes == 0 {
                panic!("Stockfish process died (EOF while waiting for '{}')", target);
            }
            if line.trim().starts_with(target) {
                break;
            }
        }
    }

    /// Evaluate a FEN position and return (cp, mate) from side-to-move perspective.
    fn evaluate(&mut self, fen: &str, nodes: u32) -> (i32, Option<i32>) {
        // Sync before each evaluation to keep Stockfish in a consistent state
        self.send("isready");
        self.wait_for("readyok");

        self.send(&format!("position fen {}", fen));
        self.send(&format!("go nodes {}", nodes));

        let mut cp = 0i32;
        let mut mate: Option<i32> = None;
        let mut line = String::new();

        loop {
            line.clear();
            let bytes = self.reader.read_line(&mut line).unwrap();
            if bytes == 0 {
                panic!("Stockfish process died (EOF during eval of FEN: {})", fen);
            }
            let trimmed = line.trim();

            if trimmed.starts_with("info") && trimmed.contains(" pv ") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if let Some(idx) = parts.iter().position(|&p| p == "score") {
                    if idx + 2 < parts.len() {
                        match parts[idx + 1] {
                            "cp" => {
                                cp = parts[idx + 2].parse().unwrap_or(0);
                                mate = None;
                            }
                            "mate" => {
                                mate = parts[idx + 2].parse().ok();
                                cp = 0;
                            }
                            _ => {}
                        }
                    }
                }
            }

            if trimmed.starts_with("bestmove") {
                break;
            }
        }

        (cp, mate)
    }
}

impl Drop for StockfishProcess {
    fn drop(&mut self) {
        let _ = writeln!(self.stdin, "quit");
        let _ = self.child.wait();
    }
}

/// Construct a null-move FEN: flip side-to-move and clear en passant.
fn null_move_fen(fen: &str) -> String {
    let parts: Vec<&str> = fen.split(' ').collect();
    if parts.len() < 4 {
        return fen.to_string();
    }
    let side = if parts[1] == "w" { "b" } else { "w" };
    format!(
        "{} {} {} - {} {}",
        parts[0],
        side,
        parts[2],
        parts.get(4).unwrap_or(&"0"),
        parts.get(5).unwrap_or(&"1"),
    )
}

#[test]
fn test_zugzwang() {
    let sf_path = match find_stockfish() {
        Some(p) => p,
        None => {
            eprintln!("SKIPPING test_zugzwang: Stockfish binary not found. Place stockfish.exe in backend-rust/ or add it to PATH.");
            return;
        }
    };

    let mut sf = StockfishProcess::new(&sf_path);
    let nodes = 100_000u32;

    let puzzles = [
        LichessPuzzle { id: "OuT8l", fen: "8/8/8/5p1k/5P2/6p1/4K1P1/8 w - - 0 71", moves: "e2e3 h5g4 e3d3 g4f4", themes: &["crushing", "endgame", "pawnEndgame", "short", "zugzwang"] },
        LichessPuzzle { id: "0R23v", fen: "8/8/5k2/5p1p/5K1P/5P2/8/8 w - - 2 67", moves: "f4g3 f6e5 g3f2 e5f4 f2e2 f4g3", themes: &["crushing", "endgame", "long", "pawnEndgame", "zugzwang"] },
        LichessPuzzle { id: "0Z9D4", fen: "8/8/2k2p2/p2p1p1p/P2K1P1P/4P1P1/8/8 w - - 3 35", moves: "d4c3 c6c5 c3d3 c5b4 d3d4 b4a4", themes: &["crushing", "endgame", "long", "pawnEndgame", "zugzwang"] },
        LichessPuzzle { id: "0k93k", fen: "8/8/8/p7/P1p5/2P1k3/2P5/3K4 w - - 8 57", moves: "d1c1 e3e2 c1b1 e2d1 b1b2 d1d2", themes: &["crushing", "endgame", "long", "pawnEndgame", "zugzwang"] },
        LichessPuzzle { id: "1G213", fen: "8/p7/8/1P6/8/3k1p1K/5P2/8 w - - 1 57", moves: "h3g3 d3e2 g3f4 e2f2 b5b6 a7b6", themes: &["crushing", "endgame", "long", "pawnEndgame", "zugzwang"] },
        LichessPuzzle { id: "Me7N2", fen: "8/2p5/3k2p1/1K1p4/3P3P/8/2P5/8 b - - 4 36", moves: "d6d7 b5c5 c7c6 c5b6", themes: &["crushing", "endgame", "pawnEndgame", "short", "zugzwang"] },
        LichessPuzzle { id: "No25v", fen: "8/8/1p6/8/k1K4p/8/2P3P1/8 w - - 2 51", moves: "c2c3 a4a3 c4d4 a3b3", themes: &["crushing", "endgame", "pawnEndgame", "short", "zugzwang"] },
        LichessPuzzle { id: "5e7N9", fen: "8/2p5/1p1p1k2/p2P2p1/P1P3P1/1P3K2/8/8 w - - 1 75", moves: "f3e3 f6e5 e3f3 e5d4", themes: &["crushing", "endgame", "pawnEndgame", "short", "zugzwang"] },
        LichessPuzzle { id: "B474U", fen: "8/8/2k5/p1p5/P1K5/2P5/8/8 w - - 2 67", moves: "c4b3 c6d5 b3c2 d5c4 c2d2 c4b3", themes: &["crushing", "endgame", "long", "pawnEndgame", "zugzwang"] },
        LichessPuzzle { id: "B7S9F", fen: "8/8/8/6p1/1p3k2/5P2/1P3K2/8 w - - 0 50", moves: "f2e2 f4g3 e2e3 b4b3 e3e2 g3g2", themes: &["crushing", "endgame", "long", "pawnEndgame", "zugzwang"] },
    ];

    let mut failures = Vec::new();

    for p in &puzzles {
        let puzzle = build_puzzle(p.id, p.fen, p.moves, 500);

        // For each solver move, evaluate the resulting position normally and with null-move
        let mut evals = Vec::new();
        for (i, node) in puzzle.mainline.iter().enumerate() {
            if i % 2 == 0 {
                continue; // skip opponent moves
            }

            let fen = node.board_after.to_string();
            let nfen = null_move_fen(&fen);

            let (cp, mate) = sf.evaluate(&fen, nodes);
            let (null_cp, null_mate) = sf.evaluate(&nfen, nodes);

            eprintln!(
                "  {} solver_move[{}]: normal=({:?},{:?}) null=({:?},{:?})",
                p.id, i / 2, cp, mate, null_cp, null_mate
            );

            evals.push(ZugzwangEval { cp, null_cp, mate, null_mate });
        }

        if !cook_zugzwang(&puzzle, &evals) {
            failures.push(format!("  MISS {}: zugzwang not detected", p.id));
        }
    }

    if !failures.is_empty() {
        panic!(
            "\nZugzwang detection failed for {}/{} puzzles:\n{}\n",
            failures.len(),
            puzzles.len(),
            failures.join("\n")
        );
    }
}

// ===========================================================================
// Quiet Move
// ===========================================================================

#[test]
fn test_quiet_move() {
    let sf_path = match find_stockfish() {
        Some(p) => p,
        None => { eprintln!("SKIPPING test_quiet_move: Stockfish not found"); return; }
    };
    let mut sf = StockfishProcess::new(&sf_path);
    let nodes = 50_000u32;

    let puzzles = [
        LichessPuzzle { id: "bOnBP", fen: "8/K7/2N5/1P1b3p/8/6k1/8/8 w - - 4 52", moves: "a7b7 h5h4 b7c7 h4h3", themes: &["crushing", "endgame", "quietMove", "short"] },
        LichessPuzzle { id: "6x75y", fen: "5n2/5p2/5K1P/8/2p5/1k6/p4P2/B7 w - - 5 53", moves: "f6f7 f8h7 f7g6 c4c3", themes: &["crushing", "endgame", "quietMove", "short"] },
        LichessPuzzle { id: "oR545", fen: "5r1k/1p4p1/3Q3p/p7/2P1q2P/1P4P1/P2R2K1/8 w - - 1 41", moves: "g2h2 f8f1 d2g2 e4b1", themes: &["crushing", "endgame", "quietMove", "short"] },
        LichessPuzzle { id: "oYHbj", fen: "3r2k1/p7/2p3p1/1q5p/8/B1P5/P1P4Q/1K5R w - - 1 35", moves: "a3b2 d8b8 h2b8 b5b8", themes: &["advantage", "endgame", "quietMove", "short"] },
        LichessPuzzle { id: "z1N0C", fen: "5r1k/1Q6/3p3p/p1p1b1q1/P1P2p2/1P3R2/7P/5R1K w - - 4 44", moves: "b7d7 f8g8 d7h3 e5d4 f3g3 f4g3", themes: &["crushing", "endgame", "long", "quietMove"] },
        LichessPuzzle { id: "09R70", fen: "8/8/P7/1P2p1p1/3b1k1p/1K6/4R3/8 w - - 2 54", moves: "b3a4 d4b6 e2h2 f4g3 h2c2 h4h3", themes: &["crushing", "endgame", "long", "quietMove"] },
        LichessPuzzle { id: "4c13X", fen: "8/8/4p3/1k1b1p2/1P3K2/4N3/8/8 w - - 7 56", moves: "e3d5 e6d5 f4e5 f5f4 e5f4 b5b4", themes: &["crushing", "endgame", "long", "quietMove"] },
        LichessPuzzle { id: "jOACJ", fen: "3r2k1/5q2/1Q2p2p/3r2p1/P2B2P1/2R2P2/1P5P/6K1 w - - 5 47", moves: "d4e3 d5d6 c3c6 d6d1 g1g2 d8f8 f3f4 g5f4", themes: &["crushing", "endgame", "quietMove", "veryLong"] },
        LichessPuzzle { id: "I0q8O", fen: "8/PK5k/8/6p1/8/8/r6p/2R5 w - - 0 45", moves: "a7a8q a2a8 b7a8 g5g4 c1h1 g4g3", themes: &["crushing", "endgame", "long", "quietMove"] },
        LichessPuzzle { id: "0n5X1", fen: "8/5p2/2k4p/P1q1p1p1/1Q4P1/K6P/8/8 w - - 1 50", moves: "b4c5 c6c5 a3a4 f7f6 a5a6 c5b6", themes: &["crushing", "endgame", "long", "quietMove", "zugzwang"] },
    ];
    assert_theme_detected_with_engine(&mut sf, &puzzles, TagKind::QuietMove, nodes);
}

// ===========================================================================
// Exposed King
// ===========================================================================

#[test]
fn test_exposed_king() {
    let sf_path = match find_stockfish() {
        Some(p) => p,
        None => { eprintln!("SKIPPING test_exposed_king: Stockfish not found"); return; }
    };
    let mut sf = StockfishProcess::new(&sf_path);
    let nodes = 50_000u32;

    let puzzles = [
        LichessPuzzle { id: "e204t", fen: "4r1k1/p1p3p1/6q1/N2P4/1P4b1/P1Q3B1/2P2p1P/R5K1 w - - 0 30", moves: "g1f2 g6b6 f2g2 e8e2 g2f1 b6h6", themes: &["crushing", "exposedKing", "long", "middlegame"] },
        LichessPuzzle { id: "Q0V0n", fen: "3Q4/8/2p5/p2b2r1/P2P1k2/2P5/8/4K3 w - - 12 55", moves: "d8a5 f4e3 a5a6 g5g1 a6f1 g1f1", themes: &["crushing", "endgame", "exposedKing", "long"] },
        LichessPuzzle { id: "Is95F", fen: "1r4k1/1b6/2R3P1/2p4r/P7/1P2N3/2P5/4R1K1 w - - 1 35", moves: "c6b6 h5h1 g1f2 b8f8 f2e2 b7f3", themes: &["advantage", "endgame", "exposedKing", "long"] },
        LichessPuzzle { id: "asULw", fen: "3r2k1/1p3p1p/1q2p1p1/8/6P1/1B6/P2R2Q1/6K1 w - - 1 39", moves: "g1h2 b6c7 h2h1 c7c1 d2d1 d8d1", themes: &["crushing", "endgame", "exposedKing", "long"] },
        LichessPuzzle { id: "7M4J4", fen: "5r2/6k1/2p2n2/p3q3/6P1/P2Q4/3K1P2/3R2R1 w - - 0 32", moves: "d1b1 f6e4 d2c1 e5c5 d3c2 c5g5", themes: &["crushing", "endgame", "exposedKing", "long"] },
        LichessPuzzle { id: "39B5Q", fen: "7k/7P/p1b3P1/1p3B2/1P2P3/P1q1K1R1/3p3R/8 w - - 0 46", moves: "e3f2 c3d4 f2g2 c6e4 f5e4 d4e4", themes: &["advantage", "endgame", "exposedKing", "long"] },
        LichessPuzzle { id: "Am1c5", fen: "8/8/8/5k1p/4p2P/2B1K1P1/7n/8 w - - 3 58", moves: "c3d2 h2f1 e3e2 f1g3 e2f2 f5g4", themes: &["crushing", "endgame", "exposedKing", "long"] },
        LichessPuzzle { id: "C08kq", fen: "2r5/8/3R4/R2p4/3k1B2/6P1/1K4r1/8 w - - 13 56", moves: "b2b3 c8c3 b3b4 g2b2 b4a4 b2a2", themes: &["advantage", "endgame", "exposedKing", "long"] },
        LichessPuzzle { id: "83E4D", fen: "r1r2k2/6p1/5b2/2P2P2/3N4/1Q6/1p6/1KR4q w - - 2 37", moves: "c1h1 a8a1 b1b2 f6d4 b2c2 a1h1", themes: &["crushing", "endgame", "exposedKing", "long"] },
        LichessPuzzle { id: "5P91Y", fen: "7R/6k1/5p2/5b2/1P3P2/pr6/R7/4K3 w - - 6 58", moves: "h8b8 b3b1 e1d2 b1b2 d2c3 b2a2", themes: &["crushing", "endgame", "exposedKing", "long"] },
    ];
    assert_theme_detected_with_engine(&mut sf, &puzzles, TagKind::ExposedKing, nodes);
}

// ===========================================================================
// Sacrifice
// ===========================================================================

#[test]
fn test_sacrifice() {
    let sf_path = match find_stockfish() {
        Some(p) => p,
        None => { eprintln!("SKIPPING test_sacrifice: Stockfish not found"); return; }
    };
    let mut sf = StockfishProcess::new(&sf_path);
    let nodes = 50_000u32;

    let puzzles = [
        LichessPuzzle { id: "5Q66m", fen: "7r/1Q3pk1/p5p1/2p1q3/1P6/5B1P/P2r2P1/R4R1K w - - 1 29", moves: "a1e1 h8h3 g2h3 e5h2", themes: &["mate", "mateIn2", "middlegame", "sacrifice", "short"] },
        LichessPuzzle { id: "yUByY", fen: "7k/p2b3p/1p1q2p1/3B4/3n1r2/P1Q5/1P3P1P/2R1R1K1 w - - 2 31", moves: "e1e4 d6d5 e4f4 d5g5", themes: &["advantage", "middlegame", "sacrifice", "short"] },
        LichessPuzzle { id: "dFjTJ", fen: "6k1/4Q1b1/R5p1/1p1p2N1/3P4/2P5/1P1BK1P1/3q1r2 w - - 7 36", moves: "e2e3 f1e1 d2e1 d1e1", themes: &["advantage", "middlegame", "sacrifice", "short"] },
        LichessPuzzle { id: "N3RzH", fen: "r4r1k/2p3p1/p1P2b1p/1p1Q4/1P2p1q1/P1B3P1/R1P2P1P/4R1K1 w - - 6 24", moves: "e1e4 g4f3 c3f6 a8d8 f6g7 h8h7 g7f8 d8d5", themes: &["advantage", "middlegame", "sacrifice", "veryLong"] },
        LichessPuzzle { id: "s7FBV", fen: "5n1k/1Q3p2/2n2q2/5r2/7P/1P1R4/P7/1K5R w - - 0 29", moves: "b7c8 f5f2 c8f8 h8h7 f8a3 c6b4", themes: &["crushing", "long", "middlegame", "sacrifice"] },
        LichessPuzzle { id: "L1d4b", fen: "4r1k1/2p2p2/5q1p/2Q1N3/1P3P2/P3P2P/3r4/R1R3K1 w - - 1 28", moves: "c1d1 e8e5 c5e5 f6g6 e5g5 d2d1 a1d1 h6g5", themes: &["crushing", "exposedKing", "middlegame", "sacrifice", "veryLong"] },
        LichessPuzzle { id: "86E4J", fen: "1r4k1/1p3p1p/p1q3p1/5r2/P7/2B1Q2P/1P2R1K1/5R2 w - - 4 29", moves: "e3e4 b8e8 e4c6 e8e2", themes: &["advantage", "middlegame", "sacrifice", "short", "skewer"] },
        LichessPuzzle { id: "mmPHc", fen: "7r/1r3k2/2n4q/p4B2/R2P1N2/2P3P1/5P2/2Q3K1 w - - 1 35", moves: "f5e4 h6h2 g1f1 h2h1 e4h1 h8h1", themes: &["advantage", "long", "middlegame", "sacrifice"] },
        LichessPuzzle { id: "d6b4o", fen: "4r1k1/1b1q1p1p/1p4p1/3p1n2/1P1B1P2/P2P3B/5P1P/1R2Q1K1 w - - 1 24", moves: "e1d2 f5d4 h3d7 d4f3 g1g2 f3d2", themes: &["advantage", "fork", "long", "middlegame", "sacrifice"] },
        LichessPuzzle { id: "DZLd8", fen: "4r1k1/p2q2p1/1p3r1p/2p1b3/3p4/N2P2P1/PPPQ1P1P/R3R1K1 w - - 2 19", moves: "f2f4 e5f4 e1e8 d7e8 g3f4 f6g6", themes: &["advantage", "long", "middlegame", "sacrifice"] },
    ];
    let sac_piece_tags = [
        TagKind::QueenSacrifice, TagKind::RookSacrifice,
        TagKind::BishopSacrifice, TagKind::KnightSacrifice,
    ];
    let mut failures = Vec::new();
    for (idx, p) in puzzles.iter().enumerate() {
        let tags = cook_with_engine(&mut sf, p, nodes);
        let piece_tags: Vec<_> = tags.iter().filter(|t| sac_piece_tags.contains(t)).collect();
        if tags.contains(&TagKind::Sacrifice) && !piece_tags.is_empty() {
            eprintln!("  [{}/{}] {} ... ok  {:?}", idx + 1, puzzles.len(), p.id, piece_tags);
        } else {
            eprintln!("  [{}/{}] {} ... MISS (piece tags: {:?}, all: {:?})", idx + 1, puzzles.len(), p.id, piece_tags, tags);
            failures.push(p.id);
        }
    }
    assert!(failures.is_empty(), "Sacrifice piece sub-tag missing for: {:?}", failures);
}

// ===========================================================================
// X-Ray Attack
// ===========================================================================

#[test]
fn test_x_ray_attack() {
    let sf_path = match find_stockfish() {
        Some(p) => p,
        None => { eprintln!("SKIPPING test_x_ray_attack: Stockfish not found"); return; }
    };
    let mut sf = StockfishProcess::new(&sf_path);
    let nodes = 50_000u32;

    let puzzles = [
        LichessPuzzle { id: "3oR1E", fen: "3r3k/3P2p1/1p4Qp/p1p5/q7/8/5P1P/1R1R3K w - - 1 33", moves: "g6e8 d8e8 d7e8q a4e8", themes: &["advantage", "endgame", "short", "xRayAttack"] },
        LichessPuzzle { id: "kn7vS", fen: "k4r2/1p3P1p/p1p3q1/8/1P6/P3Q3/7P/5R1K w - - 3 44", moves: "e3e8 f8e8 f7e8r g6e8", themes: &["crushing", "endgame", "short", "xRayAttack"] },
        LichessPuzzle { id: "8iRwK", fen: "2r3k1/5p1p/b1Q3p1/8/4B3/q3P1P1/5P1P/2R3K1 w - - 1 30", moves: "c1c3 a3a1 c3c1 a1c1 c6c1 c8c1", themes: &["crushing", "endgame", "long", "xRayAttack"] },
        LichessPuzzle { id: "C158K", fen: "k2r4/1p3p1p/p4Q2/8/1P2P1P1/q4K2/7P/2R5 w - - 0 37", moves: "f6c3 d8d3 c3d3 a3d3", themes: &["crushing", "endgame", "fork", "short", "xRayAttack"] },
        LichessPuzzle { id: "5Yxk4", fen: "4Q3/p2K4/4R3/3P4/q7/4r3/4k3/8 w - - 1 57", moves: "d7d6 a4e8 e6e8 e3e8", themes: &["crushing", "endgame", "short", "xRayAttack"] },
        LichessPuzzle { id: "IXZ19", fen: "4r2k/B5p1/2p4p/1b6/5q2/P2Q1N1P/2P3P1/5R1K w - - 1 37", moves: "f3d4 f4f1 d3f1 b5f1", themes: &["advantage", "middlegame", "short", "xRayAttack"] },
        LichessPuzzle { id: "U6K7f", fen: "8/Q3R3/3p1p1k/5b2/5q2/1P1P1B1P/P1r2PP1/6K1 w - - 1 33", moves: "a7e3 c2c1 e3c1 f4c1", themes: &["advantage", "endgame", "short", "xRayAttack"] },
        LichessPuzzle { id: "AjF56", fen: "4r2k/R3P1p1/2b4p/5Q2/1q1p4/1P1P2P1/7P/4R1K1 w - - 1 29", moves: "f5f8 e8f8 e7f8q b4f8", themes: &["crushing", "endgame", "short", "xRayAttack"] },
        LichessPuzzle { id: "RZMD8", fen: "2r5/2k2p2/8/2q5/3R3Q/7P/2r3P1/2R4K w - - 1 43", moves: "d4c4 c2c1 c4c1 c5c1", themes: &["crushing", "endgame", "short", "xRayAttack"] },
        LichessPuzzle { id: "4th1W", fen: "5r1k/6p1/7p/1Q2P3/p1pR1r1q/P1P2R1P/1P4P1/7K w - - 0 46", moves: "b5c4 h4e1 c4f1 e1f1 f3f1 f4f1", themes: &["advantage", "endgame", "long", "xRayAttack"] },
    ];
    assert_theme_detected_with_engine(&mut sf, &puzzles, TagKind::XRayAttack, nodes);
}

// ===========================================================================
// Kingside Attack
// ===========================================================================

#[test]
fn test_kingside_attack() {
    let sf_path = match find_stockfish() {
        Some(p) => p,
        None => { eprintln!("SKIPPING test_kingside_attack: Stockfish not found"); return; }
    };
    let mut sf = StockfishProcess::new(&sf_path);
    let nodes = 50_000u32;

    let puzzles = [
        LichessPuzzle { id: "c5y0T", fen: "r5k1/1p3p1p/p2p2p1/2pPr2n/P3P2q/2N2Q2/1P1N1P2/R3R1K1 w - - 0 22", moves: "d2f1 e5g5 f1g3 h5g3 f2g3 g5g3", themes: &["crushing", "kingsideAttack", "long", "middlegame"] },
        LichessPuzzle { id: "T4Of9", fen: "2Q5/1p3p2/p2b1k1B/3p3p/4n2q/5n1P/PPP2P2/R3R2K w - - 1 26", moves: "e1e2 e4f2 e2f2 h4f2", themes: &["advantage", "kingsideAttack", "middlegame", "short"] },
        LichessPuzzle { id: "051q4", fen: "r5r1/p1p2p1k/2p2p1p/3b1N2/6R1/3n3P/PPP2PP1/R1B3K1 w - - 4 20", moves: "g4h4 g8g2 g1f1 g2f2", themes: &["crushing", "kingsideAttack", "middlegame", "short"] },
        LichessPuzzle { id: "0z1am", fen: "4r1k1/1R2r1p1/2pqp1Q1/p2p2P1/P2P2n1/2P5/3N1P1P/1R4K1 w - - 1 25", moves: "b7e7 d6h2 g1f1 h2f2", themes: &["kingsideAttack", "mate", "mateIn2", "middlegame", "short"] },
        LichessPuzzle { id: "1Fdg8", fen: "r1b1r1k1/1p3p2/2p3p1/p4p2/2P2q2/B1PQ1P2/P1B3R1/1R4K1 w - - 2 26", moves: "a3c1 e8e1 g1f2 f4h4", themes: &["crushing", "kingsideAttack", "middlegame", "short"] },
        LichessPuzzle { id: "31j5b", fen: "3r1rk1/1Q4p1/p6p/6n1/2p3R1/P2p3P/1P3qPB/R4N1K w - - 1 35", moves: "h3h4 f2f1 a1f1 f8f1", themes: &["crushing", "kingsideAttack", "middlegame", "short"] },
        LichessPuzzle { id: "mYfLB", fen: "4r2k/p2Q2p1/1p3q1p/5p2/2P2P2/1P4P1/P3rP1P/R2R2K1 w - - 1 26", moves: "d7d6 e2e1 d1e1 e8e1 a1e1 f6d6", themes: &["advantage", "endgame", "kingsideAttack", "long"] },
        LichessPuzzle { id: "F61u5", fen: "4r1k1/p1pb2p1/1p3q1p/3P1p2/2P5/1P2B1P1/P4Q1P/4R1K1 w - - 0 27", moves: "e3d4 e8e1 f2e1 f6d4", themes: &["advantage", "endgame", "kingsideAttack", "short"] },
        LichessPuzzle { id: "81m1c", fen: "4r1k1/p4p1p/6p1/2p2n2/1q1p1N2/1P1Q4/P1P2PPP/3R2K1 w - - 0 22", moves: "f4d5 e8e1 d1e1 b4e1 d3f1 e1e5", themes: &["advantage", "endgame", "kingsideAttack", "long"] },
        LichessPuzzle { id: "k3qvx", fen: "8/p4q1p/1p1b1r1k/3p2p1/1P2p1P1/2Q1P2P/P3B2P/3R2K1 w - - 0 31", moves: "c3d4 f6f2 d4d5 d6h2 g1h1 f7d5 d1d5 f2e2", themes: &["crushing", "endgame", "kingsideAttack", "veryLong"] },
    ];
    assert_theme_detected_with_engine(&mut sf, &puzzles, TagKind::KingsideAttack, nodes);
}

// ===========================================================================
// Under-Promotion
// ===========================================================================

#[test]
fn test_under_promotion() {
    let sf_path = match find_stockfish() {
        Some(p) => p,
        None => { eprintln!("SKIPPING test_under_promotion: Stockfish not found"); return; }
    };
    let mut sf = StockfishProcess::new(&sf_path);
    let nodes = 50_000u32;

    let puzzles = [
        LichessPuzzle { id: "SSSOC", fen: "8/1P3R2/8/6k1/7p/5K2/1r2p1P1/8 w - - 0 52", moves: "b7b8q e2e1n f3e3 b2b8", themes: &["advancedPawn", "crushing", "endgame", "promotion", "short", "underPromotion"] },
        LichessPuzzle { id: "iBLId", fen: "8/5P1p/8/3p4/8/1k6/p1p1P2P/K7 w - - 0 45", moves: "f7f8q c2c1r", themes: &["advancedPawn", "endgame", "mate", "mateIn1", "oneMove", "promotion", "underPromotion"] },
        LichessPuzzle { id: "2Wg7H", fen: "8/1P2R3/7p/8/4p1P1/2K2k1P/1r1p4/8 w - - 0 45", moves: "b7b8q d2d1n c3d4 b2b8", themes: &["advancedPawn", "advantage", "endgame", "promotion", "short", "underPromotion"] },
        LichessPuzzle { id: "9JpcI", fen: "8/8/7p/N7/2p4P/3k4/1p6/3K4 w - - 0 49", moves: "a5c4 b2b1r", themes: &["advancedPawn", "endgame", "mate", "mateIn1", "oneMove", "promotion", "underPromotion"] },
        LichessPuzzle { id: "VbDKV", fen: "R7/P4p2/4P3/8/5k2/7K/r5p1/8 w - - 0 51", moves: "e6f7 g2g1n h3h4 a2h2", themes: &["advancedPawn", "endgame", "mate", "mateIn2", "promotion", "short", "underPromotion"] },
        LichessPuzzle { id: "VdPcJ", fen: "8/8/8/8/2K5/8/2p1Q3/1k2b3 w - - 17 69", moves: "c4b3 c2c1n b3c4 c1e2", themes: &["advancedPawn", "crushing", "endgame", "fork", "promotion", "short", "underPromotion"] },
        LichessPuzzle { id: "FyhyC", fen: "5R2/2p4p/p1p5/2P4k/1P4q1/4Q3/P2p4/6K1 w - - 0 47", moves: "g1f2 d2d1n f2e1 d1e3", themes: &["advancedPawn", "crushing", "endgame", "fork", "promotion", "short", "underPromotion"] },
        LichessPuzzle { id: "35o9x", fen: "8/p5pk/5p1p/5P1Q/7P/1P6/P2pBPPq/5K2 w - - 2 38", moves: "f2f3 h2h1 f1f2 h1e1 f2e3 d2d1n", themes: &["advancedPawn", "crushing", "endgame", "long", "promotion", "underPromotion"] },
        LichessPuzzle { id: "d66uR", fen: "8/8/p7/1p4p1/1P6/P2K3P/3Npkp1/6R1 w - - 2 50", moves: "g1c1 e2e1n c1e1 f2e1 d2f3 e1f2 d3e4 f2e2", themes: &["advancedPawn", "crushing", "endgame", "promotion", "underPromotion", "veryLong"] },
        LichessPuzzle { id: "Dip8H", fen: "1R6/8/8/8/8/3K4/p1p2P2/2k5 w - - 6 57", moves: "b8a8 c1d1 a8g8 c2c1n d3c4 a2a1q", themes: &["advancedPawn", "crushing", "endgame", "exposedKing", "long", "promotion", "rookEndgame", "underPromotion"] },
    ];
    assert_theme_detected_with_engine(&mut sf, &puzzles, TagKind::UnderPromotion, nodes);
}

// ===========================================================================
// Bulk validation against Lichess puzzle database (10,000 puzzles)
// ===========================================================================

/// Themes that cook() can produce. Excludes:
/// - CP-dependent tags (advantage/crushing/equality) — Lichess uses 40M nodes,
///   far deeper than practical for testing; our thresholds match theirs exactly.
/// - Zugzwang (needs engine null-move analysis).
/// - Phase tags (opening/middlegame/endgame) that Lichess tags but we don't.
const COMPARABLE_THEMES: &[&str] = &[
    "advancedPawn", "anastasiaMate", "arabianMate", "attraction",
    "backRankMate", "bishopEndgame", "bodenMate", "castling", "clearance",
    "defensiveMove", "deflection", "discoveredAttack",
    "doubleBishopMate", "doubleCheck", "dovetailMate", "enPassant",
    "exposedKing", "fork", "hangingPiece", "hookMate", "interference",
    "intermezzo", "kingsideAttack", "knightEndgame", "long", "mate",
    "mateIn1", "mateIn2", "mateIn3", "mateIn4", "mateIn5", "oneMove",
    "pawnEndgame", "pin", "promotion", "queenEndgame", "queenRookEndgame",
    "queensideAttack", "quietMove", "rookEndgame", "sacrifice", "short",
    "skewer", "smotheredMate", "trappedPiece", "underPromotion", "veryLong",
    "xRayAttack",
];

/// Run cook() on first N Lichess puzzles and report per-theme precision/recall.
/// Uses tiered Stockfish evaluation: 10K → 50K → 100K nodes, but only escalates
/// when CP-dependent themes (advantage/crushing/equality) mismatch.
///
/// Run with: cargo test --test classify_test bulk_validate -- --ignored --nocapture
#[test]
#[ignore]
fn bulk_validate_lichess_10k() {
    let sf_path = match find_stockfish() {
        Some(p) => p,
        None => {
            eprintln!("SKIPPING bulk_validate: Stockfish not found");
            return;
        }
    };
    let mut sf = StockfishProcess::new(&sf_path);
    let node_tiers: &[u32] = &[100_000];

    // Set to 10 for quick smoke test, 10_000 for full validation
    let max_puzzles: u32 = 10_000;

    let csv_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("lichess_puzzles_10k.csv");

    assert!(csv_path.exists(), "Missing test data: {}", csv_path.display());

    use std::collections::HashMap;
    let mut tp: HashMap<String, u32> = HashMap::new();
    let mut fp: HashMap<String, u32> = HashMap::new();
    let mut fn_: HashMap<String, u32> = HashMap::new();
    let mut parse_errors = 0u32;
    let mut total = 0u32;

    let file = std::fs::File::open(&csv_path).expect("Failed to open CSV");
    let mut rdr = std::io::BufReader::new(file);
    let mut header = String::new();
    rdr.read_line(&mut header).unwrap(); // skip header

    let mut line = String::new();
    while rdr.read_line(&mut line).unwrap() > 0 {
        if total >= max_puzzles {
            break;
        }
        total += 1;
        let fields: Vec<&str> = line.trim().splitn(10, ',').collect();
        if fields.len() < 8 {
            parse_errors += 1;
            line.clear();
            continue;
        }

        let id = fields[0];
        let fen = fields[1];
        let moves = fields[2];
        let lichess_themes: Vec<&str> = fields[7].split_whitespace().collect();
        let lichess_url = if fields.len() > 8 { fields[8] } else { "" };

        // Build puzzle structure (no engine needed yet)
        let puzzle_result = std::panic::catch_unwind(|| build_puzzle(id, fen, moves, 500));
        let base_puzzle = match puzzle_result {
            Ok(p) => p,
            Err(_) => {
                eprintln!("  [{total}] {id} ... PARSE ERROR");
                parse_errors += 1;
                line.clear();
                continue;
            }
        };

        // Evaluate position and classify
        let nodes = node_tiers[0];
        let real_cp = evaluate_puzzle_cp(&mut sf, &base_puzzle, nodes);
        let puzzle = build_puzzle(id, fen, moves, real_cp);
        let our_tags = cook(&puzzle);
        let our_tag_strs: Vec<String> = our_tags.iter().map(|t| {
            serde_json::to_string(t).unwrap().trim_matches('"').to_string()
        }).collect();

        // Print failures with all info needed to validate in an engine
        let mismatches: Vec<String> = COMPARABLE_THEMES.iter().filter_map(|theme| {
            let lichess_has = lichess_themes.contains(theme);
            let we_have = our_tag_strs.iter().any(|t| t == *theme);
            match (lichess_has, we_have) {
                (true, false) => Some(format!("-{theme}")),
                (false, true) => Some(format!("+{theme}")),
                _ => None,
            }
        }).collect();

        if !mismatches.is_empty() {
            eprintln!("  FAIL [{total}] {id}: {}", mismatches.join(", "));
            eprintln!("    FEN:      {fen}");
            eprintln!("    Our CP:   {real_cp} ({nodes} nodes)");
            eprintln!("    Lichess:  {:?}", lichess_themes);
            eprintln!("    Ours:     {:?}", our_tag_strs);
            eprintln!("    URL:      {lichess_url}");
        }

        // Record final results
        for theme in COMPARABLE_THEMES {
            let lichess_has = lichess_themes.contains(theme);
            let we_have = our_tag_strs.iter().any(|t| t == *theme);

            match (lichess_has, we_have) {
                (true, true) => *tp.entry(theme.to_string()).or_default() += 1,
                (false, true) => *fp.entry(theme.to_string()).or_default() += 1,
                (true, false) => *fn_.entry(theme.to_string()).or_default() += 1,
                (false, false) => {}
            }
        }

        if total % 500 == 0 {
            eprintln!("  [{total}/{max_puzzles}] processed...");
        }
        // Reset Stockfish state every 1000 puzzles to prevent memory buildup
        if total % 1000 == 0 {
            sf.send("ucinewgame");
        }

        line.clear();
    }

    // Print report
    eprintln!("\n========== BULK VALIDATION REPORT ==========");
    eprintln!("Total puzzles: {} (parse errors: {})\n", total, parse_errors);

    let mut all_themes: Vec<&str> = COMPARABLE_THEMES.iter()
        .filter(|t| {
            tp.get(**t).unwrap_or(&0) + fp.get(**t).unwrap_or(&0) + fn_.get(**t).unwrap_or(&0) > 0
        })
        .copied()
        .collect();
    all_themes.sort();

    eprintln!("{:<22} {:>5} {:>5} {:>5} {:>8} {:>8} {:>5}",
        "Theme", "TP", "FP", "FN", "Prec%", "Recall%", "F1%");
    eprintln!("{}", "-".repeat(72));

    let mut total_tp = 0u32;
    let mut total_fp = 0u32;
    let mut total_fn = 0u32;

    for theme in &all_themes {
        let t = tp.get(*theme).unwrap_or(&0);
        let f = fp.get(*theme).unwrap_or(&0);
        let n = fn_.get(*theme).unwrap_or(&0);

        total_tp += t;
        total_fp += f;
        total_fn += n;

        let precision = if t + f > 0 { *t as f64 / (t + f) as f64 * 100.0 } else { 0.0 };
        let recall = if t + n > 0 { *t as f64 / (t + n) as f64 * 100.0 } else { 0.0 };
        let f1 = if precision + recall > 0.0 { 2.0 * precision * recall / (precision + recall) } else { 0.0 };

        eprintln!("{:<22} {:>5} {:>5} {:>5} {:>7.1} {:>7.1} {:>5.1}",
            theme, t, f, n, precision, recall, f1);
    }

    let total_prec = if total_tp + total_fp > 0 { total_tp as f64 / (total_tp + total_fp) as f64 * 100.0 } else { 0.0 };
    let total_recall = if total_tp + total_fn > 0 { total_tp as f64 / (total_tp + total_fn) as f64 * 100.0 } else { 0.0 };
    let total_f1 = if total_prec + total_recall > 0.0 { 2.0 * total_prec * total_recall / (total_prec + total_recall) } else { 0.0 };

    eprintln!("{}", "-".repeat(72));
    eprintln!("{:<22} {:>5} {:>5} {:>5} {:>7.1} {:>7.1} {:>5.1}",
        "TOTAL", total_tp, total_fp, total_fn, total_prec, total_recall, total_f1);
}

// ===========================================================================
// Ad-hoc game analysis: replay a game, find blunders, classify puzzles
// ===========================================================================

/// Minimal SAN parser: find the legal move matching a SAN string.
fn resolve_san(board: &Board, san: &str) -> chess::ChessMove {
    let clean = san.trim_end_matches(|c: char| c == '+' || c == '#' || c == '!' || c == '?');
    let legal: Vec<chess::ChessMove> = MoveGen::new_legal(board).collect();

    // Castling
    if clean == "O-O" || clean == "0-0" {
        return *legal.iter().find(|m| {
            board.piece_on(m.get_source()) == Some(Piece::King)
                && m.get_dest().get_file().to_index() > m.get_source().get_file().to_index()
                && m.get_dest().get_file().to_index() - m.get_source().get_file().to_index() == 2
        }).expect("no kingside castling found");
    }
    if clean == "O-O-O" || clean == "0-0-0" {
        return *legal.iter().find(|m| {
            board.piece_on(m.get_source()) == Some(Piece::King)
                && m.get_source().get_file().to_index() > m.get_dest().get_file().to_index()
                && m.get_source().get_file().to_index() - m.get_dest().get_file().to_index() == 2
        }).expect("no queenside castling found");
    }

    let bytes = clean.as_bytes();
    let (piece, rest) = if bytes[0].is_ascii_uppercase() {
        let p = match bytes[0] {
            b'K' => Piece::King, b'Q' => Piece::Queen, b'R' => Piece::Rook,
            b'B' => Piece::Bishop, b'N' => Piece::Knight, _ => panic!("unknown piece"),
        };
        (p, &clean[1..])
    } else {
        (Piece::Pawn, clean)
    };

    let (rest, promotion) = if let Some(eq) = rest.find('=') {
        let promo = match rest.as_bytes().get(eq + 1) {
            Some(b'Q') => Some(Piece::Queen), Some(b'R') => Some(Piece::Rook),
            Some(b'B') => Some(Piece::Bishop), Some(b'N') => Some(Piece::Knight),
            _ => None,
        };
        (&rest[..eq], promo)
    } else {
        (rest, None)
    };

    let rest = rest.replace('x', "");
    let rb = rest.as_bytes();
    let dest_file = rb[rb.len() - 2];
    let dest_rank = rb[rb.len() - 1];
    let dest = Square::make_square(
        Rank::from_index((dest_rank - b'1') as usize),
        File::from_index((dest_file - b'a') as usize),
    );
    let disambig = &rest[..rest.len() - 2];

    let mut candidates: Vec<chess::ChessMove> = legal.into_iter().filter(|m| {
        m.get_dest() == dest
            && board.piece_on(m.get_source()) == Some(piece)
            && m.get_promotion() == promotion
    }).collect();

    if candidates.len() > 1 && !disambig.is_empty() {
        let db = disambig.as_bytes();
        candidates.retain(|m| {
            let src = m.get_source();
            for &b in db {
                if (b'a'..=b'h').contains(&b) && src.get_file().to_index() != (b - b'a') as usize { return false; }
                if (b'1'..=b'8').contains(&b) && src.get_rank().to_index() != (b - b'1') as usize { return false; }
            }
            true
        });
    }

    assert_eq!(candidates.len(), 1, "expected 1 match for SAN '{}', got {}", san, candidates.len());
    candidates[0]
}

/// Evaluate and also return the bestmove UCI string.
fn evaluate_with_bestmove(sf: &mut StockfishProcess, fen: &str, nodes: u32) -> (i32, Option<i32>, String) {
    sf.send("isready");
    sf.wait_for("readyok");
    sf.send(&format!("position fen {}", fen));
    sf.send(&format!("go nodes {}", nodes));

    let mut cp = 0i32;
    let mut mate: Option<i32> = None;
    let mut bestmove = String::new();
    let mut line = String::new();

    loop {
        line.clear();
        let bytes = sf.reader.read_line(&mut line).unwrap();
        if bytes == 0 { panic!("Stockfish EOF"); }
        let trimmed = line.trim();

        if trimmed.starts_with("info") && trimmed.contains(" pv ") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if let Some(idx) = parts.iter().position(|&p| p == "score") {
                if idx + 2 < parts.len() {
                    match parts[idx + 1] {
                        "cp" => { cp = parts[idx + 2].parse().unwrap_or(0); mate = None; }
                        "mate" => { mate = parts[idx + 2].parse().ok(); cp = 0; }
                        _ => {}
                    }
                }
            }
        }
        if trimmed.starts_with("bestmove") {
            bestmove = trimmed.split_whitespace().nth(1).unwrap_or("").to_string();
            break;
        }
    }
    (cp, mate, bestmove)
}

/// Replay a game from SAN moves, find blunders, extract puzzles, run cook().
///
/// Run with: cargo test --test classify_test analyze_game -- --ignored --nocapture
#[test]
#[ignore]
fn analyze_game() {
    let sf_path = match find_stockfish() {
        Some(p) => p,
        None => { eprintln!("SKIPPING: Stockfish not found"); return; }
    };
    let mut sf = StockfishProcess::new(&sf_path);
    let nodes = 100_000u32;

    let san_moves = [
        "e4", "e5", "Nf3", "Nc6", "Bc4", "Bc5", "b4", "Bxb4",
        "c3", "Ba5", "d4", "Qf6", "O-O", "exd4", "cxd4", "Bb6",
        "e5", "Qg6", "d5", "Na5", "Bd3", "f5", "exf6", "Qxf6",
        "Re1", "Ne7", "Bg5", "Qxa1",
    ];

    // Replay game, collect (board_before, chess_move, board_after, uci) for each ply
    let mut board = Board::default();
    let mut positions: Vec<(Board, chess::ChessMove, Board, String)> = Vec::new();

    for san in &san_moves {
        let chess_move = resolve_san(&board, san);
        let uci = format!("{}{}", chess_move.get_source(), chess_move.get_dest());
        let board_after = board.make_move_new(chess_move);
        positions.push((board, chess_move, board_after, uci));
        board = board_after;
    }

    // Evaluate every position (before each move + final position)
    eprintln!("\n=== Evaluating {} positions with Stockfish ({} nodes) ===\n", positions.len() + 1, nodes);

    let mut evals: Vec<i32> = Vec::new();

    // Starting position eval
    let (cp0, mate0, _) = evaluate_with_bestmove(&mut sf, &Board::default().to_string(), nodes);
    evals.push(if let Some(m) = mate0 { if m > 0 { 10000 } else { -10000 } } else { cp0 });

    // After each move
    for (i, (_, _, board_after, _)) in positions.iter().enumerate() {
        let fen = board_after.to_string();
        let is_white_to_move = board_after.side_to_move() == Color::White;
        let (cp, mate, _) = evaluate_with_bestmove(&mut sf, &fen, nodes);

        // Convert to white's perspective
        let white_cp = if let Some(m) = mate {
            let stm_cp = if m > 0 { 10000 } else { -10000 };
            if is_white_to_move { stm_cp } else { -stm_cp }
        } else {
            if is_white_to_move { cp } else { -cp }
        };
        evals.push(white_cp);

        let move_num = (i / 2) + 1;
        let side = if i % 2 == 0 { "W" } else { "B" };
        eprintln!("  {}.{} {:<6} eval={:>6}", move_num, side, san_moves[i], white_cp);
    }

    // Find blunders (cp_loss >= 200)
    eprintln!("\n=== Blunders (cp_loss >= 200) ===\n");
    let mut blunder_count = 0;

    for (i, (_board_before, _chess_move, _board_after, _uci)) in positions.iter().enumerate() {
        let is_white = i % 2 == 0;
        let eval_before = evals[i];
        let eval_after = evals[i + 1];

        // CP loss from the mover's perspective
        let cp_loss = if is_white {
            eval_before - eval_after  // white wants eval to stay high
        } else {
            eval_after - eval_before  // black wants eval to stay low (negative)
        };

        if cp_loss >= 200 {
            blunder_count += 1;
            let move_num = (i / 2) + 1;
            let side = if is_white { "W" } else { "B" };
            eprintln!("  BLUNDER {}.{} {} (cp_loss={})", move_num, side, san_moves[i], cp_loss);

            // Get the refutation (bestmove after the blunder)
            let fen_after = positions[i].2.to_string();
            let (ref_cp, ref_mate, bestmove_uci) = evaluate_with_bestmove(&mut sf, &fen_after, nodes);
            eprintln!("    Refutation: {} (eval: cp={} mate={:?})", bestmove_uci, ref_cp, ref_mate);

            // Build a puzzle: blunder move + refutation
            // For cook(), we need at least the blunder (ply 0) + solver response (ply 1)
            if !bestmove_uci.is_empty() {
                let blunder_board_before = positions[i].0;
                let blunder_move = positions[i].1;
                let board_after_blunder = positions[i].2;

                let refutation = parse_uci_move(&board_after_blunder, &bestmove_uci)
                    .expect("invalid refutation UCI");
                let board_after_refutation = board_after_blunder.make_move_new(refutation);

                let solver_color = if is_white { Color::Black } else { Color::White };

                // Compute CP from solver's perspective
                let solver_cp = if let Some(m) = ref_mate {
                    let stm_cp: i32 = if m > 0 { 10000 } else { -10000 };
                    // ref_cp is from side-to-move after blunder, which IS the solver
                    stm_cp.abs()
                } else {
                    ref_cp.abs()
                };

                let puzzle = Puzzle {
                    id: format!("game_m{}", i),
                    mainline: vec![
                        PuzzleNode {
                            board_before: blunder_board_before,
                            board_after: board_after_blunder,
                            chess_move: blunder_move,
                            ply: 0,
                        },
                        PuzzleNode {
                            board_before: board_after_blunder,
                            board_after: board_after_refutation,
                            chess_move: refutation,
                            ply: 1,
                        },
                    ],
                    pov: solver_color,
                    cp: solver_cp,
                };

                let tags = cook(&puzzle);
                eprintln!("    Tags: {:?}\n", tags);
            }
        }
    }

    if blunder_count == 0 {
        eprintln!("  No blunders found in this game.");
    }

    eprintln!("\n=== Done ===");
}
