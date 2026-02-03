"""
TCN (Terse Chess Notation) decoder for Chess.com games.
TCN is a compact 2-char-per-move encoding that's faster to decode than SAN.
"""
import chess
from typing import List, Tuple, Optional

# Chess.com TCN character set
TCN_CHARS = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!?{~}(^)[_]@#$,./&-*++="

# Promotion piece mapping
PROMO_PIECES = [chess.QUEEN, chess.KNIGHT, chess.ROOK, chess.BISHOP]


def decode_tcn(tcn: str) -> List[chess.Move]:
    """
    Decode Chess.com TCN string to list of chess.Move objects.
    """
    moves = []
    i = 0
    while i < len(tcn) - 1:
        from_char = tcn[i]
        to_char = tcn[i + 1]

        from_idx = TCN_CHARS.find(from_char)
        to_idx = TCN_CHARS.find(to_char)

        if from_idx == -1 or to_idx == -1:
            i += 2
            continue

        from_file = from_idx % 8
        from_rank = from_idx // 8
        from_sq = chess.square(from_file, from_rank)

        if to_idx >= 64:
            # Promotion move
            promo_value = to_idx - 64
            piece_idx = promo_value // 3
            offset = promo_value % 3

            piece_type = PROMO_PIECES[piece_idx] if piece_idx < 4 else chess.QUEEN
            to_file = from_file + (offset - 1)
            to_file = max(0, min(7, to_file))
            to_rank = 7 if from_rank == 6 else 0

            to_sq = chess.square(to_file, to_rank)
            moves.append(chess.Move(from_sq, to_sq, promotion=piece_type))
        else:
            # Regular move
            to_file = to_idx % 8
            to_rank = to_idx // 8
            to_sq = chess.square(to_file, to_rank)
            moves.append(chess.Move(from_sq, to_sq))

        i += 2

    return moves


def replay_tcn(tcn: str, callback=None) -> Tuple[chess.Board, List[chess.Move]]:
    """
    Replay a game from TCN, optionally calling a callback for each move.
    """
    decoded_moves = decode_tcn(tcn)
    board = chess.Board()
    actual_moves = []

    for i, move in enumerate(decoded_moves):
        if move in board.legal_moves:
            if callback:
                callback(board, move, i + 1)
            board.push(move)
            actual_moves.append(move)
        else:
            break

    return board, actual_moves
