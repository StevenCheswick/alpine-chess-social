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
from src.lichess_client import LichessClient
from src.pgn_parser import parse_pgns
from src.unified_analyzer import UnifiedAnalyzer
from src.analyzers import (
    UnifiedQueenSacrificeAnalyzer,
    UnifiedKnightForkAnalyzer,
    UnifiedRookSacrificeAnalyzer,
    UnifiedBackRankMateAnalyzer,
    UnifiedBestGameAnalyzer,
    UnifiedBiggestComebackAnalyzer,
    UnifiedCaptureSequenceAnalyzer,
    UnifiedCastleMateAnalyzer,
    UnifiedClutchWinAnalyzer,
    UnifiedEnPassantMateAnalyzer,
    UnifiedHungQueenAnalyzer,
    UnifiedKingMateAnalyzer,
    UnifiedKingWalkAnalyzer,
    UnifiedKnightBishopMateAnalyzer,
    UnifiedKnightPromotionMateAnalyzer,
    UnifiedLongestGameAnalyzer,
    UnifiedPawnMateAnalyzer,
    UnifiedPromotionMateAnalyzer,
    UnifiedQuickestMateAnalyzer,
    UnifiedSmotheredMateAnalyzer,
    UnifiedStalemateAnalyzer,
    UnifiedWindmillAnalyzer,
)

# Mapping of analyzer classes to their display tag names
ANALYZER_TAGS = {
    UnifiedQueenSacrificeAnalyzer: "Queen Sacrifice",
    UnifiedKnightForkAnalyzer: "Knight Fork",
    UnifiedRookSacrificeAnalyzer: "Rook Sacrifice",
    UnifiedBackRankMateAnalyzer: "Back Rank Mate",
    UnifiedSmotheredMateAnalyzer: "Smothered Mate",
    UnifiedKingMateAnalyzer: "King Mate",
    UnifiedCastleMateAnalyzer: "Castle Mate",
    UnifiedPawnMateAnalyzer: "Pawn Mate",
    UnifiedKnightPromotionMateAnalyzer: "Knight Promotion Mate",
    UnifiedPromotionMateAnalyzer: "Promotion Mate",
    UnifiedQuickestMateAnalyzer: "Quickest Mate",
    UnifiedEnPassantMateAnalyzer: "En Passant Mate",
    UnifiedKnightBishopMateAnalyzer: "Knight Bishop Mate",
    UnifiedKingWalkAnalyzer: "King Walk",
    UnifiedBiggestComebackAnalyzer: "Biggest Comeback",
    UnifiedClutchWinAnalyzer: "Clutch Win",
    UnifiedBestGameAnalyzer: "Best Game",
    UnifiedLongestGameAnalyzer: "Longest Game",
    UnifiedHungQueenAnalyzer: "Hung Queen",
    UnifiedCaptureSequenceAnalyzer: "Capture Sequence",
    UnifiedStalemateAnalyzer: "Stalemate",
    UnifiedWindmillAnalyzer: "Windmill",
}


def create_analyzers(username: str):
    """Create all analyzer instances for a user."""
    analyzers = {}
    for analyzer_class in ANALYZER_TAGS.keys():
        analyzers[analyzer_class] = analyzer_class(username)
    return analyzers


def setup_unified_analyzer(username: str):
    """Set up the unified analyzer with all registered analyzers."""
    analyzer = UnifiedAnalyzer(username)
    individual_analyzers = create_analyzers(username)
    for ind_analyzer in individual_analyzers.values():
        analyzer.register_analyzer(ind_analyzer)
    return analyzer, individual_analyzers


def build_game_tags_map(analyzers: dict) -> Dict[str, List[str]]:
    """Build a map of game_link -> tags from all analyzer findings.

    Call this ONCE after all games are analyzed, not per-game.
    Uses fast path (get_matched_game_links) when available to avoid
    expensive processing in get_final_results().
    """
    tags_map: Dict[str, List[str]] = {}

    for analyzer_class, analyzer in analyzers.items():
        tag = ANALYZER_TAGS.get(analyzer_class)
        if not tag:
            continue

        # Fast path: use get_matched_game_links if available
        if hasattr(analyzer, 'get_matched_game_links'):
            links = analyzer.get_matched_game_links()
            for link in links:
                if link:
                    if link not in tags_map:
                        tags_map[link] = []
                    if tag not in tags_map[link]:
                        tags_map[link].append(tag)

    return tags_map
from src import database as db
from src import auth
from src.opening_tree import build_opening_tree, convert_tree_for_response

app = FastAPI(title="Chess Social Media API")


# ============================================
# Pydantic models for request/response
# ============================================

class RegisterRequest(BaseModel):
    username: str
    email: EmailStr
    password: str
    chessComUsername: Optional[str] = None


