"""
SQLite database setup and operations.
"""
import sqlite3
import json
import os
import chess
from typing import List, Dict, Any, Optional
from contextlib import contextmanager
from .tcn_decoder import decode_tcn

DATABASE_PATH = os.path.join(os.path.dirname(__file__), "..", "data", "chess.db")


def tcn_to_san(tcn: Optional[str]) -> List[str]:
    """Decode TCN to SAN moves for frontend display.

    Args:
        tcn: TCN encoded string or None

    Returns:
        List of SAN move strings
    """
    if not tcn:
        return []

    try:
        moves = decode_tcn(tcn)
        board = chess.Board()
        san_moves = []
        for move in moves:
            if move in board.legal_moves:
                san_moves.append(board.san(move))
                board.push(move)
            else:
                break
        return san_moves
    except Exception:
        return []


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

        # Accounts table (for app authentication)
        cursor.execute("""
            CREATE TABLE IF NOT EXISTS accounts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT UNIQUE NOT NULL COLLATE NOCASE,
                email TEXT UNIQUE NOT NULL COLLATE NOCASE,
                password_hash TEXT NOT NULL,
                display_name TEXT,
                chess_com_username TEXT NOT NULL COLLATE NOCASE,
                bio TEXT,
                avatar_url TEXT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
        """)

        # Indexes for accounts
        cursor.execute("""
            CREATE INDEX IF NOT EXISTS idx_accounts_email
            ON accounts(email)
        """)
        cursor.execute("""
            CREATE INDEX IF NOT EXISTS idx_accounts_username
            ON accounts(username)
        """)

        # Users table (for chess.com game sync - legacy)
        cursor.execute("""
            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                chess_com_username TEXT UNIQUE NOT NULL COLLATE NOCASE,
                last_synced_at TIMESTAMP,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
        """)

        # Add last_synced_at column if it doesn't exist (migration)
        try:
            cursor.execute("ALTER TABLE users ADD COLUMN last_synced_at TIMESTAMP")
        except sqlite3.OperationalError:
            pass  # Column already exists

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
                analyzed_at TIMESTAMP,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (user_id) REFERENCES users(id),
                UNIQUE(user_id, chess_com_game_id)
            )
        """)

        # Add analyzed_at column if it doesn't exist (migration)
        try:
            cursor.execute("ALTER TABLE user_games ADD COLUMN analyzed_at TIMESTAMP")
        except sqlite3.OperationalError:
            pass  # Column already exists

        # Add source column for multi-platform support (migration)
        try:
            cursor.execute("ALTER TABLE user_games ADD COLUMN source TEXT DEFAULT 'chess_com'")
        except sqlite3.OperationalError:
            pass  # Column already exists

        # Add tcn column for compact move storage (migration)
        try:
            cursor.execute("ALTER TABLE user_games ADD COLUMN tcn TEXT")
        except sqlite3.OperationalError:
            pass  # Column already exists

        # Add lichess_username to accounts (migration)
        try:
            cursor.execute("ALTER TABLE accounts ADD COLUMN lichess_username TEXT COLLATE NOCASE")
        except sqlite3.OperationalError:
            pass  # Column already exists

        # Index for faster lookups
        cursor.execute("""
            CREATE INDEX IF NOT EXISTS idx_user_games_user_id
            ON user_games(user_id)
        """)
        cursor.execute("""
            CREATE INDEX IF NOT EXISTS idx_user_games_date
            ON user_games(date DESC)
        """)
        cursor.execute("""
            CREATE INDEX IF NOT EXISTS idx_user_games_source
            ON user_games(source)
        """)
        # Unique constraint for source + game_id combo
        cursor.execute("""
            CREATE UNIQUE INDEX IF NOT EXISTS idx_user_games_source_game
            ON user_games(user_id, source, chess_com_game_id)
        """)

        # Posts table
        cursor.execute("""
            CREATE TABLE IF NOT EXISTS posts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                account_id INTEGER NOT NULL,
                post_type TEXT NOT NULL,
                content TEXT NOT NULL,
                game_id INTEGER,
                key_position_index INTEGER DEFAULT 0,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (account_id) REFERENCES accounts(id),
                FOREIGN KEY (game_id) REFERENCES user_games(id)
            )
        """)

        # Indexes for posts
        cursor.execute("""
            CREATE INDEX IF NOT EXISTS idx_posts_account_id
            ON posts(account_id)
        """)
        cursor.execute("""
            CREATE INDEX IF NOT EXISTS idx_posts_created_at
            ON posts(created_at DESC)
        """)

        # Opening trees cache table
        cursor.execute("""
            CREATE TABLE IF NOT EXISTS user_opening_trees (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                color TEXT NOT NULL,
                tree_json TEXT NOT NULL,
                total_games INTEGER DEFAULT 0,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (user_id) REFERENCES users(id),
                UNIQUE(user_id, color)
            )
        """)

        # Normalized game tags table (replaces JSON tags column)
        cursor.execute("""
            CREATE TABLE IF NOT EXISTS game_tags (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                game_id INTEGER NOT NULL,
                tag TEXT NOT NULL,
                FOREIGN KEY (game_id) REFERENCES user_games(id) ON DELETE CASCADE,
                UNIQUE(game_id, tag)
            )
        """)

        # Indexes for fast tag queries
        cursor.execute("""
            CREATE INDEX IF NOT EXISTS idx_game_tags_game_id
            ON game_tags(game_id)
        """)
        cursor.execute("""
            CREATE INDEX IF NOT EXISTS idx_game_tags_tag
            ON game_tags(tag)
        """)

        # Game analysis table - stores Stockfish analysis results
        cursor.execute("""
            CREATE TABLE IF NOT EXISTS game_analysis (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                game_id INTEGER NOT NULL UNIQUE,
                white_accuracy REAL NOT NULL,
                black_accuracy REAL NOT NULL,
                white_avg_cp_loss REAL NOT NULL,
                black_avg_cp_loss REAL NOT NULL,
                white_classifications TEXT NOT NULL,
                black_classifications TEXT NOT NULL,
                moves TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (game_id) REFERENCES user_games(id) ON DELETE CASCADE
            )
        """)

        cursor.execute("""
            CREATE INDEX IF NOT EXISTS idx_game_analysis_game_id
            ON game_analysis(game_id)
        """)


def migrate_json_tags_to_table():
    """One-time migration: populate game_tags table from JSON tags column."""
    with get_connection() as conn:
        cursor = conn.cursor()

        # Check if migration already done (game_tags has data)
        cursor.execute("SELECT COUNT(*) FROM game_tags")
        if cursor.fetchone()[0] > 0:
            return  # Already migrated

        # Check if there are any games with tags to migrate
        cursor.execute("SELECT COUNT(*) FROM user_games WHERE tags IS NOT NULL AND tags != '[]'")
        games_with_tags = cursor.fetchone()[0]
        if games_with_tags == 0:
            return  # Nothing to migrate

        print(f"Migrating tags for {games_with_tags} games to normalized table...")

        # Fetch all games with tags
        cursor.execute("""
            SELECT id, tags FROM user_games
            WHERE tags IS NOT NULL AND tags != '[]'
        """)

        batch = []
        for row in cursor.fetchall():
            game_id = row[0]
            try:
                tags = json.loads(row[1]) if row[1] else []
                for tag in tags:
                    if tag and tag not in RESULT_TAGS:  # Skip Win/Loss/Draw virtual tags
                        batch.append((game_id, tag))
            except json.JSONDecodeError:
                continue

        # Batch insert
        if batch:
            cursor.executemany(
                "INSERT OR IGNORE INTO game_tags (game_id, tag) VALUES (?, ?)",
                batch
            )
            print(f"Migrated {len(batch)} tag entries to game_tags table.")


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


def get_user_last_synced(user_id: int) -> Optional[str]:
    """Get the last sync timestamp for a user."""
    with get_connection() as conn:
        cursor = conn.cursor()
        cursor.execute(
            "SELECT last_synced_at FROM users WHERE id = ?",
            (user_id,)
        )
        row = cursor.fetchone()
        return row["last_synced_at"] if row else None


def update_user_last_synced(user_id: int) -> None:
    """Update the last sync timestamp for a user to now."""
    with get_connection() as conn:
        cursor = conn.cursor()
        cursor.execute(
            "UPDATE users SET last_synced_at = CURRENT_TIMESTAMP WHERE id = ?",
            (user_id,)
        )


def upsert_games(user_id: int, games: List[Dict[str, Any]], source: str = "chess_com") -> int:
    """Insert or update multiple games for a user (batched for performance).

    Args:
        user_id: The user's database ID
        games: List of game dictionaries
        source: Platform source - 'chess_com' or 'lichess'

    Note: Only stores TCN for moves (compact). PGN and moves columns are deprecated.
    """
    if not games:
        return 0

    with get_connection() as conn:
        cursor = conn.cursor()
        for game in games:
            cursor.execute("""
                INSERT INTO user_games (
                    user_id, chess_com_game_id, opponent, opponent_rating, user_rating,
                    result, user_color, time_control, date, tags, source, tcn
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(user_id, source, chess_com_game_id) DO UPDATE SET
                    opponent = excluded.opponent,
                    opponent_rating = excluded.opponent_rating,
                    user_rating = excluded.user_rating,
                    result = excluded.result,
                    user_color = excluded.user_color,
                    time_control = excluded.time_control,
                    date = excluded.date,
                    tags = excluded.tags,
                    tcn = excluded.tcn,
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
                json.dumps(game.get("tags", [])),
                source,
                game.get("tcn")
            ))
        # Single commit for all games
        return len(games)


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
                "moves": tcn_to_san(row["tcn"]),
                "tags": json.loads(row["tags"]) if row["tags"] else [],
            })

        return games


