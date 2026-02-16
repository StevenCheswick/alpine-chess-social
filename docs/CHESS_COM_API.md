# Chess.com Published-Data API Reference

Complete documentation for the Chess.com Public API (PubAPI).

**Base URL:** `https://api.chess.com/pub`

## Overview

The PubAPI is a **read-only** REST API that provides JSON-LD data. It repackages all publicly available data from Chess.com including player profiles, game archives, clubs, tournaments, and more.

- **Authentication:** Not required (public data only)
- **Rate Limiting:** Be respectful; add delays between requests (~100ms recommended)
- **Format:** All responses are JSON
- **User-Agent:** Include a descriptive User-Agent header

---

## Player Endpoints

### Get Player Profile
```
GET /player/{username}
```
Returns profile information for a player.

**Response Fields:**
| Field | Type | Description |
|-------|------|-------------|
| `@id` | string | API URL for this player |
| `url` | string | Chess.com profile URL |
| `username` | string | Username (case-preserved) |
| `player_id` | integer | Unique player ID |
| `title` | string | Chess title (GM, IM, FM, etc.) if any |
| `status` | string | Account status (premium, staff, closed, etc.) |
| `name` | string | Display name |
| `avatar` | string | Avatar image URL |
| `location` | string | Player's location |
| `country` | string | Country API URL |
| `joined` | integer | Unix timestamp of account creation |
| `last_online` | integer | Unix timestamp of last login |
| `followers` | integer | Number of followers |
| `is_streamer` | boolean | Whether player is a Chess.com streamer |
| `twitch_url` | string | Twitch channel URL (if streamer) |
| `fide` | integer | FIDE rating (if available) |

**Example:**
```bash
curl https://api.chess.com/pub/player/hikaru
```

---

### Get Player Stats
```
GET /player/{username}/stats
```
Returns ratings and statistics for all game types.

**Response Structure:**
```json
{
  "chess_daily": { ... },
  "chess_rapid": { ... },
  "chess_blitz": { ... },
  "chess_bullet": { ... },
  "tactics": { ... },
  "puzzle_rush": { ... }
}
```

**Per-Category Fields:**
| Field | Type | Description |
|-------|------|-------------|
| `last` | object | Most recent rating: `rating`, `date`, `rd` |
| `best` | object | Peak rating: `rating`, `date`, `game` |
| `record` | object | W/L/D counts: `win`, `loss`, `draw` |

---

### Get Player Clubs
```
GET /player/{username}/clubs
```
Returns list of clubs the player belongs to.

**Response Fields (per club):**
| Field | Type | Description |
|-------|------|-------------|
| `@id` | string | Club API URL |
| `name` | string | Club name |
| `joined` | integer | Unix timestamp when joined |
| `last_activity` | integer | Unix timestamp of last activity |
| `icon` | string | Club icon URL |
| `url` | string | Club profile URL |

---

### Get Player Online Status
```
GET /player/{username}/is-online
```
Returns whether the player is currently online.

**Response:**
```json
{
  "online": true
}
```

---

## Game Endpoints

### Get Archives List
```
GET /player/{username}/games/archives
```
Returns list of monthly archive URLs containing the player's games.

**Response:**
```json
{
  "archives": [
    "https://api.chess.com/pub/player/hikaru/games/2024/01",
    "https://api.chess.com/pub/player/hikaru/games/2024/02",
    ...
  ]
}
```

---

### Get Monthly Games
```
GET /player/{username}/games/{YYYY}/{MM}
```
Returns all games for a player in a specific month.

**Response Fields (per game):**
| Field | Type | Description |
|-------|------|-------------|
| `url` | string | Game URL on Chess.com |
| `pgn` | string | Complete PGN of the game |
| `tcn` | string | TCN-encoded moves (compact format) |
| `uuid` | string | Unique game identifier |
| `initial_setup` | string | Starting FEN (if non-standard) |
| `fen` | string | Final position FEN |
| `time_control` | string | Time control (e.g., "600", "180+2") |
| `time_class` | string | Category: daily, rapid, blitz, bullet |
| `rules` | string | Variant: chess, chess960, bughouse, etc. |
| `rated` | boolean | Whether game was rated |
| `end_time` | integer | Unix timestamp when game ended |
| `white` | object | White player info |
| `black` | object | Black player info |

