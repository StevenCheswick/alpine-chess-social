"""
Unified analyzer for finding knight and bishop mates - checkmate delivered
when only kings, one bishop, and one knight remain on the board.
Uses the unified move-by-move approach for efficiency.
"""
import chess
from typing import List, Dict, Any, Optional
from ..unified_analyzer import UnifiedAnalyzerBase, MoveContext
from ..game_data import GameData


class UnifiedKnightBishopMateAnalyzer(UnifiedAnalyzerBase):
    """
    Unified version of KnightBishopMateAnalyzer.
    Finds games where the user delivered checkmate in an endgame position
    where only kings, one bishop, and one knight remain on the board.
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
        self.found_knight_bishop_mate = False
        self.knight_bishop_mate_ref = None  # Lightweight reference (defer FEN/ELO extraction)
    
    def process_move(self, context: MoveContext):
        """Process a single move. No action needed - we check in finish_game()."""
        pass
    
    def _is_knight_bishop_endgame(self, board: chess.Board) -> bool:
        """
        Check if the position only contains kings, one bishop, and one knight.
        
        Args:
            board: The board in the final position
            
        Returns:
            True if only K, k, B, N remain
        """
        piece_count = {
            chess.PAWN: 0,
            chess.ROOK: 0,
            chess.KNIGHT: 0,
            chess.BISHOP: 0,
            chess.QUEEN: 0,
            chess.KING: 0
        }
        
        # Count all pieces on the board
        for square in chess.SQUARES:
            piece = board.piece_at(square)
            if piece:
                piece_count[piece.piece_type] += 1
        
        # Should have exactly 2 kings
        if piece_count[chess.KING] != 2:
            return False
        
        # Should have exactly 1 knight
        if piece_count[chess.KNIGHT] != 1:
            return False
        
        # Should have exactly 1 bishop
        if piece_count[chess.BISHOP] != 1:
            return False
        
        # Should have no pawns, rooks, or queens
        if piece_count[chess.PAWN] != 0 or piece_count[chess.ROOK] != 0 or piece_count[chess.QUEEN] != 0:
            return False
        
        return True
    
    def _extract_fen(self, pgn: str) -> Optional[str]:
        """Extract FEN from PGN headers if available."""
        import re
        match = re.search(r'\[CurrentPosition\s+"([^"]+)"\]', pgn)
        if match:
            return match.group(1)
        match = re.search(r'\[FEN\s+"([^"]+)"\]', pgn)
        if match:
            return match.group(1)
        return None
    
    def _get_final_fen(self, pgn: str, num_moves: int) -> Optional[str]:
        """
        Get final FEN by replaying all moves.
        
        Args:
            pgn: PGN string
            num_moves: Number of moves in the game
        """
        try:
            import chess.pgn
            from io import StringIO
            
            pgn_io = StringIO(pgn)
            game = chess.pgn.read_game(pgn_io)
            if not game:
                return None
            
            board = game.board()
            for node in game.mainline():
                board.push(node.move)
            
            return board.fen()
        except Exception:
            return None
    
    def finish_game(self) -> List[Dict[str, Any]]:
        """
        Finalize analysis for the game using final FEN.
        
        Checks if the final position is a knight and bishop mate endgame
        using only the final board position.
        
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
        
        # Get moves list for move number tracking
        moves = self.game_data.moves
        if not moves:
            return []
        
        # Get final FEN (try headers first, fallback to replaying moves)
        final_fen = self._extract_fen(self.game_data.pgn)
        if not final_fen:
            final_fen = self._get_final_fen(self.game_data.pgn, len(moves))
            if not final_fen:
                return []
        
        # Create board from final FEN
        try:
            final_board = chess.Board(final_fen)
        except Exception:
            return []
        
        # Verify it's actually checkmate
        if not final_board.is_checkmate():
            return []
        
        # Check if it's a knight and bishop endgame
        if not self._is_knight_bishop_endgame(final_board):
            return []
        
        # Found a knight and bishop mate!
        self.found_knight_bishop_mate = True
        final_move_number = len(moves)
        
        # Get last move
        last_move = moves[-1]
        
        # Store lightweight reference (defer FEN/ELO extraction until get_final_results)
        ref = {
            "game_data": self.game_data,
            "final_move_number": final_move_number,
            "last_move": last_move,
            "user_is_white": self.user_is_white
        }
        
        self.all_findings.append(ref)
        
        return []  # Return empty - we'll build full finding in get_final_results()
    
    def get_game_points(self, config: dict) -> int:
        """Return points for current game based on existing findings."""
        if self.found_knight_bishop_mate:
            return config.get("knight_bishop_mate", 50)
        return 0
    
    def _extract_elo(self, pgn: str, elo_header: str) -> Optional[int]:
        """Extract ELO rating from PGN header."""
        import re
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
        Returns the earliest knight and bishop mate found (sorted by move_number).
        Includes total count of all knight and bishop mates found.
        NOW extracts FEN and ELO only for the selected result.
        
        Returns:
            List containing the earliest knight and bishop mate finding
        """
        if not self.all_findings:
            return []
        
        # Sort by final_move_number (ascending) to get the earliest one
        self.all_findings.sort(key=lambda x: x["final_move_number"])
        
        # Get the earliest one
        best_ref = self.all_findings[0]
        
        # Get total count of all knight and bishop mates found
        total_knight_bishop_mates = len(self.all_findings)
        
        # Extract ELO (deferred until here)
        white_elo = self._extract_elo(best_ref["game_data"].pgn, "WhiteElo")
        black_elo = self._extract_elo(best_ref["game_data"].pgn, "BlackElo")
        
        # Build replay data structure for frontend (uses config file)
        from ..replay_helper import build_replay_data
        replay_data = build_replay_data(
            game_data=best_ref["game_data"],
            key_half_move=best_ref["final_move_number"],
            feature_name="knight_bishop_mate"  # Loads settings from replay_config.json
        )
        
        # Build full finding with extracted data
        finding = {
            "feature_name": "knight_bishop_mate",
            "display_name": "Knight and Bishop Mate",
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
                "total_knight_bishop_mates": {
                    "value": total_knight_bishop_mates,
                    "label": "Total Knight and Bishop Mates"
                }
            }
        }
        
        return [finding]