def get_game_by_id(user_id: int, game_id: int) -> Optional[Dict[str, Any]]:
    """Get a single game by its ID for a user.

    Args:
        user_id: The user's database ID
        game_id: The game's internal database ID

    Returns:
        Game dict or None if not found
    """
    with get_connection() as conn:
        cursor = conn.cursor()
        cursor.execute("""
            SELECT ug.*, GROUP_CONCAT(gt.tag) as tags_str
            FROM user_games ug
            LEFT JOIN game_tags gt ON ug.id = gt.game_id
            WHERE ug.user_id = ? AND ug.id = ?
            GROUP BY ug.id
        """, (user_id, game_id))

        row = cursor.fetchone()
        if not row:
            return None

        tags = row["tags_str"].split(',') if row["tags_str"] else []
        return {
            "id": row["id"],
            "chessComGameId": row["chess_com_game_id"],
            "opponent": row["opponent"],
            "opponentRating": row["opponent_rating"],
            "userRating": row["user_rating"],
            "result": row["result"],
            "userColor": row["user_color"],
            "timeControl": row["time_control"],
            "date": row["date"],
            "moves": tcn_to_san(row["tcn"]),
            "tags": tags,
            "source": row["source"] if "source" in row.keys() else "chess_com",
        }


def save_game_analysis(game_id: int, analysis: Dict[str, Any]) -> bool:
    """Save analysis results for a game.

    Args:
        game_id: The game's internal database ID
        analysis: Analysis data matching the Lambda API format

    Returns:
        True if saved successfully
    """
    with get_connection() as conn:
        cursor = conn.cursor()
        cursor.execute("""
            INSERT OR REPLACE INTO game_analysis (
                game_id,
                white_accuracy,
                black_accuracy,
                white_avg_cp_loss,
                black_avg_cp_loss,
                white_classifications,
                black_classifications,
                moves
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        """, (
            game_id,
            analysis["white_accuracy"],
            analysis["black_accuracy"],
            analysis.get("white_avg_cp_loss", 0),
            analysis.get("black_avg_cp_loss", 0),
            json.dumps(analysis["white_classifications"]),
            json.dumps(analysis["black_classifications"]),
            json.dumps(analysis["moves"]),
        ))

        # Update the game's analyzed_at timestamp
        cursor.execute("""
            UPDATE user_games SET analyzed_at = CURRENT_TIMESTAMP WHERE id = ?
        """, (game_id,))

        return True


