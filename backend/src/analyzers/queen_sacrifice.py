"""
Unified analyzer for finding queen sacrifices.
"""
import chess
from typing import List, Dict, Any, Optional
import re
from ..unified_analyzer import UnifiedAnalyzerBase, MoveContext
from ..game_data import GameData


class UnifiedQueenSacrificeAnalyzer(UnifiedAnalyzerBase):
    """
    Finds games where the user sacrificed their queen.
    """

    def __init__(self, username: str):
        super().__init__(username)
        self.all_sacrifice_refs = []
        self.potential_check_sacrifice = None
        self.check_sacrifice_recaptured = False
        self.check_sacrifice_recapture_move = None

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

        # Track if this is a checkmate win
        self.is_checkmate_win = self.game_data.moves and '#' in self.game_data.moves[-1]

        # Track potential sacrifice state
        self.potential_sacrifice = None
        self.sacrifice_found = False
        self.sacrifice_led_to_mate = False
        self.our_queen_was_recaptured = False
        self.recapture_move_number = None

    def _calculate_material_advantage(self, board: chess.Board) -> int:
        """Calculate user's material advantage."""
        piece_values = {
            chess.PAWN: 1,
            chess.KNIGHT: 3,
            chess.BISHOP: 3,
            chess.ROOK: 5,
            chess.QUEEN: 9,
        }

        white_material = 0
        black_material = 0

        for piece_type, value in piece_values.items():
            white_material += len(board.pieces(piece_type, chess.WHITE)) * value
            black_material += len(board.pieces(piece_type, chess.BLACK)) * value

        if self.user_is_white:
            return white_material - black_material
        else:
            return black_material - white_material

    def process_move(self, context: MoveContext):
        """Process a single move to detect queen sacrifices."""
        if not self.user_won or self.exclude_time_win or self.elo_too_low or self.sacrifice_found:
            return

        # Check if user's queen captures a piece
        if context.is_user_move and context.board.is_capture(context.move):
            moving_piece = context.board.piece_at(context.move.from_square)

            if moving_piece and moving_piece.piece_type == chess.QUEEN:
                captured_square = context.move.to_square
                captured_piece = context.board.piece_at(captured_square)

                # Exclude if capturing opponent's queen (that's a trade)
                if captured_piece and captured_piece.piece_type == chess.QUEEN:
                    return

                captured_piece_type = captured_piece.piece_type if captured_piece else None
                captured_piece_value = self._get_piece_value(captured_piece_type)

                moves = self.game_data.moves
                move_index = context.move_number - 1
                move_san = moves[move_index] if move_index < len(moves) else context.move_san

                material_advantage = self._calculate_material_advantage(context.board)

                user_color = chess.WHITE if self.user_is_white else chess.BLACK
                queen_pinned = context.board.is_pinned(user_color, context.move.from_square)

                self.potential_sacrifice = {
                    'sacrifice_move': context.move,
                    'sacrifice_san': move_san,
                    'sacrifice_move_number': context.move_number,
                    'captured_square': captured_square,
                    'captured_piece_type': captured_piece_type,
                    'captured_piece_value': captured_piece_value,
                    'material_advantage': material_advantage,
                    'queen_pinned': queen_pinned,
                }
                return

        # Check if opponent recaptures
        if context.is_opponent_move and self.potential_sacrifice:
            captured_square = self.potential_sacrifice['captured_square']

            if context.move_number == self.potential_sacrifice['sacrifice_move_number'] + 1:
                if context.move.to_square == captured_square:
                    self.our_queen_was_recaptured = True
                    self.recapture_move_number = context.move_number
                    return
                else:
                    self.potential_sacrifice = None
                    return
            else:
                self.potential_sacrifice = None
                self.our_queen_was_recaptured = False

        # Check user's move after queen was recaptured
        if context.is_user_move and self.our_queen_was_recaptured and self.recapture_move_number:
            if context.move_number == self.recapture_move_number + 1:
                if context.board.is_capture(context.move):
                    captured_piece = context.board.piece_at(context.move.to_square)
                    if captured_piece and captured_piece.piece_type == chess.QUEEN:
                        # User took opponent's queen - NOT a sacrifice
                        self.potential_sacrifice = None
                        self.our_queen_was_recaptured = False
                        self.recapture_move_number = None
                        return

                # User didn't take opponent's queen - IS a sacrifice!
                if self.potential_sacrifice:
                    self._record_sacrifice(context)
                self.our_queen_was_recaptured = False
                self.recapture_move_number = None
                return

            if context.move_number > self.recapture_move_number + 1:
                if self.potential_sacrifice:
                    self._record_sacrifice(context)
                self.our_queen_was_recaptured = False
                self.recapture_move_number = None
                return

        # Check sacrifice detection (queen gives check, gets captured)
        if context.is_user_move and not context.board.is_capture(context.move):
            moving_piece = context.board.piece_at(context.move.from_square)

            if moving_piece and moving_piece.piece_type == chess.QUEEN:
                if context.board.gives_check(context.move):
                    moves = self.game_data.moves
                    move_index = context.move_number - 1
                    move_san = moves[move_index] if move_index < len(moves) else context.move_san

                    material_advantage = self._calculate_material_advantage(context.board)

                    user_color = chess.WHITE if self.user_is_white else chess.BLACK
                    queen_pinned = context.board.is_pinned(user_color, context.move.from_square)

                    self.potential_check_sacrifice = {
                        'sacrifice_move_number': context.move_number,
                        'sacrifice_san': move_san,
                        'queen_square': context.move.to_square,
                        'material_advantage': material_advantage,
                        'queen_pinned': queen_pinned,
                    }
                    return

        # Check if opponent captures our queen after a check sacrifice
        if context.is_opponent_move and self.potential_check_sacrifice:
            queen_square = self.potential_check_sacrifice['queen_square']

            if context.move_number == self.potential_check_sacrifice['sacrifice_move_number'] + 1:
                if context.move.to_square == queen_square and context.board.is_capture(context.move):
                    self.check_sacrifice_recaptured = True
                    self.check_sacrifice_recapture_move = context.move_number
                    return

            self.potential_check_sacrifice = None

        # Check user's move after check sacrifice queen was captured
        if context.is_user_move and self.check_sacrifice_recaptured and self.check_sacrifice_recapture_move:
            if context.move_number == self.check_sacrifice_recapture_move + 1:
                if context.board.is_capture(context.move):
                    captured_piece = context.board.piece_at(context.move.to_square)
                    if captured_piece and captured_piece.piece_type == chess.QUEEN:
                        # User took opponent's queen back - NOT a sacrifice
                        self.potential_check_sacrifice = None
                        self.check_sacrifice_recaptured = False
                        self.check_sacrifice_recapture_move = None
                        return

                # User's move but didn't take opponent's queen - IS a sacrifice!
                if self.potential_check_sacrifice:
                    self.potential_sacrifice = {
                        'sacrifice_move': None,
                        'sacrifice_san': self.potential_check_sacrifice['sacrifice_san'],
                        'sacrifice_move_number': self.potential_check_sacrifice['sacrifice_move_number'],
                        'captured_square': self.potential_check_sacrifice['queen_square'],
                        'captured_piece_type': None,
                        'captured_piece_value': 0,
                        'material_advantage': self.potential_check_sacrifice['material_advantage'],
                        'queen_pinned': self.potential_check_sacrifice.get('queen_pinned', False),
                    }
                    self._record_sacrifice(context)
                self.potential_check_sacrifice = None
                self.check_sacrifice_recaptured = False
                self.check_sacrifice_recapture_move = None
                return

    def _record_sacrifice(self, context: MoveContext):
        """Record a found queen sacrifice."""
        if not self.potential_sacrifice:
            return

        material_advantage = self.potential_sacrifice.get('material_advantage', 0)
        if material_advantage >= 5:
            self.potential_sacrifice = None
            return

        queen_pinned = self.potential_sacrifice.get('queen_pinned', False)
        if queen_pinned:
            self.potential_sacrifice = None
            return

        sacrifice_move_number = self.potential_sacrifice['sacrifice_move_number']
        captured_piece_value = self.potential_sacrifice['captured_piece_value']

        # Calculate moves_to_mate
        moves_to_mate = None
        is_checkmate_in_range = False
        if self.is_checkmate_win and self.game_data.moves:
            sacrifice_full_move = (sacrifice_move_number + 1) // 2
            checkmate_half_move = len(self.game_data.moves)
            checkmate_full_move = (checkmate_half_move + 1) // 2
            moves_after_sacrifice = checkmate_full_move - sacrifice_full_move
            if 1 <= moves_after_sacrifice <= 5:
                moves_to_mate = moves_after_sacrifice
            is_checkmate_in_range = 2 <= moves_after_sacrifice <= 4

        ref = {
            "game_data": self.game_data,
            "sacrifice_move_number": sacrifice_move_number,
            "sacrifice_san": self.potential_sacrifice['sacrifice_san'],
            "captured_piece_value": captured_piece_value,
            "is_checkmate_win": self.is_checkmate_win,
            "is_checkmate_in_range": is_checkmate_in_range,
            "user_is_white": self.user_is_white,
            "moves_to_mate": moves_to_mate,
        }

        self.all_sacrifice_refs.append(ref)
        self.sacrifice_found = True
        self.sacrifice_led_to_mate = moves_to_mate is not None
        self.potential_sacrifice = None

    def _get_piece_value(self, piece_type: Optional[int]) -> int:
        """Get piece value."""
        if not piece_type:
            return 5
        if piece_type == chess.PAWN:
            return 1
        elif piece_type in [chess.KNIGHT, chess.BISHOP]:
            return 3
        elif piece_type == chess.ROOK:
            return 5
        elif piece_type == chess.QUEEN:
            return 9
        else:
            return 5

    def _get_termination(self, pgn: str) -> Optional[str]:
        """Extract Termination header from PGN."""
        match = re.search(r'\[Termination\s+"([^"]+)"\]', pgn)
        if match:
            return match.group(1)
        return None

    def finish_game(self) -> List[Dict[str, Any]]:
        """Finalize analysis for the game."""
        self.potential_sacrifice = None
        self.our_queen_was_recaptured = False
        self.recapture_move_number = None
        self.potential_check_sacrifice = None
        self.check_sacrifice_recaptured = False
        self.check_sacrifice_recapture_move = None
        return []

    def get_matched_game_links(self) -> List[str]:
        """Fast path: return just the game links that matched."""
        return [ref["game_data"].metadata.link for ref in self.all_sacrifice_refs 
                if ref.get("game_data") and ref["game_data"].metadata.link]

    def get_final_results(self) -> List[Dict[str, Any]]:
        """Get final results after processing all games."""
        if not self.all_sacrifice_refs:
            return []

        total_queen_sacs = len(self.all_sacrifice_refs)

        # Return all sacrifices for the games page (not just the best one)
        results = []
        for ref in self.all_sacrifice_refs:
            white_elo = self._extract_elo(ref["game_data"].pgn, "WhiteElo")
            black_elo = self._extract_elo(ref["game_data"].pgn, "BlackElo")

            sacrifice_move_number = ref["sacrifice_move_number"]

            from ..replay_helper import build_replay_data
            key_half_move = max(0, sacrifice_move_number - 1)
            replay_data = build_replay_data(
                game_data=ref["game_data"],
                key_half_move=key_half_move,
                feature_name="queen_sacrifice"
            )

            finding = {
                "feature_name": "queen_sacrifice",
                "display_name": "Queen Sacrifice",
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
                "position_link": f"{ref['game_data'].metadata.link}?move={sacrifice_move_number}" if ref["game_data"].metadata.link else None,
            }

            if ref.get("moves_to_mate"):
                finding["moves_to_mate"] = ref["moves_to_mate"]

            results.append(finding)

        return results

    def _extract_elo(self, pgn: str, elo_header: str) -> Optional[int]:
        """Extract ELO rating from PGN header."""
        match = re.search(rf'\[{elo_header}\s+"(\d+)"\]', pgn)
        if match:
            try:
                return int(match.group(1))
            except ValueError:
                return None
        return None
