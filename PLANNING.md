# Chess Social Media App - Planning Document

## Vision
A social platform where chess players link their Chess.com and Lichess accounts, showcase their best games and achievements, and engage with a community of players through a chess-focused feed.

---

## Core Features

### 1. User System
- **Account Creation** - Email/password or OAuth (Google, Discord?)
- **Platform Linking** - Connect Chess.com and Lichess accounts
  - Verification flow (how do we verify ownership?)
- **Profile Page**
  - Display linked accounts with ratings
  - Showcase "pinned" top games
  - Achievement badges
  - Stats summary (wins, favorite openings, etc.)
  - Follow/follower counts

### 2. Chess Platform Integration
- **Chess.com** (existing implementation)
  - Fetch games via public API
  - Parse PGN/TCN formats
- **Lichess** (needs implementation)
  - Fetch games via Lichess API
  - OAuth for account verification?
  - Different data format handling

### 3. Achievements System
- Leverage existing 26 pattern analyzers:
  - Smothered Mate, Castle Mate, Queen Sacrifice, etc.
- Achievement tiers (Bronze/Silver/Gold/Platinum)?
- First-time achievement notifications
- Profile badges

### 4. Social Feed
- **Post Types:**
  - Game share (with optional annotation)
  - Achievement unlocked
  - Text post (chess thoughts, questions)
  - Puzzle/position share?
- **Interactions:**
  - Like/react
  - Comment
  - Share/repost
- **Feed Algorithm:**
  - Chronological vs algorithmic?
  - Following-only vs discovery feed?

### 5. Game Viewer
- Interactive board for viewing shared games
- Move-by-move navigation
- Highlight key moments (the move that triggered an achievement)
- Engine analysis integration?

### 6. Notifications
- New follower
- Comment on your post
- Like on your post
- Achievement unlocked
- Someone shared your game?

### 7. Discovery & Search
- Find users by username
- Search by rating range
- Leaderboards (from existing system)
- Trending games/achievements

---

## Backend Architecture

### Reusable from Chess Calculations
```
src/
├── analysis/           # All 26 analyzers - REUSE
│   └── unified/        # Pattern detection engines
├── api/
│   └── chess_com_client.py  # Chess.com integration - REUSE
├── utils/
│   ├── pgn_parser.py   # PGN parsing - REUSE
│   └── tcn_decoder.py  # TCN decoding - REUSE
└── models/
    └── game_data.py    # Game data structures - REUSE
```

### New Modules Needed
```
src/
├── auth/               # NEW - Authentication
│   ├── jwt_handler.py
│   ├── oauth.py
│   └── password.py
├── api/
│   ├── lichess_client.py    # NEW - Lichess integration
│   └── routes/              # NEW - API route organization
│       ├── auth.py
│       ├── users.py
│       ├── posts.py
│       ├── feed.py
│       └── achievements.py
├── services/           # NEW - Business logic
│   ├── user_service.py
│   ├── post_service.py
│   ├── feed_service.py
│   ├── achievement_service.py
│   └── notification_service.py
└── database/
    ├── models/         # NEW - SQLAlchemy models
    │   ├── user.py
    │   ├── post.py
    │   ├── comment.py
    │   ├── follow.py
    │   └── achievement.py
    └── db_service.py   # EXTEND existing
```

---

## Database Schema (Draft)

### Users
```
users
├── id (UUID, PK)
├── email (unique)
├── username (unique)
├── password_hash
├── display_name
├── bio
├── avatar_url
├── created_at
├── updated_at
└── is_verified
```

### Linked Accounts
```
linked_accounts
├── id (UUID, PK)
├── user_id (FK -> users)
├── platform (chess_com | lichess)
├── platform_username
├── platform_user_id
├── is_verified
├── ratings_cache (JSONB) # {blitz: 1500, rapid: 1600, ...}
├── last_synced_at
└── created_at
```

### Posts
```
posts
├── id (UUID, PK)
├── user_id (FK -> users)
├── post_type (game_share | achievement | text | puzzle)
├── content (text)
├── game_data (JSONB, nullable) # PGN, metadata, key positions
├── achievement_data (JSONB, nullable)
├── like_count
├── comment_count
├── created_at
└── updated_at
```

### Comments
```
comments
├── id (UUID, PK)
├── post_id (FK -> posts)
├── user_id (FK -> users)
├── content
├── created_at
└── updated_at
```