def get_game_analysis(game_id: int) -> Optional[Dict[str, Any]]:
    """Get analysis results for a game.

    Args:
        game_id: The game's internal database ID

    Returns:
        Analysis data or None if not analyzed
    """
    with get_connection() as conn:
        cursor = conn.cursor()
        cursor.execute("""
            SELECT * FROM game_analysis WHERE game_id = ?
        """, (game_id,))

        row = cursor.fetchone()
        if not row:
            return None

        return {
            "white_accuracy": row["white_accuracy"],
            "black_accuracy": row["black_accuracy"],
            "white_avg_cp_loss": row["white_avg_cp_loss"],
            "black_avg_cp_loss": row["black_avg_cp_loss"],
            "white_classifications": json.loads(row["white_classifications"]),
            "black_classifications": json.loads(row["black_classifications"]),
            "moves": json.loads(row["moves"]),
            "isComplete": True,
        }


def get_user_games_count(user_id: int, source: Optional[str] = None) -> int:
    """Get count of games for a user.

    Args:
        user_id: The user's database ID
        source: Optional platform filter - 'chess_com' or 'lichess'
    """
    with get_connection() as conn:
        cursor = conn.cursor()
        if source:
            cursor.execute("SELECT COUNT(*) FROM user_games WHERE user_id = ? AND source = ?", (user_id, source))
        else:
            cursor.execute("SELECT COUNT(*) FROM user_games WHERE user_id = ?", (user_id,))
        return cursor.fetchone()[0]


# Result tags are virtual tags derived from the result column
RESULT_TAGS = {"Win": "W", "Loss": "L", "Draw": "D"}
RESULT_TO_TAG = {"W": "Win", "L": "Loss", "D": "Draw"}

# Platform tags are virtual tags derived from the source column
PLATFORM_TAGS = {"Chess.com": "chess_com", "Lichess": "lichess"}
SOURCE_TO_TAG = {"chess_com": "Chess.com", "lichess": "Lichess"}


def get_user_tag_counts(user_id: int, source: Optional[str] = None) -> Dict[str, int]:
    """Get tag counts for a user's games, including Win/Loss/Draw and platform tags.

    Args:
        user_id: The user's database ID
        source: Optional platform filter - 'chess_com' or 'lichess'
    """
    with get_connection() as conn:
        cursor = conn.cursor()

        source_condition = " AND source = ?" if source else ""
        source_params = (user_id, source) if source else (user_id,)

        tag_counts: Dict[str, int] = {}

        # Get platform counts (Chess.com/Lichess)
        cursor.execute(
            "SELECT source, COUNT(*) as count FROM user_games WHERE user_id = ? GROUP BY source",
            (user_id,)
        )
        for row in cursor.fetchall():
            if row["source"] in SOURCE_TO_TAG:
                tag_counts[SOURCE_TO_TAG[row["source"]]] = row["count"]

        # Get result counts (Win/Loss/Draw)
        cursor.execute(
            f"SELECT result, COUNT(*) as count FROM user_games WHERE user_id = ?{source_condition} GROUP BY result",
            source_params
        )
        for row in cursor.fetchall():
            if row["result"] in RESULT_TO_TAG:
                tag_counts[RESULT_TO_TAG[row["result"]]] = row["count"]

        # Get regular tag counts using normalized game_tags table
        cursor.execute(f"""
            SELECT gt.tag, COUNT(*) as count
            FROM game_tags gt
            JOIN user_games ug ON gt.game_id = ug.id
            WHERE ug.user_id = ?{source_condition}
            GROUP BY gt.tag
        """, source_params)
        for row in cursor.fetchall():
            tag_counts[row["tag"]] = row["count"]

        return tag_counts


