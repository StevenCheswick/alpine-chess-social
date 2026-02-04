"""
Unified analyzer for finding the biggest comeback - winning by checkmate with the biggest material deficit.
Uses the unified move-by-move approach for efficiency.
"""
import chess
import re
from typing import List, Dict, Any, Optional, Tuple
from ..unified_analyzer import UnifiedAnalyzerBase, MoveContext
from ..game_data import GameData


class UnifiedBiggestComebackAnalyzer(UnifiedAnalyzerBase):
    """
    Unified version of BiggestComebackAnalyzer.
    Finds the game where the user won by checkmate with the biggest material deficit.
    Uses unified move-by-move processing for better performance.
    """
    
    PIECE_VALUES = {'q': 9, 'r': 5, 'b': 3, 'n': 3, 'p': 1}
    
    def __init__(self, username: str):
        """Initialize with the username to filter for."""
        super().__init__(username)
        self.biggest_deficit_ref = None  # Store reference to comeback with biggest deficit
        self.biggest_deficit = 0  # Track maximum deficit value
        self.current_game_deficit = 0  # Track deficit for current game (for best_game scoring)

    def start_game(self, game_data: GameData, user_is_white: bool, user_is_black: bool):
        """
        Initialize analyzer state for a new game.

        Args:
            game_data: The game data
            user_is_white: Whether user is playing white
            user_is_black: Whether user is playing black
        """
        super().start_game(game_data, user_is_white, user_is_black)
        self.current_game_deficit = 0  # Reset for each game
    
    def process_move(self, context: MoveContext):
        """Process a single move. No action needed - we check in finish_game()."""
        pass
    
    def finish_game(self) -> List[Dict[str, Any]]:
        """
        Finalize analysis for the game and return findings.
        Called after all moves have been processed.

        Checks if the user won by checkmate while down material.
        Awards comeback points for best_game scoring and tracks biggest comeback.

        Returns:
            List of findings for this game (empty, we track globally)
        """
        # Check if user won
        result = self.game_data.metadata.result
        if result not in ["1-0", "0-1"]:
            return []  # Not a decisive result

        # Determine winner
        winner_is_white = result == "1-0"
        user_won = (winner_is_white and self.user_is_white) or (not winner_is_white and self.user_is_black)
        if not user_won:
            return []  # User didn't win

        # Must be a checkmate win
        moves = self.game_data.moves
        if not moves or '#' not in moves[-1]:
            return []  # Not a checkmate win

        # Get final position FEN
        fen = self._extract_fen(self.game_data.pgn)
        if not fen:
            return []

        # Calculate material deficit
        white_material, black_material = self._material_from_fen(fen)

        if winner_is_white:
            deficit = black_material - white_material
        else:
            deficit = white_material - black_material

        # Store current game deficit for best_game scoring (all checkmate wins)
        self.current_game_deficit = max(0, deficit)

        # Only track if user was down material
        if deficit > self.biggest_deficit:
            self.biggest_deficit = deficit
            
            # Use half-move count directly (len(moves) is already half-move count)
            final_move_number = len(moves)
            moves_back = 6
            target_half_move = max(0, final_move_number - moves_back)
            moves_to_mate = moves[target_half_move:] if target_half_move < len(moves) else moves
            
            # Store lightweight reference (defer FEN/ELO extraction until get_final_results)
            self.biggest_deficit_ref = {
                "game_data": self.game_data,
                "final_move_number": final_move_number,
                "target_half_move": target_half_move,
                "moves_to_mate": moves_to_mate,
                "user_is_white": self.user_is_white,
                "deficit": deficit
            }
        
        return []  # Return empty - we'll build full finding in get_final_results()
    
    def _extract_fen(self, pgn: str) -> Optional[str]:
        """Extract FEN from PGN (CurrentPosition or FEN header)."""
        match = re.search(r'\[CurrentPosition\s+"([^"]+)"\]', pgn)
        if match:
            return match.group(1)
        match = re.search(r'\[FEN\s+"([^"]+)"\]', pgn)
        if match:
            return match.group(1)
        return None
    
    def _material_from_fen(self, fen: str) -> Tuple[int, int]:
        """Calculate material for white and black from FEN string."""
        piece_placement = fen.split()[0]
        
        white_material = 0
        black_material = 0
        
        for char in piece_placement:
            lower = char.lower()
            if lower in self.PIECE_VALUES:
                value = self.PIECE_VALUES[lower]
                if char.isupper():
                    white_material += value
                else:
                    black_material += value
        
        return white_material, black_material
    
    def get_game_points(self, config: dict) -> int:
        """Return points for current game based on material deficit.

        Uses tiered thresholds from config:
        - deficit >= 15: highest points (e.g., down queen + rook)
        - deficit >= 9: high points (e.g., down a queen)
        - deficit >= 5: medium points (e.g., down a rook)
        - deficit >= 3: low points (e.g., down a minor piece)
        """
        if self.current_game_deficit <= 0:
            return 0

        comeback_config = config.get("biggest_comeback", {})
        thresholds = comeback_config.get("thresholds", [])

        if not thresholds:
            return 0

        # Sort thresholds by deficit descending to find highest matching tier
        sorted_thresholds = sorted(thresholds, key=lambda x: x.get("deficit", 0), reverse=True)

        for threshold in sorted_thresholds:
            if self.current_game_deficit >= threshold.get("deficit", 0):
                return threshold.get("points", 0)

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
        Returns the comeback with the biggest material deficit.
        NOW extracts FEN and ELO only for the selected result.
        
        Returns:
            List containing the biggest comeback finding (or empty)
        """
        if not self.biggest_deficit_ref:
            return []
        
        ref = self.biggest_deficit_ref
        
        # Extract ELO (deferred until here)
        white_elo = self._extract_elo(ref["game_data"].pgn, "WhiteElo")
        black_elo = self._extract_elo(ref["game_data"].pgn, "BlackElo")
        
        # Build replay data structure for frontend (uses config file)
        from ..replay_helper import build_replay_data
        # Key position is 6 moves before the final move number
        key_half_move = max(0, ref["final_move_number"] - 6)
        replay_data = build_replay_data(
            game_data=ref["game_data"],
            key_half_move=key_half_move,
            feature_name="biggest_comeback"  # Loads settings from replay_config.json
        )
        
        
        # Build full finding with extracted data
        finding = {
            "feature_name": "biggest_comeback",
            "display_name": "Biggest Comeback",
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
                "material_deficit": {
                    "value": ref["deficit"],
                    "label": "Material Deficit"
                }
            }
        }
        
        return [finding]


