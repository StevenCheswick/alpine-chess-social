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

        # Index for faster lookups
        cursor.execute("""
            CREATE INDEX IF NOT EXISTS idx_user_games_user_id
            ON user_games(user_id)
        """)
        cursor.execute("""
            CREATE INDEX IF NOT EXISTS idx_user_games_date
            ON user_games(date DESC)
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
    """Insert or update multiple games for a user (batched for performance)."""
    if not games:
        return 0

    with get_connection() as conn:
        cursor = conn.cursor()
        for game in games:
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


def get_user_tag_counts(user_id: int) -> Dict[str, int]:
    """Get tag counts for a user's games."""
    with get_connection() as conn:
        cursor = conn.cursor()
        cursor.execute(
            "SELECT tags FROM user_games WHERE user_id = ? AND tags IS NOT NULL AND tags != '[]'",
            (user_id,)
        )

        tag_counts: Dict[str, int] = {}
        for row in cursor.fetchall():
            tags = json.loads(row["tags"])
            for tag in tags:
                tag_counts[tag] = tag_counts.get(tag, 0) + 1

        return tag_counts


def get_user_games_paginated(
    user_id: int,
    limit: int = 50,
    offset: int = 0,
    tag_filter: Optional[str] = None
) -> List[Dict[str, Any]]:
    """Get games for a user with pagination and optional tag filter."""
    with get_connection() as conn:
        cursor = conn.cursor()

        if tag_filter:
            # Filter by tag (using LIKE for JSON array search)
            cursor.execute("""
                SELECT id, chess_com_game_id, opponent, opponent_rating, user_rating,
                       result, user_color, time_control, date, moves, tags
                FROM user_games
                WHERE user_id = ? AND tags LIKE ?
                ORDER BY date DESC
                LIMIT ? OFFSET ?
            """, (user_id, f'%"{tag_filter}"%', limit, offset))
        else:
            cursor.execute("""
                SELECT id, chess_com_game_id, opponent, opponent_rating, user_rating,
                       result, user_color, time_control, date, moves, tags
                FROM user_games
                WHERE user_id = ?
                ORDER BY date DESC
                LIMIT ? OFFSET ?
            """, (user_id, limit, offset))

        games = []
        for row in cursor.fetchall():
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
                "moves": json.loads(row["moves"]) if row["moves"] else [],
                "tags": json.loads(row["tags"]) if row["tags"] else [],
            })

        return games


def get_user_games_count_filtered(user_id: int, tag_filter: Optional[str] = None) -> int:
    """Get count of games for a user, optionally filtered by tag."""
    with get_connection() as conn:
        cursor = conn.cursor()
        if tag_filter:
            cursor.execute(
                "SELECT COUNT(*) FROM user_games WHERE user_id = ? AND tags LIKE ?",
                (user_id, f'%"{tag_filter}"%')
            )
        else:
            cursor.execute("SELECT COUNT(*) FROM user_games WHERE user_id = ?", (user_id,))
        return cursor.fetchone()[0]


def get_unanalyzed_games(user_id: int, limit: int = 100) -> List[Dict[str, Any]]:
    """Get games that haven't been analyzed yet."""
    with get_connection() as conn:
        cursor = conn.cursor()
        cursor.execute("""
            SELECT * FROM user_games
            WHERE user_id = ? AND analyzed_at IS NULL
            ORDER BY date DESC
            LIMIT ?
        """, (user_id, limit))

        games = []
        for row in cursor.fetchall():
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
                "pgn": row["pgn"],
                "moves": json.loads(row["moves"]) if row["moves"] else [],
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
        for game_id in game_ids:
            tags = tags_map.get(game_id, [])
            cursor.execute("""
                UPDATE user_games
                SET analyzed_at = CURRENT_TIMESTAMP, tags = ?, updated_at = CURRENT_TIMESTAMP
                WHERE id = ?
            """, (json.dumps(tags), game_id))
            updated += cursor.rowcount
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
                   chess_com_username, bio, avatar_url, created_at
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
                   chess_com_username, bio, avatar_url, created_at
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
                   chess_com_username, bio, avatar_url, created_at
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
    chess_com_username: Optional[str] = None
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
                   bio, avatar_url, created_at
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
            "bio": row["bio"],
            "avatarUrl": row["avatar_url"],
            "createdAt": row["created_at"],
        }


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
                   result, user_color, time_control, date, moves, tags
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
                "moves": json.loads(row["moves"]) if row["moves"] else [],
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
                g.moves,
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
                    "moves": json.loads(row["moves"]) if row["moves"] else [],
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
                g.moves,
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
                    "moves": json.loads(row["moves"]) if row["moves"] else [],
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
                g.moves,
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
                "moves": json.loads(row["moves"]) if row["moves"] else [],
                "tags": json.loads(row["tags"]) if row["tags"] else [],
                "keyPositionIndex": row["key_position_index"] or 0,
            }

        return post


# Initialize database on module import
init_db()
