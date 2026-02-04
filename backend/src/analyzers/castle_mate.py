"""
Unified analyzer for detecting checkmate delivered by castling.
Uses the unified move-by-move approach for efficiency.
"""
import chess
from typing import List, Dict, Any, Optional, Tuple
from ..unified_analyzer import UnifiedAnalyzerBase, MoveContext
from ..game_data import GameData


class UnifiedCastleMateAnalyzer(UnifiedAnalyzerBase):
    """
    Unified version of CastleMateAnalyzer.
    Detects checkmate delivered by castling (O-O# or O-O-O#).
    Uses unified move-by-move processing for better performance.
    """
    
    def __init__(self, username: str):
        """Initialize with the username to filter for."""
        super().__init__(username)
        # Track all castle mates across games
        self.all_castle_mates = []
    
    def start_game(self, game_data: GameData, user_is_white: bool, user_is_black: bool):
        """
        Initialize analyzer state for a new game.
        
        Args:
            game_data: The game data
            user_is_white: Whether user is playing white
            user_is_black: Whether user is playing black
        """
        super().start_game(game_data, user_is_white, user_is_black)
        
        # Pre-filter: must end in checkmate and user must win
        # Quick check: if result is not a decisive win, skip early
        self.user_won = (self.game_data.metadata.result == "1-0" and user_is_white) or \
                       (self.game_data.metadata.result == "0-1" and user_is_black)
        self.found_castle_mate = False
        
        # Use Termination header to check if game ended in checkmate (much faster than searching PGN!)
        # Termination format: "PlayerName won by checkmate" or "PlayerName won by resignation" etc.
        # Fallback: if no Termination header (e.g., test games), check last move for #
        if self.user_won:
            termination = self._get_termination(self.game_data.pgn)
            if termination:
                self.has_checkmate = 'checkmate' in termination.lower()
            else:
                # Fallback for games without Termination header (e.g., test games)
                # Check if last move ends with # or if # is in the last part of PGN
                if self.game_data.moves:
                    last_move = self.game_data.moves[-1]
                    self.has_checkmate = last_move.endswith('#') or '#' in self.game_data.pgn[-50:]
                else:
                    self.has_checkmate = False
        else:
            self.has_checkmate = False
    
    def process_move(self, context: MoveContext):
        """
        Process a single move to detect castle mates.
        
        Args:
            context: Move context with board state and move information
        """
        # We only need to check the last move, so we'll do that in finish_game
        pass
    
    def finish_game(self) -> List[Dict[str, Any]]:
        """
        Finalize analysis for the game and return findings.
        Called after all moves have been processed.
        
        Returns:
            List of findings for this game
        """
        if not self.has_checkmate or not self.user_won:
            return []
        
        if not self.game_data.moves:
            return []
        
        last_move = self.game_data.moves[-1]
        
        # Check for castling mate
        # We already know it's checkmate (from has_checkmate check above)
        # Just verify the last move is castling by checking the END of PGN
        # (checkmate can only be on the last move, so we only need to check the end)
        castle_type = None
        pgn_end = self.game_data.pgn[-50:]  # Last 50 chars should contain final move
        
        if last_move == 'O-O' or last_move == 'O-O+':
            # Check if PGN ends with O-O# (only check end, not entire PGN)
            if 'O-O#' in pgn_end:
                castle_type = "short"
        elif last_move == 'O-O-O' or last_move == 'O-O-O+':
            # Check if PGN ends with O-O-O# (only check end, not entire PGN)
            if 'O-O-O#' in pgn_end:
                castle_type = "long"
        
        if castle_type:
            # Store lightweight reference (defer FEN/ELO extraction)
            self.all_castle_mates.append({
                'game_data': self.game_data,
                'castle_type': castle_type,
                'final_move': last_move,
                'user_is_white': self.user_is_white,
                'move_number': len(self.game_data.moves)  # Use half-move count directly
            })
            self.found_castle_mate = True
            return [{
                'castle_type': castle_type,
                'final_move': last_move,
                'move_number': len(self.game_data.moves)  # Use half-move count directly
            }]
        
        self.found_castle_mate = False
        return []
    
    def _extract_elo(self, game_data: GameData) -> Tuple[Optional[int], Optional[int]]:
        """Extract ELO ratings from game data."""
        import re
        pgn = game_data.pgn
        white_elo = None
        black_elo = None
        
        white_match = re.search(r'\[WhiteElo\s+"(\d+)"\]', pgn)
        if white_match:
            try:
                white_elo = int(white_match.group(1))
            except ValueError:
                pass
        
        black_match = re.search(r'\[BlackElo\s+"(\d+)"\]', pgn)
        if black_match:
            try:
                black_elo = int(black_match.group(1))
            except ValueError:
                pass
        
        return white_elo, black_elo
    
    def _get_termination(self, pgn: str) -> Optional[str]:
        """Extract Termination header from PGN."""
        import re
        match = re.search(r'\[Termination\s+"([^"]+)"\]', pgn)
        if match:
            return match.group(1)
        return None
    
    def _get_fen_at_move(self, game_data: GameData, move_number: int) -> Optional[str]:
        """
        Get FEN at a specific move number.
        
        Args:
            game_data: The game data
            move_number: Full move number (1-indexed)
        
        Returns:
            FEN string or None if extraction fails
        """
        try:
            from io import StringIO
            pgn_io = StringIO(game_data.pgn)
            game = chess.pgn.read_game(pgn_io)
            if not game:
                return None
            
            board = game.board()
            
            # Convert full move number to half-move count
            # move_number is full move (1 = after white's first move)
            # We want FEN after the last move, so we need to push all moves
            for node in game.mainline():
                board.push(node.move)
            
            return board.fen()
        except Exception:
            return None
    
    def get_final_results(self) -> List[Dict[str, Any]]:
        """
        Get final results after all games have been processed.
        Returns the first castle mate found.
        
        Returns:
            List containing a single castle mate finding (or empty list)
        """
        if not self.all_castle_mates:
            return []
        
        # Return only the first castle mate found
        ref = self.all_castle_mates[0]
        game_data = ref['game_data']
        
        # Extract ELO (deferred heavy calculations)
        white_elo, black_elo = self._extract_elo(game_data)
        
        # Build position link
        base_link = game_data.metadata.link
        half_move = len(game_data.moves)  # Last move
        position_link = f"{base_link}?move={half_move}" if base_link else None
        
        # Build replay data structure for frontend (uses config file)
        from ..replay_helper import build_replay_data
        replay_data = build_replay_data(
            game_data=game_data,
            key_half_move=half_move,
            feature_name="castle_mate"  # Loads settings from replay_config.json
        )
        
        # Get total count of all castle mates found
        total_castle_mates = len(self.all_castle_mates)
        
        finding = {
            "feature_name": "castle_mate",
            "display_name": "Castle Mate",
            "game_metadata": {
                "white": game_data.metadata.white,
                "black": game_data.metadata.black,
                "result": game_data.metadata.result,
                "date": game_data.metadata.date,
                "link": game_data.metadata.link,
                "white_elo": white_elo,
                "black_elo": black_elo,
                "user_color": "white" if ref["user_is_white"] else "black",
                # New simplified format for frontend navigation
                "all_moves": replay_data["all_moves"],
                "key_position_index": replay_data["key_position_index"],
                "fen": replay_data["fen"],
                # Include full PGN - contains all clock annotations/timestamps
                "pgn": game_data.pgn
            },
            "position_link": position_link,
            "result_data": {
                "castle_type": {
                    "value": ref['castle_type'],
                    "label": "Castle Type"
                },
                "final_move": {
                    "value": ref['final_move'],
                    "label": "Final Move"
                },
                "total_castle_mates": {
                    "value": total_castle_mates,
                    "label": "Total Castle Mates"
                }
            }
        }
        
        return [finding]
    
    def get_game_points(self, config: dict) -> int:
        """Return points for current game based on existing findings."""
        if self.found_castle_mate:
            return config.get("castle_mate", 50)
        return 0

