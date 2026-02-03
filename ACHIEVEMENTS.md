# Chess Social Media - Achievements System

## Overview
Achievements are a core feature of the platform. Players earn achievements by playing games on Chess.com/Lichess, and can share them on their feed. The backend analyzes games using pattern detection to automatically discover achievements.

---

## Achievement Categories

### 1. Basic Checkmates (13)

**Piece Mates** - Deliver checkmate with each piece type
| Achievement | Description |
|-------------|-------------|
| Pawn Mate | Deliver checkmate with a pawn |
| Knight Mate | Deliver checkmate with a knight |
| Bishop Mate | Deliver checkmate with a bishop |
| Rook Mate | Deliver checkmate with a rook |
| Queen Mate | Deliver checkmate with the queen |
| King Mate | Your king delivers the final checkmate |

**Castle Mates** - Deliver checkmate by castling
| Achievement | Description |
|-------------|-------------|
| Kingside Castle Mate | Deliver checkmate with O-O |
| Queenside Castle Mate | Deliver checkmate with O-O-O |

**Special Move Mates**
| Achievement | Description |
|-------------|-------------|
| En Passant Mate | En passant capture delivers checkmate |

**Promotion Mates** - Promote a pawn and deliver checkmate
| Achievement | Description |
|-------------|-------------|
| Queen Promotion Mate | Promote to queen and deliver checkmate |
| Rook Promotion Mate | Promote to rook and deliver checkmate |
| Bishop Promotion Mate | Promote to bishop and deliver checkmate |
| Knight Promotion Mate | Promote to knight and deliver checkmate |

---

### 2. Named Checkmate Patterns (53)

| Pattern | Pattern | Pattern |
|---------|---------|---------|
| Anastasia's Mate | Escalator Mate | Railroad Mate |
| Anderssen's Mate | Fool's Mate | Reti's Mate |
| Arabian Mate | Greco's Mate | Scholar's Mate |
| Back Rank Mate | H-file Mate | Seizing a Square Mate |
| Balestra Mate | Hook Mate | Side File Mate |
| Blackburne's Mate | Kill Box Mate | Smothered Mate |
| Blind Swine Mate | Lawnmower Mate | Suffocation Mate |
| Boden's Mate | Legal's Mate | Swallow's Tail Mate |
| Corner Mate | Lolli's Mate | Threading the Needle Mate |
| Corridor Mate | Max Lange's Mate | Triangle Mate |
| Counter Check Checkmate | Mayet's Mate | Vukovic Mate |
| Damiano's Mate | Monorail Mate | Walking the Plank Mate |
| Damiano's Bishop Mate | Morphy's Mate | X-Ray Mate |
| David and Goliath Mate | Opera Mate | |
| Diagonal Corridor Mate | Pillsbury's Mate | |
| Discovered Mate | Queen and Knight Mate | |
| Double Bishop Mate | Queen Cutoff Mate | |
| Double Checkmate | Edge Mate | |
| Dovetail Mate | Edge Pin Mate | |
| Dovetail Mate - Bishop | Epaulette Mate | |

---

### 3. Tactics
(TODO)

---

### 4. Sacrifices
(TODO)

---

### 5. Game Achievements

**King Walk** - March your king deep into enemy territory
| Achievement | Description |
|-------------|-------------|
| King Walk (3rd Rank) | Your king reaches the 3rd rank |
| King Walk (2nd Rank) | Your king reaches the 2nd rank |
| King Walk (1st Rank) | Your king reaches the opponent's back rank |

---

### 6. Endgame
(TODO)

---

## Legacy Reference (from Chess Calculations backend)

<details>
<summary>Existing 26 analyzers for reference</summary>

### Checkmate Patterns (11)
- Smothered Mate
- Castle Mate
- Back Rank Mate
- Pawn Mate
- King Mate
- Knight Promotion Mate
- En Passant Mate
- Promotion Mate
- Knight + Bishop Mate
- Queen Sacrifice Mate
- Quickest Mate

### Sacrifice Patterns (2)
- Queen Sacrifice
- Rook Sacrifice

