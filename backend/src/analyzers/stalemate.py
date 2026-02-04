"""
Unified analyzer for finding stalemate games - games ending in stalemate with most material.
Uses the unified move-by-move approach for efficiency.
"""
import chess
import re
from typing import List, Dict, Any, Optional
from ..unified_analyzer import UnifiedAnalyzerBase, MoveContext
from ..game_data import GameData


class UnifiedStalemateAnalyzer(UnifiedAnalyzerBase):
    """
    Unified version of StalemateAnalyzer.
    Finds the stalemate game where the user was stalemated with the most material remaining.
    Uses unified move-by-move processing for better performance.
    """
    
    PIECE_VALUES = {'Q': 9, 'R': 5, 'B': 3, 'N': 3, 'P': 1, 
                    'q': 9, 'r': 5, 'b': 3, 'n': 3, 'p': 1}
    
    def __init__(self, username: str):
        """Initialize with the username to filter for."""
        super().__init__(username)
        self.max_material_stalemate_ref = None  # Store reference to stalemate with most material
        self.max_material = 0  # Track maximum material value
    
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
        
        Checks if the game ended in stalemate and tracks the one with most material.
        
        Returns:
            List of findings for this game (empty, we track globally)
        """
        # Check if game ended in stalemate (1/2-1/2 result)
        result = self.game_data.metadata.result
        if result != "1/2-1/2":
            return []  # Not a draw
        
        # Check termination reason - must be stalemate
        termination = self._get_termination(self.game_data.pgn)
        if not termination or 'stalemate' not in termination.lower():
            return []
        
        # Get final position FEN from CurrentPosition header
        fen = self._get_fen_from_pgn(self.game_data.pgn)
        if not fen:
            return []
        
        # Determine who was stalemated (side to move in FEN)
        fen_parts = fen.split()
        side_to_move = fen_parts[1] if len(fen_parts) > 1 else 'w'
        stalemated_is_white = side_to_move == 'w'
        
        # We want games where the USER was stalemated (escaped with stalemate)
        user_was_stalemated = (self.user_is_white and stalemated_is_white) or (self.user_is_black and not stalemated_is_white)
        if not user_was_stalemated:
            return []
        
        # Calculate total material on board
        total_material = self._calculate_material_from_fen(fen)
        
        # Track the stalemate with most material (opponent's material = user's escape)
        if total_material > 0 and total_material > self.max_material:
            self.max_material = total_material
            
            moves = self.game_data.moves
            # Use half-move count directly (len(moves) is already half-move count)
            final_move_number = len(moves) if moves else 0
            moves_back = 6
            target_half_move = max(0, final_move_number - moves_back)
            moves_to_stalemate = moves[target_half_move:] if moves and target_half_move < len(moves) else (moves or [])
            
            # Store lightweight reference (defer FEN/ELO extraction until get_final_results)
            self.max_material_stalemate_ref = {
                "game_data": self.game_data,
                "final_move_number": final_move_number,
                "target_half_move": target_half_move,
                "moves_to_stalemate": moves_to_stalemate,
                "user_is_white": self.user_is_white,
                "total_material": total_material
            }
        
        return []  # Return empty - we'll build full finding in get_final_results()
    
    def _calculate_material_from_fen(self, fen: str) -> int:
        """Calculate total material on board from FEN."""
        board_part = fen.split()[0] if ' ' in fen else fen
        total = 0
        for char in board_part:
            if char in self.PIECE_VALUES:
                total += self.PIECE_VALUES[char]
        return total
    
    def _get_fen_from_pgn(self, pgn: str) -> Optional[str]:
        """Extract final position FEN from PGN (CurrentPosition header)."""
        match = re.search(r'\[CurrentPosition\s+"([^"]+)"\]', pgn)
        return match.group(1) if match else None
    
    def _get_termination(self, pgn: str) -> Optional[str]:
        """Extract Termination header from PGN."""
        match = re.search(r'\[Termination\s+"([^"]+)"\]', pgn)
        if match:
            return match.group(1)
        return None
    
    def get_game_points(self, config: dict) -> int:
        """Return points for current game based on existing findings."""
        # Stalemate doesn't contribute to best game scoring (it's a draw, not an achievement)
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
        Returns the stalemate with the most material remaining.
        NOW extracts FEN and ELO only for the selected result.
        
        Returns:
            List containing the stalemate finding with most material (or empty)
        """
        if not self.max_material_stalemate_ref:
            return []
        
        ref = self.max_material_stalemate_ref
        
        # Extract ELO (deferred until here)
        white_elo = self._extract_elo(ref["game_data"].pgn, "WhiteElo")
        black_elo = self._extract_elo(ref["game_data"].pgn, "BlackElo")
        
        # Build replay data structure for frontend (uses config file)
        from ..replay_helper import build_replay_data
        replay_data = build_replay_data(
            game_data=ref["game_data"],
            key_half_move=ref["final_move_number"],
            feature_name="stalemate"  # Loads settings from replay_config.json
        )
        
        # Build full finding with extracted data
        finding = {
            "feature_name": "stalemate",
            "display_name": "Stalemate with Most Material",
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
                "total_material": {
                    "value": ref["total_material"],
                    "label": "Total Material"
                }
            }
        }
        
        return [finding]

