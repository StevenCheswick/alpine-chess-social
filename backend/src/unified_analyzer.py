"""
Unified analyzer that processes games move-by-move in a single pass.
"""
import chess
from typing import List, Dict, Any, Optional
from .game_data import GameData
from .tcn_decoder import decode_tcn


class MoveContext:
    """Context information available to analyzers at each move."""
    def __init__(self, move: chess.Move, move_number: int, board: chess.Board,
                 is_user_move: bool, is_opponent_move: bool, user_color: chess.Color,
                 game_data: GameData, previous_move: Optional[chess.Move] = None,
                 next_move: Optional[chess.Move] = None):
        self.move = move
        self.move_number = move_number
        self.board = board
        self.is_user_move = is_user_move
        self.is_opponent_move = is_opponent_move
        self.user_color = user_color
        self.game_data = game_data
        self.previous_move = previous_move
        self.next_move = next_move

        # Lazy evaluation
        self._move_san = None

    @property
    def move_san(self) -> str:
        """Get SAN notation for current move (lazy evaluation)."""
        if self._move_san is None:
            self._move_san = self.board.san(self.move)
        return self._move_san


class UnifiedAnalyzerBase:
    """Base class for analyzers that work with the unified move-by-move approach."""

    def __init__(self, username: str):
        self.username = username.lower()
        self.findings = []

    def start_game(self, game_data: GameData, user_is_white: bool, user_is_black: bool):
        """Initialize analyzer state for a new game."""
        self.findings = []
        self.game_data = game_data
        self.user_is_white = user_is_white
        self.user_is_black = user_is_black
        self.user_color = chess.WHITE if user_is_white else chess.BLACK

    def process_move(self, context: MoveContext):
        """Process a single move. Analyzers override this."""
        pass

    def finish_game(self) -> List[Dict[str, Any]]:
        """Finalize analysis for the game and return findings."""
        return self.findings

    def get_game_points(self, config: dict) -> int:
        """Calculate points for the current game based on existing findings."""
        return 0