### Tactical Patterns (5)
- Knight Fork
- Windmill
- Capture Sequence
- Hung Queen
- Rare Moves

### Game Achievement Patterns (6)
- Biggest Comeback
- Clutch Win
- Longest Game
- King Walk
- Stalemate
- Best Game

### Opening Patterns (2)
- Favorite Gambit
- Signature Opening

</details>

---

## Achievement Tiers

### Tier System
Each achievement can have tiers based on count:

| Tier | Icon | Requirement | Color |
|------|------|-------------|-------|
| Bronze | 🥉 | 1 occurrence | Bronze/Brown |
| Silver | 🥈 | 5 occurrences | Silver/Gray |
| Gold | 🥇 | 25 occurrences | Gold/Yellow |
| Platinum | 💎 | 100 occurrences | Cyan/Diamond |
| Master | 👑 | 500 occurrences | Purple |

### Special Achievements
Some achievements are one-time or unique:
- **First Blood** - First ever achievement unlocked
- **Collector** - Earn 10 different achievement types
- **Completionist** - Earn all achievements (at least bronze)
- **Legendary** - Earn a "Legendary" rarity achievement

---

## Display & Sharing

### Profile Display
- Achievement badge grid (show top achievements)
- Total achievement count
- Rarest achievement highlight
- Achievement tier breakdown

### Feed Posts
When a user unlocks a new achievement:
- Auto-generate shareable post
- Include the game board at key moment
- Show achievement badge
- Link to full game

### Achievement Card Design
```
┌─────────────────────────────────────┐
│  🏆 SMOTHERED MATE                  │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│                                     │
│  [Chess board at mate position]     │
│                                     │
│  Knight delivers checkmate while    │
│  enemy king is surrounded.          │
│                                     │
│  🥇 Gold · 27 times · Rare          │
│  First: Jan 15, 2025                │
└─────────────────────────────────────┘
```

---

## Technical Considerations

### Detection Priority
1. **Easy to detect** - Checkmate patterns, material counts
2. **Medium** - Tactical patterns requiring move analysis
3. **Hard** - Requires engine evaluation (swindles, etc.)

### What We Can Detect Without Engine
- All checkmate patterns (board state analysis)
- Sacrifices (material exchange tracking)
- Forks, pins, skewers (piece position analysis)
- Time-based achievements (game metadata)
- Rating-based achievements (game metadata)
- Move count achievements (PGN parsing)

### What Needs Engine Analysis
- Swindle detection (eval swing)
- Brilliant move detection
- Missed win detection
- Position assessment (fortress, etc.)

### Performance
- Analyze games in batches
- Cache results per game
- Only re-analyze new games
- Background job for heavy analysis

---

## Questions to Decide

1. **How many achievements to launch with?**
   - Start with existing 26?
   - Add 10-20 new ones?
   - Save some for future updates?

2. **Engine analysis - yes or no?**
   - Without: Simpler, faster, already implemented
   - With: More sophisticated achievements, but adds complexity

3. **Retroactive achievements?**
   - Analyze all historical games on account link?
   - Only new games going forward?
   - Option to "scan history" on demand?

4. **Leaderboards per achievement?**
   - Global leaderboard for each achievement type
   - Friends leaderboard
   - Monthly/yearly resets?

5. **Achievement rarity - static or dynamic?**
   - Static: We define rarity tiers
   - Dynamic: Based on actual unlock percentages

---

## Implementation Phases

### Phase 1: Core (Launch)
- Port existing 26 analyzers
- Basic tier system (Bronze → Gold)
- Achievement unlocks on game sync
- Profile achievement display
- Manual game share with achievement tag

### Phase 2: Enhanced
- Add 10-15 new pattern achievements
- Auto-post on first unlock
- Achievement notifications
- Leaderboards

### Phase 3: Advanced
- Engine-assisted achievements (optional)
- Dynamic rarity calculation
- Achievement challenges ("Get a smothered mate this week")
- Seasonal achievements

---

## Notes

(Add brainstorming notes here)


