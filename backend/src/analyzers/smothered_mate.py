"""
Unified analyzer for detecting smothered mates - knight checkmate where king is
surrounded by its own pieces.
Uses the unified move-by-move approach for efficiency.
"""
import chess
from typing import List, Dict, Any, Optional
import re
from ..unified_analyzer import UnifiedAnalyzerBase, MoveContext
from ..game_data import GameData


class UnifiedSmotheredMateAnalyzer(UnifiedAnalyzerBase):
    """
    Unified version of SmotheredMateAnalyzer.
    Finds games where the user delivered smothered mate (knight checkmate with king surrounded).
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
        
        # OPTIMIZATION: Pre-filter to only games that end in knight checkmate
        # This filters out the vast majority of games (only ~0.1% of games end in knight mate)
        moves = self.game_data.moves
        self.has_knight_checkmate = False
        if moves:
            last_move_str = moves[-1]
            # Must be knight move with checkmate
            if last_move_str.startswith('N') and '#' in last_move_str:
                # Also verify user won (knight mate must be by user, not opponent)
                result = self.game_data.metadata.result
                user_won = (result == "1-0" and user_is_white) or (result == "0-1" and user_is_black)
                if user_won:
                    self.has_knight_checkmate = True
        
        # Track last move info (only needed if has_knight_checkmate)
        self.last_move_context = None
        self.last_move_piece = None
        self.found_smothered_mate = False
    
    def process_move(self, context: MoveContext):
        """
        Process a single move.
        We track the last move to check if it's a smothered mate in finish_game().
        """
        # OPTIMIZATION: Skip tracking if game doesn't end in knight checkmate by user
        # This filters out >99% of games, making the analyzer extremely fast
        if not self.has_knight_checkmate:
            return
        
        # Store last move context for checking in finish_game()
        # IMPORTANT: Store a copy of the board state because UnifiedAnalyzer
        # will push the move after this, modifying the shared board
        self.last_move_context = context
        # Store the piece that will make the move (before move is pushed)
        # This avoids the issue where the board is modified after process_move
        if context.move:
            self.last_move_piece = context.board.piece_at(context.move.from_square)
        else:
            self.last_move_piece = None
    
    def finish_game(self) -> List[Dict[str, Any]]:
        """
        Finalize analysis for the game and return findings.
        Called after all moves have been processed.
        
        Checks if the last move was a knight delivering smothered mate.
        
        Returns:
            List of findings for this game
        """
        # Quick filter: must have knight checkmate (pre-filtered in start_game)
        if not self.has_knight_checkmate:
            return []
        
        # Check if we have a last move (should always have one if has_knight_checkmate)
        if not self.last_move_context:
            return []
        
        # Get last move string (already verified in start_game, but need it for result)
        moves = self.game_data.moves
        if not moves:
            return []
        last_move_str = moves[-1]
        
        # Verify the move was actually made by a knight
        move = self.last_move_context.move
        if not move:
            return []
        
        # Use the piece we stored in process_move (before board was modified)
        # The board in context is stale because UnifiedAnalyzer pushed the move
        moving_piece = self.last_move_piece
        if not moving_piece or moving_piece.piece_type != chess.KNIGHT:
            return []
        
        # Verify it was the user's knight
        if moving_piece.color != self.user_color:
            return []
        
        # Try to extract final FEN from PGN headers first (fast path)
        final_fen = self._extract_fen(self.game_data.pgn)
        
        # If no FEN in headers, reconstruct by replaying moves (fallback, more expensive)
        if not final_fen:
            moves = self.game_data.moves
            if not moves:
                return []
            # Get FEN after all moves (half_move_number is 0-indexed, so len(moves) = after all moves)
            final_fen = self._get_fen_at_move(self.game_data.pgn, len(moves))
            if not final_fen:
                return []
        
        # Create board from final FEN to check smothered condition
        final_board = chess.Board(final_fen)
        
        # Verify it's actually checkmate
        if not final_board.is_checkmate():
            return []
        
        # Check if king is smothered in final position
        # Loser's king color
        loser_is_white = not self.user_is_white
        
        if not self._is_smothered(final_board, loser_is_white):
            return []
        
        # Found a smothered mate!
        self.found_smothered_mate = True
        # Use half-move number directly
        final_move_number = self.last_move_context.move_number
        # Key position is 3 moves before the mate
        key_half_move = max(0, final_move_number - 3)
        
        # Build replay data structure for frontend (uses config file)
        from ..replay_helper import build_replay_data
        replay_data = build_replay_data(
            game_data=self.game_data,
            key_half_move=key_half_move,
            feature_name="smothered_mate"  # Loads settings from replay_config.json
        )
        
        # Extract ELO ratings from PGN
        white_elo = self._extract_elo(self.game_data.pgn, "WhiteElo")
        black_elo = self._extract_elo(self.game_data.pgn, "BlackElo")
        
        finding = {
            "feature_name": "smothered_mate",
            "display_name": "Smothered Mate",
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
                    "value": last_move_str,
                    "label": "Mate Move"
                }
            }
        }
        
        self.findings.append(finding)
        self.all_findings.append(finding)
        
        # Reset state
        self.last_move_context = None
        self.last_move_piece = None
        
        return self.findings
    
    def get_game_points(self, config: dict) -> int:
        """Return points for current game based on existing findings."""
        if self.found_smothered_mate:
            return config.get("smothered_mate", 50)
        return 0
    
    def _is_smothered(self, board: chess.Board, king_is_white: bool) -> bool:
        """
        Check if king is smothered (all adjacent squares blocked by own pieces).
        
        Args:
            board: Chess board in final position (after checkmate)
            king_is_white: Whether the losing king is white
        
        Returns:
            True if king is smothered
        """
        # Find the king
        king_square = board.king(king_is_white)
        if king_square is None:
            return False
        
        # Get all 8 adjacent squares
        directions = [
            (-1, -1), (-1, 0), (-1, 1),
            (0, -1),          (0, 1),
            (1, -1),  (1, 0), (1, 1)
        ]
        
        rank = chess.square_rank(king_square)
        file = chess.square_file(king_square)
        
        blocked_count = 0
        valid_squares = 0
        
        for dr, df in directions:
            new_rank = rank + dr
            new_file = file + df
            
            # Check bounds
            if 0 <= new_rank < 8 and 0 <= new_file < 8:
                valid_squares += 1
                square = chess.square(new_file, new_rank)
                piece = board.piece_at(square)
                
                # Check if square is blocked by own piece
                if piece:
                    # Own pieces: same color as king
                    if piece.color == king_is_white:
                        blocked_count += 1
        
        # All valid adjacent squares must be blocked by own pieces
        return valid_squares > 0 and blocked_count == valid_squares
    
    def _extract_fen(self, pgn: str) -> Optional[str]:
        """Extract FEN from PGN."""
        match = re.search(r'\[CurrentPosition\s+"([^"]+)"\]', pgn)
        if match:
            return match.group(1)
        match = re.search(r'\[FEN\s+"([^"]+)"\]', pgn)
        if match:
            return match.group(1)
        return None
    
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
        Returns the earliest smothered mate found, with total count.
        
        Returns:
            List containing a single smothered mate finding (earliest one) with total count
        """
        if not self.all_findings:
            return []
        
        # Sort by move_number (earliest first) and return the first one
        self.all_findings.sort(key=lambda x: x.get("move_number", float('inf')))
        earliest_finding = self.all_findings[0]
        
        # Add total count to result_data
        total_count = len(self.all_findings)
        if "result_data" in earliest_finding:
            earliest_finding["result_data"]["total_smothered_mates"] = {
                "value": total_count,
                "label": "Total Smothered Mates"
            }
        else:
            earliest_finding["result_data"] = {
                "total_smothered_mates": {
                    "value": total_count,
                    "label": "Total Smothered Mates"
                }
            }
        
        return [earliest_finding]