class UnifiedAnalyzer:
    """Main unified analyzer that processes games move-by-move."""

    # Pre-filter 1: Skip mate analyzers if game doesn't end in checkmate
    MATE_ANALYZERS = {
        'UnifiedSmotheredMateAnalyzer',
        'UnifiedKingMateAnalyzer',
        'UnifiedCastleMateAnalyzer',
        'UnifiedPawnMateAnalyzer',
        'UnifiedKnightPromotionMateAnalyzer',
        'UnifiedPromotionMateAnalyzer',
        'UnifiedQuickestMateAnalyzer',
        'UnifiedEnPassantMateAnalyzer',
        'UnifiedBackRankMateAnalyzer',
        'UnifiedKnightBishopMateAnalyzer',
        'UnifiedKingWalkAnalyzer',
    }

    # Pre-filter 2: Skip win-based analyzers if user didn't win
    WIN_ANALYZERS = {
        'UnifiedQueenSacrificeAnalyzer',
        'UnifiedKnightForkAnalyzer',
        'UnifiedRookSacrificeAnalyzer',
        'UnifiedQuickestMateAnalyzer',
        'UnifiedBiggestComebackAnalyzer',
        'UnifiedClutchWinAnalyzer',
        'UnifiedBestGameAnalyzer',
        'UnifiedLongestGameAnalyzer',
        'UnifiedKingWalkAnalyzer',
        'UnifiedWindmillAnalyzer',
    }

    # Pre-filter 3: Skip stalemate analyzer if game isn't a draw
    DRAW_ANALYZERS = {
        'UnifiedStalemateAnalyzer',
    }

    def __init__(self, username: str):
        self.username = username.lower()
        self.analyzers: List[UnifiedAnalyzerBase] = []

    def register_analyzer(self, analyzer: UnifiedAnalyzerBase):
        """Register an analyzer to run during unified analysis."""
        self.analyzers.append(analyzer)

    def _is_hyper_bullet(self, time_control: str) -> bool:
        """Check if game is hyper bullet (base time < 60 seconds)."""
        if not time_control:
            return False
        try:
            base = time_control.split('+')[0]
            return int(float(base)) < 60
        except (ValueError, IndexError):
            return False

    def _get_active_analyzers(self, game_data: GameData, user_is_white: bool) -> List[UnifiedAnalyzerBase]:
        """Get list of analyzers that should be active for this game based on pre-filters."""
        # Determine game conditions
        has_checkmate = game_data.moves and '#' in game_data.moves[-1] if game_data.moves else False
        result = game_data.metadata.result
        user_won = (result == "1-0" and user_is_white) or (result == "0-1" and not user_is_white)
        is_draw = result == "1/2-1/2"

        active_analyzers = []
        for analyzer in self.analyzers:
            analyzer_name = analyzer.__class__.__name__

            # Filter 1: Skip mate analyzers if no checkmate
            if analyzer_name in self.MATE_ANALYZERS and not has_checkmate:
                continue

            # Filter 2: Skip win-based analyzers if user didn't win
            if analyzer_name in self.WIN_ANALYZERS and not user_won:
                continue

            # Filter 3: Skip stalemate analyzer if not a draw
            if analyzer_name in self.DRAW_ANALYZERS and not is_draw:
                continue

            active_analyzers.append(analyzer)

        return active_analyzers

    def analyze_game(self, game_data: GameData) -> Dict[str, List[Dict[str, Any]]]:
        """Analyze a single game move-by-move with all registered analyzers."""
        # Check if user played this game
        user_is_white = game_data.metadata.white.lower() == self.username
        user_is_black = game_data.metadata.black.lower() == self.username
        if not user_is_white and not user_is_black:
            return {}

        # Skip hyper bullet games
        if self._is_hyper_bullet(game_data.metadata.time_control):
            return {}

        user_color = chess.WHITE if user_is_white else chess.BLACK

        # Initialize all analyzers
        for analyzer in self.analyzers:
            analyzer.start_game(game_data, user_is_white, user_is_black)

        # Get active analyzers based on pre-filters
        active_analyzers = self._get_active_analyzers(game_data, user_is_white)

        # Replay game move-by-move
        try:
            if not game_data.moves and not game_data.tcn:
                return {}

            board = chess.Board()
            move_number = 0
            previous_move = None

            # Decode moves from TCN or use SAN list
            if game_data.tcn:
                tcn_moves = decode_tcn(game_data.tcn)
            else:
                tcn_moves = None

            num_moves = len(tcn_moves) if tcn_moves else len(game_data.moves)

            # Process each move
            for i in range(num_moves):
                if tcn_moves:
                    move = tcn_moves[i]
                else:
                    move = board.parse_san(game_data.moves[i])

                is_user_move = (board.turn == user_color)
                is_opponent_move = not is_user_move

                context = MoveContext(
                    move=move,
                    move_number=move_number + 1,
                    board=board,
                    is_user_move=is_user_move,
                    is_opponent_move=is_opponent_move,
                    user_color=user_color,
                    game_data=game_data,
                    previous_move=previous_move,
                    next_move=None
                )

                # Let active analyzers process this move
                for analyzer in active_analyzers:
                    analyzer.process_move(context)

                # Make the move
                board.push(move)
                move_number += 1
                previous_move = move

        except Exception as e:
            print(f"Error parsing game: {e}")
            return {}

        # Finalize all analyzers
        all_findings = {}
        for analyzer in self.analyzers:
            findings = analyzer.finish_game()
            if findings:
                analyzer_name = analyzer.__class__.__name__
                all_findings[analyzer_name] = findings

        return all_findings

    def analyze_games(self, games: List[GameData]) -> Dict[str, List[Dict[str, Any]]]:
        """Analyze multiple games."""
        all_findings = {analyzer.__class__.__name__: [] for analyzer in self.analyzers}

        for i, game in enumerate(games):
            if (i + 1) % 100 == 0:
                print(f"Processed {i + 1}/{len(games)} games...")

            game_findings = self.analyze_game(game)

            for analyzer_name, findings in game_findings.items():
                all_findings[analyzer_name].extend(findings)

        return all_findings
