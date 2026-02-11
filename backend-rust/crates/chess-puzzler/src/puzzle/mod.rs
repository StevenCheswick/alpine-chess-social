/// Puzzle data model and tactical theme classification

pub mod cook;
pub mod extraction;

use chess::{Board, ChessMove, Color};
use serde::{Deserialize, Serialize};

/// All possible puzzle tags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TagKind {
    AdvancedPawn,
    Advantage,
    AnastasiaMate,
    ArabianMate,
    Attraction,
    BackRankMate,
    BishopEndgame,
    BodenMate,
    Castling,
    Clearance,
    Crushing,
    DefensiveMove,
    DiscoveredAttack,
    Deflection,
    DoubleBishopMate,
    DoubleCheck,
    DovetailMate,
    Equality,
    EnPassant,
    ExposedKing,
    Fork,
    HangingPiece,
    HookMate,
    Interference,
    Intermezzo,
    KingsideAttack,
    KnightEndgame,
    Long,
    Mate,
    MateIn5,
    MateIn4,
    MateIn3,
    MateIn2,
    MateIn1,
    OneMove,
    PawnEndgame,
    Pin,
    Promotion,
    QueenEndgame,
    QueensideAttack,
    QuietMove,
    RookEndgame,
    QueenRookEndgame,
    Sacrifice,
    Short,
    Skewer,
    SmotheredMate,
    TrappedPiece,
    UnderPromotion,
    VeryLong,
    XRayAttack,
    Zugzwang,
}

/// A single node in the puzzle mainline
#[derive(Debug, Clone)]
pub struct PuzzleNode {
    /// Board state BEFORE this move
    pub board_before: Board,
    /// Board state AFTER this move
    pub board_after: Board,
    /// The move played
    pub chess_move: ChessMove,
    /// Ply index in the puzzle (0 = opponent's mistake, 1 = first solver move, etc.)
    pub ply: usize,
}

/// A chess puzzle with a solution line
#[derive(Debug, Clone)]
pub struct Puzzle {
    /// Puzzle identifier
    pub id: String,
    /// The mainline: [opponent_mistake, solver_move_1, opp_response_1, solver_move_2, ...]
    pub mainline: Vec<PuzzleNode>,
    /// The side solving the puzzle (the side that plays the good moves)
    pub pov: Color,
    /// Evaluation advantage (centipawns, positive = solver winning)
    pub cp: i32,
}

impl Puzzle {
    /// Get solver's moves (odd indices: 1, 3, 5, ...)
    pub fn solver_moves(&self) -> Vec<&PuzzleNode> {
        self.mainline.iter().skip(1).step_by(2).collect()
    }

    /// Get opponent's moves (even indices: 0, 2, 4, ...)
    pub fn opponent_moves(&self) -> Vec<&PuzzleNode> {
        self.mainline.iter().step_by(2).collect()
    }

    /// Get the final board position
    pub fn end_board(&self) -> &Board {
        &self.mainline.last().unwrap().board_after
    }

    /// Get the initial board (before the opponent's mistake)
    pub fn initial_board(&self) -> &Board {
        &self.mainline[0].board_before
    }
}
