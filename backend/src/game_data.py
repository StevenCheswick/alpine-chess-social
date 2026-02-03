"""
Data models for chess games.
"""
from dataclasses import dataclass
from typing import List, Optional


@dataclass
class GameMetadata:
    """Metadata about a chess game."""
    white: str
    black: str
    result: str  # "1-0", "0-1", "1/2-1/2"
    date: Optional[str] = None
    time_control: Optional[str] = None
    eco: Optional[str] = None  # Opening code
    event: Optional[str] = None
    link: Optional[str] = None  # Game URL


@dataclass
class GameData:
    """Complete game data including moves and metadata."""
    metadata: GameMetadata
    moves: List[str]  # List of moves in SAN notation
    pgn: str  # Full PGN string
    tcn: Optional[str] = None  # Chess.com TCN encoded moves (faster to decode)

    def __post_init__(self):
        """Validate game data."""
        if not self.moves and not self.tcn:
            raise ValueError("Game must have moves or TCN")