class LoginRequest(BaseModel):
    email: EmailStr
    password: str


class UserResponse(BaseModel):
    id: int
    username: str
    displayName: str
    email: str
    chessComUsername: Optional[str] = None
    lichessUsername: Optional[str] = None
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
    allow_origins=["*"],
    allow_credentials=False,
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
        lichessUsername=account.get("lichessUsername", ""),
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
            chess_com_username=request.chessComUsername or "",
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
# Profile endpoints
# ============================================

class ProfileResponse(BaseModel):
    id: int
    username: str
    displayName: str
    chessComUsername: Optional[str] = None
    lichessUsername: Optional[str] = None
    bio: Optional[str] = None
    avatarUrl: Optional[str] = None
    createdAt: str
    gamesCount: int = 0
    isOwnProfile: bool = False


class UpdateProfileRequest(BaseModel):
    displayName: Optional[str] = None
    bio: Optional[str] = None
    chessComUsername: Optional[str] = None
    lichessUsername: Optional[str] = None


@app.get("/api/users/{username}", response_model=ProfileResponse)
async def get_user_profile(
    username: str,
    authorization: Optional[str] = Header(None)
):
    """Get a user's public profile by username."""
    profile = db.get_public_profile(username)

    if not profile:
        raise HTTPException(status_code=404, detail="User not found")

    # Get games count
    games_count = db.get_games_count_by_chess_com_username(
        profile.get("chessComUsername", "")
    )

    # Check if this is the current user's own profile
    current_user = get_current_user(authorization)
    is_own_profile = current_user is not None and current_user["id"] == profile["id"]

    return ProfileResponse(
        id=profile["id"],
        username=profile["username"],
        displayName=profile.get("displayName") or profile["username"],
        chessComUsername=profile.get("chessComUsername", ""),
        lichessUsername=profile.get("lichessUsername", ""),
        bio=profile.get("bio"),
        avatarUrl=profile.get("avatarUrl"),
        createdAt=profile.get("createdAt", ""),
        gamesCount=games_count,
        isOwnProfile=is_own_profile,
    )


@app.put("/api/users/me", response_model=UserResponse)
async def update_user_profile(
    request: UpdateProfileRequest,
    user: Dict[str, Any] = Depends(require_auth)
):
    """Update the current user's profile."""
    # Validate display name if provided
    if request.displayName is not None:
        if len(request.displayName) < 1:
            raise HTTPException(status_code=400, detail="Display name cannot be empty")
        if len(request.displayName) > 50:
            raise HTTPException(status_code=400, detail="Display name must be at most 50 characters")

    # Validate bio if provided
    if request.bio is not None:
        if len(request.bio) > 500:
            raise HTTPException(status_code=400, detail="Bio must be at most 500 characters")

    # Validate chess.com username if provided
    if request.chessComUsername is not None:
        if len(request.chessComUsername) > 50:
            raise HTTPException(status_code=400, detail="Chess.com username must be at most 50 characters")

    # Validate lichess username if provided
    if request.lichessUsername is not None:
        if len(request.lichessUsername) > 50:
            raise HTTPException(status_code=400, detail="Lichess username must be at most 50 characters")

    # Update the account
    updated_account = db.update_account(
        account_id=user["id"],
        display_name=request.displayName,
        bio=request.bio,
        chess_com_username=request.chessComUsername,
        lichess_username=request.lichessUsername,
    )

    if not updated_account:
        raise HTTPException(status_code=500, detail="Failed to update profile")

    return account_to_user_response(updated_account)


class GameResponse(BaseModel):
    """Game data for selection."""
    id: int
    chessComGameId: str
    opponent: str
    opponentRating: Optional[int] = None
    userRating: Optional[int] = None
    result: str
    userColor: str
    timeControl: Optional[str] = None
    date: Optional[str] = None
    tags: List[str] = []
    moves: List[str] = []


class UserGamesResponse(BaseModel):
    """Response for user's games list."""
    games: List[GameResponse]
    total: int


@app.get("/api/users/me/games", response_model=UserGamesResponse)
async def get_my_games(
    limit: int = Query(50, le=100, description="Max games to return"),
    user: Dict[str, Any] = Depends(require_auth)
):
    """Get the current user's synced games."""
    chess_com_username = user.get("chessComUsername", "")
    if not chess_com_username:
        return UserGamesResponse(games=[], total=0)

    games = db.get_games_by_chess_com_username(chess_com_username, limit)
    return UserGamesResponse(
        games=[GameResponse(**g) for g in games],
        total=len(games)
    )


# ============================================
# Posts models and endpoints
# ============================================

class CreatePostRequest(BaseModel):
    content: str
    postType: str  # 'text' or 'game_share'
    gameId: Optional[int] = None
    keyPositionIndex: Optional[int] = 0  # Move index to display from