### Follows
```
follows
├── follower_id (FK -> users)
├── following_id (FK -> users)
├── created_at
└── PK(follower_id, following_id)
```

### Likes
```
likes
├── user_id (FK -> users)
├── post_id (FK -> posts)
├── created_at
└── PK(user_id, post_id)
```

### User Achievements
```
user_achievements
├── id (UUID, PK)
├── user_id (FK -> users)
├── achievement_type (smothered_mate, queen_sacrifice, etc.)
├── count (total times achieved)
├── best_game_data (JSONB) # The most impressive instance
├── first_achieved_at
├── last_achieved_at
└── tier (bronze | silver | gold | platinum)
```

### Notifications
```
notifications
├── id (UUID, PK)
├── user_id (FK -> users)
├── type (follow | like | comment | achievement)
├── data (JSONB) # Context-specific data
├── is_read
├── created_at
```

---

## API Endpoints (Draft)

### Auth
- `POST /api/auth/register` - Create account
- `POST /api/auth/login` - Login, get JWT
- `POST /api/auth/refresh` - Refresh token
- `POST /api/auth/logout` - Invalidate token

### Users
- `GET /api/users/:username` - Get profile
- `PATCH /api/users/me` - Update own profile
- `POST /api/users/me/link-account` - Link Chess.com/Lichess
- `DELETE /api/users/me/link-account/:platform` - Unlink
- `POST /api/users/:username/follow` - Follow user
- `DELETE /api/users/:username/follow` - Unfollow
- `GET /api/users/:username/followers` - List followers
- `GET /api/users/:username/following` - List following

### Posts
- `POST /api/posts` - Create post
- `GET /api/posts/:id` - Get single post
- `DELETE /api/posts/:id` - Delete own post
- `POST /api/posts/:id/like` - Like post
- `DELETE /api/posts/:id/like` - Unlike
- `GET /api/posts/:id/comments` - Get comments
- `POST /api/posts/:id/comments` - Add comment

### Feed
- `GET /api/feed` - Get personalized feed
- `GET /api/feed/discover` - Discovery/trending feed
- `GET /api/users/:username/posts` - User's posts

### Achievements
- `GET /api/users/:username/achievements` - User's achievements
- `POST /api/achievements/sync` - Trigger re-analysis of games

### Games
- `GET /api/games/sync` - Sync latest games from platforms
- `GET /api/games/:id` - Get game details

---

## Frontend Architecture

### Decision: React + Vite + TypeScript (Fresh Start)
- Familiar stack from previous project
- Fast development with Vite
- Reuse chess board components from `front-end` project
- Traditional social media layout (not wrapped-style)

### Tech Stack
```
React 19 + TypeScript
Vite (build tool)
React Router (routing)
Tailwind CSS (styling)
Framer Motion (animations)
react-chessboard + chess.js (chess board)
Zustand or React Context (state management)
```

### Reusing from Previous Frontend
```
FROM: C:\Users\steve\OneDrive\Desktop\front-end

COPY DIRECTLY:
├── components/ChessBoard/     # Interactive board with move playback
├── utils/pgnParser.ts         # PGN parsing utilities
├── utils/chessSounds.ts       # Move sound effects
├── utils/replayData.ts        # Move replay extraction

ADAPT PATTERNS:
├── services/api.ts            # API service pattern (rewrite for new endpoints)
├── tailwind.config.js         # Color schemes, dark theme
└── Progressive loading        # Pattern for game analysis
```

### Layout Structure (Traditional Social Media)
```
┌─────────────────────────────────────────────────────────────┐
│  Navbar (Logo, Search, Create Post, Notifications, Profile) │
├─────────────┬───────────────────────────┬───────────────────┤
│             │                           │                   │
│  Left       │      Main Feed            │   Right           │
│  Sidebar    │      (scrollable)         │   Sidebar         │
│             │                           │                   │
│  - Home     │  ┌─────────────────────┐  │   - Suggested     │
│  - Profile  │  │ Post Card           │  │     Users         │
│  - Games    │  │ - Author info       │  │                   │
│  - Achieve- │  │ - Content/Game      │  │   - Trending      │
│    ments    │  │ - Chess board       │  │     Games         │
│  - Settings │  │ - Like/Comment      │  │                   │
│             │  └─────────────────────┘  │   - Leaderboard   │
│             │                           │     Preview       │
│             │  ┌─────────────────────┐  │                   │
│             │  │ Post Card           │  │                   │
│             │  └─────────────────────┘  │                   │
│             │                           │                   │
└─────────────┴───────────────────────────┴───────────────────┘
```

