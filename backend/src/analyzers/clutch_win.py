"""
Unified analyzer for finding clutch wins - games won with minimal time remaining.
Uses the unified move-by-move approach for efficiency.
"""
import re
from typing import List, Dict, Any, Optional
from ..unified_analyzer import UnifiedAnalyzerBase, MoveContext
from ..game_data import GameData


class UnifiedClutchWinAnalyzer(UnifiedAnalyzerBase):
    """
    Unified version of ClutchWinAnalyzer.
    Finds the game where the user won with the least time remaining on the clock.
    Uses unified move-by-move processing for better performance.
    """
    
    def __init__(self, username: str):
        """Initialize with the username to filter for."""
        super().__init__(username)
        self.clutch_game_ref = None  # Store reference to clutch win with least time
        self.min_time_remaining = float('inf')  # Track minimum time remaining
    
    def start_game(self, game_data: GameData, user_is_white: bool, user_is_black: bool):
        """
        Initialize analyzer state for a new game.
        
        Args:
            game_data: The game data
            user_is_white: Whether user is playing white
            user_is_black: Whether user is playing black
        """
        super().start_game(game_data, user_is_white, user_is_black)
    
    def process_move(self, context: MoveContext):
        """Process a single move. No action needed - we check in finish_game()."""
        pass
    
    def finish_game(self) -> List[Dict[str, Any]]:
        """
        Finalize analysis for the game and return findings.
        Called after all moves have been processed.
        
        Checks if the user won and tracks the one with least time remaining.
        
        Returns:
            List of findings for this game (empty, we track globally)
        """
        # Skip daily/correspondence games
        if self._is_daily_game(self.game_data.metadata.time_control):
            return []
        
        # Check if user won
        result = self.game_data.metadata.result
        user_won = (result == "1-0" and self.user_is_white) or (result == "0-1" and self.user_is_black)
        if not user_won:
            return []
        
        # Check if game ended in checkmate (last move has #)
        moves = self.game_data.moves
        if not moves or '#' not in moves[-1]:
            return []  # Not a checkmate win
        
        # Get final clock time
        final_time = self._get_final_clock_time(self.game_data.pgn, self.user_is_white)
        if final_time is None:
            return []
        
        # Only include games with 0.5 seconds or less remaining
        if final_time > 0.5:
            return []
        
        # Track the clutchest win
        if final_time < self.min_time_remaining:
            self.min_time_remaining = final_time
            
            # Store lightweight reference (defer ELO extraction until get_final_results)
            self.clutch_game_ref = {
                "game_data": self.game_data,
                "final_time": final_time,
                "user_is_white": self.user_is_white
            }
        
        return []  # Return empty - we'll build full finding in get_final_results()
    
    def _parse_clock_time(self, clock_str: str) -> float:
        """Parse clock time string like '0:00:05.2' to seconds."""
        try:
            # Format: H:MM:SS.s or M:SS.s or M:SS
            parts = clock_str.split(':')
            if len(parts) == 3:
                hours, mins, secs = parts
                return int(hours) * 3600 + int(mins) * 60 + float(secs)
            elif len(parts) == 2:
                mins, secs = parts
                return int(mins) * 60 + float(secs)
            else:
                return float(clock_str)
        except:
            return float('inf')
    
    def _get_final_clock_time(self, pgn: str, user_is_white: bool) -> Optional[float]:
        """Extract the user's final clock time from PGN."""
        # Find all clock annotations: {[%clk H:MM:SS.s]}
        clock_pattern = r'\[%clk ([0-9:\.]+)\]'
        
        # Find all clock times in the PGN
        all_clocks = list(re.finditer(clock_pattern, pgn))
        
        if not all_clocks:
            return None
        
        # Get user's last clock time
        # In PGN, clocks alternate: white's clock after white's move, black's after black's
        # We need to find the last clock for the user
        user_clocks = []
        for i, match in enumerate(all_clocks):
            # Even index (0, 2, 4...) = white's clock, odd = black's clock
            is_white_clock = i % 2 == 0
            if (is_white_clock and user_is_white) or (not is_white_clock and not user_is_white):
                user_clocks.append(match.group(1))
        
        if not user_clocks:
            return None
        
        # Return the last (final) clock time
        return self._parse_clock_time(user_clocks[-1])
    
    def _is_daily_game(self, time_control: str) -> bool:
        """Check if time control indicates a daily/correspondence game."""
        if not time_control:
            return False
        # Daily games have format like "1/604800" (1 move per X seconds, where X is days)
        # Or very long time controls. Typical daily = 86400+ seconds per move
        if '/' in time_control:
            return True  # Daily format
        try:
            # If base time > 1 hour (3600 seconds), likely daily
            base_time = int(time_control.split('+')[0])
            return base_time > 3600
        except:
            return False
    
    def _format_time(self, seconds: float) -> str:
        """Format seconds into readable time."""
        if seconds < 60:
            return f"{seconds:.1f} seconds"
        else:
            mins = int(seconds // 60)
            secs = seconds % 60
            return f"{mins}:{secs:05.2f}"
    
    def get_game_points(self, config: dict) -> int:
        """Return points for current game based on existing findings."""
        # Clutch win doesn't contribute to best game scoring (it's a statistical find)
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
    
    def _get_fen_at_move(self, pgn: str, half_move_number: int) -> Optional[str]:
        """
        Get FEN at a specific half-move number.
        Replays the game to the specified move.
        
        Args:
            pgn: PGN string
            half_move_number: 0-indexed half-move number (0 = starting position, 1 = after first move, etc.)
        
        Returns:
            FEN string or None if error
        """
        try:
            import chess.pgn
            from io import StringIO
            
            pgn_io = StringIO(pgn)
            game = chess.pgn.read_game(pgn_io)
            if not game:
                return None
            
            board = game.board()
            move_count = 0
            
            for node in game.mainline():
                if move_count >= half_move_number:
                    break
                board.push(node.move)
                move_count += 1
            
            return board.fen()
        except Exception as e:
            return None
    
    def get_matched_game_links(self) -> List[str]:
        """Fast path: return just the game links that matched."""
        if self.clutch_game_ref and self.clutch_game_ref.get("game_data"):
            link = self.clutch_game_ref["game_data"].metadata.link
            return [link] if link else []
        return []

    def get_final_results(self) -> List[Dict[str, Any]]:
        """
        Get final results after processing all games.
        Returns the clutch win with the least time remaining.
        NOW extracts ELO only for the selected result.
        
        Returns:
            List containing the clutch win finding (or empty)
        """
        if not self.clutch_game_ref:
            return []
        
        ref = self.clutch_game_ref
        
        # Extract ELO (deferred until here)
        white_elo = self._extract_elo(ref["game_data"].pgn, "WhiteElo")
        black_elo = self._extract_elo(ref["game_data"].pgn, "BlackElo")
        
        # Get final move number (checkmate is the last move)
        # Convert to 0-indexed: last move is at len(moves) - 1
        final_move_number = len(ref["game_data"].moves) - 1 if ref["game_data"].moves else 0
        
        # Get context_moves_before to adjust key position
        # Frontend starts replay from key_position_index, so we move it back by context_moves_before
        context_moves_before = 5  # Default context moves before the key moment
        
        # Adjust key position to be context_moves_before moves before the final move
        # This allows frontend to start replay from this position and show context leading to clutch finish
        adjusted_key_half_move = max(0, final_move_number - (context_moves_before * 2))
        
        # Build replay data structure for frontend (uses config file)
        from ..replay_helper import build_replay_data
        replay_data = build_replay_data(
            game_data=ref["game_data"],
            key_half_move=adjusted_key_half_move,
            feature_name="clutch_win"  # Loads settings from replay_config.json
        )
        
        # Build full finding with extracted data
        finding = {
            "feature_name": "clutch_win",
            "display_name": "Clutch Win",
            "game_metadata": {
                "white": ref["game_data"].metadata.white,
                "black": ref["game_data"].metadata.black,
                "result": ref["game_data"].metadata.result,
                "date": ref["game_data"].metadata.date,
                "time_control": ref["game_data"].metadata.time_control,
                "link": ref["game_data"].metadata.link,
                "white_elo": white_elo,  # Extracted here, not during processing!
                "black_elo": black_elo,  # Extracted here, not during processing!
                "user_color": "white" if ref["user_is_white"] else "black",
                # New simplified format for frontend navigation
                "all_moves": replay_data["all_moves"],
                "key_position_index": replay_data["key_position_index"],
                "fen": replay_data["fen"],
                # Include full PGN - contains all clock annotations/timestamps
                "pgn": ref["game_data"].pgn
            },
            "position_link": f"{ref['game_data'].metadata.link}?move={final_move_number}" if ref["game_data"].metadata.link else None,  # Link still points to final move (clutch finish)
            "result_data": {
                "time_remaining_display": {
                    "value": self._format_time(ref["final_time"]),
                    "label": "Time Remaining"
                }
            }
        }
        
        return [finding]

