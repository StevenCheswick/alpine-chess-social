"""
Chess Social Media Backend API
"""
from fastapi import FastAPI, Query, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from typing import Optional, List, Dict, Any
from datetime import datetime
import re

from src.chess_com_client import ChessComClient
from src.pgn_parser import parse_pgns
from src.unified_analyzer import UnifiedAnalyzer
from src.analyzers.queen_sacrifice import UnifiedQueenSacrificeAnalyzer
from src import database as db

app = FastAPI(title="Chess Social Media API")

# Enable CORS for frontend
app.add_middleware(
    CORSMiddleware,
    allow_origins=["http://localhost:5173", "http://localhost:3000"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


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