class AuthorResponse(BaseModel):
    id: int
    username: str
    displayName: str
    avatarUrl: Optional[str] = None


class GameDataResponse(BaseModel):
    id: str
    opponent: str
    opponentRating: Optional[int] = None
    userRating: Optional[int] = None
    result: str
    userColor: str
    timeControl: Optional[str] = None
    date: Optional[str] = None
    moves: List[str] = []
    tags: List[str] = []
    keyPositionIndex: int = 0


class PostResponse(BaseModel):
    id: int
    author: AuthorResponse
    postType: str
    content: str
    gameData: Optional[GameDataResponse] = None
    createdAt: str


class PostsListResponse(BaseModel):
    posts: List[PostResponse]
    total: int
    hasMore: bool


@app.post("/api/posts", response_model=PostResponse)
async def create_post(
    request: CreatePostRequest,
    user: Dict[str, Any] = Depends(require_auth)
):
    """Create a new post."""
    # Validate content
    if not request.content or not request.content.strip():
        raise HTTPException(status_code=400, detail="Content cannot be empty")
    
    if len(request.content) > 2000:
        raise HTTPException(status_code=400, detail="Content must be at most 2000 characters")
    
    # Validate post type
    if request.postType not in ("text", "game_share"):
        raise HTTPException(status_code=400, detail="Invalid post type")
    
    # If game_share, gameId is required
    if request.postType == "game_share" and not request.gameId:
        raise HTTPException(status_code=400, detail="gameId is required for game_share posts")
    
    # Create the post
    post_id = db.create_post(
        account_id=user["id"],
        post_type=request.postType,
        content=request.content.strip(),
        game_id=request.gameId,
        key_position_index=request.keyPositionIndex or 0,
    )
    
    # Fetch and return the created post
    post = db.get_post_by_id(post_id)
    if not post:
        raise HTTPException(status_code=500, detail="Failed to create post")
    
    return PostResponse(
        id=post["id"],
        author=AuthorResponse(
            id=post["author"]["id"],
            username=post["author"]["username"],
            displayName=post["author"]["displayName"],
            avatarUrl=post["author"]["avatarUrl"],
        ),
        postType=post["postType"],
        content=post["content"],
        gameData=GameDataResponse(**post["gameData"]) if post["gameData"] else None,
        createdAt=post["createdAt"],
    )


@app.get("/api/posts", response_model=PostsListResponse)
async def get_posts(
    limit: int = Query(20, ge=1, le=100),
    offset: int = Query(0, ge=0),
    user: Dict[str, Any] = Depends(require_auth)
):
    """Get posts feed."""
    posts = db.get_posts(limit=limit, offset=offset)
    total = db.get_posts_count()
    
    post_responses = []
    for post in posts:
        post_responses.append(PostResponse(
            id=post["id"],
            author=AuthorResponse(
                id=post["author"]["id"],
                username=post["author"]["username"],
                displayName=post["author"]["displayName"],
                avatarUrl=post["author"]["avatarUrl"],
            ),
            postType=post["postType"],
            content=post["content"],
            gameData=GameDataResponse(**post["gameData"]) if post["gameData"] else None,
            createdAt=post["createdAt"],
        ))
    
    return PostsListResponse(
        posts=post_responses,
        total=total,
        hasMore=offset + limit < total,
    )