def get_user_tag_counts_filtered(user_id: int, selected_tags: List[str]) -> Dict[str, int]:
    """Get tag counts for games that have ALL selected tags.

    Returns counts of games that have ALL selected_tags AND each other tag.
    Handles Win/Loss/Draw and Chess.com/Lichess as virtual tags.
    """
    with get_connection() as conn:
        cursor = conn.cursor()

        # Separate virtual tags from regular tags
        result_filters = [RESULT_TAGS[t] for t in selected_tags if t in RESULT_TAGS]
        platform_filters = [PLATFORM_TAGS[t] for t in selected_tags if t in PLATFORM_TAGS]
        regular_tags = [t for t in selected_tags if t not in RESULT_TAGS and t not in PLATFORM_TAGS]

        # Build base conditions for user_games
        conditions = ["ug.user_id = ?"]
        params: List[Any] = [user_id]

        # Add platform filter (can only be one since Chess.com/Lichess are mutually exclusive)
        if platform_filters:
            conditions.append("ug.source = ?")
            params.append(platform_filters[0])

        # Add result filter (can only be one since Win/Loss/Draw are mutually exclusive)
        if result_filters:
            conditions.append("ug.result = ?")
            params.append(result_filters[0])

        # For regular tags, use subquery to find games with ALL selected tags
        if regular_tags:
            tag_placeholders = ','.join(['?' for _ in regular_tags])
            conditions.append(f"""
                ug.id IN (
                    SELECT game_id FROM game_tags
                    WHERE tag IN ({tag_placeholders})
                    GROUP BY game_id
                    HAVING COUNT(DISTINCT tag) = ?
                )
            """)
            params.extend(regular_tags)
            params.append(len(regular_tags))

        where_clause = ' AND '.join(conditions)

        tag_counts: Dict[str, int] = {}

        # Get platform counts for filtered games
        cursor.execute(
            f"SELECT ug.source, COUNT(*) as count FROM user_games ug WHERE {where_clause} GROUP BY ug.source",
            params
        )
        for row in cursor.fetchall():
            if row["source"] in SOURCE_TO_TAG:
                tag_counts[SOURCE_TO_TAG[row["source"]]] = row["count"]

        # Get result counts for filtered games
        cursor.execute(
            f"SELECT ug.result, COUNT(*) as count FROM user_games ug WHERE {where_clause} GROUP BY ug.result",
            params
        )
        for row in cursor.fetchall():
            if row["result"] in RESULT_TO_TAG:
                tag_counts[RESULT_TO_TAG[row["result"]]] = row["count"]

        # Get regular tag counts for filtered games using game_tags table
        cursor.execute(f"""
            SELECT gt.tag, COUNT(*) as count
            FROM game_tags gt
            JOIN user_games ug ON gt.game_id = ug.id
            WHERE {where_clause}
            GROUP BY gt.tag
        """, params)
        for row in cursor.fetchall():
            tag_counts[row["tag"]] = row["count"]

        return tag_counts


def get_user_games_paginated(
    user_id: int,
    limit: int = 50,
    offset: int = 0,
    tag_filters: Optional[List[str]] = None,
    source: Optional[str] = None
) -> List[Dict[str, Any]]:
    """Get games for a user with pagination and optional tag filters (must have ALL tags).

    Args:
        user_id: The user's database ID
        limit: Max games to return
        offset: Pagination offset
        tag_filters: Optional list of tags to filter by (must have ALL)
        source: Optional platform filter - 'chess_com' or 'lichess'

    Handles Win/Loss/Draw and Chess.com/Lichess as virtual tags.
    """
    with get_connection() as conn:
        cursor = conn.cursor()

        conditions = ["ug.user_id = ?"]
        params: List[Any] = [user_id]

        if source:
            conditions.append("ug.source = ?")
            params.append(source)

        if tag_filters:
            # Separate virtual tags from regular tags
            result_filters = [RESULT_TAGS[t] for t in tag_filters if t in RESULT_TAGS]
            platform_filters = [PLATFORM_TAGS[t] for t in tag_filters if t in PLATFORM_TAGS]
            regular_tags = [t for t in tag_filters if t not in RESULT_TAGS and t not in PLATFORM_TAGS]

            # Add platform filter
            if platform_filters:
                conditions.append("ug.source = ?")
                params.append(platform_filters[0])

            # Add result filter
            if result_filters:
                conditions.append("ug.result = ?")
                params.append(result_filters[0])

            # Add regular tag filters using game_tags table
            if regular_tags:
                tag_placeholders = ','.join(['?' for _ in regular_tags])
                conditions.append(f"""
                    ug.id IN (
                        SELECT game_id FROM game_tags
                        WHERE tag IN ({tag_placeholders})
                        GROUP BY game_id
                        HAVING COUNT(DISTINCT tag) = ?
                    )
                """)
                params.extend(regular_tags)
                params.append(len(regular_tags))

        query = f"""
            SELECT ug.id, ug.chess_com_game_id, ug.opponent, ug.opponent_rating, ug.user_rating,
                   ug.result, ug.user_color, ug.time_control, ug.date, ug.tcn, ug.source,
                   GROUP_CONCAT(DISTINCT gt.tag) as tags_str,
                   CASE WHEN ga.id IS NOT NULL THEN 1 ELSE 0 END as has_analysis,
                   ga.white_accuracy, ga.black_accuracy
            FROM user_games ug
            LEFT JOIN game_tags gt ON ug.id = gt.game_id
            LEFT JOIN game_analysis ga ON ug.id = ga.game_id
            WHERE {' AND '.join(conditions)}
            GROUP BY ug.id
            ORDER BY ug.date DESC
            LIMIT ? OFFSET ?
        """
        params.extend([limit, offset])
        cursor.execute(query, params)

        games = []
        for row in cursor.fetchall():
            tags = row["tags_str"].split(',') if row["tags_str"] else []
            game_data = {
                "id": row["id"],
                "chessComGameId": row["chess_com_game_id"],
                "opponent": row["opponent"],
                "opponentRating": row["opponent_rating"],
                "userRating": row["user_rating"],
                "result": row["result"],
                "userColor": row["user_color"],
                "timeControl": row["time_control"],
                "date": row["date"],
                "moves": tcn_to_san(row["tcn"]),
                "tags": tags,
                "source": row["source"],
                "hasAnalysis": bool(row["has_analysis"]),
            }
            # Include accuracy if analysis exists
            if row["has_analysis"]:
                game_data["whiteAccuracy"] = row["white_accuracy"]
                game_data["blackAccuracy"] = row["black_accuracy"]
            games.append(game_data)

        return games


