"""
Chess.com API client for fetching user game data.
"""
import requests
import os
import time
import logging
from typing import List, Optional

logger = logging.getLogger(__name__)


class ChessComClient:
    """Client for interacting with Chess.com Published Data API."""

    BASE_URL = "https://api.chess.com/pub"
    RATE_LIMIT_DELAY = 0.1  # Delay between requests in seconds

    def __init__(self, cache_dir: str = "data/games"):
        """
        Initialize the Chess.com API client.

        Args:
            cache_dir: Directory to cache downloaded games
        """
        self.cache_dir = cache_dir
        os.makedirs(cache_dir, exist_ok=True)
        self.session = requests.Session()
        self.session.headers.update({
            'User-Agent': 'ChessSocialMedia/1.0'
        })

    def _make_request(self, url: str, allow_404: bool = False) -> Optional[dict]:
        """
        Make an API request with error handling and rate limiting.
        """
        time.sleep(self.RATE_LIMIT_DELAY)
        try:
            response = self.session.get(url, timeout=10)
            if response.status_code == 404 and allow_404:
                return None
            response.raise_for_status()
            return response.json()
        except requests.exceptions.HTTPError as e:
            if e.response.status_code == 404 and allow_404:
                return None
            logger.warning(f"HTTP error fetching {url}: {e}")
            return None
        except requests.exceptions.RequestException as e:
            logger.error(f"Request error fetching {url}: {str(e)}")
            return None

    def get_user_archives(self, username: str, year: int = 2025, month: Optional[int] = None) -> List[str]:
        """Get list of monthly archive URLs for a user in a given year."""
        archives = []
        if month:
            # Single month
            month_str = f"{month:02d}"
            archive_url = f"{self.BASE_URL}/player/{username}/games/{year}/{month_str}"
            archives.append(archive_url)
        else:
            # All months in the year
            for m in range(1, 13):
                month_str = f"{m:02d}"
                archive_url = f"{self.BASE_URL}/player/{username}/games/{year}/{month_str}"
                archives.append(archive_url)
        return archives

    def get_monthly_games(self, archive_url: str) -> Optional[dict]:
        """Get games from a monthly archive."""
        return self._make_request(archive_url, allow_404=True)

    def fetch_user_games(self, username: str, year: Optional[int] = None, month: Optional[int] = None, include_tcn: bool = False, rated_only: bool = True) -> List:
        """
        Fetch all games for a user, optionally filtered by year and month.

        Args:
            username: Chess.com username
            year: Year to fetch (None = all time)
            month: Month to fetch (1-12, None = all months in year)
            include_tcn: If True, return list of (pgn, tcn) tuples
            rated_only: If True (default), skip unrated/casual games

        Returns:
            List of PGN strings, or list of (pgn, tcn) tuples if include_tcn=True
        """
        all_games = []
        skipped_unrated = 0
        skipped_variant = 0

        if year:
            archives = self.get_user_archives(username, year=year, month=month)
        else:
            archives_url = f"{self.BASE_URL}/player/{username}/games/archives"
            archives_data = self._make_request(archives_url)
            if not archives_data or 'archives' not in archives_data:
                return []
            archives = archives_data['archives']

        if not archives:
            return []

        for archive_url in archives:
            monthly_data = self.get_monthly_games(archive_url)

            if not monthly_data:
                continue

            games = monthly_data.get('games', [])

            if not games:
                continue

            for game in games:
                if rated_only and not game.get('rated', True):
                    skipped_unrated += 1
                    continue

                rules = game.get('rules', 'chess')
                if rules != 'chess':
                    skipped_variant += 1
                    continue

                if 'pgn' in game:
                    if include_tcn:
                        all_games.append((game['pgn'], game.get('tcn')))
                    else:
                        all_games.append(game['pgn'])

            time.sleep(self.RATE_LIMIT_DELAY)

        if skipped_unrated > 0:
            logger.info(f"Skipped {skipped_unrated} unrated games for {username}")
        if skipped_variant > 0:
            logger.info(f"Skipped {skipped_variant} variant games for {username}")

        return all_games