@app.get("/api/users/{username}/posts", response_model=PostsListResponse)
async def get_user_posts(
    username: str,
    limit: int = Query(20, le=100, description="Max posts to return"),
    offset: int = Query(0, ge=0, description="Offset for pagination"),
):
    """Get posts by a specific user."""
    posts = db.get_posts_by_username(username, limit=limit, offset=offset)
    total = db.get_posts_count_by_username(username)

    return PostsListResponse(
        posts=[PostResponse(
            id=p["id"],
            author=AuthorResponse(**p["author"]),
            postType=p["postType"],
            content=p["content"],
            gameData=GameDataResponse(**p["gameData"]) if p["gameData"] else None,
            createdAt=p["createdAt"],
        ) for p in posts],
        total=total,
        hasMore=offset + len(posts) < total,
    )


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
    months_back: int = Query(6, description="Number of months to fetch (default: 6)"),
    year: Optional[int] = Query(None, description="Specific year to fetch (overrides months_back)"),
    month: Optional[int] = Query(None, description="Specific month to fetch 1-12 (requires year)")
):
    """
    Fetch and analyze games for a user.
    Returns games with tags for detected patterns (Queen Sacrifice, etc.)
    Default: fetches last 6 months of games.
    """
    if not username:
        raise HTTPException(status_code=400, detail="Username is required")

    now = datetime.now()
    client = ChessComClient()
    all_pgn_tcn_pairs = []

    # If specific year/month provided, use that
    if year is not None:
        if month:
            print(f"Fetching games for {username} in {year}/{month:02d}...")
        else:
            print(f"Fetching games for {username} in {year} (all months)...")
        try:
            pgn_tcn_pairs = client.fetch_user_games(username, year=year, month=month, include_tcn=True)
            all_pgn_tcn_pairs.extend(pgn_tcn_pairs or [])
        except Exception as e:
            raise HTTPException(status_code=500, detail=f"Error fetching games: {str(e)}")
    else:
        # Fetch last N months
        print(f"Fetching games for {username} for last {months_back} months...")
        for i in range(months_back):
            # Calculate year/month going backwards
            target_month = now.month - i
            target_year = now.year
            while target_month <= 0:
                target_month += 12
                target_year -= 1
            
            try:
                pgn_tcn_pairs = client.fetch_user_games(username, year=target_year, month=target_month, include_tcn=True)
                if pgn_tcn_pairs:
                    all_pgn_tcn_pairs.extend(pgn_tcn_pairs)
                    print(f"  {target_year}/{target_month:02d}: {len(pgn_tcn_pairs)} games")
            except Exception as e:
                print(f"  {target_year}/{target_month:02d}: Error - {e}")

    if not all_pgn_tcn_pairs:
        return {"username": username, "months_back": months_back, "games": [], "total": 0}
    
    pgn_tcn_pairs = all_pgn_tcn_pairs

    # Parse PGNs
    pgns = [pgn for pgn, tcn in pgn_tcn_pairs]
    tcns = [tcn for pgn, tcn in pgn_tcn_pairs]
    games = parse_pgns(pgns, tcn_list=tcns)

    print(f"Parsed {len(games)} games")

    # Analyze games with all analyzers
    analyzer, individual_analyzers = setup_unified_analyzer(username)
    analyzer.analyze_games(games)

    # Build maps of game links to findings for each analyzer
    analyzer_findings = {}
    for analyzer_class, ind_analyzer in individual_analyzers.items():
        if hasattr(ind_analyzer, 'get_final_results'):
            findings = ind_analyzer.get_final_results()
            findings_map = {}
            for finding in findings:
                link = finding.get("game_metadata", {}).get("link")
                if link:
                    findings_map[link] = finding
            analyzer_findings[analyzer_class] = findings_map

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

        # Check if this game has tactical highlights
        tags = []
        if game.metadata.link:
            for analyzer_class, findings_map in analyzer_findings.items():
                if game.metadata.link in findings_map:
                    tag = ANALYZER_TAGS.get(analyzer_class)
                    if tag:
                        tags.append(tag)

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
            "tcn": game.tcn,
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
    limit: int = Query(50, le=200, description="Max games to return"),
    offset: int = Query(0, ge=0, description="Offset for pagination"),
    tags: Optional[str] = Query(None, description="Comma-separated list of tags to filter by (games must have ALL tags)"),
    platform: Optional[str] = Query(None, description="Platform filter: 'chess_com', 'lichess', or None for all"),
    user: Dict[str, Any] = Depends(require_auth)
):
    """
    Get previously synced games from the database with pagination and optional tag filter.
    Returns games from all platforms by default.
    """
    # Get user_id from both linked accounts
    chess_com_username = user.get("chessComUsername", "")
    lichess_username = user.get("lichessUsername", "")

    if not chess_com_username and not lichess_username:
        return {"games": [], "total": 0, "limit": limit, "offset": offset, "tags": None}

    # Use chess_com username as primary identifier, fallback to lichess
    primary_username = chess_com_username or lichess_username
    user_id = db.get_or_create_user(primary_username)

    # Validate platform if provided
    source = None
    if platform and platform in ("chess_com", "lichess"):
        source = platform

    tags_list = [t.strip() for t in tags.split(",") if t.strip()] if tags else None

    games = db.get_user_games_paginated(user_id, limit=limit, offset=offset, tag_filters=tags_list, source=source)
    total = db.get_user_games_count_filtered(user_id, tag_filters=tags_list, source=source)

    return {
        "platform": platform,
        "games": games,
        "total": total,
        "limit": limit,
        "offset": offset,
        "tags": tags_list,
        "hasMore": offset + len(games) < total,
    }