**Player Object Fields:**
| Field | Type | Description |
|-------|------|-------------|
| `username` | string | Player username |
| `rating` | integer | Rating at time of game |
| `result` | string | Result: win, checkmated, resigned, timeout, stalemate, etc. |
| `@id` | string | Player API URL |
| `uuid` | string | Player UUID |

**Example:**
```bash
curl https://api.chess.com/pub/player/hikaru/games/2024/03
```

---

### Get Monthly Games (PGN Download)
```
GET /player/{username}/games/{YYYY}/{MM}/pgn
```
Returns all games for a month as a single PGN file (text/plain).

---

### Get Current Daily Games
```
GET /player/{username}/games
```
Returns list of Daily Chess games where it's the player's turn or where the player is waiting.

---

### Get Games To Move
```
GET /player/{username}/games/to-move
```
Returns list of Daily Chess games where it's the player's turn to move.

**Response Fields (per game):**
| Field | Type | Description |
|-------|------|-------------|
| `url` | string | Game URL |
| `move_by` | integer | Unix timestamp deadline for move |
| `last_activity` | integer | Unix timestamp of last activity |

---

## Club Endpoints

### Get Club Profile
```
GET /club/{url-ID}
```
Returns club profile information.

**Response Fields:**
| Field | Type | Description |
|-------|------|-------------|
| `@id` | string | Club API URL |
| `name` | string | Club name |
| `club_id` | integer | Unique club ID |
| `icon` | string | Club icon URL |
| `country` | string | Country API URL |
| `average_daily_rating` | integer | Average daily rating |
| `members_count` | integer | Total member count |
| `created` | integer | Unix timestamp of creation |
| `last_activity` | integer | Unix timestamp of last activity |
| `admin` | array | List of admin usernames |
| `description` | string | Club description (HTML) |

---

### Get Club Members
```
GET /club/{url-ID}/members
```
Returns club members grouped by activity level.

**Response:**
```json
{
  "weekly": [...],
  "monthly": [...],
  "all_time": [...]
}
```

---

### Get Club Matches
```
GET /club/{url-ID}/matches
```
Returns team matches the club is participating in.

**Response:**
```json
{
  "finished": [...],
  "in_progress": [...],
  "registered": [...]
}
```

---

## Country Endpoints

### Get Country Profile
```
GET /country/{iso}
```
Returns country profile (use 2-letter ISO code).

---

### Get Country Players
```
GET /country/{iso}/players
```
Returns list of player usernames from that country.

---

### Get Country Clubs
```
GET /country/{iso}/clubs
```
Returns list of clubs from that country.

---

## Tournament Endpoints

### Get Tournament
```
GET /tournament/{url-ID}
```
Returns tournament details. The `url-ID` is the slug from the tournament URL.

**Response Fields:**
| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Tournament name |
| `url` | string | Tournament URL |
| `description` | string | Tournament description |
| `creator` | string | Creator username |
| `status` | string | finished, in_progress, registration |
| `finish_time` | integer | Unix timestamp when finished |
| `settings` | object | Tournament settings |
| `players` | array | List of participants |
| `rounds` | array | Round URLs |

---

### Get Tournament Round
```
GET /tournament/{url-ID}/{round}
```
Returns details for a specific round.

---

### Get Tournament Round Group
```
GET /tournament/{url-ID}/{round}/{group}
```
Returns details for a specific group in a round.

---

## Team Match Endpoints

### Get Team Match
```
GET /match/{ID}
```
Returns team match details. The `ID` is numeric (e.g., `12803`).

**Response Fields:**
| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Match name |
| `url` | string | Match URL |
| `status` | string | finished, in_progress, registered |
| `boards` | integer | Number of boards |
| `teams` | object | Team information with scores |

---

### Get Team Match Board
```
GET /match/{ID}/{board}
```
Returns games for a specific board in the match.

---

### Get Live Team Match
```
GET /match/live/{ID}
```
Returns details for a live team match.

---

## Puzzle Endpoints

