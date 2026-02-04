"""
Unified analyzer for finding the longest game (most moves).
Uses the unified move-by-move approach for efficiency.
"""
import chess
import re
from typing import List, Dict, Any, Optional
from ..unified_analyzer import UnifiedAnalyzerBase, MoveContext
from ..game_data import GameData


class UnifiedLongestGameAnalyzer(UnifiedAnalyzerBase):
    """
    Finds the longest game (by move count) played by the user.
    Tracks the single longest game across all games.
    Uses unified move-by-move processing for better performance.
    """

    def __init__(self, username: str):
        """Initialize with the username to filter for."""
        super().__init__(username)
        self.longest_game = 0  # Track longest game (in half-moves)
        self.longest_game_ref = None  # Lightweight reference to best game

    def start_game(self, game_data: GameData, user_is_white: bool, user_is_black: bool):
        """
        Initialize analyzer state for a new game.

        Args:
            game_data: The game data
            user_is_white: Whether user is playing white
            user_is_black: Whether user is playing black
        """
        super().start_game(game_data, user_is_white, user_is_black)

        # Track state for this game
        self.game_move_count = 0

    def process_move(self, context: MoveContext):
        """Process a single move. Track move count."""
        self.game_move_count += 1

    def finish_game(self) -> List[Dict[str, Any]]:
        """
        Finalize analysis for the game.
        Checks if this game is longer than current best.

        Returns:
            List of findings for this game (empty - we track across all games)
        """
        move_count = len(self.game_data.moves) if self.game_data.moves else 0

        # Check if this is longer than current best
        if move_count > self.longest_game:
            self.longest_game = move_count
            self.longest_game_ref = {
                "game_data": self.game_data,
                "move_count": move_count,
                "user_is_white": self.user_is_white
            }

        return []  # Return empty - we track across all games

    def get_game_points(self, config: dict) -> int:
        """Return points for current game based on existing findings."""
        # Longest game doesn't contribute to best game scoring
        # (it's a stat, not a feature to score)
        return 0

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
        Returns the single longest game found.
        NOW extracts FEN and ELO only for the selected result.

        Returns:
            List containing the longest game finding (or empty if none found)
        """
        if not self.longest_game_ref:
            return []

        ref = self.longest_game_ref

        # Extract ELO (deferred until here)
        white_elo = self._extract_elo(ref["game_data"].pgn, "WhiteElo")
        black_elo = self._extract_elo(ref["game_data"].pgn, "BlackElo")

        # Determine result for user
        result = ref["game_data"].metadata.result
        if result == "1-0":
            user_result = "Won" if ref["user_is_white"] else "Lost"
        elif result == "0-1":
            user_result = "Lost" if ref["user_is_white"] else "Won"
        else:
            user_result = "Draw"

        # Build replay data structure for frontend
        from ..replay_helper import build_replay_data
        replay_data = build_replay_data(
            game_data=ref["game_data"],
            key_half_move=0,  # Start from move 0
            feature_name="longest_game"  # Loads settings from replay_config.json
        )

        # Build full finding with extracted data
        finding = {
            "feature_name": "longest_game",
            "display_name": "Longest Game",
            "game_metadata": {
                "white": ref["game_data"].metadata.white,
                "black": ref["game_data"].metadata.black,
                "link": ref["game_data"].metadata.link,
                "white_elo": white_elo,
                "black_elo": black_elo,
                "user_color": "white" if ref["user_is_white"] else "black",
                "pgn": ref["game_data"].pgn,
                # New simplified format for frontend navigation
                "all_moves": replay_data["all_moves"],
                "key_position_index": replay_data["key_position_index"],
                "fen": replay_data["fen"]
            },
            "position_link": ref["game_data"].metadata.link,
            "result_data": {
                "total_moves": {
                    "value": (ref["move_count"] + 1) // 2,  # Display full move number
                    "label": "Total Moves"
                },
                "result": {
                    "value": user_result,
                    "label": "Result"
                }
            }
        }

        return [finding]
