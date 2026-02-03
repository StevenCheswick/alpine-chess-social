"""
Chess Social Media Backend API
"""
from fastapi import FastAPI, Query, HTTPException, Header, Depends
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel, EmailStr
from typing import Optional, List, Dict, Any
from datetime import datetime
import re

from src.chess_com_client import ChessComClient
from src.pgn_parser import parse_pgns
from src.unified_analyzer import UnifiedAnalyzer
from src.analyzers.queen_sacrifice import UnifiedQueenSacrificeAnalyzer
from src import database as db
from src import auth

app = FastAPI(title="Chess Social Media API")


# ============================================
# Pydantic models for request/response
# ============================================

class RegisterRequest(BaseModel):
    username: str
    email: EmailStr
    password: str
    chessComUsername: str


class LoginRequest(BaseModel):
    email: EmailStr
    password: str


class UserResponse(BaseModel):
    id: int
    username: str
    displayName: str
    email: str
    chessComUsername: str
    bio: Optional[str] = None
    avatarUrl: Optional[str] = None
    createdAt: str
    isVerified: bool = False
    followerCount: int = 0
    followingCount: int = 0


class AuthResponse(BaseModel):
    user: UserResponse
    token: str

# Enable CORS for frontend
app.add_middleware(
    CORSMiddleware,
    allow_origins=["http://localhost:5173", "http://localhost:5174", "http://localhost:5175", "http://localhost:3000"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


# ============================================
# Auth helper functions
# ============================================

def get_current_user(authorization: Optional[str] = Header(None)) -> Optional[Dict[str, Any]]:
    """Extract and verify the current user from the Authorization header."""
    if not authorization:
        return None

    # Expect "Bearer <token>"
    parts = authorization.split()
    if len(parts) != 2 or parts[0].lower() != "bearer":
        return None

    token = parts[1]
    user_id = auth.get_user_id_from_token(token)

    if user_id is None:
        return None

    return db.get_account_by_id(user_id)


def require_auth(authorization: Optional[str] = Header(None)) -> Dict[str, Any]:
    """Dependency that requires authentication."""
    user = get_current_user(authorization)
    if user is None:
        raise HTTPException(status_code=401, detail="Not authenticated")
    return user


def account_to_user_response(account: Dict[str, Any]) -> UserResponse:
    """Convert account dict to UserResponse."""
    return UserResponse(
        id=account["id"],
        username=account["username"],
        displayName=account.get("displayName") or account["username"],
        email=account["email"],
        chessComUsername=account.get("chessComUsername", ""),
        bio=account.get("bio"),
        avatarUrl=account.get("avatarUrl"),
        createdAt=account.get("createdAt", ""),
        isVerified=False,
        followerCount=0,
        followingCount=0,
    )


# ============================================
# Auth endpoints
# ============================================

@app.post("/api/auth/register", response_model=AuthResponse)
async def register(request: RegisterRequest):
    """Register a new user account."""
    # Validate username length
    if len(request.username) < 3:
        raise HTTPException(status_code=400, detail="Username must be at least 3 characters")
    if len(request.username) > 20:
        raise HTTPException(status_code=400, detail="Username must be at most 20 characters")

    # Validate username format (alphanumeric and underscores only)
    if not re.match(r'^[a-zA-Z0-9_]+$', request.username):
        raise HTTPException(status_code=400, detail="Username can only contain letters, numbers, and underscores")

    # Validate password length
    if len(request.password) < 8:
        raise HTTPException(status_code=400, detail="Password must be at least 8 characters")

    # Check if email already exists
    if db.email_exists(request.email):
        raise HTTPException(status_code=400, detail="Email already registered")

    # Check if username already exists
    if db.username_exists(request.username):
        raise HTTPException(status_code=400, detail="Username already taken")

    # Hash password and create account
    password_hash = auth.hash_password(request.password)

    try:
        account_id = db.create_account(
            username=request.username,
            email=request.email,
            password_hash=password_hash,
            chess_com_username=request.chessComUsername,
        )
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to create account: {str(e)}")

    # Get the created account
    account = db.get_account_by_id(account_id)
    if not account:
        raise HTTPException(status_code=500, detail="Failed to retrieve created account")

    # Create JWT token
    token = auth.create_access_token({"user_id": account_id})

    return AuthResponse(
        user=account_to_user_response(account),
        token=token,
    )


@app.post("/api/auth/login", response_model=AuthResponse)
async def login(request: LoginRequest):
    """Login with email and password."""
    # Get account by email
    account = db.get_account_by_email(request.email)

    if not account:
        raise HTTPException(status_code=401, detail="Invalid email or password")

    # Verify password
    if not auth.verify_password(request.password, account["password_hash"]):
        raise HTTPException(status_code=401, detail="Invalid email or password")

    # Create JWT token
    token = auth.create_access_token({"user_id": account["id"]})

    return AuthResponse(
        user=account_to_user_response(account),
        token=token,
    )


@app.get("/api/auth/me", response_model=UserResponse)
async def get_current_user_endpoint(user: Dict[str, Any] = Depends(require_auth)):
    """Get the current authenticated user."""
    return account_to_user_response(user)


# ============================================
# Helper functions
# ============================================

def format_date(date_str: Optional[str]) -> str:
    """Format date string from PGN to ISO format."""
    if not date_str:
        return ""
    try:
        # PGN dates are like "2025.01.28"
        return date_str.replace(".", "-")
    except:
        return date_str


def get_result_code(result: str, user_is_white: bool) -> str:
    """Convert result string to W/L/D from user's perspective."""
    if result == "1-0":
        return "W" if user_is_white else "L"
    elif result == "0-1":
        return "L" if user_is_white else "W"
    else:
        return "D"


@app.get("/api/games")
async def get_games(
    username: str = Query(..., description="Chess.com username"),
    year: Optional[int] = Query(None, description="Year to fetch (default: current year)"),
    month: Optional[int] = Query(None, description="Month to fetch 1-12 (default: current month)")
):
    """
    Fetch and analyze games for a user.
    Returns games with tags for detected patterns (Queen Sacrifice, etc.)
    """
    if not username:
        raise HTTPException(status_code=400, detail="Username is required")

    # Default to current year, all months
    now = datetime.now()
    if year is None:
        year = now.year

    # Fetch games from Chess.com
    client = ChessComClient()
    if month:
        print(f"Fetching games for {username} in {year}/{month:02d}...")
    else:
        print(f"Fetching games for {username} in {year} (all months)...")

    try:
        pgn_tcn_pairs = client.fetch_user_games(username, year=year, month=month, include_tcn=True)
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Error fetching games: {str(e)}")

    if not pgn_tcn_pairs:
        return {"username": username, "year": year, "month": month, "games": [], "total": 0}

    # Parse PGNs
    pgns = [pgn for pgn, tcn in pgn_tcn_pairs]
    tcns = [tcn for pgn, tcn in pgn_tcn_pairs]
    games = parse_pgns(pgns, tcn_list=tcns)

    print(f"Parsed {len(games)} games")

    # Analyze games with Queen Sacrifice analyzer
    analyzer = UnifiedAnalyzer(username)
    queen_sac_analyzer = UnifiedQueenSacrificeAnalyzer(username)
    analyzer.register_analyzer(queen_sac_analyzer)

    # Analyze all games
    analyzer.analyze_games(games)

    # Get queen sacrifice findings
    queen_sac_findings = queen_sac_analyzer.get_final_results()

    # Build a set of game links that have queen sacrifices
    queen_sac_games = {}
    for finding in queen_sac_findings:
        link = finding.get("game_metadata", {}).get("link")
        if link:
            queen_sac_games[link] = finding

    # Build response with all games
    response_games = []
    for game in games:
        user_is_white = game.metadata.white.lower() == username.lower()
        opponent = game.metadata.black if user_is_white else game.metadata.white

        # Get opponent's ELO
        opponent_elo_header = "BlackElo" if user_is_white else "WhiteElo"
        user_elo_header = "WhiteElo" if user_is_white else "BlackElo"

        opponent_elo_match = re.search(rf'\[{opponent_elo_header}\s+"(\d+)"\]', game.pgn)
        opponent_elo = int(opponent_elo_match.group(1)) if opponent_elo_match else None

        user_elo_match = re.search(rf'\[{user_elo_header}\s+"(\d+)"\]', game.pgn)
        user_elo = int(user_elo_match.group(1)) if user_elo_match else None

        # Check if this game has a queen sacrifice
        tags = []
        if game.metadata.link and game.metadata.link in queen_sac_games:
            tags.append("Queen Sacrifice")

        response_games.append({
            "id": game.metadata.link or f"{game.metadata.white}_{game.metadata.black}_{game.metadata.date}",
            "opponent": opponent,
            "opponentRating": opponent_elo,
            "userRating": user_elo,
            "result": get_result_code(game.metadata.result, user_is_white),
            "timeControl": game.metadata.time_control,
            "date": format_date(game.metadata.date),
            "userColor": "white" if user_is_white else "black",
            "moves": game.moves,
            "tags": tags,
        })

    # Sort by date descending (most recent first)
    response_games.sort(key=lambda g: g["date"] or "", reverse=True)

    # Save to database
    user_id = db.get_or_create_user(username)
    saved_count = db.upsert_games(user_id, response_games)
    print(f"Saved {saved_count} games to database for {username}")

    # Return ALL stored games (not just this sync)
    all_games = db.get_user_games(user_id)

    return {
        "username": username,
        "year": year,
        "month": month,
        "games": all_games,
        "total": len(all_games),
        "synced": saved_count,
    }


@app.get("/api/games/stored")
async def get_stored_games(
    username: str = Query(..., description="Chess.com username")
):
    """
    Get previously synced games from the database (no Chess.com fetch).
    """
    if not username:
        raise HTTPException(status_code=400, detail="Username is required")

    user_id = db.get_or_create_user(username)
    games = db.get_user_games(user_id)

    return {
        "username": username,
        "games": games,
        "total": len(games),
    }


@app.get("/health")
async def health_check():
    """Health check endpoint."""
    return {"status": "ok"}