def get_user_games_count_filtered(
    user_id: int,
    tag_filters: Optional[List[str]] = None,
    source: Optional[str] = None
) -> int:
    """Get count of games for a user, optionally filtered by tags (must have ALL tags).

    Args:
        user_id: The user's database ID
        tag_filters: Optional list of tags to filter by
        source: Optional platform filter - 'chess_com' or 'lichess'

    Handles Win/Loss/Draw and Chess.com/Lichess as virtual tags.
    """
    with get_connection() as conn:
        cursor = conn.cursor()

        conditions = ["user_id = ?"]
        params: List[Any] = [user_id]

        if source:
            conditions.append("source = ?")
            params.append(source)

        if tag_filters:
            # Separate virtual tags from regular tags
            result_filters = [RESULT_TAGS[t] for t in tag_filters if t in RESULT_TAGS]
            platform_filters = [PLATFORM_TAGS[t] for t in tag_filters if t in PLATFORM_TAGS]
            regular_tags = [t for t in tag_filters if t not in RESULT_TAGS and t not in PLATFORM_TAGS]

            # Add platform filter
            if platform_filters:
                conditions.append("source = ?")
                params.append(platform_filters[0])

            # Add result filter
            if result_filters:
                conditions.append("result = ?")
                params.append(result_filters[0])

            # Add regular tag filters using game_tags table
            if regular_tags:
                tag_placeholders = ','.join(['?' for _ in regular_tags])
                conditions.append(f"""
                    id IN (
                        SELECT game_id FROM game_tags
                        WHERE tag IN ({tag_placeholders})
                        GROUP BY game_id
                        HAVING COUNT(DISTINCT tag) = ?
                    )
                """)
                params.extend(regular_tags)
                params.append(len(regular_tags))

        query = f"SELECT COUNT(*) FROM user_games WHERE {' AND '.join(conditions)}"
        cursor.execute(query, params)
        return cursor.fetchone()[0]


def get_unanalyzed_games(user_id: int, limit: int = 100, source: str = None) -> List[Dict[str, Any]]:
    """Get games that haven't been analyzed yet."""
    with get_connection() as conn:
        cursor = conn.cursor()
        if source:
            cursor.execute("""
                SELECT * FROM user_games
                WHERE user_id = ? AND analyzed_at IS NULL AND source = ?
                ORDER BY date DESC
                LIMIT ?
            """, (user_id, source, limit))
        else:
            cursor.execute("""
                SELECT * FROM user_games
                WHERE user_id = ? AND analyzed_at IS NULL
                ORDER BY date DESC
                LIMIT ?
            """, (user_id, limit))

        games = []
        for row in cursor.fetchall():
            # Get moves from TCN if available, otherwise fall back to moves column
            if row["tcn"]:
                moves = tcn_to_san(row["tcn"])
            elif row["moves"]:
                moves = json.loads(row["moves"]) if isinstance(row["moves"], str) else row["moves"]
            else:
                moves = []

            games.append({
                "id": row["id"],
                "chessComGameId": row["chess_com_game_id"],
                "opponent": row["opponent"],
                "opponentRating": row["opponent_rating"],
                "userRating": row["user_rating"],
                "result": row["result"],
                "userColor": row["user_color"],
                "timeControl": row["time_control"],
                "date": row["date"],
                "tcn": row["tcn"],
                "moves": moves,
                "tags": json.loads(row["tags"]) if row["tags"] else [],
            })
        return games


def get_unanalyzed_games_count(user_id: int) -> int:
    """Get count of unanalyzed games for a user."""
    with get_connection() as conn:
        cursor = conn.cursor()
        cursor.execute(
            "SELECT COUNT(*) FROM user_games WHERE user_id = ? AND analyzed_at IS NULL",
            (user_id,)
        )
        return cursor.fetchone()[0]


def mark_games_analyzed(game_ids: List[int], tags_map: Dict[int, List[str]]) -> int:
    """Mark games as analyzed and update their tags."""
    if not game_ids:
        return 0

    with get_connection() as conn:
        cursor = conn.cursor()
        updated = 0
        tag_inserts = []

        for game_id in game_ids:
            tags = tags_map.get(game_id, [])

            # Update user_games table (keep JSON for backward compatibility)
            cursor.execute("""
                UPDATE user_games
                SET analyzed_at = CURRENT_TIMESTAMP, tags = ?, updated_at = CURRENT_TIMESTAMP
                WHERE id = ?
            """, (json.dumps(tags), game_id))
            updated += cursor.rowcount

            # Collect tags for batch insert into game_tags
            for tag in tags:
                if tag not in RESULT_TAGS:  # Skip virtual tags
                    tag_inserts.append((game_id, tag))

        # Delete existing tags for these games and insert new ones
        if game_ids:
            placeholders = ','.join(['?' for _ in game_ids])
            cursor.execute(f"DELETE FROM game_tags WHERE game_id IN ({placeholders})", game_ids)

        if tag_inserts:
            cursor.executemany(
                "INSERT OR IGNORE INTO game_tags (game_id, tag) VALUES (?, ?)",
                tag_inserts
            )

        return updated


# ============================================
# Account functions (for app authentication)
# ============================================

def create_account(
    username: str,
    email: str,
    password_hash: str,
    chess_com_username: str,
    display_name: Optional[str] = None
) -> int:
    """Create a new account. Returns the account ID."""
    with get_connection() as conn:
        cursor = conn.cursor()
        cursor.execute("""
            INSERT INTO accounts (username, email, password_hash, chess_com_username, display_name)
            VALUES (?, ?, ?, ?, ?)
        """, (username, email, password_hash, chess_com_username, display_name or username))
        return cursor.lastrowid


