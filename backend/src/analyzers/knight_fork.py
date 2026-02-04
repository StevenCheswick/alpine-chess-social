"""
Unified analyzer for finding knight forks.
"""
import chess
from typing import List, Dict, Any, Optional
import re
from ..unified_analyzer import UnifiedAnalyzerBase, MoveContext
from ..game_data import GameData


class UnifiedKnightForkAnalyzer(UnifiedAnalyzerBase):
    """
    Finds games where the user executed a knight fork (attacking 2+ valuable pieces).
    """

    # Piece values for determining "valuable" targets
    PIECE_VALUES = {
        chess.PAWN: 1,
        chess.KNIGHT: 3,
        chess.BISHOP: 3,
        chess.ROOK: 5,
        chess.QUEEN: 9,
        chess.KING: 100,  # King is always valuable target
    }

    # Minimum total value of forked pieces to count as significant
    MIN_FORK_VALUE = 8  # e.g., King + Rook, Queen + anything, two Rooks

    def __init__(self, username: str):
        super().__init__(username)
        self.all_fork_refs = []

    def start_game(self, game_data: GameData, user_is_white: bool, user_is_black: bool):
        super().start_game(game_data, user_is_white, user_is_black)

        # Pre-filter: Only consider games where the user won
        self.user_won = (self.game_data.metadata.result == "1-0" and user_is_white) or \
                       (self.game_data.metadata.result == "0-1" and user_is_black)

        # Exclude games won on time
        termination = self._get_termination(self.game_data.pgn)
        self.exclude_time_win = termination and 'time' in termination.lower()

        # Exclude games where user's ELO is below 600
        elo_header = "WhiteElo" if user_is_white else "BlackElo"
        user_elo = self._extract_elo(self.game_data.pgn, elo_header)
        self.elo_too_low = user_elo is not None and user_elo < 600

        # Track if we've found a fork in this game (limit to one per game)
        self.fork_found_in_game = False

    def process_move(self, context: MoveContext):
        """Process a single move to detect knight forks."""
        if not self.user_won or self.exclude_time_win or self.elo_too_low:
            return

        # Only check user's moves
        if not context.is_user_move:
            return

        # Only one fork per game to avoid spam
        if self.fork_found_in_game:
            return

        # Check if user is moving a knight
        moving_piece = context.board.piece_at(context.move.from_square)
        if not moving_piece or moving_piece.piece_type != chess.KNIGHT:
            return

        # Create a copy of the board and make the move to see the resulting position
        board_after = context.board.copy()
        board_after.push(context.move)

        # Get the knight's destination square
        knight_square = context.move.to_square

        # Find all opponent pieces attacked by the knight after the move
        opponent_color = chess.BLACK if self.user_is_white else chess.WHITE
        attacked_pieces = self._get_attacked_pieces(board_after, knight_square, opponent_color)

        # Need at least 2 attacked pieces
        if len(attacked_pieces) < 2:
            return

        # Calculate total value of forked pieces
        total_value = sum(self.PIECE_VALUES.get(piece_type, 0) for _, piece_type in attacked_pieces)

        # Check if it's a significant fork
        if total_value < self.MIN_FORK_VALUE:
            return

        # Check if one of the forked pieces is the king (royal fork)
        is_royal_fork = any(piece_type == chess.KING for _, piece_type in attacked_pieces)

        # Get the piece names for display
        forked_piece_names = [self._piece_name(piece_type) for _, piece_type in attacked_pieces]

        # Record the fork
        moves = self.game_data.moves
        move_index = context.move_number - 1
        move_san = moves[move_index] if move_index < len(moves) else context.move_san

        ref = {
            "game_data": self.game_data,
            "fork_move_number": context.move_number,
            "fork_san": move_san,
            "forked_pieces": forked_piece_names,
            "total_value": total_value,
            "is_royal_fork": is_royal_fork,
            "user_is_white": self.user_is_white,
            "knight_square": chess.square_name(knight_square),
            "attacked_squares": [chess.square_name(sq) for sq, _ in attacked_pieces],
        }

        self.all_fork_refs.append(ref)
        self.fork_found_in_game = True

    def _get_attacked_pieces(
        self, board: chess.Board, knight_square: int, opponent_color: chess.Color
    ) -> List[tuple]:
        """
        Get all opponent pieces attacked by the knight.
        Returns list of (square, piece_type) tuples.
        """
        attacked = []
        knight_attacks = board.attacks(knight_square)

        for square in knight_attacks:
            piece = board.piece_at(square)
            if piece and piece.color == opponent_color:
                attacked.append((square, piece.piece_type))

        return attacked

    def _piece_name(self, piece_type: int) -> str:
        """Get human-readable piece name."""
        names = {
            chess.PAWN: "Pawn",
            chess.KNIGHT: "Knight",
            chess.BISHOP: "Bishop",
            chess.ROOK: "Rook",
            chess.QUEEN: "Queen",
            chess.KING: "King",
        }
        return names.get(piece_type, "Piece")

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
        """Finalize analysis for the game."""
        self.fork_found_in_game = False
        return []

    def get_final_results(self) -> List[Dict[str, Any]]:
        """Get final results after processing all games."""
        if not self.all_fork_refs:
            return []

        results = []
        for ref in self.all_fork_refs:
            white_elo = self._extract_elo(ref["game_data"].pgn, "WhiteElo")
            black_elo = self._extract_elo(ref["game_data"].pgn, "BlackElo")

            fork_move_number = ref["fork_move_number"]

            from ..replay_helper import build_replay_data
            key_half_move = max(0, fork_move_number - 1)
            replay_data = build_replay_data(
                game_data=ref["game_data"],
                key_half_move=key_half_move,
                feature_name="knight_fork"
            )

            # Build display name based on fork type
            if ref["is_royal_fork"]:
                display_name = "Royal Knight Fork"
            else:
                display_name = "Knight Fork"

            forked_desc = " & ".join(ref["forked_pieces"])

            finding = {
                "feature_name": "knight_fork",
                "display_name": display_name,
                "description": f"Knight forks {forked_desc}",
                "game_metadata": {
                    "white": ref["game_data"].metadata.white,
                    "black": ref["game_data"].metadata.black,
                    "link": ref["game_data"].metadata.link,
                    "white_elo": white_elo,
                    "black_elo": black_elo,
                    "user_color": "white" if ref["user_is_white"] else "black",
                    "all_moves": replay_data["all_moves"],
                    "key_position_index": replay_data["key_position_index"],
                    "fen": replay_data["fen"],
                    "pgn": ref["game_data"].pgn,
                    "date": ref["game_data"].metadata.date,
                    "time_control": ref["game_data"].metadata.time_control,
                    "result": ref["game_data"].metadata.result,
                },
                "position_link": f"{ref['game_data'].metadata.link}?move={fork_move_number}" if ref["game_data"].metadata.link else None,
                "forked_pieces": ref["forked_pieces"],
                "is_royal_fork": ref["is_royal_fork"],
                "knight_square": ref["knight_square"],
                "attacked_squares": ref["attacked_squares"],
            }

            results.append(finding)

        return results