### Pages & Routes
```
/                     → Home feed (requires auth)
/login                → Login page
/register             → Registration page
/u/:username          → User profile
/u/:username/games    → User's game library
/u/:username/achievements → User's achievements
/post/:id             → Single post view (with comments)
/game/:id             → Full game viewer
/search               → Search users/games
/settings             → User settings
/settings/accounts    → Link Chess.com/Lichess
/notifications        → Notifications list
```

### Component Hierarchy
```
src/
├── components/
│   ├── layout/
│   │   ├── Navbar.tsx
│   │   ├── LeftSidebar.tsx
│   │   ├── RightSidebar.tsx
│   │   └── MainLayout.tsx
│   │
│   ├── auth/
│   │   ├── LoginForm.tsx
│   │   ├── RegisterForm.tsx
│   │   └── ProtectedRoute.tsx
│   │
│   ├── feed/
│   │   ├── Feed.tsx
│   │   ├── PostCard.tsx
│   │   ├── PostComposer.tsx
│   │   └── PostActions.tsx (like, comment, share)
│   │
│   ├── profile/
│   │   ├── ProfileHeader.tsx
│   │   ├── ProfileStats.tsx
│   │   ├── PinnedGames.tsx
│   │   ├── AchievementBadges.tsx
│   │   └── FollowButton.tsx
│   │
│   ├── chess/                    # REUSED FROM PREVIOUS PROJECT
│   │   ├── ChessBoard.tsx        # Interactive board
│   │   ├── GameViewer.tsx        # Full game with controls
│   │   ├── MoveList.tsx          # PGN move display
│   │   └── GameCard.tsx          # Compact game preview
│   │
│   ├── comments/
│   │   ├── CommentList.tsx
│   │   ├── CommentItem.tsx
│   │   └── CommentForm.tsx
│   │
│   ├── notifications/
│   │   ├── NotificationBell.tsx
│   │   ├── NotificationList.tsx
│   │   └── NotificationItem.tsx
│   │
│   └── common/
│       ├── Avatar.tsx
│       ├── Button.tsx
│       ├── Modal.tsx
│       ├── Dropdown.tsx
│       └── LoadingSpinner.tsx
│
├── pages/
│   ├── HomePage.tsx
│   ├── LoginPage.tsx
│   ├── RegisterPage.tsx
│   ├── ProfilePage.tsx
│   ├── PostPage.tsx
│   ├── GamePage.tsx
│   ├── SearchPage.tsx
│   ├── SettingsPage.tsx
│   └── NotificationsPage.tsx
│
├── services/
│   ├── api.ts                # Base API client with auth
│   ├── authService.ts        # Login, register, tokens
│   ├── userService.ts        # Profile, follow, etc.
│   ├── postService.ts        # CRUD posts
│   ├── feedService.ts        # Feed fetching
│   ├── gameService.ts        # Game sync, fetch
│   └── notificationService.ts
│
├── stores/                   # Zustand stores (or Context)
│   ├── authStore.ts          # Current user, tokens
│   ├── feedStore.ts          # Feed state
│   └── notificationStore.ts  # Unread count, etc.
│
├── hooks/
│   ├── useAuth.ts
│   ├── useFeed.ts
│   ├── useProfile.ts
│   └── useChessBoard.ts      # REUSED
│
├── utils/
│   ├── pgnParser.ts          # REUSED
│   ├── chessSounds.ts        # REUSED
│   ├── formatDate.ts
│   └── validation.ts
│
├── types/
│   └── index.ts              # TypeScript interfaces
│
└── config/
    └── api.ts                # API base URL config
```

