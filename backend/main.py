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
from src.analyzers.knight_fork import UnifiedKnightForkAnalyzer
from src.analyzers.rook_sacrifice import UnifiedRookSacrificeAnalyzer
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
    bio: Optional[str] = None
    avatarUrl: Optional[str] = None
    createdAt: str
    gamesCount: int = 0
    isOwnProfile: bool = False


class UpdateProfileRequest(BaseModel):
    displayName: Optional[str] = None
    bio: Optional[str] = None
    chessComUsername: Optional[str] = None


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

    # Update the account
    updated_account = db.update_account(
        account_id=user["id"],
        display_name=request.displayName,
        bio=request.bio,
        chess_com_username=request.chessComUsername,
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

    # Analyze games with analyzers
    analyzer = UnifiedAnalyzer(username)
    queen_sac_analyzer = UnifiedQueenSacrificeAnalyzer(username)
    knight_fork_analyzer = UnifiedKnightForkAnalyzer(username)
    rook_sac_analyzer = UnifiedRookSacrificeAnalyzer(username)
    analyzer.register_analyzer(queen_sac_analyzer)
    analyzer.register_analyzer(knight_fork_analyzer)
    analyzer.register_analyzer(rook_sac_analyzer)

    # Analyze all games
    analyzer.analyze_games(games)

    # Get queen sacrifice findings
    queen_sac_findings = queen_sac_analyzer.get_final_results()

    # Get knight fork findings
    knight_fork_findings = knight_fork_analyzer.get_final_results()

    # Get rook sacrifice findings
    rook_sac_findings = rook_sac_analyzer.get_final_results()

    # Build a set of game links that have queen sacrifices
    queen_sac_games = {}
    for finding in queen_sac_findings:
        link = finding.get("game_metadata", {}).get("link")
        if link:
            queen_sac_games[link] = finding

    # Build a set of game links that have knight forks
    knight_fork_games = {}
    for finding in knight_fork_findings:
        link = finding.get("game_metadata", {}).get("link")
        if link:
            knight_fork_games[link] = finding

    # Build a set of game links that have rook sacrifices
    rook_sac_games = {}
    for finding in rook_sac_findings:
        link = finding.get("game_metadata", {}).get("link")
        if link:
            rook_sac_games[link] = finding

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
        if game.metadata.link and game.metadata.link in queen_sac_games:
            tags.append("Queen Sacrifice")
        if game.metadata.link and game.metadata.link in knight_fork_games:
            tags.append("Knight Fork")
        if game.metadata.link and game.metadata.link in rook_sac_games:
            tags.append("Rook Sacrifice")

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
