"""
Unified analyzer for detecting windmill tactics.
Pattern: A true windmill requires a discovered check mechanism where:
- A rook alternates between giving direct checks (from a "home" square) and
  discovered checks (from capture squares) enabled by a stationary back piece (bishop/queen)
- The back piece stays on a diagonal, providing discovered check when the rook moves away
Uses the unified move-by-move approach for efficiency.
"""
import chess
import re
from typing import List, Dict, Any, Optional, Tuple
from collections import Counter
from ..unified_analyzer import UnifiedAnalyzerBase, MoveContext
from ..game_data import GameData


class UnifiedWindmillAnalyzer(UnifiedAnalyzerBase):
    """
    Unified version of WindmillAnalyzer.
    Detects true windmill tactics that require:
    - Discovered check mechanism (bishop/queen on diagonal giving check when rook moves)
    - Rook alternating between direct and discovered checks
    - Captures during the sequence (first move capture + discovered check captures)
    Uses unified move-by-move processing for better performance.
    """

    MIN_CAPTURES = 2           # Must have at least 2 captures in the sequence
    MIN_DISCOVERED_CHECKS = 1  # Must have at least 1 discovered check (proves back piece exists)
    
    def __init__(self, username: str):
        """Initialize with the username to filter for."""
        super().__init__(username)
        # Store lightweight references (defer FEN/ELO extraction)
        self.all_windmill_refs = []  # Store references across games for final selection
    
    def start_game(self, game_data: GameData, user_is_white: bool, user_is_black: bool):
        """
        Initialize analyzer state for a new game.
        
        Args:
            game_data: The game data
            user_is_white: Whether user is playing white
            user_is_black: Whether user is playing black
        """
        super().start_game(game_data, user_is_white, user_is_black)
        
        # Quick pre-filter: must have multiple rook checks (like original analyzer)
        # This skips games that can't possibly have a windmill
        rook_checks = re.findall(r'R[a-h]?[1-8]?x?[a-h][1-8]\+', game_data.pgn)
        self.skip_game = len(rook_checks) < 3
        
        # Windmill tracking state
        self.windmill_sequence = []  # List of moves in current potential windmill
        self.return_square = None  # Square the rook keeps returning to
        self.windmill_found = False  # Flag to stop processing after finding one
    
    def _is_rook_check(self, context: MoveContext) -> bool:
        """Check if the current move is a rook check."""
        if not context.is_user_move:
            return False
        
        moving_piece = context.board.piece_at(context.move.from_square)
        if not moving_piece or moving_piece.piece_type != chess.ROOK:
            return False
        
        # Use board.gives_check() instead of creating temp board - much faster!
        return context.board.gives_check(context.move)
    
    def _is_king_move(self, context: MoveContext) -> bool:
        """Check if the current move is a king move."""
        if not context.is_opponent_move:
            return False

        moving_piece = context.board.piece_at(context.move.from_square)
        return moving_piece and moving_piece.piece_type == chess.KING

    def _king_is_trapped(self, context: MoveContext) -> bool:
        """
        Check if the king has limited escape squares after a check.
        A true windmill traps the king to 1-2 legal moves max.

        Args:
            context: Move context after the rook check was played

        Returns:
            True if king has 1-2 legal moves (trapped), False if more options
        """
        # Make the move to get the resulting position
        temp_board = context.board.copy()
        temp_board.push(context.move)

        # Count legal moves for the side to move (opponent's king)
        legal_moves = list(temp_board.legal_moves)

        # In a true windmill, the king should have very few options (1-2 squares)
        return len(legal_moves) <= 2
    
    def _get_rook_destination_square(self, context: MoveContext) -> Optional[str]:
        """Get the destination square of a rook move as a string (e.g., 'e4')."""
        if not context.is_user_move:
            return None
        
        moving_piece = context.board.piece_at(context.move.from_square)
        if not moving_piece or moving_piece.piece_type != chess.ROOK:
            return None
        
        return chess.square_name(context.move.to_square)
    
    def _is_capture(self, context: MoveContext) -> bool:
        """Check if the current move is a capture."""
        return context.board.is_capture(context.move)

    def _is_discovered_check(self, context: MoveContext) -> bool:
        """
        Check if the rook move gives a discovered check.
        A discovered check means another piece (bishop/queen) is giving check
        because the rook moved out of the way. This can be a pure discovered check
        or a double check (rook + discovered piece both checking).

        Args:
            context: Move context with board state and move information

        Returns:
            True if there's a discovered check component, False if only direct check
        """
        # Make the move on a copy to check who's giving check
        temp_board = context.board.copy()
        temp_board.push(context.move)

        if not temp_board.is_check():
            return False

        # Get the opponent's king square
        opponent_color = not context.user_color
        king_square = temp_board.king(opponent_color)

        # Get all pieces attacking the king
        attackers = temp_board.attackers(context.user_color, king_square)

        # Check if there's any attacker OTHER than the rook that moved
        # This includes double checks (rook + bishop both attacking)
        rook_square = context.move.to_square
        other_attackers = [sq for sq in attackers if sq != rook_square]

        # If there's at least one other attacker, it's a discovered check
        return len(other_attackers) > 0

    def process_move(self, context: MoveContext):
        """
        Process a single move to detect windmill tactics.
        
        Args:
            context: Move context with board state and move information
        """
        # Skip if pre-filter determined this game can't have a windmill
        if self.skip_game or self.windmill_found:
            return
        
        # Check if this is a user's rook check
        if self._is_rook_check(context):
            dest_square = self._get_rook_destination_square(context)
            if not dest_square:
                return

            # CRITICAL: Check if king is trapped (1-2 legal moves only)
            # A true windmill forces the king to shuttle between limited squares
            if not self._king_is_trapped(context):
                # King has multiple escape routes - this check doesn't continue the windmill
                # But we should finalize any existing valid sequence first!
                if self.windmill_sequence:
                    self._check_and_record_windmill(context)
                    self.windmill_sequence = []
                    self.return_square = None
                return

            # Get move string from game_data.moves
            moves = self.game_data.moves
            move_index = context.move_number - 1
            move_str = moves[move_index] if move_index < len(moves) else context.move_san
            is_capture = self._is_capture(context)
            is_discovered = self._is_discovered_check(context)

            # If we have an active windmill sequence, add this rook check to it
            # The rook can check to different squares, we'll count returns at the end
            if self.windmill_sequence:
                # Add this rook check to the sequence
                self.windmill_sequence.append({
                    'half_move': context.move_number - 1,  # 0-indexed
                    'move': move_str,
                    'dest': dest_square,
                    'is_capture': is_capture,
                    'is_discovered': is_discovered,
                    'is_user_move': True
                })
            else:
                # Start a new potential windmill sequence
                # Store the first destination as the potential return square
                self.return_square = dest_square
                self.windmill_sequence = [{
                    'half_move': context.move_number - 1,
                    'move': move_str,
                    'dest': dest_square,
                    'is_capture': is_capture,
                    'is_discovered': is_discovered,
                    'is_user_move': True
                }]
            return
        
        # Check if this is an opponent's king move (continues windmill)
        if self._is_king_move(context) and self.windmill_sequence:
            moves = self.game_data.moves
            move_index = context.move_number - 1
            move_str = moves[move_index] if move_index < len(moves) else context.move_san
            is_capture = self._is_capture(context)
            
            # Add opponent's king move to sequence
            self.windmill_sequence.append({
                'half_move': context.move_number - 1,
                'move': move_str,
                'dest': None,  # King moves don't have a destination for rook checks
                'is_capture': is_capture,
                'is_user_move': False
            })
            return
        
        # If we have a sequence and this move doesn't continue it, check if it's valid
        if self.windmill_sequence and not self._is_king_move(context):
            # Sequence ended - check if it's a valid windmill
            self._check_and_record_windmill(context)
            # Reset for next potential windmill
            self.windmill_sequence = []
            self.return_square = None
    
    def _check_and_record_windmill(self, context: MoveContext):
        """
        Check if current sequence is a valid windmill and record it.

        A true windmill requires:
        - At least 2 captures in the sequence (including first move)
        - At least 1 discovered check (proves a back piece is enabling the windmill)
        """
        if len(self.windmill_sequence) < 3:
            return

        # Only count destinations from rook moves (user's moves), not king moves
        rook_moves = [s for s in self.windmill_sequence if s['dest'] is not None]
        if len(rook_moves) < 2:
            return

        # Count total captures in the sequence
        total_captures = sum(1 for s in rook_moves if s.get('is_capture', False))

        # Count discovered checks (proves back piece exists)
        discovered_checks = sum(1 for s in rook_moves if s.get('is_discovered', False))

        # Must have at least MIN_CAPTURES and MIN_DISCOVERED_CHECKS
        if total_captures < self.MIN_CAPTURES or discovered_checks < self.MIN_DISCOVERED_CHECKS:
            return

        # Valid windmill found!
        # Get the starting half-move (0-indexed from sequence, convert to 1-indexed)
        start_half_move_0_indexed = self.windmill_sequence[0]['half_move']
        start_half_move_1_indexed = start_half_move_0_indexed + 1  # Convert to 1-indexed half-move

        # Store lightweight reference (defer FEN/ELO extraction)
        self.all_windmill_refs.append({
            'game_data': self.game_data,
            'start_half_move': start_half_move_0_indexed,
            'start_half_move_1_indexed': start_half_move_1_indexed,  # Store 1-indexed for key_move_number
            'sequence': self.windmill_sequence.copy(),
            'captures': total_captures,  # Track total captures in sequence
            'user_is_white': self.user_is_white  # Store user color for THIS game
        })

        self.windmill_found = True
    
    def finish_game(self) -> List[Dict[str, Any]]:
        """
        Finalize analysis for the game and return findings.
        Called after all moves have been processed.

        Returns:
            List of findings for this game (empty for windmill - we track across games)
        """
        # Check if we have a windmill sequence that wasn't checked yet
        if self.windmill_sequence and len(self.windmill_sequence) >= 3:
            rook_moves = [s for s in self.windmill_sequence if s['dest'] is not None]
            if len(rook_moves) >= 2:
                # Count total captures and discovered checks
                total_captures = sum(1 for s in rook_moves if s.get('is_capture', False))
                discovered_checks = sum(1 for s in rook_moves if s.get('is_discovered', False))

                if total_captures >= self.MIN_CAPTURES and discovered_checks >= self.MIN_DISCOVERED_CHECKS:
                    start_half_move_0_indexed = self.windmill_sequence[0]['half_move']
                    start_half_move_1_indexed = start_half_move_0_indexed + 1

                    self.all_windmill_refs.append({
                        'game_data': self.game_data,
                        'start_half_move': start_half_move_0_indexed,
                        'start_half_move_1_indexed': start_half_move_1_indexed,
                        'sequence': self.windmill_sequence.copy(),
                        'captures': total_captures,
                        'user_is_white': self.user_is_white
                    })

        # Reset state for next game
        self.windmill_sequence = []
        self.return_square = None
        self.windmill_found = False

        return []  # Windmill findings are tracked across games
    
    def get_game_points(self, config: dict) -> int:
        """Return points for current game based on existing findings."""
        if self.windmill_found:
            return config.get("windmill", 25)
        return 0
    
    def _get_fen_at_move(self, game_data: GameData, half_move_number: int) -> Optional[str]:
        """
        Get FEN at a specific half-move number.
        
        Args:
            game_data: The game data
            half_move_number: 0-indexed half-move number (0 = starting position, 1 = after first move, etc.)
        
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
            
            # half_move_number is 0-indexed (0 = starting position, 1 = after first move, etc.)
            for i, node in enumerate(game.mainline()):
                if i >= half_move_number:
                    break
                board.push(node.move)
            
            return board.fen()
        except Exception:
            return None
    
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
    
    def get_final_results(self) -> List[Dict[str, Any]]:
        """
        Get final results after all games have been processed.
        Returns the windmill with the most captures.
        
        Returns:
            List containing the best windmill finding
        """
        if not self.all_windmill_refs:
            return []
        
        # Sort by number of captures (descending) - most captures first
        self.all_windmill_refs.sort(key=lambda x: x['captures'], reverse=True)
        
        # Get the best windmill
        best_ref = self.all_windmill_refs[0]
        game_data = best_ref['game_data']
        
        # Extract ELO
        white_elo, black_elo = self._extract_elo(game_data)
        
        # Build replay data structure for frontend (uses config file)
        from ..replay_helper import build_replay_data
        replay_data = build_replay_data(
            game_data=game_data,
            key_half_move=best_ref['start_half_move'],
            feature_name="windmill"  # Loads settings from replay_config.json
        )
        
        # Build position link
        base_link = game_data.metadata.link
        position_link = f"{base_link}?move={best_ref['start_half_move_1_indexed']}" if base_link else None
        
        finding = {
            "feature_name": "windmill",
            "display_name": "Windmill Tactic",
            "game_metadata": {
                "white": game_data.metadata.white,
                "black": game_data.metadata.black,
                "link": game_data.metadata.link,
                "white_elo": white_elo,
                "black_elo": black_elo,
                "user_color": "white" if best_ref['user_is_white'] else "black",
                # New simplified format for frontend navigation
                "all_moves": replay_data["all_moves"],
                "key_position_index": replay_data["key_position_index"],
                "fen": replay_data["fen"],
                # Include full PGN - contains all clock annotations/timestamps
                "pgn": game_data.pgn
            },
            "position_link": position_link,
            "result_data": {
                "captures": {
                    "value": best_ref['captures'],
                    "label": "Captures"
                },
                "total_windmills": {
                    "value": len(self.all_windmill_refs),
                    "label": "Total Windmills"
                }
            }
        }
        
        return [finding]

