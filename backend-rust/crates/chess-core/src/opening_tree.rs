//! Opening tree builder for chess repertoire analysis.

use serde_json::Value as JsonValue;
use shakmaty::{Chess, Position, san::San};
use std::collections::HashMap;

/// Input game data for tree building.
pub struct TreeGame {
    pub moves: Vec<String>, // SAN moves
    pub result: String,     // "W", "L", "D"
}

struct TreeNode {
    mv: String,
    fen: String,
    games: i64,
    wins: i64,
    losses: i64,
    draws: i64,
    children: HashMap<String, TreeNode>,
}

impl TreeNode {
    fn new(mv: &str, fen: &str) -> Self {
        Self {
            mv: mv.to_string(),
            fen: fen.to_string(),
            games: 0,
            wins: 0,
            losses: 0,
            draws: 0,
            children: HashMap::new(),
        }
    }
}

/// Build an opening tree from a list of games.
pub fn build_opening_tree(games: &[TreeGame], max_depth: usize) -> JsonValue {
    let starting_fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let mut root = TreeNode::new("start", starting_fen);

    for game in games {
        let moves = &game.moves[..game.moves.len().min(max_depth * 2)];

        let mut current = &mut root;
        let mut pos = Chess::default();

        for move_san in moves {
            // Parse and apply the move
            let san: San = match move_san.parse() {
                Ok(s) => s,
                Err(_) => break,
            };

            let mv = match san.to_move(&pos) {
                Ok(m) => m,
                Err(_) => break,
            };

            pos.play_unchecked(&mv);

            let fen = {
                use shakmaty::fen::Fen;
                Fen::from_position(pos.clone(), shakmaty::EnPassantMode::Legal).to_string()
            };

            let child = current
                .children
                .entry(move_san.clone())
                .or_insert_with(|| TreeNode::new(move_san, &fen));

            child.games += 1;
            match game.result.as_str() {
                "W" => child.wins += 1,
                "L" => child.losses += 1,
                _ => child.draws += 1,
            }

            current = current
                .children
                .get_mut(move_san)
                .unwrap();
        }
    }

    node_to_json(&root)
}

/// Convert tree to JSON response format (children as sorted array).
pub fn convert_tree_for_response(tree: &JsonValue) -> JsonValue {
    // The tree is already in the correct format from build_opening_tree
    tree.clone()
}

fn node_to_json(node: &TreeNode) -> JsonValue {
    let mut children: Vec<JsonValue> = node
        .children
        .values()
        .map(|child| node_to_json(child))
        .collect();

    // Sort by game count (most played first)
    children.sort_by(|a, b| {
        b["games"]
            .as_i64()
            .unwrap_or(0)
            .cmp(&a["games"].as_i64().unwrap_or(0))
    });

    let win_rate = if node.games > 0 {
        ((node.wins as f64 / node.games as f64) * 1000.0).round() / 10.0
    } else {
        0.0
    };

    serde_json::json!({
        "move": node.mv,
        "fen": node.fen,
        "games": node.games,
        "wins": node.wins,
        "losses": node.losses,
        "draws": node.draws,
        "winRate": win_rate,
        "children": children,
    })
}
