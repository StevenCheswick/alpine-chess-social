"""
Unified analyzer for finding the quickest checkmate win (excluding Qxf7# scholar's mates).
Uses the unified move-by-move approach for efficiency.
"""
import chess
import re
from typing import List, Dict, Any, Optional
from ..unified_analyzer import UnifiedAnalyzerBase, MoveContext
from ..game_data import GameData


class UnifiedQuickestMateAnalyzer(UnifiedAnalyzerBase):
    """
    Unified version of QuickestMateAnalyzer.
    Finds the fastest checkmate win (excluding scholar's mate pattern).
    Tracks the single fastest mate across all games.
    Uses unified move-by-move processing for better performance.
    """
    
    def __init__(self, username: str):
        """Initialize with the username to filter for."""
        super().__init__(username)
        self.quickest_mate = float('inf')  # Track fastest mate (in half-moves)
        self.quickest_game_ref = None  # Lightweight reference to best game
    
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
        self.game_ended_in_mate = False
        self.game_move_count = 0
    
    def process_move(self, context: MoveContext):
        """Process a single move. Track move count."""
        self.game_move_count += 1
    
    def finish_game(self) -> List[Dict[str, Any]]:
        """
        Finalize analysis for the game and return findings.
        Called after all moves have been processed.
        
        Checks if the game ended in checkmate and if it's faster than current best.
        
        Returns:
            List of findings for this game (empty - we track across all games)
        """
        # Check if user won
        result = self.game_data.metadata.result
        user_won = (result == "1-0" and self.user_is_white) or (result == "0-1" and self.user_is_black)
        if not user_won:
            return []
        
        # Check if game ended in checkmate
        moves = self.game_data.moves
        if not moves or '#' not in moves[-1]:
            return []
        
        last_move = moves[-1]
        
        # Exclude Qxf7# or Qf7# (scholar's mate pattern)
        if last_move.startswith('Q') and 'f7' in last_move and '#' in last_move:
            return []
        # Also exclude Qxf2# for black's version
        if last_move.startswith('Q') and 'f2' in last_move and '#' in last_move:
            return []
        
        move_count = len(moves)
        
        # Check if this is faster than current best
        if move_count < self.quickest_mate:
            self.quickest_mate = move_count
            self.quickest_game_ref = {
                "game_data": self.game_data,
                "move_count": move_count,
                "last_move": last_move,
                "user_is_white": self.user_is_white
            }
        
        return []  # Return empty - we track across all games
    
    def get_game_points(self, config: dict) -> int:
        """Return points for current game based on existing findings."""
        # Quickest mate doesn't contribute to best game scoring
        # (it's a stat, not a feature to score)
        return 0
    
    def _extract_fen(self, pgn: str) -> Optional[str]:
        """Extract FEN from PGN headers (CurrentPosition or FEN)."""
        # Look for FEN in headers
        match = re.search(r'\[FEN\s+"([^"]+)"\]', pgn)
        if match:
            return match.group(1)
        # Chess.com includes CurrentPosition for final position
        match = re.search(r'\[CurrentPosition\s+"([^"]+)"\]', pgn)
        if match:
            return match.group(1)
        return None
    
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
        Returns the single fastest checkmate found.
        NOW extracts FEN and ELO only for the selected result.
        
        Returns:
            List containing the fastest checkmate finding (or empty if none found)
        """
        if not self.quickest_game_ref:
            return []
        
        ref = self.quickest_game_ref
        
        # Extract ELO (deferred until here)
        white_elo = self._extract_elo(ref["game_data"].pgn, "WhiteElo")
        black_elo = self._extract_elo(ref["game_data"].pgn, "BlackElo")
        
        # Determine winner and loser
        winner_is_white = ref["user_is_white"]
        winner = ref["game_data"].metadata.white if winner_is_white else ref["game_data"].metadata.black
        loser = ref["game_data"].metadata.black if winner_is_white else ref["game_data"].metadata.white
        
        # Use half-move count - convert to 0-indexed (last move is at len(moves) - 1)
        # move_count is the total number of half-moves, so last move index is move_count - 1
        half_move_number = ref["move_count"] - 1 if ref["move_count"] > 0 else 0
        
        # Build replay data structure for frontend (uses config file - 0/0 = full game)
        from ..replay_helper import build_replay_data
        replay_data = build_replay_data(
            game_data=ref["game_data"],
            key_half_move=0,  # Start from move 0 like favorite_gambit
            feature_name="quickest_mate"  # Loads settings from replay_config.json (0/0 = full game)
        )
        
        # Build full finding with extracted data
        finding = {
            "feature_name": "quickest_mate",
            "display_name": "Quickest Mate",
            "game_metadata": {
                "white": ref["game_data"].metadata.white,
                "black": ref["game_data"].metadata.black,
                "link": ref["game_data"].metadata.link,
                "white_elo": white_elo,  # Extracted here, not during processing!
                "black_elo": black_elo,  # Extracted here, not during processing!
                "user_color": "white" if ref["user_is_white"] else "black",
                "pgn": ref["game_data"].pgn,
                # New simplified format for frontend navigation
                "all_moves": replay_data["all_moves"],
                "key_position_index": replay_data["key_position_index"],
                "fen": replay_data["fen"]
            },
            "position_link": f"{ref['game_data'].metadata.link}?move={half_move_number}" if ref["game_data"].metadata.link else None,
            "result_data": {
                "move_count": {
                    "value": (ref["move_count"] + 1) // 2,  # Display full move number, not half-moves
                    "label": "Moves to Mate"
                }
            }
        }
        
        return [finding]


