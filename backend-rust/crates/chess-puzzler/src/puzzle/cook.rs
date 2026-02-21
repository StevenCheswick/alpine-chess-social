/// Cook orchestrator — calls tactical detectors and builds tag list
/// Port of tagger/cook.py cook() function

use super::{Puzzle, TagKind};
use crate::tactics;
use crate::tactics::zugzwang::ZugzwangEval;

/// Analyze a puzzle and return all applicable tags
pub fn cook(puzzle: &Puzzle) -> Vec<TagKind> {
    let mut tags = Vec::new();

    // Mate detection (elif chain for mate patterns)
    let mate_tag = tactics::simple::mate_in(puzzle);
    if let Some(mt) = mate_tag {
        tags.push(mt);
        tags.push(TagKind::Mate);

        if tactics::mate_patterns::smothered_mate(puzzle) {
            tags.push(TagKind::SmotheredMate);
        } else if tactics::mate_patterns::back_rank_mate(puzzle) {
            tags.push(TagKind::BackRankMate);
        } else if tactics::mate_patterns::anastasia_mate(puzzle) {
            tags.push(TagKind::AnastasiaMate);
        } else if tactics::mate_patterns::hook_mate(puzzle) {
            tags.push(TagKind::HookMate);
        } else if tactics::mate_patterns::arabian_mate(puzzle) {
            tags.push(TagKind::ArabianMate);
        } else if let Some(boden_tag) = tactics::mate_patterns::boden_or_double_bishop_mate(puzzle) {
            tags.push(boden_tag);
        } else if tactics::mate_patterns::dovetail_mate(puzzle) {
            tags.push(TagKind::DovetailMate);
        }
    } else if puzzle.cp > 600 {
        tags.push(TagKind::Crushing);
    } else if puzzle.cp > 200 {
        tags.push(TagKind::Advantage);
    } else {
        tags.push(TagKind::Equality);
    }

    if tactics::positional::attraction(puzzle) {
        tags.push(TagKind::Attraction);
    }

    if tactics::positional::deflection(puzzle) {
        tags.push(TagKind::Deflection);
    }

    if tactics::simple::advanced_pawn(puzzle) {
        tags.push(TagKind::AdvancedPawn);
    }

    if tactics::simple::double_check(puzzle) {
        tags.push(TagKind::DoubleCheck);
    }

    if tactics::positional::quiet_move(puzzle) {
        tags.push(TagKind::QuietMove);
    }

    if tactics::positional::defensive_move(puzzle) || tactics::simple::check_escape(puzzle) {
        tags.push(TagKind::DefensiveMove);
    }

    if let Some(piece) = tactics::material::sacrifice(puzzle) {
        tags.push(TagKind::Sacrifice);
        match piece {
            chess::Piece::Queen => tags.push(TagKind::QueenSacrifice),
            chess::Piece::Rook => tags.push(TagKind::RookSacrifice),
            chess::Piece::Bishop => tags.push(TagKind::BishopSacrifice),
            chess::Piece::Knight => tags.push(TagKind::KnightSacrifice),
            _ => {}
        }
    }

    if tactics::line_geometry::x_ray(puzzle) {
        tags.push(TagKind::XRayAttack);
    }

    if tactics::attacks::fork(puzzle) {
        tags.push(TagKind::Fork);
    }

    if tactics::attacks::hanging_piece(puzzle) {
        tags.push(TagKind::HangingPiece);
    }

    if tactics::attacks::trapped_piece(puzzle) {
        tags.push(TagKind::TrappedPiece);
    }

    if tactics::line_geometry::discovered_attack(puzzle) {
        tags.push(TagKind::DiscoveredAttack);
    }

    if tactics::material::exposed_king(puzzle) {
        tags.push(TagKind::ExposedKing);
    }

    if tactics::line_geometry::skewer(puzzle) {
        tags.push(TagKind::Skewer);
    }

    if tactics::positional::self_interference(puzzle) || tactics::positional::interference(puzzle) {
        tags.push(TagKind::Interference);
    }

    if tactics::positional::intermezzo(puzzle) {
        tags.push(TagKind::Intermezzo);
    }

    if tactics::pins::pin_prevents_attack(puzzle) || tactics::pins::pin_prevents_escape(puzzle) {
        tags.push(TagKind::Pin);
    }

    if tactics::positional::clearance(puzzle) {
        tags.push(TagKind::Clearance);
    }

    if tactics::simple::en_passant(puzzle) {
        tags.push(TagKind::EnPassant);
    }

    if tactics::simple::castling(puzzle) {
        tags.push(TagKind::Castling);
    }

    if tactics::simple::promotion(puzzle) {
        tags.push(TagKind::Promotion);
    }

    if tactics::simple::under_promotion(puzzle) {
        tags.push(TagKind::UnderPromotion);
    }

    // Endgame types (elif chain)
    if tactics::material::piece_endgame(puzzle, chess::Piece::Pawn) {
        tags.push(TagKind::PawnEndgame);
    } else if tactics::material::piece_endgame(puzzle, chess::Piece::Queen) {
        tags.push(TagKind::QueenEndgame);
    } else if tactics::material::piece_endgame(puzzle, chess::Piece::Rook) {
        tags.push(TagKind::RookEndgame);
    } else if tactics::material::piece_endgame(puzzle, chess::Piece::Bishop) {
        tags.push(TagKind::BishopEndgame);
    } else if tactics::material::piece_endgame(puzzle, chess::Piece::Knight) {
        tags.push(TagKind::KnightEndgame);
    } else if tactics::material::queen_rook_endgame(puzzle) {
        tags.push(TagKind::QueenRookEndgame);
    }

    // Zugzwang is detected via engine null-move analysis in the WS orchestrator,
    // not here. See cook_zugzwang() below.

    // Side attacks (only if no backRankMate and no fork)
    if !tags.contains(&TagKind::BackRankMate) && !tags.contains(&TagKind::Fork) {
        if tactics::side_attacks::kingside_attack(puzzle) {
            tags.push(TagKind::KingsideAttack);
        } else if tactics::side_attacks::queenside_attack(puzzle) {
            tags.push(TagKind::QueensideAttack);
        }
    }

    // Length tags
    let mainline_len = puzzle.mainline.len();
    if mainline_len == 2 {
        tags.push(TagKind::OneMove);
    } else if mainline_len == 4 {
        tags.push(TagKind::Short);
    } else if mainline_len >= 8 {
        tags.push(TagKind::VeryLong);
    } else {
        tags.push(TagKind::Long);
    }

    tags
}

/// Check if a puzzle exhibits zugzwang using pre-computed engine evals.
/// Called as a post-processing step after cook(), once engine null-move
/// evaluations are available from the WebSocket orchestrator.
pub fn cook_zugzwang(puzzle: &Puzzle, evals: &[ZugzwangEval]) -> bool {
    tactics::zugzwang::zugzwang(puzzle, evals)
}
