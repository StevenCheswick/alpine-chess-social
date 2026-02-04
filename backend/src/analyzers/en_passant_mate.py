"""
Unified analyzer for finding en passant mates - checkmate delivered via en passant capture.
Uses the unified move-by-move approach for efficiency.

En passant rule (from https://www.chess.com/terms/en-passant):
- The capturing pawn must have advanced exactly three ranks (white on 5th rank, black on 4th rank)
- The captured pawn must have moved two squares in one move, landing right next to the capturing pawn
- The en passant capture must be performed immediately on the next turn
- Notation: exd6# (captures pawn on d5, lands on d6 with mate)
"""
import chess
import re
from typing import List, Dict, Any, Optional
from ..unified_analyzer import UnifiedAnalyzerBase, MoveContext
from ..game_data import GameData


class UnifiedEnPassantMateAnalyzer(UnifiedAnalyzerBase):
    """
    Unified version of EnPassantMateAnalyzer.
    Finds games where the user delivered checkmate via en passant capture.
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
        self.found_en_passant_mate = False
    
    def process_move(self, context: MoveContext):
        """Process a single move. No action needed - we check in finish_game()."""
        pass
    
    def finish_game(self) -> List[Dict[str, Any]]:
        """
        Finalize analysis for the game and return findings.
        Called after all moves have been processed.
        
        Checks if the last move was an en passant checkmate.
        En passant mate pattern: [a-h]x[a-h][36]# (pawn capture to 3rd or 6th rank with mate)
        
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
        
        # Check moves list
        moves = self.game_data.moves
        if not moves:
            return []
        
        last_move = moves[-1]
        
        # En passant mate pattern:
        # - Pawn capture (file x file)
        # - Lands on 6th rank (white) or 3rd rank (black)
        # - Ends in mate
        # Pattern: [a-h]x[a-h][36]#
        ep_pattern = r'^([a-h])x([a-h])([36])#$'
        match = re.match(ep_pattern, last_move)
        
        if not match:
            return []
        
        from_file = match.group(1)
        to_file = match.group(2)
        to_rank = match.group(3)
        
        # En passant: capture must be to adjacent file
        if abs(ord(from_file) - ord(to_file)) != 1:
            return []
        
        # White en passant lands on rank 6, black on rank 3
        is_white_ep = to_rank == '6' and self.user_is_white
        is_black_ep = to_rank == '3' and self.user_is_black
        
        if not (is_white_ep or is_black_ep):
            return []
        
        # Verify the last move was made by the user
        # If user is white, last move should be on odd move number (white moves)
        # If user is black, last move should be on even move number (black moves)
        last_move_number = len(moves)
        if self.user_is_white:
            # White moves on odd numbers (1, 3, 5, ...)
            if last_move_number % 2 == 0:
                return []  # Last move was black's, not user's
        else:  # user_is_black
            # Black moves on even numbers (2, 4, 6, ...)
            if last_move_number % 2 == 1:
                return []  # Last move was white's, not user's
        
        # Verify it's actually an en passant capture using board state
        if not self._verify_is_en_passant(last_move):
            return []  # Pattern matched, but it's not a real en passant
        
        # Found an en passant mate!
        self.found_en_passant_mate = True
        # Use half-move count directly (len(moves) is already half-move count)
        final_move_number = len(moves)
        
        # Store lightweight reference (defer FEN/ELO extraction until get_final_results)
        moves_back = 6
        target_half_move = max(0, final_move_number - moves_back)
        moves_to_mate = moves[target_half_move:] if target_half_move < len(moves) else moves
        
        ref = {
            "game_data": self.game_data,
            "final_move_number": final_move_number,
            "target_half_move": target_half_move,
            "moves_to_mate": moves_to_mate,
            "last_move": last_move,
            "user_is_white": self.user_is_white
        }
        
        self.all_findings.append(ref)
        
        return []  # Return empty - we'll build full finding in get_final_results()
    
    def _verify_is_en_passant(self, last_move_str: str) -> bool:
        """
        Verify that the last move is actually an en passant capture.
        
        Args:
            last_move_str: SAN string of the last move (e.g., 'exd6#')
            
        Returns:
            True if it's a real en passant capture, False otherwise
        """
        try:
            # Reconstruct board position BEFORE the last move
            from io import StringIO
            import chess.pgn
            
            pgn_io = StringIO(self.game_data.pgn)
            game = chess.pgn.read_game(pgn_io)
            if not game:
                return False
            
            board = game.board()
            
            # Replay all moves EXCEPT the last one
            moves_list = list(game.mainline())
            for node in moves_list[:-1]:
                board.push(node.move)
            
            # Check if en passant square exists
            # (Only set when opponent pawn moved 2 squares forward last move)
            if board.ep_square is None:
                return False  # No en passant available - can't be en passant!
            
            # Parse the last move and verify it's en passant
            # Remove the # and + symbols for parsing
            last_move_san = last_move_str.rstrip('#+')
            last_move_obj = board.parse_san(last_move_san)
            
            # Use chess library's built-in check
            if not board.is_en_passant(last_move_obj):
                return False  # Not an en passant capture
            
            # All checks passed - it's a real en passant!
            return True
            
        except Exception:
            # If anything goes wrong, fail safe (don't claim it's en passant)
            return False
    
    def get_game_points(self, config: dict) -> int:
        """Return points for current game based on existing findings."""
        if self.found_en_passant_mate:
            return config.get("en_passant_mate", 55)
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
    
    def get_final_results(self) -> List[Dict[str, Any]]:
        """
        Get final results after processing all games.
        Returns the first en passant mate found.
        NOW extracts FEN and ELO only for the selected result.
        
        Returns:
            List containing a single en passant mate finding (or empty list)
        """
        if not self.all_findings:
            return []
        
        # Return only the first en passant mate found
        ref = self.all_findings[0]
        
        # Extract ELO (deferred until here)
        white_elo = self._extract_elo(ref["game_data"].pgn, "WhiteElo")
        black_elo = self._extract_elo(ref["game_data"].pgn, "BlackElo")
        
        # Build replay data structure for frontend (uses config file)
        from ..replay_helper import build_replay_data
        replay_data = build_replay_data(
            game_data=ref["game_data"],
            key_half_move=ref["final_move_number"],
            feature_name="en_passant_mate"  # Loads settings from replay_config.json
        )
        
        # Get total count of all en passant mates found
        total_en_passant_mates = len(self.all_findings)
        
        # Build full finding with extracted data
        finding = {
            "feature_name": "en_passant_mate",
            "display_name": "En Passant Mate",
            "game_metadata": {
                "white": ref["game_data"].metadata.white,
                "black": ref["game_data"].metadata.black,
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
            "position_link": f"{ref['game_data'].metadata.link}?move={ref['final_move_number']}" if ref["game_data"].metadata.link else None,
            "result_data": {
                "mate_move": {
                    "value": ref["last_move"],
                    "label": "Mate Move"
                },
                "total_en_passant_mates": {
                    "value": total_en_passant_mates,
                    "label": "Total En Passant Mates"
                }
            }
        }
        
        return [finding]