@app.get("/api/games/tags")
async def get_game_tags_endpoint(
    selected_tags: Optional[str] = Query(None, description="Comma-separated list of tags to filter by"),
    user: Dict[str, Any] = Depends(require_auth)
):
    """
    Get tag counts for a user's games (from all platforms).
    If selected_tags is provided, returns counts of games that have ALL selected tags AND each other tag.
    """
    chess_com_username = user.get("chessComUsername", "")
    lichess_username = user.get("lichessUsername", "")

    if not chess_com_username and not lichess_username:
        return {"tags": {}, "selectedTags": []}

    primary_username = chess_com_username or lichess_username
    user_id = db.get_or_create_user(primary_username)

    if selected_tags:
        tags_list = [t.strip() for t in selected_tags.split(",") if t.strip()]
        if tags_list:
            tag_counts = db.get_user_tag_counts_filtered(user_id, tags_list)
        else:
            tag_counts = db.get_user_tag_counts(user_id)
    else:
        tag_counts = db.get_user_tag_counts(user_id)

    return {
        "tags": tag_counts,
        "selectedTags": selected_tags.split(",") if selected_tags else [],
    }


@app.get("/api/games/{game_id}")
async def get_game_by_id(
    game_id: int,
    user: Dict[str, Any] = Depends(require_auth)
):
    """
    Get a single game by its ID.
    """
    chess_com_username = user.get("chessComUsername", "")
    lichess_username = user.get("lichessUsername", "")

    if not chess_com_username and not lichess_username:
        raise HTTPException(status_code=404, detail="Game not found")

    primary_username = chess_com_username or lichess_username
    user_id = db.get_or_create_user(primary_username)

    game = db.get_game_by_id(user_id, game_id)
    if not game:
        raise HTTPException(status_code=404, detail="Game not found")

    return game


@app.get("/api/games/{game_id}/analysis")
async def get_game_analysis(
    game_id: int,
    user: Dict[str, Any] = Depends(require_auth)
):
    """
    Get analysis results for a game.
    """
    chess_com_username = user.get("chessComUsername", "")
    lichess_username = user.get("lichessUsername", "")

    if not chess_com_username and not lichess_username:
        raise HTTPException(status_code=404, detail="Game not found")

    primary_username = chess_com_username or lichess_username
    user_id = db.get_or_create_user(primary_username)

    # Verify the game belongs to this user
    game = db.get_game_by_id(user_id, game_id)
    if not game:
        raise HTTPException(status_code=404, detail="Game not found")

    analysis = db.get_game_analysis(game_id)
    if not analysis:
        return None

    return analysis


@app.post("/api/games/{game_id}/analysis")
async def save_game_analysis(
    game_id: int,
    analysis: Dict[str, Any],
    user: Dict[str, Any] = Depends(require_auth)
):
    """
    Save analysis results for a game.
    """
    chess_com_username = user.get("chessComUsername", "")
    lichess_username = user.get("lichessUsername", "")

    if not chess_com_username and not lichess_username:
        raise HTTPException(status_code=404, detail="Game not found")

    primary_username = chess_com_username or lichess_username
    user_id = db.get_or_create_user(primary_username)

    # Verify the game belongs to this user
    game = db.get_game_by_id(user_id, game_id)
    if not game:
        raise HTTPException(status_code=404, detail="Game not found")

    db.save_game_analysis(game_id, analysis)
    return {"success": True}


class OpeningTreeResponse(BaseModel):
    """Response for opening tree endpoint."""
    color: str
    rootNode: Dict[str, Any]
    totalGames: int
    depth: int


@app.get("/api/opening-tree", response_model=OpeningTreeResponse)
async def get_opening_tree(
    color: str = Query(..., description="Color to build tree for: 'white' or 'black'"),
    rebuild: bool = Query(False, description="Force rebuild of cached tree"),
    user: Dict[str, Any] = Depends(require_auth)
):
    """
    Get opening tree for the user's repertoire.
    Returns cached tree if available, otherwise builds and caches it.
    """
    MAX_DEPTH = 15

    # Validate color
    color_lower = color.lower()
    if color_lower not in ("white", "black"):
        raise HTTPException(status_code=400, detail="Color must be 'white' or 'black'")

    chess_com_username = user.get("chessComUsername", "")
    if not chess_com_username:
        raise HTTPException(status_code=400, detail="No Chess.com username linked to account")

    user_id = db.get_or_create_user(chess_com_username)

    # Check for cached tree (unless rebuild requested)
    if not rebuild:
        cached = db.get_cached_opening_tree(user_id, color_lower)
        if cached:
            return OpeningTreeResponse(
                color=color_lower,
                rootNode=cached["tree"],
                totalGames=cached["totalGames"],
                depth=MAX_DEPTH,
            )

    # No cache or rebuild requested - build the tree
    games = db.get_user_games_by_color(user_id, color_lower)

    if not games:
        # Return empty tree (don't cache empty trees)
        return OpeningTreeResponse(
            color=color_lower,
            rootNode={
                "move": "start",
                "fen": "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                "games": 0,
                "wins": 0,
                "losses": 0,
                "draws": 0,
                "winRate": 0,
                "children": [],
            },
            totalGames=0,
            depth=MAX_DEPTH,
        )

    # Build the tree
    tree = build_opening_tree(games, max_depth=MAX_DEPTH)
    root_node = convert_tree_for_response(tree)

    # Cache the result
    db.save_opening_tree(user_id, color_lower, root_node, len(games))

    return OpeningTreeResponse(
        color=color_lower,
        rootNode=root_node,
        totalGames=len(games),
        depth=MAX_DEPTH,
    )


