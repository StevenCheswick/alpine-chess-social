"""
Unified analyzer for finding back rank mates - checkmate delivered by Rook or Queen
when the king is trapped on the back rank by its own pawns.
Uses the unified move-by-move approach for efficiency.
"""
import chess
from typing import List, Dict, Any, Optional
import re
from ..unified_analyzer import UnifiedAnalyzerBase, MoveContext
from ..game_data import GameData


class UnifiedBackRankMateAnalyzer(UnifiedAnalyzerBase):
    """
    Unified version of BackRankMateAnalyzer.
    Finds games where the user delivered checkmate with a Rook or Queen
    when the opponent's king is trapped on the back rank by its own pawns.
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
        
        # Track state
        self.found_back_rank_mate = False
        self.back_rank_mate_ref = None  # Lightweight reference (defer FEN/ELO extraction)
    
    def process_move(self, context: MoveContext):
        """
        Process a single move - no action needed, all checks happen in finish_game().
        """
        pass  # All logic in finish_game() using final FEN
    
    def _is_king_properly_blocked(self, board: chess.Board, mated_king_is_white: bool) -> bool:
        """
        Check if the position is a back rank mate.
        
        A true back rank mate requires:
        1. King is on the back rank (rank 7 for white, rank 0 for black)
        2. King's forward escape squares are blocked by its OWN pawns
        3. Mate is delivered by Rook or Queen along the back rank
        
        Args:
            board: The board in the final position (after checkmate)
            mated_king_is_white: Whether the mated king is white
            
        Returns:
            True if it's a back rank mate
        """
        # Get the mated king's square
        king_square = board.king(mated_king_is_white)
        if king_square is None:
            return False
        
        # Get the king's rank and file
        king_rank = chess.square_rank(king_square)
        king_file = chess.square_file(king_square)
        
        # Check if king is on back rank
        if mated_king_is_white:
            # White king should be on rank 0 (1st rank, where white king starts)
            if king_rank != 0:
                return False
            # Forward escape squares are on rank 1 (2nd rank)
            escape_rank = 1
        else:
            # Black king should be on rank 7 (8th rank, where black king starts)
            if king_rank != 7:
                return False
            # Forward escape squares are on rank 6 (7th rank)
            escape_rank = 6
        
        # Check if the king's forward escape squares are blocked by its own pawns
        # King can move to adjacent squares on the escape rank (file-1, file, file+1)
        escape_squares_blocked = 0
        escape_squares_total = 0
        
        for file_offset in [-1, 0, 1]:
            target_file = king_file + file_offset
            if 0 <= target_file <= 7:
                target_square = chess.square(target_file, escape_rank)
                escape_squares_total += 1
                
                piece = board.piece_at(target_square)
                # Count if blocked by any own piece (pawn, rook, queen, etc.)
                if piece and piece.color == mated_king_is_white:
                    escape_squares_blocked += 1
        
        # For a back rank mate, most escape squares must be blocked by own pieces
        # Allow 1 empty square (common in real back rank mate patterns)
        # This means: at least (total - 1) squares must be blocked
        return escape_squares_blocked >= escape_squares_total - 1 and escape_squares_total > 0
    
    def _extract_fen(self, pgn: str) -> Optional[str]:
        """Extract FEN from PGN headers if available."""
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
    
    def _calculate_material(self, board: chess.Board, color: bool) -> int:
        """Calculate total material for a color (excluding king)."""
        piece_values = {
            chess.PAWN: 1,
            chess.KNIGHT: 3,
            chess.BISHOP: 3,
            chess.ROOK: 5,
            chess.QUEEN: 9,
        }
        total = 0
        for piece_type, value in piece_values.items():
            total += len(board.pieces(piece_type, color)) * value
        return total

    def finish_game(self) -> List[Dict[str, Any]]:
        """
        Finalize analysis for the game using final FEN.
        
        Checks all conditions using only the final board position:
        1. Is it checkmate?
        2. Is it delivered by Rook or Queen?
        3. Is the mate square on the back rank?
        4. Is the king on the back rank?
        5. Are the king's escape squares blocked?
        
        Returns:
            List of findings for this game
        """
        # Quick filter: must end in checkmate
        if '#' not in self.game_data.pgn:
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
        
        # Quick filter: must be delivered by Rook or Queen
        if not (re.match(r'^[RQ]', last_move) and '#' in last_move):
            return []
        
        # Verify the last move was made by the user
        last_move_number = len(moves)
        if self.user_is_white:
            if last_move_number % 2 == 0:
                return []  # Last move was black's, not user's
        else:  # user_is_black
            if last_move_number % 2 == 1:
                return []  # Last move was white's, not user's
        
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
        
        # 1. Verify it's actually checkmate
        if not final_board.is_checkmate():
            return []
        
        # 2. Determine which king was mated
        # If it's Black's turn and checkmate, Black was mated
        if final_board.turn == chess.BLACK:
            mated_king_is_white = False
        else:
            mated_king_is_white = True
        
        # Verify the mated king matches our expectation
        if (mated_king_is_white and not self.user_is_white) or (not mated_king_is_white and self.user_is_white):
            # This matches: user won, so opponent was mated
            pass
        else:
            return []  # Unexpected - user should have won
        
        # 3. Find the mated king
        mated_king_square = final_board.king(mated_king_is_white)
        if mated_king_square is None:
            return []
        
        mated_king_rank = chess.square_rank(mated_king_square)
        
        # Determine back rank (rank 0 for white, rank 7 for black)
        if mated_king_is_white:
            back_rank = 0
        else:
            back_rank = 7
        
        # 4. Check if king is on back rank
        if mated_king_rank != back_rank:
            return []
        
        # 5. Find what's checking the king (must be Rook or Queen on back rank)
        checking_color = not mated_king_is_white  # Attacker is opposite color
        attackers = final_board.attackers(checking_color, mated_king_square)
        
        if not attackers:
            return []
        
        # Check if any attacker is a Rook or Queen on the back rank
        found_rook_or_queen_on_back_rank = False
        for attacker_square in attackers:
            piece = final_board.piece_at(attacker_square)
            if piece and piece.piece_type in (chess.ROOK, chess.QUEEN):
                attacker_rank = chess.square_rank(attacker_square)
                if attacker_rank == back_rank:  # Attacker is on the back rank
                    found_rook_or_queen_on_back_rank = True
                    break
        
        if not found_rook_or_queen_on_back_rank:
            return []
        
        # 6. Check if king's escape squares are blocked
        if not self._is_king_properly_blocked(final_board, mated_king_is_white):
            return []
        
        # Found a back rank mate!
        self.found_back_rank_mate = True
        final_move_number = len(moves)

        # Calculate enemy material (more impressive if they had lots left)
        enemy_material = self._calculate_material(final_board, mated_king_is_white)

        # Store lightweight reference (defer FEN/ELO extraction until get_final_results)
        # Track if it's a rook mate (more impressive than queen mate)
        is_rook_mate = last_move.startswith('R')

        ref = {
            "game_data": self.game_data,
            "final_move_number": final_move_number,
            "last_move": last_move,
            "user_is_white": self.user_is_white,
            "is_rook_mate": is_rook_mate,
            "enemy_material": enemy_material
        }
        
        self.all_findings.append(ref)
        
        return []  # Return empty - we'll build full finding in get_final_results()
    
    def get_game_points(self, config: dict) -> int:
        """Return points for current game based on existing findings."""
        if self.found_back_rank_mate:
            return config.get("back_rank_mate", 35)
        return 0
    
    
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
        Returns the earliest back rank mate found (sorted by move_number).
        Includes total count of all back rank mates found.
        NOW extracts FEN and ELO only for the selected result.
        
        Returns:
            List containing the earliest back rank mate finding
        """
        if not self.all_findings:
            return []

        # Sort by:
        # 1) Rook mates first (more impressive) - 0 for rook, 1 for queen
        # 2) Most enemy material (descending - more is better, so negate)
        # 3) Earlier move number (ascending - earlier is better)
        self.all_findings.sort(key=lambda x: (
            0 if x.get("is_rook_mate") else 1,
            -x.get("enemy_material", 0),
            x["final_move_number"]
        ))

        # Get the best one
        best_ref = self.all_findings[0]
        
        # Get total count of all back rank mates found
        total_back_rank_mates = len(self.all_findings)
        
        # Extract ELO (deferred until here)
        white_elo = self._extract_elo(best_ref["game_data"].pgn, "WhiteElo")
        black_elo = self._extract_elo(best_ref["game_data"].pgn, "BlackElo")
        
        # Build replay data structure for frontend (uses config file)
        from ..replay_helper import build_replay_data
        replay_data = build_replay_data(
            game_data=best_ref["game_data"],
            key_half_move=best_ref["final_move_number"],
            feature_name="back_rank_mate"  # Loads settings from replay_config.json
        )
        
        # Build full finding with extracted data
        finding = {
            "feature_name": "back_rank_mate",
            "display_name": "Back Rank Mate",
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
                "total_back_rank_mates": {
                    "value": total_back_rank_mates,
                    "label": "Total Back Rank Mates"
                }
            }
        }
        
        return [finding]


