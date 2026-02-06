"""
Unified analyzer for finding king mates - checkmate delivered by a king move.
Uses the unified move-by-move approach for efficiency.
"""
import chess
from typing import List, Dict, Any, Optional
import re
from ..unified_analyzer import UnifiedAnalyzerBase, MoveContext
from ..game_data import GameData


class UnifiedKingMateAnalyzer(UnifiedAnalyzerBase):
    """
    Unified version of KingMateAnalyzer.
    Finds games where the user delivered checkmate with a king move.
    Uses unified move-by-move processing for better performance.
    """
    
    def __init__(self, username: str):
        """Initialize with the username to filter for."""
        super().__init__(username)
        self.all_findings = []  # Store all findings across games
    
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
        
        # Track last move info - store only what we need (not entire board)
        self.last_move = None
        self.last_move_number = None
        self.last_move_piece_type = None
        self.last_move_piece_color = None
        self.found_king_mate = False

    def process_move(self, context: MoveContext):
        """
        Process a single move.
        We track the last move to check if it's a king mate in finish_game().

        OPTIMIZATION: Store only what we need - piece type and color at move time.
        This avoids needing board.copy() which is expensive.
        """
        # Store the info we need NOW (before board changes)
        self.last_move = context.move
        self.last_move_number = context.move_number
        # Get piece info from the board BEFORE the move
        piece = context.board.piece_at(context.move.from_square)
        if piece:
            self.last_move_piece_type = piece.piece_type
            self.last_move_piece_color = piece.color
        else:
            self.last_move_piece_type = None
            self.last_move_piece_color = None
    
    def finish_game(self) -> List[Dict[str, Any]]:
        """
        Finalize analysis for the game and return findings.
        Called after all moves have been processed.
        
        Checks if the last move was a king delivering checkmate.
        
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
        
        # Check if we have a last move
        if not self.last_move:
            return []

        # Check if the move was made by a king (using stored piece info)
        if self.last_move_piece_type != chess.KING:
            return []

        # Verify it was the user's king
        if self.last_move_piece_color != self.user_color:
            return []
        
        # Now check if it's checkmate by looking at the moves list
        # The last move in the moves list should contain '#'
        moves = self.game_data.moves
        if not moves or '#' not in moves[-1]:
            return []
        
        # Get SAN from moves list (we already verified it contains '#')
        # Also verify it starts with 'K' for king move
        last_move_san = moves[-1]
        if not last_move_san.startswith('K'):
            return []

        # Found a king mate!
        self.found_king_mate = True
        final_move_number = self.last_move_number
        
        # Extract ELO ratings from PGN
        white_elo = self._extract_elo(self.game_data.pgn, "WhiteElo")
        black_elo = self._extract_elo(self.game_data.pgn, "BlackElo")
        
        # Build replay data structure for frontend (uses config file)
        from ..replay_helper import build_replay_data
        replay_data = build_replay_data(
            game_data=self.game_data,
            key_half_move=final_move_number,
            feature_name="king_mate"  # Loads settings from replay_config.json
        )
        
        finding = {
            "feature_name": "king_mate",
            "display_name": "King Mate",
            "move_number": final_move_number,  # For sorting in get_final_results
            "game_metadata": {
                "white": self.game_data.metadata.white,
                "black": self.game_data.metadata.black,
                "link": self.game_data.metadata.link,
                "white_elo": white_elo,
                "black_elo": black_elo,
                "user_color": "white" if self.user_is_white else "black",
                # New simplified format for frontend navigation
                "all_moves": replay_data["all_moves"],
                "key_position_index": replay_data["key_position_index"],
                "fen": replay_data["fen"],
                # Include full PGN - contains all clock annotations/timestamps
                "pgn": self.game_data.pgn
            },
            "position_link": f"{self.game_data.metadata.link}?move={final_move_number}" if self.game_data.metadata.link else None,
            "result_data": {
                "mate_move": {
                    "value": last_move_san,
                    "label": "Mate Move"
                }
            }
        }
        
        self.findings.append(finding)
        self.all_findings.append(finding)
        
        # Reset state
        self.last_move_context = None
        
        return self.findings
    
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
        Returns the earliest king mate found.

        Returns:
            List containing a single king mate finding (earliest one)
        """
        if not self.all_findings:
            return []

        # Sort by move_number (earliest first) and return the first one
        self.all_findings.sort(key=lambda x: x.get("move_number", float('inf')))
        best = self.all_findings[0].copy()

        # Add total count to result_data
        if "result_data" not in best:
            best["result_data"] = {}
        best["result_data"]["total_king_mates"] = {
            "value": len(self.all_findings),
            "label": "Total King Mates"
        }

        return [best]

