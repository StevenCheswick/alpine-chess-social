"""
PGN parsing utilities - lightweight version.
"""
import re
from typing import List, Optional
from .game_data import GameData, GameMetadata


STANDARD_START_FEN = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"


def parse_pgn(pgn_string: str, filter_non_standard: bool = True, tcn: Optional[str] = None) -> Optional[GameData]:
    """
    Parse a PGN string into a GameData object.
    Uses regex for fast header extraction and move parsing.
    """
    try:
        # Extract headers with regex
        headers = {}
        for match in re.finditer(r'\[(\w+)\s+"([^"]*)"\]', pgn_string):
            headers[match.group(1)] = match.group(2)

        # Filter out non-standard games
        if filter_non_standard:
            if headers.get('SetUp') == '1':
                fen = headers.get('FEN', '')
                if fen and fen != STANDARD_START_FEN:
                    return None

        metadata = GameMetadata(
            white=headers.get('White', 'Unknown'),
            black=headers.get('Black', 'Unknown'),
            result=headers.get('Result', '*'),
            date=headers.get('Date', None),
            time_control=headers.get('TimeControl', None),
            eco=headers.get('ECO', None),
            event=headers.get('Event', None),
            link=headers.get('Link', None)
        )

        # Extract moves
        move_text = re.sub(r'\[[^\]]*\]', '', pgn_string)  # Remove headers
        move_text = re.sub(r'\{[^}]*\}', '', move_text)    # Remove comments
        move_text = re.sub(r'\([^)]*\)', '', move_text)    # Remove variations

        # Extract individual moves
        moves = re.findall(r'[KQRBN]?[a-h]?[1-8]?x?[a-h][1-8](?:=[QRBN])?[+#]?|O-O-O|O-O', move_text)

        if not moves and not tcn:
            return None

        return GameData(
            metadata=metadata,
            moves=moves,
            pgn=pgn_string,
            tcn=tcn
        )
    except Exception as e:
        print(f"Error parsing PGN: {e}")
        return None


def parse_pgns(pgn_strings: List[str], tcn_list: Optional[List[str]] = None) -> List[GameData]:
    """Parse multiple PGN strings with optional TCN data."""
    games = []
    for i, pgn in enumerate(pgn_strings):
        if (i + 1) % 500 == 0:
            print(f"Parsed {i + 1}/{len(pgn_strings)} games...")
        tcn = tcn_list[i] if tcn_list and i < len(tcn_list) else None
        game = parse_pgn(pgn, tcn=tcn)
        if game:
            games.append(game)
    return games