class SyncResponse(BaseModel):
    username: str
    synced: int
    total: int
    lastSyncedAt: Optional[str] = None
    isFirstSync: bool = False


@app.post("/api/games/sync")
async def sync_games(user: Dict[str, Any] = Depends(require_auth)):
    """
    Sync games from Chess.com.
    - First sync: Downloads ALL games (full history)
    - Re-sync: Only downloads current month's games (incremental)
    """
    chess_com_username = user.get("chessComUsername", "")
    if not chess_com_username:
        raise HTTPException(status_code=400, detail="No Chess.com username linked to account")

    # Get or create user record for game storage
    user_id = db.get_or_create_user(chess_com_username)
    last_synced = db.get_user_last_synced(user_id)
    is_first_sync = last_synced is None

    client = ChessComClient()
    all_pgn_tcn_pairs = []
    now = datetime.now()

    # Cap at 1000 games for initial sync
    MAX_GAMES = 1000

    if is_first_sync:
        # FIRST SYNC: Fetch games (capped at MAX_GAMES)
        print(f"First sync for {chess_com_username} - fetching up to {MAX_GAMES} games...")
        current_year, current_month = now.year, now.month

        while True:
            try:
                pgn_tcn_pairs = client.fetch_user_games(
                    chess_com_username, year=current_year, month=current_month, include_tcn=True
                )
                if pgn_tcn_pairs:
                    all_pgn_tcn_pairs.extend(pgn_tcn_pairs)
                    print(f"  {current_year}/{current_month:02d}: {len(pgn_tcn_pairs)} games (total: {len(all_pgn_tcn_pairs)})")

                    # Stop if we've hit the cap
                    if len(all_pgn_tcn_pairs) >= MAX_GAMES:
                        all_pgn_tcn_pairs = all_pgn_tcn_pairs[:MAX_GAMES]
                        print(f"  Reached {MAX_GAMES} game cap")
                        break
            except Exception as e:
                print(f"  {current_year}/{current_month:02d}: Error or no games - {e}")

            # Move to previous month
            current_month -= 1
            if current_month == 0:
                current_month = 12
                current_year -= 1

            # Stop at 2010 as safety limit
            if current_year < 2010:
                break

        print(f"First sync: {len(all_pgn_tcn_pairs)} games total")
    else:
        # RE-SYNC: Only fetch current month (incremental)
        print(f"Re-sync for {chess_com_username} - fetching current month only...")
        try:
            pgn_tcn_pairs = client.fetch_user_games(
                chess_com_username, year=now.year, month=now.month, include_tcn=True
            )
            if pgn_tcn_pairs:
                all_pgn_tcn_pairs.extend(pgn_tcn_pairs)
                print(f"  {now.year}/{now.month:02d}: {len(pgn_tcn_pairs)} games")
        except Exception as e:
            print(f"  {now.year}/{now.month:02d}: Error - {e}")

        print(f"Incremental sync: {len(all_pgn_tcn_pairs)} games to check")

    synced_count = 0
    if all_pgn_tcn_pairs:
        # Parse PGNs
        pgns = [pgn for pgn, tcn in all_pgn_tcn_pairs]
        tcns = [tcn for pgn, tcn in all_pgn_tcn_pairs]
        games = parse_pgns(pgns, tcn_list=tcns)

        print(f"Parsed {len(games)} games")

        # Build game records WITHOUT analysis (tags will be added by analyze endpoint)
        response_games = []
        for game in games:
            user_is_white = game.metadata.white.lower() == chess_com_username.lower()
            opponent = game.metadata.black if user_is_white else game.metadata.white

            opponent_elo_header = "BlackElo" if user_is_white else "WhiteElo"
            user_elo_header = "WhiteElo" if user_is_white else "BlackElo"

            opponent_elo_match = re.search(rf'\[{opponent_elo_header}\s+"(\d+)"\]', game.pgn)
            opponent_elo = int(opponent_elo_match.group(1)) if opponent_elo_match else None

            user_elo_match = re.search(rf'\[{user_elo_header}\s+"(\d+)"\]', game.pgn)
            user_elo = int(user_elo_match.group(1)) if user_elo_match else None

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
                "tcn": game.tcn,
                "pgn": game.pgn,  # Include PGN for analysis
                "tags": [],  # No tags until analyzed
            })

        # Save to database
        synced_count = db.upsert_games(user_id, response_games)
        print(f"Saved {synced_count} games to database for {chess_com_username}")

        # Invalidate opening tree cache (will be rebuilt on next request)
        db.invalidate_opening_trees(user_id)

        # Analyze the newly synced games
        analyzed_count, _ = analyze_user_games(user_id, chess_com_username, "chess_com", limit=synced_count)
        print(f"Analyzed {analyzed_count} games for {chess_com_username}")

    # Update last synced timestamp
    db.update_user_last_synced(user_id)
    new_last_synced = db.get_user_last_synced(user_id)

    # Get total games count
    total_games = db.get_user_games_count(user_id)

    return SyncResponse(
        username=chess_com_username,
        synced=synced_count,
        total=total_games,
        lastSyncedAt=new_last_synced,
        isFirstSync=is_first_sync,
    )


