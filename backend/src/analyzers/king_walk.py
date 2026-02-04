"""
Unified analyzer for detecting king walks - checkmates where the opponent's king
has been hunted far from its home rank through a series of checks.
Uses the unified move-by-move approach for efficiency.
"""
import chess
import json
import os
import re
from typing import List, Dict, Any, Optional
from ..unified_analyzer import UnifiedAnalyzerBase, MoveContext
from ..game_data import GameData


class UnifiedKingWalkAnalyzer(UnifiedAnalyzerBase):
    """
    Unified version of KingWalkAnalyzer.
    Finds the game where the user delivered checkmate after hunting the opponent's
    king far from its home rank.

    Scoring: score = square_value × move_mult × hunt_mult × mat_mult
    - square_value: Points based on how far king was dragged (configurable)
    - move_mult: Game length multiplier (configurable)
    - hunt_mult: checks × hunt_multiplier_per_check (configurable)
    - mat_mult: remaining_material / 78 (more pieces = harder)

    All scoring weights are loaded from config/king_walk_scoring_config.json
    """

    # Material values for calculating mat_mult (fixed)
    PIECE_VALUES = {
        chess.PAWN: 1,
        chess.KNIGHT: 3,
        chess.BISHOP: 3,
        chess.ROOK: 5,
        chess.QUEEN: 9,
        chess.KING: 0
    }

    # Full board material (16 pawns + 4 knights + 4 bishops + 4 rooks + 2 queens)
    FULL_BOARD_MATERIAL = 78

    def __init__(self, username: str, config_path: str = "config/king_walk_scoring_config.json"):
        """Initialize with the username and config path."""
        super().__init__(username)
        self.config = self._load_config(config_path)
        self.best_king_walk_ref = None  # Store reference to best king walk
        self.best_score = 0  # Track highest score
        self.total_king_walks = 0  # Count all qualifying games

    def _load_config(self, config_path: str) -> dict:
        """Load scoring configuration from JSON file."""
        default_config = {
            "square_values": {"1": 100, "2": 95, "3": 90, "4": 80, "5": 60, "6": 0, "7": 0, "8": 0},
            "move_multipliers": {"1-14": 0.5, "15-19": 0.75, "20-30": 1.0, "31-35": 0.75},
            "hunt_multiplier_per_check": 0.1,
            "max_moves": 35,
            "min_base_time_seconds": 60
        }

        if not os.path.exists(config_path):
            return default_config

        try:
            with open(config_path, 'r', encoding='utf-8') as f:
                return json.load(f)
        except Exception:
            return default_config

    def start_game(self, game_data: GameData, user_is_white: bool, user_is_black: bool):
        """
        Initialize analyzer state for a new game.
        Pre-filters to skip games that can't have a king walk.
        """
        super().start_game(game_data, user_is_white, user_is_black)

        # Reset per-game state
        self.current_game_qualifies = False
        self.current_game_is_king_walk = False
        self.current_game_king_walk_score = 0  # Actual score for get_game_points()

        # Pre-filter 1: Must end in checkmate
        moves = self.game_data.moves
        if not moves or '#' not in moves[-1]:
            return

        # Pre-filter 2: User must have won
        result = self.game_data.metadata.result
        user_won = (result == "1-0" and user_is_white) or (result == "0-1" and user_is_black)
        if not user_won:
            return

        # Pre-filter 3: Max moves (configurable)
        full_moves = (len(moves) + 1) // 2
        if full_moves > self.config.get("max_moves", 35):
            return

        self.current_game_qualifies = True

    def process_move(self, context: MoveContext):
        """Process a single move. No action needed - we analyze final position in finish_game()."""
        pass

    def _extract_fen(self, pgn: str) -> Optional[str]:
        """Extract final FEN from PGN headers (fast path - no replay needed)."""
        match = re.search(r'\[CurrentPosition\s+"([^"]+)"\]', pgn)
        if match:
            return match.group(1)
        match = re.search(r'\[FEN\s+"([^"]+)"\]', pgn)
        if match:
            return match.group(1)
        return None

    def _get_final_fen_by_replay(self, moves: List[str]) -> Optional[str]:
        """Fallback: Get final FEN by replaying moves (slow path)."""
        try:
            board = chess.Board()
            for move in moves:
                board.push_san(move)
            return board.fen()
        except Exception:
            return None

    def finish_game(self) -> List[Dict[str, Any]]:
        """
        Finalize analysis for the game and return findings.
        Calculates king walk score for qualifying games.
        """
        if not self.current_game_qualifies:
            return []

        moves = self.game_data.moves

        # Get final position FEN - try headers first (fast), fallback to replay (slow)
        final_fen = self._extract_fen(self.game_data.pgn)
        if not final_fen:
            final_fen = self._get_final_fen_by_replay(moves)
            if not final_fen:
                return []

        # Create board from final FEN
        try:
            board = chess.Board(final_fen)
        except Exception:
            return []

        # Verify checkmate
        if not board.is_checkmate():
            return []

        # Get loser's king position
        winner_is_white = self.user_is_white
        loser_color = chess.BLACK if winner_is_white else chess.WHITE
        king_square = board.king(loser_color)

        if king_square is None:
            return []

        square_name = chess.square_name(king_square)
        rank = chess.square_rank(king_square) + 1  # Convert to 1-8

        # Calculate effective rank (from winner's perspective)
        if winner_is_white:
            effective_rank = rank  # Black king: rank 1 = best
        else:
            effective_rank = 9 - rank  # White king: rank 8 = best (eff 1)

        # Get square value from config - if 0, not a qualifying king walk
        square_values = self.config.get("square_values", {})
        square_value = square_values.get(str(effective_rank), 0)
        if square_value == 0:
            return []

        # Calculate multipliers
        full_moves = (len(moves) + 1) // 2
        move_mult = self._get_move_multiplier(full_moves)
        hunt_mult, check_count = self._get_hunt_multiplier(moves)

        # Calculate material at hunt start (not final position)
        # This way defender isn't rewarded for sacrificing pieces to escape
        mat_mult = self._get_material_at_hunt_start(moves)

        # If no checks in hunt, score is 0
        if hunt_mult == 0:
            return []

        # If opponent material is below minimum, not a qualifying king walk
        # (just mopping up a won game, not an impressive hunt)
        material_minimum = self.config.get("material_minimum", 0.0)
        if mat_mult < material_minimum:
            return []

        # Calculate final score
        score = square_value * move_mult * hunt_mult * mat_mult

        # If score is below minimum, not a qualifying king walk
        score_minimum = self.config.get("score_minimum", 0)
        if score < score_minimum:
            return []

        # Track this as a qualifying king walk
        self.total_king_walks += 1
        self.current_game_is_king_walk = True
        self.current_game_king_walk_score = int(round(score))  # For get_game_points()

        # Track if this is the best one
        if score > self.best_score:
            self.best_score = score
            self.best_king_walk_ref = {
                "game_data": self.game_data,
                "user_is_white": self.user_is_white,
                "score": round(score, 1),
                "king_square": square_name,
                "effective_rank": effective_rank,
                "full_moves": full_moves,
                "check_count": check_count,
                "material": round(mat_mult, 2),
            }

        return []  # Defer to get_final_results()

    def _get_move_multiplier(self, full_moves: int) -> float:
        """Get multiplier based on move count from config."""
        move_mults = self.config.get("move_multipliers", {})
        # Parse ranges like "1-14", "15-19", etc.
        for range_str, mult in move_mults.items():
            try:
                parts = range_str.split("-")
                low, high = int(parts[0]), int(parts[1])
                if low <= full_moves <= high:
                    return mult
            except (ValueError, IndexError):
                continue
        return 0.75  # Default fallback

    def _get_hunt_multiplier(self, moves: List[str]) -> tuple:
        """Get multiplier based on checks in last 10 full moves (20 ply)."""
        last_moves = moves[-20:] if len(moves) >= 20 else moves
        check_count = sum(1 for m in last_moves if '+' in m or '#' in m)
        mult_per_check = self.config.get("hunt_multiplier_per_check", 0.1)
        return check_count * mult_per_check, check_count

    def _get_material_multiplier(self, board: chess.Board) -> float:
        """Get multiplier based on remaining material."""
        if not self.config.get("material_multiplier_enabled", True):
            return 1.0

        material = sum(
            self.PIECE_VALUES[piece.piece_type]
            for piece in board.piece_map().values()
        )
        base = self.config.get("material_multiplier_base", self.FULL_BOARD_MATERIAL)
        return material / base

    def _get_material_at_hunt_start(self, moves: List[str]) -> float:
        """Get material multiplier at the START of the king hunt (first check in last 20 ply).

        Only counts OPPONENT's material - a king walk against a fully armed opponent
        is more impressive than hunting a king after you've already won their pieces.
        """
        if not self.config.get("material_multiplier_enabled", True):
            return 1.0

        # Find the hunt window (last 20 ply)
        hunt_start_idx = max(0, len(moves) - 20)

        # Find the first check in the hunt window
        first_check_idx = None
        for i in range(hunt_start_idx, len(moves)):
            if '+' in moves[i] or '#' in moves[i]:
                first_check_idx = i
                break

        if first_check_idx is None:
            # No checks found, use final position
            first_check_idx = len(moves) - 1

        # Replay game up to the position BEFORE the first check
        try:
            board = chess.Board()
            for i, move in enumerate(moves):
                if i >= first_check_idx:
                    break
                board.push_san(move)

            # Only count OPPONENT's material (the loser / person being hunted)
            loser_color = chess.BLACK if self.user_is_white else chess.WHITE
            opponent_material = sum(
                self.PIECE_VALUES[piece.piece_type]
                for square, piece in board.piece_map().items()
                if piece.color == loser_color
            )
            # Opponent's max material is 39 (Q=9, 2R=10, 2B=6, 2N=6, 8P=8)
            base = self.config.get("material_multiplier_base", 39)
            return opponent_material / base
        except Exception:
            # Fallback to 1.0 if replay fails
            return 1.0

    def get_game_points(self, config: dict) -> int:
        """Return points for current game using internal king walk score."""
        return self.current_game_king_walk_score

    def _extract_elo(self, pgn: str, elo_header: str) -> Optional[int]:
        """Extract ELO rating from PGN header."""
        match = re.search(rf'\[{elo_header}\s+"(\d+)"\]', pgn)
        if match:
            try:
                return int(match.group(1))
            except ValueError:
                return None
        return None

    def get_final_results(self) -> List[Dict[str, Any]]:
        """
        Get final results after processing all games.
        Returns the single best king walk (highest score).
        """
        if not self.best_king_walk_ref:
            return []

        ref = self.best_king_walk_ref
        game_data = ref["game_data"]

        # Extract ELO (deferred until here)
        white_elo = self._extract_elo(game_data.pgn, "WhiteElo")
        black_elo = self._extract_elo(game_data.pgn, "BlackElo")

        # Build replay data - show 10 moves before mate to capture the hunt
        from ..replay_helper import build_replay_data
        final_move_number = len(game_data.moves)
        key_half_move = max(0, final_move_number - 20)  # 10 full moves before end

        replay_data = build_replay_data(
            game_data=game_data,
            key_half_move=key_half_move,
            feature_name="king_walk"
        )

        finding = {
            "feature_name": "king_walk",
            "display_name": "King Walk",
            "game_metadata": {
                "white": game_data.metadata.white,
                "black": game_data.metadata.black,
                "link": game_data.metadata.link,
                "white_elo": white_elo,
                "black_elo": black_elo,
                "user_color": "white" if ref["user_is_white"] else "black",
                "all_moves": replay_data["all_moves"],
                "key_position_index": replay_data["key_position_index"],
                "fen": replay_data["fen"],
                "pgn": game_data.pgn
            },
            "position_link": f"{game_data.metadata.link}?move={final_move_number}" if game_data.metadata.link else None,
            "result_data": {
                "king_square": {
                    "value": ref["king_square"],
                    "label": "King Mated On"
                },
                "score": {
                    "value": ref["score"],
                    "label": "King Walk Score"
                }
            }
        }

        return [finding]