def get_account_by_email(email: str) -> Optional[Dict[str, Any]]:
    """Get an account by email address."""
    with get_connection() as conn:
        cursor = conn.cursor()
        cursor.execute("""
            SELECT id, username, email, password_hash, display_name,
                   chess_com_username, lichess_username, bio, avatar_url, created_at
            FROM accounts
            WHERE email = ? COLLATE NOCASE
        """, (email,))
        row = cursor.fetchone()

        if not row:
            return None

        return {
            "id": row["id"],
            "username": row["username"],
            "email": row["email"],
            "password_hash": row["password_hash"],
            "displayName": row["display_name"],
            "chessComUsername": row["chess_com_username"],
            "lichessUsername": row["lichess_username"],
            "bio": row["bio"],
            "avatarUrl": row["avatar_url"],
            "createdAt": row["created_at"],
        }


def get_account_by_username(username: str) -> Optional[Dict[str, Any]]:
    """Get an account by username."""
    with get_connection() as conn:
        cursor = conn.cursor()
        cursor.execute("""
            SELECT id, username, email, password_hash, display_name,
                   chess_com_username, lichess_username, bio, avatar_url, created_at
            FROM accounts
            WHERE username = ? COLLATE NOCASE
        """, (username,))
        row = cursor.fetchone()

        if not row:
            return None

        return {
            "id": row["id"],
            "username": row["username"],
            "email": row["email"],
            "password_hash": row["password_hash"],
            "displayName": row["display_name"],
            "chessComUsername": row["chess_com_username"],
            "lichessUsername": row["lichess_username"],
            "bio": row["bio"],
            "avatarUrl": row["avatar_url"],
            "createdAt": row["created_at"],
        }


def get_account_by_id(account_id: int) -> Optional[Dict[str, Any]]:
    """Get an account by ID."""
    with get_connection() as conn:
        cursor = conn.cursor()
        cursor.execute("""
            SELECT id, username, email, display_name,
                   chess_com_username, lichess_username, bio, avatar_url, created_at
            FROM accounts
            WHERE id = ?
        """, (account_id,))
        row = cursor.fetchone()

        if not row:
            return None

        return {
            "id": row["id"],
            "username": row["username"],
            "email": row["email"],
            "displayName": row["display_name"],
            "chessComUsername": row["chess_com_username"],
            "lichessUsername": row["lichess_username"],
            "bio": row["bio"],
            "avatarUrl": row["avatar_url"],
            "createdAt": row["created_at"],
        }


def email_exists(email: str) -> bool:
    """Check if an email is already registered."""
    with get_connection() as conn:
        cursor = conn.cursor()
        cursor.execute("SELECT 1 FROM accounts WHERE email = ? COLLATE NOCASE", (email,))
        return cursor.fetchone() is not None


def username_exists(username: str) -> bool:
    """Check if a username is already taken."""
    with get_connection() as conn:
        cursor = conn.cursor()
        cursor.execute("SELECT 1 FROM accounts WHERE username = ? COLLATE NOCASE", (username,))
        return cursor.fetchone() is not None


def update_account(
    account_id: int,
    display_name: Optional[str] = None,
    bio: Optional[str] = None,
    chess_com_username: Optional[str] = None,
    lichess_username: Optional[str] = None
) -> Optional[Dict[str, Any]]:
    """Update an account's profile fields. Returns updated account."""
    with get_connection() as conn:
        cursor = conn.cursor()

        # Build dynamic update query
        updates = []
        params = []

        if display_name is not None:
            updates.append("display_name = ?")
            params.append(display_name)

        if bio is not None:
            updates.append("bio = ?")
            params.append(bio)

        if chess_com_username is not None:
            updates.append("chess_com_username = ?")
            params.append(chess_com_username)

        if lichess_username is not None:
            updates.append("lichess_username = ?")
            params.append(lichess_username)

        if not updates:
            return get_account_by_id(account_id)

        params.append(account_id)

        cursor.execute(f"""
            UPDATE accounts
            SET {', '.join(updates)}
            WHERE id = ?
        """, params)

    return get_account_by_id(account_id)


def get_public_profile(username: str) -> Optional[Dict[str, Any]]:
    """Get public profile data by username (no password_hash or email)."""
    with get_connection() as conn:
        cursor = conn.cursor()
        cursor.execute("""
            SELECT id, username, display_name, chess_com_username,
                   lichess_username, bio, avatar_url, created_at
            FROM accounts
            WHERE username = ? COLLATE NOCASE
        """, (username,))
        row = cursor.fetchone()

        if not row:
            return None

        return {
            "id": row["id"],
            "username": row["username"],
            "displayName": row["display_name"],
            "chessComUsername": row["chess_com_username"],
            "lichessUsername": row["lichess_username"],
            "bio": row["bio"],
            "avatarUrl": row["avatar_url"],
            "createdAt": row["created_at"],
        }


def get_user_games_by_color(user_id: int, color: str) -> List[Dict[str, Any]]:
    """Get all games where user played as specified color (for opening tree).

    Args:
        user_id: The user's database ID
        color: 'white' or 'black'

    Returns:
        List of games with moves array and result
    """
    with get_connection() as conn:
        cursor = conn.cursor()
        cursor.execute("""
            SELECT chess_com_game_id, result, tcn
            FROM user_games
            WHERE user_id = ? AND LOWER(user_color) = LOWER(?)
            ORDER BY date DESC
        """, (user_id, color))

        games = []
        for row in cursor.fetchall():
            games.append({
                "id": row["chess_com_game_id"],
                "result": row["result"],
                "moves": tcn_to_san(row["tcn"]),
            })

        return games


