"""
Helper functions for building replay data structures for frontend.
"""
import chess
import chess.pgn
from io import StringIO
from typing import Dict, Any, Optional
from .game_data import GameData


def build_replay_data(
    game_data: GameData,
    key_half_move: int,
    feature_name: Optional[str] = None
) -> Dict[str, Any]:
    """
    Build standardized replay data structure for frontend.

    Args:
        game_data: Game data object
        key_half_move: 0-indexed half-move where tactic occurs
        feature_name: Name of the feature (unused, kept for compatibility)

    Returns:
        Dictionary with replay structure
    """
    # Ensure key_half_move is within bounds
    max_half_move = len(game_data.moves) - 1 if game_data.moves else 0
    key_half_move = min(max(0, key_half_move), max_half_move)

    # Get FEN at the key position
    key_fen = _get_fen_at_move(game_data.pgn, key_half_move)
    if not key_fen:
        key_fen = chess.Board().fen()

    return {
        "all_moves": game_data.moves,
        "key_position_index": key_half_move,
        "fen": key_fen
    }


def _get_fen_at_move(pgn: str, half_move_number: int) -> Optional[str]:
    """
    Get FEN at a specific half-move number.
    """
    try:
        pgn_io = StringIO(pgn)
        game = chess.pgn.read_game(pgn_io)
        if not game:
            return None

        board = game.board()

        for i, node in enumerate(game.mainline()):
            if i >= half_move_number:
                break
            board.push(node.move)

        return board.fen()
    except Exception:
        return None
