//! Unified analyzer that orchestrates all individual analyzers in a single pass.

use chess_core::game_data::GameData;
use shakmaty::{Chess, Color, Position, san::San};
use std::collections::HashMap;

use crate::analyzer_trait::{GameAnalyzer, MoveContext};
use crate::analyzers;
use crate::ANALYZER_TAGS;

// Pre-filter sets (same as Python)
const MATE_ANALYZERS: &[&str] = &[
    "smothered_mate", "king_mate", "castle_mate", "pawn_mate",
    "knight_promotion_mate", "promotion_mate", "quickest_mate",
    "en_passant_mate", "back_rank_mate", "knight_bishop_mate", "king_walk",
];

const WIN_ANALYZERS: &[&str] = &[
    "queen_sacrifice", "knight_fork", "rook_sacrifice", "quickest_mate",
    "biggest_comeback", "clutch_win", "best_game", "longest_game",
    "king_walk", "windmill",
];

const DRAW_ANALYZERS: &[&str] = &["stalemate"];

/// Create all 21 analyzer instances.
fn create_all_analyzers(username: &str) -> Vec<Box<dyn GameAnalyzer>> {
    vec![
        Box::new(analyzers::queen_sacrifice::QueenSacrificeAnalyzer::new(username)),
        Box::new(analyzers::knight_fork::KnightForkAnalyzer::new(username)),
        Box::new(analyzers::rook_sacrifice::RookSacrificeAnalyzer::new(username)),
        Box::new(analyzers::back_rank_mate::BackRankMateAnalyzer::new(username)),
        Box::new(analyzers::smothered_mate::SmotheredMateAnalyzer::new(username)),
        Box::new(analyzers::king_mate::KingMateAnalyzer::new(username)),
        Box::new(analyzers::castle_mate::CastleMateAnalyzer::new(username)),
        Box::new(analyzers::pawn_mate::PawnMateAnalyzer::new(username)),
        Box::new(analyzers::knight_promotion_mate::KnightPromotionMateAnalyzer::new(username)),
        Box::new(analyzers::promotion_mate::PromotionMateAnalyzer::new(username)),
        Box::new(analyzers::quickest_mate::QuickestMateAnalyzer::new(username)),
        Box::new(analyzers::en_passant_mate::EnPassantMateAnalyzer::new(username)),
        Box::new(analyzers::knight_bishop_mate::KnightBishopMateAnalyzer::new(username)),
        Box::new(analyzers::king_walk::KingWalkAnalyzer::new(username)),
        Box::new(analyzers::biggest_comeback::BiggestComebackAnalyzer::new(username)),
        Box::new(analyzers::clutch_win::ClutchWinAnalyzer::new(username)),
        Box::new(analyzers::best_game::BestGameAnalyzer::new(username)),
        Box::new(analyzers::longest_game::LongestGameAnalyzer::new(username)),
        Box::new(analyzers::hung_queen::HungQueenAnalyzer::new(username)),
        Box::new(analyzers::capture_sequence::CaptureSequenceAnalyzer::new(username)),
        Box::new(analyzers::stalemate::StalemateAnalyzer::new(username)),
        Box::new(analyzers::windmill::WindmillAnalyzer::new(username)),
    ]
}

fn is_hyper_bullet(time_control: &str) -> bool {
    time_control
        .split('+')
        .next()
        .and_then(|base| base.parse::<f64>().ok())
        .map(|base| base < 60.0)
        .unwrap_or(false)
}

/// Analyze multiple games with all analyzers. Returns game_link -> tags map.
pub fn analyze_games(
    username: &str,
    games: &[GameData],
) -> HashMap<String, Vec<String>> {
    let mut all_analyzers = create_all_analyzers(username);
    let username_lower = username.to_lowercase();

    for game in games.iter() {
        // Determine user's side
        let user_is_white = game.metadata.white.to_lowercase() == username_lower;
        let user_is_black = game.metadata.black.to_lowercase() == username_lower;
        if !user_is_white && !user_is_black {
            continue;
        }

        // Skip hyper bullet
        if let Some(ref tc) = game.metadata.time_control {
            if is_hyper_bullet(tc) {
                continue;
            }
        }

        let user_color = if user_is_white {
            Color::White
        } else {
            Color::Black
        };

        // Pre-filter conditions
        let has_checkmate = game.moves.last().map(|m| m.contains('#')).unwrap_or(false);
        let user_won = (game.metadata.result == "1-0" && user_is_white)
            || (game.metadata.result == "0-1" && !user_is_white);
        let is_draw = game.metadata.result == "1/2-1/2";

        // Initialize all analyzers
        for analyzer in all_analyzers.iter_mut() {
            analyzer.start_game(game, user_is_white);
        }

        // Determine active analyzers based on pre-filters
        let active_indices: Vec<usize> = all_analyzers
            .iter()
            .enumerate()
            .filter(|(_, a)| {
                let name = a.name();
                if MATE_ANALYZERS.contains(&name) && !has_checkmate {
                    return false;
                }
                if WIN_ANALYZERS.contains(&name) && !user_won {
                    return false;
                }
                if DRAW_ANALYZERS.contains(&name) && !is_draw {
                    return false;
                }
                true
            })
            .map(|(i, _)| i)
            .collect();

        // Replay game move-by-move
        let mut pos = Chess::default();
        let mut move_number = 0usize;

        // Decode moves from TCN or parse SAN
        let decoded_moves: Vec<shakmaty::Move> = if let Some(ref tcn) = game.tcn {
            chess_core::tcn::decode_tcn(tcn)
        } else {
            // Parse SAN moves
            let mut moves = Vec::new();
            let mut temp_pos = Chess::default();
            for san_str in &game.moves {
                if let Ok(san) = san_str.parse::<San>() {
                    if let Ok(mv) = san.to_move(&temp_pos) {
                        temp_pos.play_unchecked(&mv);
                        moves.push(mv);
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            moves
        };

        for mv in &decoded_moves {
            move_number += 1;
            let is_user_move = (pos.turn() == Color::White) == user_is_white;

            let ctx = MoveContext {
                mv,
                move_number,
                board: &pos,
                is_user_move,
                is_opponent_move: !is_user_move,
                user_color,
                game_data: game,
            };

            for &idx in &active_indices {
                all_analyzers[idx].process_move(&ctx);
            }

            pos.play_unchecked(mv);
        }

        // Finalize all analyzers for this game
        for analyzer in all_analyzers.iter_mut() {
            analyzer.finish_game();
        }
    }

    // Build tags map from all analyzers
    let mut tags_map: HashMap<String, Vec<String>> = HashMap::new();

    for analyzer in &all_analyzers {
        let name = analyzer.name();
        let display_tag = ANALYZER_TAGS
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, tag)| *tag)
            .unwrap_or(name);

        for link in analyzer.matched_game_links() {
            if !link.is_empty() {
                tags_map
                    .entry(link)
                    .or_default()
                    .push(display_tag.to_string());
            }
        }
    }

    // Deduplicate tags per game
    for tags in tags_map.values_mut() {
        tags.sort();
        tags.dedup();
    }

    tags_map
}