# ============================================
# Opening Tree Cache functions
# ============================================

def get_cached_opening_tree(user_id: int, color: str) -> Optional[Dict[str, Any]]:
    """Get cached opening tree for a user and color.

    Returns:
        Dict with 'tree_json', 'total_games', 'updated_at' or None if not cached
    """
    with get_connection() as conn:
        cursor = conn.cursor()
        cursor.execute("""
            SELECT tree_json, total_games, updated_at
            FROM user_opening_trees
            WHERE user_id = ? AND color = ?
        """, (user_id, color.lower()))

        row = cursor.fetchone()
        if not row:
            return None

        return {
            "tree": json.loads(row["tree_json"]),
            "totalGames": row["total_games"],
            "updatedAt": row["updated_at"],
        }


def save_opening_tree(user_id: int, color: str, tree: Dict[str, Any], total_games: int) -> None:
    """Save or update cached opening tree for a user and color."""
    with get_connection() as conn:
        cursor = conn.cursor()
        cursor.execute("""
            INSERT INTO user_opening_trees (user_id, color, tree_json, total_games, updated_at)
            VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(user_id, color) DO UPDATE SET
                tree_json = excluded.tree_json,
                total_games = excluded.total_games,
                updated_at = CURRENT_TIMESTAMP
        """, (user_id, color.lower(), json.dumps(tree), total_games))


def invalidate_opening_trees(user_id: int) -> None:
    """Delete cached opening trees for a user (call after sync)."""
    with get_connection() as conn:
        cursor = conn.cursor()
        cursor.execute("DELETE FROM user_opening_trees WHERE user_id = ?", (user_id,))


def get_games_count_by_chess_com_username(chess_com_username: str) -> int:
    """Get count of synced games for a Chess.com username."""
    with get_connection() as conn:
        cursor = conn.cursor()
        # First get the user_id from users table (for game sync)
        cursor.execute(
            "SELECT id FROM users WHERE chess_com_username = ? COLLATE NOCASE",
            (chess_com_username,)
        )
        row = cursor.fetchone()
        if not row:
            return 0

        cursor.execute("SELECT COUNT(*) FROM user_games WHERE user_id = ?", (row["id"],))
        return cursor.fetchone()[0]


def get_games_by_chess_com_username(chess_com_username: str, limit: int = 50) -> List[Dict[str, Any]]:
    """Get games for a Chess.com username."""
    with get_connection() as conn:
        cursor = conn.cursor()
        # First get the user_id from users table
        cursor.execute(
            "SELECT id FROM users WHERE chess_com_username = ? COLLATE NOCASE",
            (chess_com_username,)
        )
        row = cursor.fetchone()
        if not row:
            return []

        user_id = row["id"]
        cursor.execute("""
            SELECT id, chess_com_game_id, opponent, opponent_rating, user_rating,
                   result, user_color, time_control, date, tcn, tags
            FROM user_games
            WHERE user_id = ?
            ORDER BY date DESC
            LIMIT ?
        """, (user_id, limit))

        games = []
        for row in cursor.fetchall():
            games.append({
                "id": row["id"],  # Database ID for game_share posts
                "chessComGameId": row["chess_com_game_id"],
                "opponent": row["opponent"],
                "opponentRating": row["opponent_rating"],
                "userRating": row["user_rating"],
                "result": row["result"],
                "userColor": row["user_color"],
                "timeControl": row["time_control"],
                "date": row["date"],
                "moves": tcn_to_san(row["tcn"]),
                "tags": json.loads(row["tags"]) if row["tags"] else [],
            })

        return games


# ============================================
# Posts functions
# ============================================

def create_post(
    account_id: int,
    post_type: str,
    content: str,
    game_id: Optional[int] = None,
    key_position_index: int = 0
) -> int:
    """Create a new post. Returns the post ID."""
    with get_connection() as conn:
        cursor = conn.cursor()
        cursor.execute("""
            INSERT INTO posts (account_id, post_type, content, game_id, key_position_index)
            VALUES (?, ?, ?, ?, ?)
        """, (account_id, post_type, content, game_id, key_position_index))
        return cursor.lastrowid


def get_posts(limit: int = 20, offset: int = 0) -> List[Dict[str, Any]]:
    """Get posts feed with author info and optional game data."""
    with get_connection() as conn:
        cursor = conn.cursor()
        
        cursor.execute("""
            SELECT
                p.id,
                p.post_type,
                p.content,
                p.game_id,
                p.key_position_index,
                p.created_at,
                p.updated_at,
                a.id as author_id,
                a.username as author_username,
                a.display_name as author_display_name,
                a.avatar_url as author_avatar_url,
                g.chess_com_game_id,
                g.opponent,
                g.opponent_rating,
                g.user_rating,
                g.result,
                g.user_color,
                g.time_control,
                g.date as game_date,
                g.tcn,
                g.tags
            FROM posts p
            JOIN accounts a ON p.account_id = a.id
            LEFT JOIN user_games g ON p.game_id = g.id
            ORDER BY p.created_at DESC
            LIMIT ? OFFSET ?
        """, (limit, offset))
        
        posts = []
        for row in cursor.fetchall():
            post = {
                "id": row["id"],
                "postType": row["post_type"],
                "content": row["content"],
                "createdAt": row["created_at"],
                "updatedAt": row["updated_at"],
                "author": {
                    "id": row["author_id"],
                    "username": row["author_username"],
                    "displayName": row["author_display_name"] or row["author_username"],
                    "avatarUrl": row["author_avatar_url"],
                },
                "gameData": None,
            }
            
            # Add game data if this is a game share
            if row["game_id"] and row["chess_com_game_id"]:
                post["gameData"] = {
                    "id": row["chess_com_game_id"],
                    "opponent": row["opponent"],
                    "opponentRating": row["opponent_rating"],
                    "userRating": row["user_rating"],
                    "result": row["result"],
                    "userColor": row["user_color"],
                    "timeControl": row["time_control"],
                    "date": row["game_date"],
                    "moves": tcn_to_san(row["tcn"]),
                    "tags": json.loads(row["tags"]) if row["tags"] else [],
                    "keyPositionIndex": row["key_position_index"] or 0,
                }

            posts.append(post)

        return posts


