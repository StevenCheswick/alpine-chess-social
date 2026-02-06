"""
TCN (Terse Chess Notation) encoder/decoder for Chess.com games.
TCN is a compact 2-char-per-move encoding that's faster to decode than SAN.
"""
import chess
from typing import List, Tuple, Optional

# Chess.com TCN character set (64 squares + promotion codes)
TCN_CHARS = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!?{~}(^)[_]@#$,./&-*++="

# Promotion piece mapping for decoding
PROMO_PIECES = [chess.QUEEN, chess.KNIGHT, chess.ROOK, chess.BISHOP]

# Promotion piece to index for encoding
PROMO_PIECE_TO_IDX = {
    chess.QUEEN: 0,
    chess.KNIGHT: 1,
    chess.ROOK: 2,
    chess.BISHOP: 3,
}


def encode_tcn(moves: List[chess.Move], board: Optional[chess.Board] = None) -> str:
    """
    Encode a list of chess.Move objects to TCN string.

    Args:
        moves: List of chess.Move objects
        board: Optional starting board position (defaults to standard start)

    Returns:
        TCN encoded string
    """
    if board is None:
        board = chess.Board()
    else:
        board = board.copy()

    tcn = []
    for move in moves:
        from_sq = move.from_square
        to_sq = move.to_square

        # Encode from square (0-63 -> TCN char)
        from_char = TCN_CHARS[from_sq]

        if move.promotion:
            # Promotion move - encode specially
            from_file = chess.square_file(from_sq)
            to_file = chess.square_file(to_sq)

            # offset: -1=left capture, 0=straight, 1=right capture
            offset = to_file - from_file + 1  # 0, 1, or 2

            piece_idx = PROMO_PIECE_TO_IDX.get(move.promotion, 0)
            promo_value = 64 + (piece_idx * 3) + offset
            to_char = TCN_CHARS[promo_value]
        else:
            # Regular move
            to_char = TCN_CHARS[to_sq]

        tcn.append(from_char + to_char)

        # Push move to track board state
        if move in board.legal_moves:
            board.push(move)

    return ''.join(tcn)


def encode_san_to_tcn(san_moves: List[str], starting_fen: Optional[str] = None) -> str:
    """
    Convert SAN moves to TCN string.

    Args:
        san_moves: List of SAN move strings (e.g., ["e4", "e5", "Nf3"])
        starting_fen: Optional FEN for non-standard starting position

    Returns:
        TCN encoded string
    """
    if starting_fen:
        board = chess.Board(starting_fen)
    else:
        board = chess.Board()

    moves = []
    for san in san_moves:
        try:
            # Clean the SAN move
            clean_san = san.strip()
            if not clean_san or clean_san in ('1-0', '0-1', '1/2-1/2'):
                continue
            move = board.parse_san(clean_san)
            moves.append(move)
            board.push(move)
        except (chess.InvalidMoveError, chess.AmbiguousMoveError):
            # Skip invalid moves
            continue

    # Reset board and encode
    if starting_fen:
        board = chess.Board(starting_fen)
    else:
        board = chess.Board()

    return encode_tcn(moves, board)


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
