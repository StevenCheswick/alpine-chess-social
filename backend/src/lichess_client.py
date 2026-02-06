"""
Lichess API client for fetching user game data.
"""
import requests
import json
import time
import logging
from typing import List, Optional

logger = logging.getLogger(__name__)


class LichessClient:
    """Client for interacting with Lichess API."""

    BASE_URL = "https://lichess.org/api"
    RATE_LIMIT_DELAY = 1.0  # Lichess allows ~60 req/min

    def __init__(self):
        self.session = requests.Session()
        self.session.headers.update({
            'Accept': 'application/x-ndjson',
            'User-Agent': 'ChessSocialMedia/1.0'
        })

    def _make_request(self, url: str, params: dict = None) -> Optional[str]:
        """Make API request with NDJSON response handling."""
        time.sleep(self.RATE_LIMIT_DELAY)
        try:
            response = self.session.get(url, params=params, timeout=120, stream=True)
            response.raise_for_status()
            return response.text
        except requests.exceptions.HTTPError as e:
            if e.response.status_code == 404:
                logger.warning(f"User not found: {url}")
                return None
            logger.error(f"HTTP error fetching {url}: {str(e)}")
            return None
        except requests.exceptions.RequestException as e:
            logger.error(f"Request error fetching {url}: {str(e)}")
            return None

    def verify_username(self, username: str) -> bool:
        """Verify that a Lichess username exists."""
        url = f"{self.BASE_URL}/user/{username}"
        try:
            response = self.session.get(url, timeout=10)
            return response.status_code == 200
        except:
            return False

    def fetch_user_games(
        self,
        username: str,
        since: Optional[int] = None,
        max_games: Optional[int] = None,
        rated_only: bool = True,
    ) -> List[tuple]:
        """
        Fetch games for a user from Lichess.

        Args:
            username: Lichess username
            since: Fetch games since this timestamp (Unix ms)
            max_games: Maximum number of games to fetch (None for unlimited)
            rated_only: Only fetch rated games

        Returns:
            List of (pgn_string, None) tuples (no TCN for Lichess)
        """
        url = f"{self.BASE_URL}/games/user/{username}"
        params = {
            'pgnInJson': 'true',
            'opening': 'true',
        }

        if max_games is not None:
            params['max'] = max_games

        if rated_only:
            params['rated'] = 'true'

        if since:
            params['since'] = since

        print(f"Fetching Lichess games for {username}...")
        response_text = self._make_request(url, params)

        if not response_text:
            return []

        # Parse NDJSON response (newline-delimited JSON)
        games = []
        skipped_no_pgn = 0

        for line in response_text.strip().split('\n'):
            if not line.strip():
                continue
            try:
                game_data = json.loads(line)
                pgn = game_data.get('pgn', '')
                if pgn:
                    # Store game ID in a way we can extract it
                    game_id = game_data.get('id', '')
                    games.append((pgn, game_id))
                else:
                    skipped_no_pgn += 1
            except json.JSONDecodeError as e:
                logger.warning(f"Failed to parse game JSON: {e}")
                continue

        if skipped_no_pgn > 0:
            logger.info(f"Skipped {skipped_no_pgn} games without PGN for {username}")

        print(f"Fetched {len(games)} Lichess games for {username}")
        return games
