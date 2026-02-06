"""
Unified analyzer for finding knight promotion mates - checkmate delivered by promoting a pawn to a knight.
Uses the unified move-by-move approach for efficiency.
"""
import chess
import re
from typing import List, Dict, Any, Optional
from ..unified_analyzer import UnifiedAnalyzerBase, MoveContext
from ..game_data import GameData


class UnifiedKnightPromotionMateAnalyzer(UnifiedAnalyzerBase):
    """
    Unified version of KnightPromotionMateAnalyzer.
    Finds games where the user delivered checkmate with a knight underpromotion.
    Uses unified move-by-move processing for better performance.
    """
    
    def __init__(self, username: str):
        """Initialize with the username to filter for."""
        super().__init__(username)
        self.all_findings = []  # Store all findings across games (for get_final_results)
    
    def start_game(self, game_data: GameData, user_is_white: bool, user_is_black: bool):
        """
        Initialize analyzer state for a new game.
        
        Args:
            game_data: The game data
            user_is_white: Whether user is playing white
            user_is_black: Whether user is playing black
        """
        super().start_game(game_data, user_is_white, user_is_black)
        
        # Quick filter: must end in checkmate
        self.has_checkmate = '#' in self.game_data.pgn
        
        # Track state
        self.found_knight_promo_mate = False
    
    def process_move(self, context: MoveContext):
        """Process a single move. No action needed - we check in finish_game()."""
        pass
    
    def finish_game(self) -> List[Dict[str, Any]]:
        """
        Finalize analysis for the game and return findings.
        Called after all moves have been processed.
        
        Checks if the last move was a knight promotion checkmate.
        Matches original analyzer logic: check move string pattern.
        
        Returns:
            List of findings for this game
        """
        # Quick filters
        if not self.has_checkmate:
            return []
        
        # Check if user won
        result = self.game_data.metadata.result
        user_won = (result == "1-0" and self.user_is_white) or (result == "0-1" and self.user_is_black)
        if not user_won:
            return []
        
        # Check moves list (more reliable than last_move_context if PGN parsing fails)
        moves = self.game_data.moves
        if not moves:
            return []
        
        last_move = moves[-1]
        
        # Knight underpromotion checkmate: ends with =N# (promotion to knight with checkmate)
        # Examples: e8=N# (simple promotion), exd8=N# (capture promotion), h8=N#
        if '=N#' not in last_move:
            return []
        
        # Verify the last move was made by the user
        # If user is white, last move should be on odd move number (white moves)
        # If user is black, last move should be on even move number (black moves)
        # Move numbers: 1=white, 2=black, 3=white, 4=black, etc.
        last_move_number = len(moves)
        if self.user_is_white:
            # White moves on odd numbers (1, 3, 5, ...)
            if last_move_number % 2 == 0:
                return []  # Last move was black's, not user's
        else:  # user_is_black
            # Black moves on even numbers (2, 4, 6, ...)
            if last_move_number % 2 == 1:
                return []  # Last move was white's, not user's
        
        # Found a knight promotion mate!
        self.found_knight_promo_mate = True
        # Use half-move count directly (len(moves) is already half-move count)
        final_move_number = len(moves)
        
        # Store lightweight reference (defer FEN/ELO extraction until get_final_results)
        moves_back = 6
        target_half_move = max(0, final_move_number - moves_back)
        moves_to_mate = moves[target_half_move:] if target_half_move < len(moves) else moves
        
        # Check if it's a capture promotion
        is_capture = 'x' in last_move
        
        ref = {
            "game_data": self.game_data,
            "final_move_number": final_move_number,
            "target_half_move": target_half_move,
            "moves_to_mate": moves_to_mate,
            "last_move": last_move,
            "is_capture": is_capture,
            "user_is_white": self.user_is_white
        }
        
        self.all_findings.append(ref)
        
        return []  # Return empty - we'll build full finding in get_final_results()
    
    def get_game_points(self, config: dict) -> int:
        """Return points for current game based on existing findings."""
        if self.found_knight_promo_mate:
            return config.get("knight_promotion_mate", 50)
        return 0
    
    def _get_fen_at_move(self, pgn: str, half_move_number: int) -> Optional[str]:
        """
        Get FEN at a specific half-move number.
        
        Args:
            pgn: PGN string
            half_move_number: 0-indexed half-move number (0 = starting position, 1 = after first move, etc.)
        """
        try:
            import chess.pgn
            from io import StringIO
            
            pgn_io = StringIO(pgn)
            game = chess.pgn.read_game(pgn_io)
            if not game:
                return None
            
            board = game.board()
            
            # half_move_number is 0-indexed (0 = starting position, 1 = after first move, etc.)
            for i, node in enumerate(game.mainline()):
                if i >= half_move_number:
                    break
                board.push(node.move)
            
            return board.fen()
        except Exception:
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
    
    def get_matched_game_links(self) -> List[str]:
        """Fast path: return just the game links that matched."""
        return [ref["game_data"].metadata.link for ref in self.all_findings 
                if ref.get("game_data") and ref["game_data"].metadata.link]

    def get_final_results(self) -> List[Dict[str, Any]]:
        """
        Get final results after processing all games.
        Returns the most recent knight promotion mate found (sorted by move_number descending).
        NOW extracts FEN and ELO only for the selected result.

        Returns:
            List containing the most recent knight promotion mate finding
        """
        if not self.all_findings:
            return []

        # Sort by final_move_number (descending) to get the most recent one
        # (matching original analyzer behavior)
        self.all_findings.sort(key=lambda x: x["final_move_number"], reverse=True)

        # Get the most recent one
        best_ref = self.all_findings[0]
        
        # Extract ELO (deferred until here)
        white_elo = self._extract_elo(best_ref["game_data"].pgn, "WhiteElo")
        black_elo = self._extract_elo(best_ref["game_data"].pgn, "BlackElo")
        
        # Build replay data structure for frontend (uses config file)
        from ..replay_helper import build_replay_data
        replay_data = build_replay_data(
            game_data=best_ref["game_data"],
            key_half_move=best_ref["final_move_number"],
            feature_name="knight_promotion_mate"  # Loads settings from replay_config.json
        )
        
        # Build full finding with extracted data
        finding = {
            "feature_name": "knight_promotion_mate",
            "display_name": "Knight Promotion Mate",
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
            "position_link": f"{best_ref['game_data'].metadata.link}?move={best_ref['final_move_number']}" if best_ref["game_data"].metadata.link else None,
            "result_data": {
                "mate_move": {
                    "value": best_ref["last_move"],
                    "label": "Mate Move"
                }
            }
        }

        return [finding]