def get_posts_count() -> int:
    """Get total count of posts."""
    with get_connection() as conn:
        cursor = conn.cursor()
        cursor.execute("SELECT COUNT(*) FROM posts")
        return cursor.fetchone()[0]


def get_posts_by_username(username: str, limit: int = 20, offset: int = 0) -> List[Dict[str, Any]]:
    """Get posts by a specific user's username."""
    with get_connection() as conn:
        cursor = conn.cursor()

        cursor.execute("""
            SELECT
                p.id,
                p.post_type,
                p.content,
                p.game_id,
                p.key_position_index,
                p.created_at,
                p.updated_at,
                a.id as author_id,
                a.username as author_username,
                a.display_name as author_display_name,
                a.avatar_url as author_avatar_url,
                g.chess_com_game_id,
                g.opponent,
                g.opponent_rating,
                g.user_rating,
                g.result,
                g.user_color,
                g.time_control,
                g.date as game_date,
                g.tcn,
                g.tags
            FROM posts p
            JOIN accounts a ON p.account_id = a.id
            LEFT JOIN user_games g ON p.game_id = g.id
            WHERE a.username = ? COLLATE NOCASE
            ORDER BY p.created_at DESC
            LIMIT ? OFFSET ?
        """, (username, limit, offset))

        posts = []
        for row in cursor.fetchall():
            post = {
                "id": row["id"],
                "postType": row["post_type"],
                "content": row["content"],
                "createdAt": row["created_at"],
                "updatedAt": row["updated_at"],
                "author": {
                    "id": row["author_id"],
                    "username": row["author_username"],
                    "displayName": row["author_display_name"] or row["author_username"],
                    "avatarUrl": row["author_avatar_url"],
                },
                "gameData": None,
            }

            if row["game_id"] and row["chess_com_game_id"]:
                post["gameData"] = {
                    "id": row["chess_com_game_id"],
                    "opponent": row["opponent"],
                    "opponentRating": row["opponent_rating"],
                    "userRating": row["user_rating"],
                    "result": row["result"],
                    "userColor": row["user_color"],
                    "timeControl": row["time_control"],
                    "date": row["game_date"],
                    "moves": tcn_to_san(row["tcn"]),
                    "tags": json.loads(row["tags"]) if row["tags"] else [],
                    "keyPositionIndex": row["key_position_index"] or 0,
                }

            posts.append(post)

        return posts


def get_posts_count_by_username(username: str) -> int:
    """Get total count of posts by a user."""
    with get_connection() as conn:
        cursor = conn.cursor()
        cursor.execute("""
            SELECT COUNT(*) FROM posts p
            JOIN accounts a ON p.account_id = a.id
            WHERE a.username = ? COLLATE NOCASE
        """, (username,))
        return cursor.fetchone()[0]


def get_post_by_id(post_id: int) -> Optional[Dict[str, Any]]:
    """Get a single post by ID with author and game data."""
    with get_connection() as conn:
        cursor = conn.cursor()
        
        cursor.execute("""
            SELECT
                p.id,
                p.post_type,
                p.content,
                p.game_id,
                p.key_position_index,
                p.created_at,
                p.updated_at,
                a.id as author_id,
                a.username as author_username,
                a.display_name as author_display_name,
                a.avatar_url as author_avatar_url,
                g.chess_com_game_id,
                g.opponent,
                g.opponent_rating,
                g.user_rating,
                g.result,
                g.user_color,
                g.time_control,
                g.date as game_date,
                g.tcn,
                g.tags
            FROM posts p
            JOIN accounts a ON p.account_id = a.id
            LEFT JOIN user_games g ON p.game_id = g.id
            WHERE p.id = ?
        """, (post_id,))
        
        row = cursor.fetchone()
        if not row:
            return None
        
        post = {
            "id": row["id"],
            "postType": row["post_type"],
            "content": row["content"],
            "createdAt": row["created_at"],
            "updatedAt": row["updated_at"],
            "author": {
                "id": row["author_id"],
                "username": row["author_username"],
                "displayName": row["author_display_name"] or row["author_username"],
                "avatarUrl": row["author_avatar_url"],
            },
            "gameData": None,
        }
        
        if row["game_id"] and row["chess_com_game_id"]:
            post["gameData"] = {
                "id": row["chess_com_game_id"],
                "opponent": row["opponent"],
                "opponentRating": row["opponent_rating"],
                "userRating": row["user_rating"],
                "result": row["result"],
                "userColor": row["user_color"],
                "timeControl": row["time_control"],
                "date": row["game_date"],
                "moves": tcn_to_san(row["tcn"]),
                "tags": json.loads(row["tags"]) if row["tags"] else [],
                "keyPositionIndex": row["key_position_index"] or 0,
            }

        return post


# Initialize database on module import
init_db()
migrate_json_tags_to_table()