### Post Card Design (Game Share)
```
┌──────────────────────────────────────────────┐
│  [Avatar] Username · @chesscom_user · 2h ago │
│                                              │
│  "Just pulled off my first smothered mate!   │
│   Been trying to set this up for weeks 🎯"   │
│                                              │
│  ┌────────────────────────────────────────┐  │
│  │                                        │  │
│  │         ♜ ♞ ♝ ♛ ♚ ♝ ♞ ♜              │  │
│  │         ♟ ♟ ♟ ♟ ♟ ♟ ♟ ♟              │  │
│  │         (Chess Board - key position)   │  │
│  │         ♙ ♙ ♙ ♙ ♙ ♙ ♙ ♙              │  │
│  │         ♖ ♘ ♗ ♕ ♔ ♗ ♘ ♖              │  │
│  │                                        │  │
│  │  [< Prev] Move 23. Nf7# [Next >]       │  │
│  │  [View Full Game]                      │  │
│  └────────────────────────────────────────┘  │
│                                              │
│  🏆 Achievement: Smothered Mate              │
│                                              │
│  ♡ 42    💬 7    ↗ Share                    │
└──────────────────────────────────────────────┘
```

### Mobile Responsive Behavior
- **Desktop (1024px+):** Three-column layout
- **Tablet (768px-1024px):** Two columns (hide right sidebar)
- **Mobile (<768px):** Single column, bottom nav bar

---

## Open Questions

1. **Account Verification** - How do we verify a user owns a Chess.com/Lichess account?
   - Option A: OAuth (Lichess supports this, Chess.com doesn't)
   - Option B: Verify by having them add a code to their profile bio
   - Option C: Trust but verify (check if games exist)

2. **Real-time Features** - Do we need WebSockets for:
   - Live notifications
   - Live game spectating
   - Real-time feed updates

3. **Game Storage** - Do we store full game PGNs or fetch on-demand?
   - Store: Faster, but storage costs
   - Fetch: Slower, but always fresh

4. **Monetization** (future) - Premium features?
   - Extended analysis
   - No ads
   - Custom profile themes

5. **Moderation** - How do we handle:
   - Spam posts
   - Inappropriate content
   - Fake accounts

---

## Tech Stack Summary

### Backend
- **Framework:** FastAPI (Python) - continuing from chess calculations
- **Database:** PostgreSQL
- **Auth:** JWT + OAuth (Google, Discord?)
- **Caching:** Redis (for feed, sessions)
- **Task Queue:** Celery or AWS Lambda (for game analysis)

### Frontend
- **Framework:** React 19 + TypeScript
- **Build:** Vite
- **Routing:** React Router v6
- **Styling:** Tailwind CSS
- **Animations:** Framer Motion
- **Chess:** react-chessboard + chess.js
- **State:** Zustand (lightweight) or React Context

### Infrastructure
- **Hosting:** AWS (App Runner, Lambda, RDS)
- **Frontend Hosting:** AWS Amplify or Vercel
- **CDN:** CloudFront for static assets
- **Storage:** S3 for avatars, game exports

---

## Development Phases

### Phase 1: Foundation
- [ ] Set up project structure
- [ ] User auth system
- [ ] Basic profile pages
- [ ] Chess.com account linking

### Phase 2: Core Social
- [ ] Post creation (game shares)
- [ ] Feed implementation
- [ ] Follow system
- [ ] Likes and comments

### Phase 3: Achievements
- [ ] Integrate analysis engine
- [ ] Achievement detection and storage
- [ ] Achievement posts (auto-generated)
- [ ] Profile badges

### Phase 4: Polish
- [ ] Lichess integration
- [ ] Notifications
- [ ] Search and discovery
- [ ] Mobile responsiveness

### Phase 5: Scale
- [ ] Performance optimization
- [ ] Caching layer
- [ ] Rate limiting
- [ ] Monitoring and analytics

---

## Discussion Notes

### Session 1 Decisions
- **Chess Wrapped feature:** NOT including for now - focus on social media core
- **Layout style:** Traditional social media (navbar, sidebar, scrollable feed) - NOT immersive/fullscreen
- **Frontend approach:** Fresh start, but reuse chess board components from `front-end` project
- **Existing code to leverage:**
  - Backend: `chess calculations` project (26 analyzers, Chess.com client, PGN parsing)
  - Frontend: `front-end` project (ChessBoard component, utils, Tailwind config)

### Open Items to Discuss
- [ ] State management: Zustand vs React Context?
- [ ] OAuth providers: Google? Discord? Both?
- [ ] Account verification method for Chess.com (no OAuth available)
- [ ] Real-time: WebSockets for notifications?
- [ ] Where to start: Backend first or Frontend first?