@app.post("/api/games/sync/lichess")
async def sync_lichess_games(user: Dict[str, Any] = Depends(require_auth)):
    """
    Sync games from Lichess.
    - First sync: Downloads ALL games (full history)
    - Re-sync: Downloads all games (Lichess API handles this efficiently)
    """
    lichess_username = user.get("lichessUsername", "")
    if not lichess_username:
        raise HTTPException(status_code=400, detail="No Lichess username linked to account")

    # Get or create user record for game storage (use lichess username)
    user_id = db.get_or_create_user(lichess_username)
    last_synced = db.get_user_last_synced(user_id)
    is_first_sync = last_synced is None

    client = LichessClient()
    all_pgn_pairs = []

    # First sync: cap at 1000 games. Re-sync: fetch all
    max_games = 1000 if is_first_sync else None
    print(f"{'First sync' if is_first_sync else 'Re-sync'} for {lichess_username}...")
    try:
        # Fetch games (returns list of (pgn, game_id) tuples)
        pgn_pairs = client.fetch_user_games(lichess_username, max_games=max_games)
        if pgn_pairs:
            all_pgn_pairs.extend(pgn_pairs)
            print(f"Fetched {len(pgn_pairs)} games from Lichess")
    except Exception as e:
        print(f"Error fetching Lichess games: {e}")
        raise HTTPException(status_code=500, detail=f"Failed to fetch games from Lichess: {str(e)}")

    synced_count = 0
    if all_pgn_pairs:
        # Parse PGNs
        pgns = [pgn for pgn, game_id in all_pgn_pairs]
        game_ids = [game_id for pgn, game_id in all_pgn_pairs]
        games = parse_pgns(pgns)

        print(f"Parsed {len(games)} Lichess games")

        # Build game records
        response_games = []
        for i, game in enumerate(games):
            user_is_white = game.metadata.white.lower() == lichess_username.lower()
            opponent = game.metadata.black if user_is_white else game.metadata.white

            opponent_elo_header = "BlackElo" if user_is_white else "WhiteElo"
            user_elo_header = "WhiteElo" if user_is_white else "BlackElo"

            opponent_elo_match = re.search(rf'\[{opponent_elo_header}\s+"(\d+)"\]', game.pgn)
            opponent_elo = int(opponent_elo_match.group(1)) if opponent_elo_match else None

            user_elo_match = re.search(rf'\[{user_elo_header}\s+"(\d+)"\]', game.pgn)
            user_elo = int(user_elo_match.group(1)) if user_elo_match else None

            # Use Lichess game ID or generate one
            lichess_game_id = game_ids[i] if i < len(game_ids) else None
            game_id = lichess_game_id or game.metadata.link or f"{game.metadata.white}_{game.metadata.black}_{game.metadata.date}"

            response_games.append({
                "id": game_id,
                "opponent": opponent,
                "opponentRating": opponent_elo,
                "userRating": user_elo,
                "result": get_result_code(game.metadata.result, user_is_white),
                "timeControl": game.metadata.time_control,
                "date": format_date(game.metadata.date),
                "userColor": "white" if user_is_white else "black",
                "moves": game.moves,
                "tcn": game.tcn,  # Generated from moves
                "pgn": game.pgn,
                "tags": [],  # No analysis for Lichess games
            })

        # Save to database with source='lichess'
        synced_count = db.upsert_games(user_id, response_games, source="lichess")
        print(f"Saved {synced_count} Lichess games to database for {lichess_username}")

        # Analyze the newly synced games
        analyzed_count, _ = analyze_user_games(user_id, lichess_username, "lichess", limit=synced_count)
        print(f"Analyzed {analyzed_count} Lichess games for {lichess_username}")

    # Update last synced timestamp
    db.update_user_last_synced(user_id)
    new_last_synced = db.get_user_last_synced(user_id)

    # Get total Lichess games count
    total_games = db.get_user_games_count(user_id, source="lichess")

    return SyncResponse(
        username=lichess_username,
        synced=synced_count,
        total=total_games,
        lastSyncedAt=new_last_synced,
        isFirstSync=is_first_sync,
    )


