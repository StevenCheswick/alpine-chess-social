"""
SQLite database setup and operations.
"""
import sqlite3
import json
import os
from typing import List, Dict, Any, Optional
from contextlib import contextmanager

DATABASE_PATH = os.path.join(os.path.dirname(__file__), "..", "data", "chess.db")


def get_db_path() -> str:
    """Get the database path, creating directory if needed."""
    db_dir = os.path.dirname(DATABASE_PATH)
    os.makedirs(db_dir, exist_ok=True)
    return DATABASE_PATH


@contextmanager
def get_connection():
    """Context manager for database connections."""
    conn = sqlite3.connect(get_db_path())
    conn.row_factory = sqlite3.Row
    try:
        yield conn
        conn.commit()
    except Exception:
        conn.rollback()
        raise
    finally:
        conn.close()


def init_db():
    """Initialize the database schema."""
    with get_connection() as conn:
        cursor = conn.cursor()

        # Users table
        cursor.execute("""
            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                chess_com_username TEXT UNIQUE NOT NULL COLLATE NOCASE,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
        """)

        # User games table
        cursor.execute("""
            CREATE TABLE IF NOT EXISTS user_games (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                chess_com_game_id TEXT NOT NULL,
                opponent TEXT NOT NULL,
                opponent_rating INTEGER,
                user_rating INTEGER,
                result TEXT NOT NULL,
                user_color TEXT NOT NULL,
                time_control TEXT,
                date TEXT,
                pgn TEXT,
                moves TEXT,
                tags TEXT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (user_id) REFERENCES users(id),
                UNIQUE(user_id, chess_com_game_id)
            )
        """)

        # Index for faster lookups
        cursor.execute("""
            CREATE INDEX IF NOT EXISTS idx_user_games_user_id
            ON user_games(user_id)
        """)
        cursor.execute("""
            CREATE INDEX IF NOT EXISTS idx_user_games_date
            ON user_games(date DESC)
        """)


def get_or_create_user(username: str) -> int:
    """Get user ID, creating if doesn't exist."""
    with get_connection() as conn:
        cursor = conn.cursor()

        # Try to get existing user
        cursor.execute(
            "SELECT id FROM users WHERE chess_com_username = ? COLLATE NOCASE",
            (username,)
        )
        row = cursor.fetchone()

        if row:
            return row["id"]

        # Create new user
        cursor.execute(
            "INSERT INTO users (chess_com_username) VALUES (?)",
            (username,)
        )
        return cursor.lastrowid


def upsert_game(user_id: int, game: Dict[str, Any]) -> None:
    """Insert or update a game for a user."""
    with get_connection() as conn:
        cursor = conn.cursor()

        cursor.execute("""
            INSERT INTO user_games (
                user_id, chess_com_game_id, opponent, opponent_rating, user_rating,
                result, user_color, time_control, date, pgn, moves, tags
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(user_id, chess_com_game_id) DO UPDATE SET
                opponent = excluded.opponent,
                opponent_rating = excluded.opponent_rating,
                user_rating = excluded.user_rating,
                result = excluded.result,
                user_color = excluded.user_color,
                time_control = excluded.time_control,
                date = excluded.date,
                pgn = excluded.pgn,
                moves = excluded.moves,
                tags = excluded.tags,
                updated_at = CURRENT_TIMESTAMP
        """, (
            user_id,
            game["id"],
            game["opponent"],
            game.get("opponentRating"),
            game.get("userRating"),
            game["result"],
            game["userColor"],
            game.get("timeControl"),
            game.get("date"),
            game.get("pgn"),
            json.dumps(game.get("moves", [])),
            json.dumps(game.get("tags", []))
        ))


def upsert_games(user_id: int, games: List[Dict[str, Any]]) -> int:
    """Insert or update multiple games for a user."""
    count = 0
    for game in games:
        upsert_game(user_id, game)
        count += 1
    return count


def get_user_games(user_id: int) -> List[Dict[str, Any]]:
    """Get all games for a user."""
    with get_connection() as conn:
        cursor = conn.cursor()

        cursor.execute("""
            SELECT * FROM user_games
            WHERE user_id = ?
            ORDER BY date DESC
        """, (user_id,))

        games = []
        for row in cursor.fetchall():
            games.append({
                "id": row["chess_com_game_id"],
                "opponent": row["opponent"],
                "opponentRating": row["opponent_rating"],
                "userRating": row["user_rating"],
                "result": row["result"],
                "userColor": row["user_color"],
                "timeControl": row["time_control"],
                "date": row["date"],
                "moves": json.loads(row["moves"]) if row["moves"] else [],
                "tags": json.loads(row["tags"]) if row["tags"] else [],
            })

        return games


def get_user_games_count(user_id: int) -> int:
    """Get count of games for a user."""
    with get_connection() as conn:
        cursor = conn.cursor()
        cursor.execute("SELECT COUNT(*) FROM user_games WHERE user_id = ?", (user_id,))
        return cursor.fetchone()[0]


# Initialize database on module import
init_db()
