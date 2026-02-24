//! Classify opening mistake puzzles using the chess-puzzler cook() function.
//!
//! Reads puzzles_raw.json, classifies each puzzle, outputs puzzles_classified.json.
//!
//! Usage:
//!   cargo run --bin classify-openings -- ../../lichess-eval-db/puzzles_raw.json ../../lichess-eval-db/puzzles_classified.json

use std::env;
use std::fs;
use std::str::FromStr;

use chess::Board;
use analysis_worker::puzzle::cook::cook;
use analysis_worker::puzzle::extraction::parse_uci_move;
use analysis_worker::puzzle::{Puzzle, PuzzleNode};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct RawPuzzle {
    id: String,
    fen: String,
    mainline_uci: String,
    cp: i32,
    eco: String,
    games: i32,
    cp_loss: i32,
    eval_before: i32,
    eval_after: i32,
    mistake_san: String,
    sequence: String,
    side: String,
}

#[derive(Serialize)]
struct ClassifiedPuzzle {
    id: String,
    fen: String,
    mainline_uci: String,
    cp: i32,
    eco: String,
    games: i32,
    cp_loss: i32,
    eval_before: i32,
    eval_after: i32,
    mistake_san: String,
    sequence: String,
    side: String,
    themes: Vec<String>,
}

fn build_puzzle(raw: &RawPuzzle) -> Option<Puzzle> {
    let mut board = Board::from_str(&raw.fen).ok()?;
    let move_strs: Vec<&str> = raw.mainline_uci.split_whitespace().collect();

    if move_strs.len() < 2 {
        return None;
    }

    let mut mainline = Vec::new();

    for (i, uci) in move_strs.iter().enumerate() {
        let chess_move = parse_uci_move(&board, uci)?;
        let board_after = board.make_move_new(chess_move);
        mainline.push(PuzzleNode {
            board_before: board,
            board_after,
            chess_move,
            ply: i,
        });
        board = board_after;
    }

    // The opponent (who made the mistake) moves first
    // The solver is the OTHER side
    let opponent_color = Board::from_str(&raw.fen).unwrap().side_to_move();
    let pov = !opponent_color;

    Some(Puzzle {
        id: raw.id.clone(),
        mainline,
        pov,
        cp: raw.cp,
    })
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let input_path = args
        .get(1)
        .map(|s| s.as_str())
        .unwrap_or("../../lichess-eval-db/puzzles_raw.json");
    let output_path = args
        .get(2)
        .map(|s| s.as_str())
        .unwrap_or("../../lichess-eval-db/puzzles_classified.json");

    let data = fs::read_to_string(input_path).expect("Failed to read input file");
    let raw_puzzles: Vec<RawPuzzle> = serde_json::from_str(&data).expect("Failed to parse JSON");

    println!("Classifying {} puzzles...", raw_puzzles.len());

    let mut classified = Vec::new();
    let mut errors = 0;

    for raw in &raw_puzzles {
        let puzzle = match build_puzzle(raw) {
            Some(p) => p,
            None => {
                eprintln!("  SKIP {}: failed to build puzzle", raw.id);
                errors += 1;
                continue;
            }
        };

        let tags = cook(&puzzle);
        let theme_strings: Vec<String> = tags
            .iter()
            .map(|t| serde_json::to_value(t).unwrap().as_str().unwrap().to_string())
            .collect();

        classified.push(ClassifiedPuzzle {
            id: raw.id.clone(),
            fen: raw.fen.clone(),
            mainline_uci: raw.mainline_uci.clone(),
            cp: raw.cp,
            eco: raw.eco.clone(),
            games: raw.games,
            cp_loss: raw.cp_loss,
            eval_before: raw.eval_before,
            eval_after: raw.eval_after,
            mistake_san: raw.mistake_san.clone(),
            sequence: raw.sequence.clone(),
            side: raw.side.clone(),
            themes: theme_strings,
        });
    }

    let json = serde_json::to_string_pretty(&classified).expect("Failed to serialize");
    fs::write(output_path, json).expect("Failed to write output");

    println!(
        "Done: {} classified, {} errors",
        classified.len(),
        errors
    );
    println!("Wrote to {output_path}");
}