class AnalyzeResponse(BaseModel):
    analyzed: int
    remaining: int
    total: int
    skippedNoPgn: int = 0


ANALYZE_BATCH_SIZE = 100


def analyze_user_games(user_id: int, platform_username: str, source: str, limit: int = 1000) -> tuple[int, int]:
    """
    Analyze unanalyzed games for a user. Called automatically after sync.
    Returns (total_analyzed, total_skipped).
    """
    from src.game_data import GameData, GameMetadata

    total_analyzed = 0
    total_skipped = 0

    while total_analyzed < limit:
        batch_limit = min(ANALYZE_BATCH_SIZE, limit - total_analyzed)

        # Get next batch of unanalyzed games for this platform
        unanalyzed = db.get_unanalyzed_games(user_id, limit=batch_limit, source=source)
        if not unanalyzed:
            break

        print(f"Analyzing batch of {len(unanalyzed)} {source} games for {platform_username}...")

        # Convert to GameData format
        game_data_list = []
        game_id_map = {}

        for g in unanalyzed:
            # Check for moves (decoded from TCN)
            if not g.get("tcn") and not g.get("moves"):
                total_skipped += 1
                continue

            # Convert W/L/D to standard chess result
            user_color = g["userColor"]
            if g["result"] == "W":
                chess_result = "1-0" if user_color == "white" else "0-1"
            elif g["result"] == "L":
                chess_result = "0-1" if user_color == "white" else "1-0"
            else:
                chess_result = "1/2-1/2"

            metadata = GameMetadata(
                white=platform_username if user_color == "white" else g["opponent"],
                black=g["opponent"] if user_color == "white" else platform_username,
                result=chess_result,
                date=g["date"],
                time_control=g["timeControl"],
                link=g["chessComGameId"],
            )
            game_data = GameData(
                pgn="",
                moves=g["moves"],
                metadata=metadata,
            )
            game_data_list.append(game_data)
            game_id_map[g["chessComGameId"]] = g["id"]

        if not game_data_list:
            # Mark games with no moves as analyzed to prevent infinite loop
            no_moves_ids = [g["id"] for g in unanalyzed if not g.get("tcn") and not g.get("moves")]
            if no_moves_ids:
                db.mark_games_analyzed(no_moves_ids, {gid: [] for gid in no_moves_ids})
            continue

        # Run analyzers on this batch
        analyzer, individual_analyzers = setup_unified_analyzer(platform_username)
        analyzer.analyze_games(game_data_list)

        # Build tags map from analyzer findings
        game_tags = build_game_tags_map(individual_analyzers)

        # Map to database IDs
        tags_map = {}
        for game_data in game_data_list:
            link = game_data.metadata.link
            if link and link in game_id_map:
                db_id = game_id_map[link]
                tags_map[db_id] = game_tags.get(link, [])

        # Save this batch to database
        game_ids = list(tags_map.keys())
        updated = db.mark_games_analyzed(game_ids, tags_map)
        total_analyzed += updated
        print(f"Batch complete: {updated} games tagged (total: {total_analyzed})")

    return total_analyzed, total_skipped


@app.post("/api/games/analyze")
async def analyze_games(
    limit: int = Query(1000, le=5000, description="Max games to analyze"),
    platform: str = Query("chess_com", description="Platform: chess_com or lichess"),
    user: Dict[str, Any] = Depends(require_auth)
):
    """
    Analyze unanalyzed games and add tags.
    Note: Analysis now happens automatically during sync, but this endpoint
    can be used to re-analyze or catch up on any missed games.
    """
    if platform == "lichess":
        platform_username = user.get("lichessUsername", "")
        source = "lichess"
        if not platform_username:
            raise HTTPException(status_code=400, detail="No Lichess username linked to account")
    else:
        platform_username = user.get("chessComUsername", "")
        source = "chess_com"
        if not platform_username:
            raise HTTPException(status_code=400, detail="No Chess.com username linked to account")

    user_id = db.get_or_create_user(platform_username)

    total_analyzed, total_skipped = analyze_user_games(user_id, platform_username, source, limit)

    remaining = db.get_unanalyzed_games_count(user_id)
    total = db.get_user_games_count(user_id, source=source)

    return AnalyzeResponse(
        analyzed=total_analyzed,
        remaining=remaining,
        total=total,
        skippedNoPgn=total_skipped,
    )


@app.get("/health")
async def health_check():
    """Health check endpoint."""
    return {"status": "ok"}
