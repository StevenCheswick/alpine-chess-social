"""
Unified analyzer for finding hung queens - where the user's queen is captured
by a non-queen piece without capturing the opponent's queen, and the opponent's queen is still on the board.
Uses the unified move-by-move approach for efficiency.
"""
import chess
from typing import List, Dict, Any, Optional
import re
from ..unified_analyzer import UnifiedAnalyzerBase, MoveContext
from ..game_data import GameData


class UnifiedHungQueenAnalyzer(UnifiedAnalyzerBase):
    """
    Unified version of HungQueenAnalyzer.
    Finds games where the user hung their queen (lost it without capturing opponent's queen).
    Uses unified move-by-move processing for better performance.
    """
    
    def __init__(self, username: str):
        """Initialize with the username to filter for."""
        super().__init__(username)
        # Store lightweight references (defer FEN/ELO extraction)
        self.all_hung_queen_refs = []  # Store references across games for final selection
    
    def start_game(self, game_data: GameData, user_is_white: bool, user_is_black: bool):
        """
        Initialize analyzer state for a new game.
        
        Args:
            game_data: The game data
            user_is_white: Whether user is playing white
            user_is_black: Whether user is playing black
        """
        super().start_game(game_data, user_is_white, user_is_black)
        
        # Pre-filter: Only consider games where the user lost
        self.user_lost = (self.game_data.metadata.result == "0-1" and user_is_white) or \
                        (self.game_data.metadata.result == "1-0" and user_is_black)
        
        if not self.user_lost:
            return
        
        # Track queen and king positions
        if user_is_white:
            self.opponent_queen_square = "d8"  # Black's queen starts on d8
            self.user_queen_square = "d1"  # White's queen starts on d1
            self.user_king_square = "e1"  # White's king starts on e1
        else:
            self.opponent_queen_square = "d1"  # White's queen starts on d1
            self.user_queen_square = "d8"  # Black's queen starts on d8
            self.user_king_square = "e8"  # Black's king starts on e8
        
        # Track state for exclusion logic
        self.user_queen_just_captured_square = None  # Track if user's queen just captured
        self.user_queen_just_moved_to_square = None  # Track if user's queen just moved (not captured)
        self.opponent_just_gave_check = False  # Track if opponent just gave check
        self.hung_queen_found = False  # Flag to stop processing after finding one
    
    def process_move(self, context: MoveContext):
        """
        Process a single move to detect hung queens.
        
        Logic matches original HungQueenAnalyzer:
        1. Track queen and king positions
        2. Check if opponent captures user's queen with non-queen piece
        3. Exclude various scenarios (pins, trades, tactical sequences, etc.)
        """
        # Skip if user didn't lose or we already found a hung queen
        if not self.user_lost or self.hung_queen_found:
            return
        
        # Update queen positions when queens move
        if context.is_opponent_move and context.move and context.board.piece_at(context.move.from_square):
            moving_piece = context.board.piece_at(context.move.from_square)
            if moving_piece and moving_piece.piece_type == chess.QUEEN:
                # Opponent's queen moved - update position
                self.opponent_queen_square = chess.square_name(context.move.to_square)
        
        if context.is_user_move and context.move and context.board.piece_at(context.move.from_square):
            moving_piece = context.board.piece_at(context.move.from_square)
            if moving_piece and moving_piece.piece_type == chess.QUEEN:
                if not context.board.is_capture(context.move):
                    # User's queen moved (not a capture) - update position
                    self.user_queen_square = chess.square_name(context.move.to_square)
                    self.user_queen_just_moved_to_square = self.user_queen_square
                    self.user_queen_just_captured_square = None
                else:
                    # User's queen captures - handle separately
                    captured_square = context.move.to_square
                    # Check if capturing opponent's queen
                    captured_piece = context.board.piece_at(captured_square)
                    if captured_piece and captured_piece.piece_type == chess.QUEEN:
                        self.opponent_queen_square = None
                    # Update user's queen position
                    self.user_queen_square = chess.square_name(captured_square)
                    self.user_queen_just_captured_square = self.user_queen_square
                    self.user_queen_just_moved_to_square = None
        
        # Track user's king position
        # CORRECTLY track castling (unlike original analyzer which has a bug)
        if context.is_user_move and context.move:
            moving_piece = context.board.piece_at(context.move.from_square)
            if moving_piece and moving_piece.piece_type == chess.KING:
                # Check if it's castling by checking if king moves 2 squares horizontally
                from_file = chess.square_file(context.move.from_square)
                to_file = chess.square_file(context.move.to_square)
                if abs(to_file - from_file) == 2:
                    # Castling - king moves 2 squares
                    if to_file > from_file:
                        # Kingside castling
                        self.user_king_square = "g1" if self.user_is_white else "g8"
                    else:
                        # Queenside castling
                        self.user_king_square = "c1" if self.user_is_white else "c8"
                else:
                    # Regular king move
                    self.user_king_square = chess.square_name(context.move.to_square)
        
        # Clear tracking if user makes any non-queen move
        if context.is_user_move:
            moving_piece = context.board.piece_at(context.move.from_square) if context.move else None
            if not moving_piece or moving_piece.piece_type != chess.QUEEN:
                self.user_queen_just_captured_square = None
                self.user_queen_just_moved_to_square = None
        
        # Check if user's pieces capture opponent's queen
        if context.is_user_move and context.board.is_capture(context.move):
            captured_square = context.move.to_square
            captured_piece = context.board.piece_at(captured_square)
            if captured_piece and captured_piece.piece_type == chess.QUEEN:
                self.opponent_queen_square = None
        
        # Check if opponent captures user's queen
        # OPTIMIZATION: Pre-filter to only check captures (skips ~76% of opponent moves)
        if context.is_opponent_move and self.user_queen_square:
            moves = self.game_data.moves
            move_index = context.move_number - 1
            if move_index < len(moves):
                move_str = moves[move_index]
                
                # Pre-filter: Only process capture moves (contain 'x')
                # This skips the expensive capture checking logic for ~76% of moves
                if 'x' in move_str:
                    # Check if move string contains 'x' + user_queen_square (matches original logic)
                    if 'x' + self.user_queen_square in move_str:
                        # Also verify it's actually a capture to the queen's square
                        if context.move.to_square == chess.parse_square(self.user_queen_square):
                            # Opponent captured user's queen
                            moving_piece = context.board.piece_at(context.move.from_square)
                            
                            # OPTIMIZATION: Reorder exclusions - fast checks first, expensive pin checking last
                            # 1. Quick check: Exclude if captured by queen (queen trade)
                            if moving_piece and moving_piece.piece_type == chess.QUEEN:
                                self.user_queen_square = None
                                self.user_queen_just_captured_square = None
                                self.user_queen_just_moved_to_square = None
                                self.opponent_just_gave_check = False
                                return
                            
                            # 2. Quick check: Exclude if opponent just gave check (tactical sequence)
                            if self.opponent_just_gave_check:
                                self.user_queen_square = None
                                self.user_queen_just_captured_square = None
                                self.user_queen_just_moved_to_square = None
                                self.opponent_just_gave_check = False
                                return
                            
                            # 3. Quick check: Exclude if user's queen just captured on this square (recapture)
                            if self.user_queen_just_captured_square == self.user_queen_square:
                                self.user_queen_square = None
                                self.user_queen_just_captured_square = None
                                self.user_queen_just_moved_to_square = None
                                self.opponent_just_gave_check = False
                                return
                            
                            # 4. Quick check: Exclude if user's queen just moved to this square (immediate capture)
                            if self.user_queen_just_moved_to_square == self.user_queen_square:
                                self.user_queen_square = None
                                self.user_queen_just_captured_square = None
                                self.user_queen_just_moved_to_square = None
                                self.opponent_just_gave_check = False
                                return
                            
                            # 5. Quick check: Check if opponent's queen is still on the board
                            if not self.opponent_queen_square:
                                # Opponent's queen already captured - not a hung queen
                                self.user_queen_square = None
                                self.user_queen_just_captured_square = None
                                self.user_queen_just_moved_to_square = None
                                self.opponent_just_gave_check = False
                                return
                            
                            # 6. Medium check: Check if opponent's queen gets captured soon after (queen trade scenario)
                            # Look ahead a few moves to see if opponent's queen is captured
                            next_move_index = context.move_number  # Next move index (after current capture)
                            opponent_queen_captured_soon_after = False
                            
                            if next_move_index < len(moves) and self.opponent_queen_square:
                                # Check next few moves (within 2-3 moves) for opponent queen capture
                                for j in range(next_move_index, min(next_move_index + 4, len(moves))):
                                    next_move_str = moves[j]
                                    # Check if this is the user's move (user captures opponent's queen)
                                    is_user_next_move = (j % 2 == 0 and self.user_is_white) or (j % 2 == 1 and self.user_is_black)
                                    if is_user_next_move and 'x' + self.opponent_queen_square in next_move_str:
                                        opponent_queen_captured_soon_after = True
                                        break
                            
                            # Exclude if opponent's queen is captured soon after (queen trade, not hung queen)
                            if opponent_queen_captured_soon_after:
                                self.user_queen_square = None
                                self.user_queen_just_captured_square = None
                                self.user_queen_just_moved_to_square = None
                                self.opponent_just_gave_check = False
                                return
                            
                            # 7. EXPENSIVE check (moved to end): Check if queen is pinned (rook/bishop aligned with king)
                            # CORRECT pin detection: must check:
                            # 1. King and queen are aligned (same file/rank for rook, same diagonal for bishop)
                            # 2. Capturing piece is on that line
                            # 3. No pieces between king and queen
                            if self.user_king_square:
                                capturing_piece_from_square = chess.square_name(context.move.from_square)
                                
                                if move_str.startswith('R') or move_str.startswith('r'):
                                    # Rook capture - check if this is a true pin
                                    if self._is_queen_pinned_by_rook(context.board, self.user_queen_square, self.user_king_square, capturing_piece_from_square):
                                        # Queen is pinned by rook - not a hung queen
                                        self.user_queen_square = None
                                        self.user_queen_just_captured_square = None
                                        self.user_queen_just_moved_to_square = None
                                        self.opponent_just_gave_check = False
                                        return
                                elif move_str.startswith('B') or move_str.startswith('b'):
                                    # Bishop capture - check if this is a true pin
                                    if self._is_queen_pinned_by_bishop(context.board, self.user_queen_square, self.user_king_square, capturing_piece_from_square):
                                        # Queen is pinned by bishop - not a hung queen
                                        self.user_queen_square = None
                                        self.user_queen_just_captured_square = None
                                        self.user_queen_just_moved_to_square = None
                                        self.opponent_just_gave_check = False
                                        return
                            
                            # Found a hung queen!
                            self._record_hung_queen(context)
                            return
        
        # Check if opponent gives check (track for next move)
        if context.is_opponent_move:
            # Check for both '+' (check) and '#' (checkmate)
            moves = self.game_data.moves
            move_index = context.move_number - 1
            if move_index < len(moves):
                move_str = moves[move_index]
                if '+' in move_str or '#' in move_str:
                    self.opponent_just_gave_check = True
                else:
                    # Clear check tracking if opponent didn't capture queen and didn't give check
                    if not (self.user_queen_square and context.move.to_square == chess.parse_square(self.user_queen_square)):
                        self.opponent_just_gave_check = False
            
            # Clear tracking after opponent's move
            self.user_queen_just_captured_square = None
            self.user_queen_just_moved_to_square = None
    
    def _are_squares_aligned(self, square1: str, square2: str, piece_type: str) -> bool:
        """
        Check if two squares are aligned for a pin check.
        
        Args:
            square1: First square (e.g., "c4")
            square2: Second square (e.g., "c1")
            piece_type: "rook" (file/rank) or "bishop" (diagonal)
        
        Returns:
            True if squares are aligned for the piece type
        """
        file1, rank1 = square1[0], int(square1[1])
        file2, rank2 = square2[0], int(square2[1])
        
        if piece_type == "rook":
            # Same file or same rank
            return file1 == file2 or rank1 == rank2
        elif piece_type == "bishop":
            # Same diagonal: |file_diff| == |rank_diff|
            file_diff = abs(ord(file1) - ord(file2))
            rank_diff = abs(rank1 - rank2)
            return file_diff == rank_diff
        return False
    
    def _is_queen_pinned_by_rook(self, board: chess.Board, queen_square: str, king_square: str, rook_from_square: str) -> bool:
        """
        Check if queen is truly pinned by rook.
        
        Requirements:
        1. King and queen must be on same file OR same rank
        2. Rook's FROM square must be on that line
        3. No pieces between king and queen (excluding the queen itself)
        
        Args:
            board: Chess board BEFORE the capture move
            queen_square: Square where queen is (e.g., "g4")
            king_square: Square where king is (e.g., "g8")
            rook_from_square: Square where rook is coming from (e.g., "d4")
        
        Returns:
            True if queen is truly pinned
        """
        queen_file, queen_rank = queen_square[0], int(queen_square[1])
        king_file, king_rank = king_square[0], int(king_square[1])
        rook_file, rook_rank = rook_from_square[0], int(rook_from_square[1])
        
        # Check if king and queen are aligned
        same_file = queen_file == king_file
        same_rank = queen_rank == king_rank
        
        if not (same_file or same_rank):
            return False  # Not aligned
        
        # Check if rook is on the line between king and queen
        if same_file:
            # Same file - rook must be on same file
            if rook_file != queen_file:
                return False  # Rook not on the line
            # Check if rook is between king and queen
            min_rank = min(king_rank, queen_rank)
            max_rank = max(king_rank, queen_rank)
            if not (min_rank < rook_rank < max_rank):
                return False  # Rook not between king and queen
        else:  # same_rank
            # Same rank - rook must be on same rank
            if rook_rank != queen_rank:
                return False  # Rook not on the line
            # Check if rook is between king and queen
            min_file = min(ord(king_file), ord(queen_file))
            max_file = max(ord(king_file), ord(queen_file))
            rook_file_ord = ord(rook_file)
            if not (min_file < rook_file_ord < max_file):
                return False  # Rook not between king and queen
        
        # Check if there are pieces between king and queen (excluding queen itself)
        # We need to check squares between king and queen (not including endpoints)
        if same_file:
            file = ord(queen_file) - ord('a')
            for rank in range(min(king_rank, queen_rank) + 1, max(king_rank, queen_rank)):
                square = chess.square(file, rank - 1)  # chess uses 0-7, ranks are 1-8
                piece = board.piece_at(square)
                if piece:
                    # Found a piece between - not a true pin
                    return False
        else:  # same_rank
            rank = queen_rank - 1  # chess uses 0-7, ranks are 1-8
            min_file_ord = min(ord(king_file), ord(queen_file)) - ord('a')
            max_file_ord = max(ord(king_file), ord(queen_file)) - ord('a')
            for file in range(min_file_ord + 1, max_file_ord):
                square = chess.square(file, rank)
                piece = board.piece_at(square)
                if piece:
                    # Found a piece between - not a true pin
                    return False
        
        # All conditions met - queen is truly pinned
        return True
    
    def _is_queen_pinned_by_bishop(self, board: chess.Board, queen_square: str, king_square: str, bishop_from_square: str) -> bool:
        """
        Check if queen is truly pinned by bishop.
        
        Requirements:
        1. King and queen must be on same diagonal
        2. Bishop's FROM square must be on that diagonal
        3. No pieces between king and queen (excluding the queen itself)
        
        Args:
            board: Chess board BEFORE the capture move
            queen_square: Square where queen is (e.g., "d4")
            king_square: Square where king is (e.g., "a1")
            bishop_from_square: Square where bishop is coming from
        
        Returns:
            True if queen is truly pinned
        """
        queen_file, queen_rank = queen_square[0], int(queen_square[1])
        king_file, king_rank = king_square[0], int(king_square[1])
        bishop_file, bishop_rank = bishop_from_square[0], int(bishop_from_square[1])
        
        # Check if king and queen are on same diagonal
        file_diff_king_queen = abs(ord(queen_file) - ord(king_file))
        rank_diff_king_queen = abs(queen_rank - king_rank)
        if file_diff_king_queen != rank_diff_king_queen:
            return False  # Not on same diagonal
        
        # Check if bishop is on the diagonal between king and queen
        file_diff_bishop_queen = abs(ord(bishop_file) - ord(queen_file))
        rank_diff_bishop_queen = abs(bishop_rank - queen_rank)
        if file_diff_bishop_queen != rank_diff_bishop_queen:
            return False  # Bishop not on the diagonal
        
        # Check if bishop is between king and queen
        # Determine direction of diagonal
        king_file_ord = ord(king_file) - ord('a')
        queen_file_ord = ord(queen_file) - ord('a')
        bishop_file_ord = ord(bishop_file) - ord('a')
        
        # Check if bishop is between king and queen on the diagonal
        if (king_file_ord < queen_file_ord and king_rank < queen_rank) or \
           (king_file_ord > queen_file_ord and king_rank > queen_rank):
            # Diagonal going up-right or down-left
            if not (min(king_file_ord, queen_file_ord) < bishop_file_ord < max(king_file_ord, queen_file_ord)):
                return False
            if not (min(king_rank, queen_rank) < bishop_rank < max(king_rank, queen_rank)):
                return False
        else:
            # Diagonal going up-left or down-right
            if not (min(king_file_ord, queen_file_ord) < bishop_file_ord < max(king_file_ord, queen_file_ord)):
                return False
            if not (min(king_rank, queen_rank) < bishop_rank < max(king_rank, queen_rank)):
                return False
        
        # Check if there are pieces between king and queen on the diagonal
        # Walk along the diagonal from king towards queen
        file_step = 1 if queen_file_ord > king_file_ord else -1
        rank_step = 1 if queen_rank > king_rank else -1
        
        current_file = king_file_ord + file_step
        current_rank = king_rank + rank_step
        
        while current_file != queen_file_ord and current_rank != queen_rank:
            square = chess.square(current_file, current_rank - 1)  # chess uses 0-7, ranks are 1-8
            piece = board.piece_at(square)
            if piece:
                # Found a piece between - not a true pin
                return False
            current_file += file_step
            current_rank += rank_step
        
        # All conditions met - queen is truly pinned
        return True
    
    def _record_hung_queen(self, context: MoveContext):
        """Record a found hung queen (store lightweight reference, defer FEN/ELO extraction)."""
        if not self.user_queen_square:
            return
        
        # Store lightweight reference (defer FEN/ELO extraction until get_final_results())
        move_number = (context.move_number + 1) // 2  # Full move number
        
        # Check if game ended by resignation soon after
        termination = self._get_termination(self.game_data.pgn)
        resigned_after = False
        if termination and 'resign' in termination.lower():
            # Check if resignation happened soon after (within 2 moves)
            moves_after_capture = len(self.game_data.moves) - context.move_number
            if moves_after_capture <= 2:
                resigned_after = True
        
        ref = {
            "game_data": self.game_data,
            "move_number": move_number,
            "half_move_number": context.move_number,
            "resigned_after": resigned_after,
            "user_is_white": self.user_is_white,
        }
        
        self.all_hung_queen_refs.append(ref)
        self.hung_queen_found = True  # Only count one hung queen per game
    
    def _get_termination(self, pgn: str) -> Optional[str]:
        """Extract Termination header from PGN."""
        match = re.search(r'\[Termination\s+"([^"]+)"\]', pgn)
        if match:
            return match.group(1)
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
    
    def finish_game(self) -> List[Dict[str, Any]]:
        """
        Finalize analysis for the game and return findings.
        Called after all moves have been processed.
        
        Returns:
            List of findings for this game (empty for hung queen - we track across all games)
        """
        # Reset state
        self.user_queen_square = None
        self.opponent_queen_square = None
        self.user_king_square = None
        self.user_queen_just_captured_square = None
        self.user_queen_just_moved_to_square = None
        self.opponent_just_gave_check = False
        self.hung_queen_found = False
        
        # Hung queen tracks across all games, so return empty list per game
        return []
    
    def get_game_points(self, config: dict) -> int:
        """Return points for current game based on existing findings."""
        if self.hung_queen_found:
            return config.get("hung_queen", -5)
        return 0
    
    def get_matched_game_links(self) -> List[str]:
        """Fast path: return just the game links that matched."""
        return [ref["game_data"].metadata.link for ref in self.all_hung_queen_refs 
                if ref.get("game_data") and ref["game_data"].metadata.link]

    def get_final_results(self) -> List[Dict[str, Any]]:
        """
        Get final results after processing all games.
        Selects the best hung queen based on priority.
        NOW extracts FEN and ELO only for the best hung queen.
        
        Returns:
            List containing the best hung queen (or empty list if none found)
        """
        if not self.all_hung_queen_refs:
            return []
        
        # Sort by priority:
        # 1. First priority: resigned_after (True first)
        # 2. Second priority: move_number (lowest first)
        self.all_hung_queen_refs.sort(key=lambda x: (
            not x["resigned_after"],  # False (0) comes before True (1)
            x["move_number"]
        ))
        
        # Select only the best one - NOW extract FEN and ELO
        best_ref = self.all_hung_queen_refs[0]
        
        # Extract ELO ratings from PGN
        white_elo = self._extract_elo(best_ref["game_data"].pgn, "WhiteElo")
        black_elo = self._extract_elo(best_ref["game_data"].pgn, "BlackElo")
        
        # Build replay data structure for frontend (uses config file)
        # Use 2 moves before the capture (the position where the queen was hung)
        from ..replay_helper import build_replay_data
        hung_move = max(0, best_ref["half_move_number"] - 2)  # Two moves before capture
        replay_data = build_replay_data(
            game_data=best_ref["game_data"],
            key_half_move=hung_move,
            feature_name="hung_queen"  # Loads settings from replay_config.json
        )
        
        # Calculate statistics
        total_hung_queens = len(self.all_hung_queen_refs)
        total_resignations = sum(1 for r in self.all_hung_queen_refs if r["resigned_after"])
        resignation_percentage = round((total_resignations / total_hung_queens * 100), 1) if total_hung_queens > 0 else 0
        
        # Build full finding with extracted data
        best_finding = {
            "feature_name": "hung_queen",
            "display_name": "Hung Queen",
            "game_metadata": {
                "white": best_ref["game_data"].metadata.white,
                "black": best_ref["game_data"].metadata.black,
                "result": best_ref["game_data"].metadata.result,
                "date": best_ref["game_data"].metadata.date,
                "link": best_ref["game_data"].metadata.link,
                "white_elo": white_elo,
                "black_elo": black_elo,
                "user_color": "white" if best_ref["user_is_white"] else "black",
                # New simplified format for frontend navigation
                "all_moves": replay_data["all_moves"],
                "key_position_index": replay_data["key_position_index"],
                "fen": replay_data["fen"],
                # Include full PGN - contains all clock annotations/timestamps
                "pgn": best_ref["game_data"].pgn
            },
            "position_link": f"{best_ref['game_data'].metadata.link}?move={best_ref['half_move_number']}" if best_ref["game_data"].metadata.link else None,
            "result_data": {
                "total_hung_queens": {
                    "value": total_hung_queens,
                    "label": "Total Hung Queens"
                },
                "resignation_percentage": {
                    "value": resignation_percentage,
                    "label": "Resignation Percentage"
                }
            }
        }
        
        return [best_finding]
    
    def _get_fen_at_move(self, pgn: str, half_move_number: int) -> Optional[str]:
        """
        Get FEN at a specific half-move number.
        Replays the game to the specified move.
        
        Args:
            pgn: PGN string
            half_move_number: 1-indexed half-move number (1 = after first move, 2 = after second move, etc.)
        
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
                move_count += 1
                if move_count >= half_move_number:
                    break
                board.push(node.move)
            
            return board.fen()
        except Exception:
            return None
