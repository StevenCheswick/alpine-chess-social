"""
Unified analyzer for finding the longest sequence of consecutive captures in a game.
Uses the unified move-by-move approach for efficiency.
"""
import chess
from typing import List, Dict, Any, Optional
import re
from ..unified_analyzer import UnifiedAnalyzerBase, MoveContext
from ..game_data import GameData


class UnifiedCaptureSequenceAnalyzer(UnifiedAnalyzerBase):
    """
    Unified version of CaptureSequenceAnalyzer.
    Finds the longest sequence of consecutive captures in games.
    Uses unified move-by-move processing for better performance.
    """
    
    def __init__(self, username: str):
        """Initialize with the username to filter for."""
        super().__init__(username)
        # Store lightweight references (defer FEN/ELO extraction)
        self.all_sequence_refs = []  # Store references across games for final selection
        self.longest_sequence = 0
    
    def start_game(self, game_data: GameData, user_is_white: bool, user_is_black: bool):
        """
        Initialize analyzer state for a new game.
        
        Args:
            game_data: The game data
            user_is_white: Whether user is playing white
            user_is_black: Whether user is playing black
        """
        super().start_game(game_data, user_is_white, user_is_black)
        
        # Track sequence length for this game (will be computed in finish_game)
        self.sequence_length_this_game = 0
        self.sequence_start_move_number = None  # 1-indexed half-move
    
    def process_move(self, context: MoveContext):
        """
        OPTIMIZED: This method is now a no-op.
        All analysis is done in finish_game() using game_data.moves directly
        for massive performance improvement (~85x faster).
        """
        # No-op - all work done in finish_game() using string-based analysis
        pass
    
    def finish_game(self) -> List[Dict[str, Any]]:
        """
        OPTIMIZED: Analyze capture sequence using game_data.moves directly.
        String-based analysis is ~85x faster than board operations.
        
        Returns:
            List of findings for this game (empty - we track across all games)
        """
        # Quick filter: must have captures
        if 'x' not in self.game_data.pgn:
            self.sequence_length_this_game = 0
            return []
        
        # Find longest consecutive capture sequence using string-based analysis
        max_length = 0
        max_start = None  # 1-indexed half-move
        current_length = 0
        current_start = None
        
        for i, move in enumerate(self.game_data.moves, start=1):  # start=1 for 1-indexed
            if 'x' in move:
                # This is a capture
                if current_length == 0:
                    current_start = i
                current_length += 1
            else:
                # Not a capture - check if we found a new longest sequence
                if current_length > max_length:
                    max_length = current_length
                    max_start = current_start
                current_length = 0
                current_start = None
        
        # Check final sequence (in case game ends with captures)
        if current_length > max_length:
            max_length = current_length
            max_start = current_start
        
        # Store results
        self.sequence_length_this_game = max_length
        
        if max_length > 0 and max_start is not None:
            # Extract the list of capture moves in order
            capture_moves = self.game_data.moves[max_start - 1:max_start - 1 + max_length]
            
            # Store lightweight reference (defer FEN/ELO extraction until get_final_results)
            ref = {
                "game_data": self.game_data,
                "start_move": max_start,  # 1-indexed half-move
                "sequence_length": max_length,
                "capture_moves": capture_moves,
                "user_is_white": self.user_is_white,
            }
            
            self.all_sequence_refs.append(ref)
            
            # Track longest overall
            if max_length > self.longest_sequence:
                self.longest_sequence = max_length
        
        return []  # Return empty - we track across all games
    
    def get_game_points(self, config: dict) -> int:
        """Return points for current game based on existing findings."""
        if self.sequence_length_this_game == 0:
            return 0
        
        seq_config = config.get("capture_sequence", {})
        if isinstance(seq_config, dict):
            base_points = seq_config.get("base_points", 10)
            per_capture = seq_config.get("per_capture", 2)
            return base_points + (self.sequence_length_this_game * per_capture)
        else:
            # Fallback if config is just a number
            return seq_config if isinstance(seq_config, int) else 10
    
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
        Selects the longest capture sequence based on priority.
        NOW extracts FEN and ELO only for the longest sequence(s).
        
        Returns:
            List containing the longest capture sequence (or empty list if none found)
        """
        if not self.all_sequence_refs:
            return []
        
        # Sort by sequence length (descending), then by move_number (descending) for ties
        # This ensures if multiple games have the same length, we prefer the one starting latest
        self.all_sequence_refs.sort(key=lambda x: (x["sequence_length"], x["start_move"]), reverse=True)
        
        # Find the longest length
        longest_length = self.all_sequence_refs[0]["sequence_length"]
        
        # Filter to only keep games with the longest length
        longest_refs = [r for r in self.all_sequence_refs if r["sequence_length"] == longest_length]
        
        # If there are ties, only keep the one with the latest start_move (already first after sorting)
        best_ref = longest_refs[0]
        
        # Extract ELO ratings from PGN
        white_elo = self._extract_elo(best_ref["game_data"].pgn, "WhiteElo")
        black_elo = self._extract_elo(best_ref["game_data"].pgn, "BlackElo")
        
        # start_move is 1-indexed half-move, convert to 0-indexed for replay helper
        start_half_move_0_indexed = best_ref["start_move"] - 1
        
        # Calculate end half-move: last capture in the sequence
        # start_move is 1-indexed half-move of first capture
        # sequence_length is number of consecutive captures
        # Captures are at: start_move, start_move+1, ..., start_move+sequence_length-1 (1-indexed)
        # In 0-indexed: start_move-1, start_move, ..., start_move+sequence_length-2
        # Last capture is at: start_move + sequence_length - 2 (0-indexed)
        # Replay helper adds +1 to include it, so we pass the 0-indexed position of last capture
        end_half_move_0_indexed = best_ref["start_move"] + best_ref["sequence_length"] - 2
        
        # Build replay data structure for frontend
        from ..replay_helper import build_replay_data
        replay_data = build_replay_data(
            game_data=best_ref["game_data"],
            key_half_move=start_half_move_0_indexed,
            feature_name="capture_sequence"
        )
        
        # Build full finding with extracted data
        finding = {
            "feature_name": "capture_sequence",
            "display_name": "Longest Capture Sequence",
            "game_metadata": {
                "white": best_ref["game_data"].metadata.white,
                "black": best_ref["game_data"].metadata.black,
                "link": best_ref["game_data"].metadata.link,
                "white_elo": white_elo,  # Extracted here, not during processing!
                "black_elo": black_elo,  # Extracted here, not during processing!
                "user_color": "white" if best_ref["user_is_white"] else "black",
                # New simplified format for frontend navigation
                "all_moves": replay_data["all_moves"],
                "key_position_index": replay_data["key_position_index"],
                "fen": replay_data["fen"],
                # Include full PGN - contains all clock annotations/timestamps
                "pgn": best_ref["game_data"].pgn
            },
            "position_link": f"{best_ref['game_data'].metadata.link}?move={(best_ref['start_move'] + 1) // 2}" if best_ref["game_data"].metadata.link else None,
            "result_data": {
                "length": {
                    "value": best_ref["sequence_length"],
                    "label": "Consecutive Captures"
                }
            }
        }
        
        return [finding]
    
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
        except Exception:
            return None