### Get Daily Puzzle
```
GET /puzzle
```
Returns the daily puzzle.

**Response Fields:**
| Field | Type | Description |
|-------|------|-------------|
| `title` | string | Puzzle title |
| `url` | string | Puzzle URL |
| `publish_time` | integer | Unix timestamp |
| `fen` | string | Starting position FEN |
| `pgn` | string | Solution PGN |
| `image` | string | Puzzle image URL |

---

### Get Random Puzzle
```
GET /puzzle/random
```
Returns a random puzzle (same format as daily puzzle).

---

## Leaderboard Endpoints

### Get Leaderboards
```
GET /leaderboards
```
Returns top players across all categories.

**Response Structure:**
```json
{
  "daily": [...],
  "daily960": [...],
  "live_rapid": [...],
  "live_blitz": [...],
  "live_bullet": [...],
  "live_bughouse": [...],
  "live_blitz960": [...],
  "live_threecheck": [...],
  "live_crazyhouse": [...],
  "live_kingofthehill": [...],
  "tactics": [...],
  "rush": [...],
  "battle": [...]
}
```

**Per-Player Fields:**
| Field | Type | Description |
|-------|------|-------------|
| `player_id` | integer | Player ID |
| `@id` | string | Player API URL |
| `url` | string | Profile URL |
| `username` | string | Username |
| `score` | integer | Rating/score |
| `rank` | integer | Leaderboard rank |
| `title` | string | Chess title (if any) |
| `name` | string | Display name |
| `status` | string | Account status |
| `avatar` | string | Avatar URL |
| `trend_score` | object | Rating change info |
| `trend_rank` | object | Rank change info |
| `country` | string | Country API URL |
| `flair_code` | string | Flair identifier |
| `win_count` | integer | Total wins |
| `loss_count` | integer | Total losses |
| `draw_count` | integer | Total draws |

---

## Streamer Endpoints

### Get Streamers
```
GET /streamers
```
Returns list of Chess.com streamers.

**Response Fields (per streamer):**
| Field | Type | Description |
|-------|------|-------------|
| `username` | string | Chess.com username |
| `avatar` | string | Avatar URL |
| `twitch_url` | string | Twitch channel URL |
| `url` | string | Chess.com profile URL |
| `is_live` | boolean | Currently streaming |
| `is_community_streamer` | boolean | Community streamer status |

---

## Titled Player Endpoints

### Get Titled Players
```
GET /titled/{title}
```
Returns list of usernames with the specified title.

**Valid Titles:**
- `GM` - Grandmaster
- `WGM` - Woman Grandmaster
- `IM` - International Master
- `WIM` - Woman International Master
- `FM` - FIDE Master
- `WFM` - Woman FIDE Master
- `NM` - National Master
- `WNM` - Woman National Master
- `CM` - Candidate Master
- `WCM` - Woman Candidate Master

**Example:**
```bash
curl https://api.chess.com/pub/titled/GM
```

**Response:**
```json
{
  "players": ["hikaru", "magnuscarlsen", "firouzja2003", ...]
}
```

---

## Error Responses

The API returns standard HTTP status codes:

| Code | Meaning |
|------|---------|
| 200 | Success |
| 301 | Moved (follow redirect) |
| 304 | Not Modified (use cache) |
| 404 | Not Found |
| 410 | Gone (account closed) |
| 429 | Rate Limited |

**Error Response Format:**
```json
{
  "code": 0,
  "message": "Error description"
}
```

---

## Rate Limiting Best Practices

1. Add 100ms delay between requests
2. Use caching (respect 304 responses)
3. Include descriptive User-Agent header
4. Batch requests where possible (e.g., use `/games/{YYYY}/{MM}/pgn`)

---

## Sources

- [Chess.com Published-Data API (Official)](https://support.chess.com/en/articles/9650547-published-data-api)
- [Chess.com API Announcement](https://www.chess.com/announcements/view/published-data-api)
- [PublicAPI.dev - Chess.com API](https://publicapi.dev/chess-com-api)
- [chesscompubapi Go Package](https://pkg.go.dev/github.com/agoblet/chesscompubapi)
