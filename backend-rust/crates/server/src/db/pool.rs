use sqlx::postgres::{PgPool, PgPoolOptions};

pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(20)
        .connect(database_url)
        .await
}

/// Run the full Postgres schema migration inline.
pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::raw_sql(SCHEMA_SQL).execute(pool).await?;
    Ok(())
}

const SCHEMA_SQL: &str = r#"
-- Accounts table (app authentication + game ownership)
CREATE TABLE IF NOT EXISTS accounts (
    id          BIGSERIAL PRIMARY KEY,
    username    TEXT UNIQUE NOT NULL,
    email       TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    display_name  TEXT,
    chess_com_username TEXT,
    bio           TEXT,
    avatar_url    TEXT,
    chess_com_last_synced_at TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_accounts_email_lower
    ON accounts (LOWER(email));
CREATE INDEX IF NOT EXISTS idx_accounts_username_lower
    ON accounts (LOWER(username));

-- User games table (owned by accounts)
CREATE TABLE IF NOT EXISTS user_games (
    id                BIGSERIAL PRIMARY KEY,
    user_id           BIGINT NOT NULL REFERENCES accounts(id),
    chess_com_game_id TEXT NOT NULL,
    opponent          TEXT NOT NULL,
    opponent_rating   INTEGER,
    user_rating       INTEGER,
    result            TEXT NOT NULL,
    user_color        TEXT NOT NULL,
    time_control      TEXT,
    date              TEXT,
    tags              JSONB DEFAULT '[]'::jsonb,
    tcn               TEXT,
    source            TEXT NOT NULL DEFAULT 'chess_com',
    analyzed_at       TIMESTAMPTZ,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_user_games_source_game
    ON user_games (user_id, source, chess_com_game_id);
CREATE INDEX IF NOT EXISTS idx_user_games_user_id
    ON user_games (user_id);
CREATE INDEX IF NOT EXISTS idx_user_games_date
    ON user_games (date DESC);
CREATE INDEX IF NOT EXISTS idx_user_games_source
    ON user_games (source);

-- Game tags (normalized)
CREATE TABLE IF NOT EXISTS game_tags (
    id      BIGSERIAL PRIMARY KEY,
    game_id BIGINT NOT NULL REFERENCES user_games(id) ON DELETE CASCADE,
    tag     TEXT NOT NULL,
    UNIQUE(game_id, tag)
);

CREATE INDEX IF NOT EXISTS idx_game_tags_game_id ON game_tags (game_id);
CREATE INDEX IF NOT EXISTS idx_game_tags_tag     ON game_tags (tag);

-- Game analysis (Stockfish results)
CREATE TABLE IF NOT EXISTS game_analysis (
    id                    BIGSERIAL PRIMARY KEY,
    game_id               BIGINT NOT NULL UNIQUE REFERENCES user_games(id) ON DELETE CASCADE,
    white_accuracy        DOUBLE PRECISION NOT NULL,
    black_accuracy        DOUBLE PRECISION NOT NULL,
    white_avg_cp_loss     DOUBLE PRECISION NOT NULL,
    black_avg_cp_loss     DOUBLE PRECISION NOT NULL,
    white_classifications JSONB NOT NULL,
    black_classifications JSONB NOT NULL,
    moves                 JSONB NOT NULL,
    phase_accuracy        JSONB,
    first_inaccuracy_move JSONB,
    puzzles               JSONB,
    endgame_segments      JSONB,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_game_analysis_game_id
    ON game_analysis (game_id);

-- Add columns that may be missing on older schemas
DO $$ BEGIN
    ALTER TABLE game_analysis ADD COLUMN IF NOT EXISTS puzzles JSONB;
    ALTER TABLE game_analysis ADD COLUMN IF NOT EXISTS endgame_segments JSONB;
EXCEPTION WHEN OTHERS THEN NULL;
END $$;

-- Track how far back we've synced Chess.com history
-- NULL = never synced, "YYYY-MM" = oldest month fetched, "complete" = all history fetched
DO $$ BEGIN
    ALTER TABLE accounts ADD COLUMN IF NOT EXISTS chess_com_oldest_synced_month TEXT;
EXCEPTION WHEN OTHERS THEN NULL;
END $$;

-- Titled players lookup (Chess.com usernames → title)
CREATE TABLE IF NOT EXISTS titled_players (
    username TEXT PRIMARY KEY,
    title    TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_titled_players_title ON titled_players (title);

-- Per-position opening stats (replaces monolithic JSONB cache)
CREATE TABLE IF NOT EXISTS user_opening_moves (
    id          BIGSERIAL PRIMARY KEY,
    user_id     BIGINT NOT NULL REFERENCES accounts(id),
    color       TEXT NOT NULL,
    parent_fen  TEXT NOT NULL,
    move_san    TEXT NOT NULL,
    result_fen  TEXT NOT NULL,
    depth       SMALLINT NOT NULL,
    games       INTEGER NOT NULL DEFAULT 0,
    wins        INTEGER NOT NULL DEFAULT 0,
    losses      INTEGER NOT NULL DEFAULT 0,
    draws       INTEGER NOT NULL DEFAULT 0,
    eval_cp     INTEGER,
    total_cp_loss BIGINT NOT NULL DEFAULT 0,
    cp_loss_count INTEGER NOT NULL DEFAULT 0,
    UNIQUE(user_id, color, parent_fen, move_san)
);
CREATE INDEX IF NOT EXISTS idx_uom_user_color_fen
    ON user_opening_moves (user_id, color, parent_fen);

-- Add cp_loss columns if missing on older schemas
DO $$ BEGIN
    ALTER TABLE user_opening_moves ADD COLUMN IF NOT EXISTS total_cp_loss BIGINT NOT NULL DEFAULT 0;
    ALTER TABLE user_opening_moves ADD COLUMN IF NOT EXISTS cp_loss_count INTEGER NOT NULL DEFAULT 0;
EXCEPTION WHEN OTHERS THEN NULL;
END $$;

-- Track which games have been processed for opening stats
DO $$ BEGIN
    ALTER TABLE user_games ADD COLUMN IF NOT EXISTS opening_stats_at TIMESTAMPTZ;
EXCEPTION WHEN OTHERS THEN NULL;
END $$;

-- Drop old monolithic JSONB opening tree cache (replaced by user_opening_moves)
DROP TABLE IF EXISTS user_opening_trees;

-- Master opening book (FEN-keyed, from KingBase 2500+ games)
CREATE TABLE IF NOT EXISTS opening_book (
    parent_fen  TEXT NOT NULL,
    move_san    TEXT NOT NULL,
    result_fen  TEXT NOT NULL,
    games       INTEGER NOT NULL,
    white_wins  INTEGER NOT NULL,
    draws       INTEGER NOT NULL,
    black_wins  INTEGER NOT NULL,
    avg_rating  SMALLINT,
    PRIMARY KEY (parent_fen, move_san)
);
CREATE INDEX IF NOT EXISTS idx_opening_book_parent_fen ON opening_book (parent_fen);

-- Opening trainer puzzles (generated from common mistakes)
CREATE TABLE IF NOT EXISTS trainer_puzzles (
    id              TEXT PRIMARY KEY,
    eco             TEXT NOT NULL,
    opening_name    TEXT NOT NULL,
    mistake_san     TEXT NOT NULL,
    mistake_uci     TEXT NOT NULL,
    pre_mistake_fen TEXT NOT NULL,
    solver_color    TEXT NOT NULL,
    root_eval       INTEGER NOT NULL,
    cp_loss         INTEGER NOT NULL,
    games           INTEGER NOT NULL DEFAULT 0,
    tree            JSONB NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_trainer_puzzles_opening_name ON trainer_puzzles (opening_name);

-- Per-user puzzle completion tracking
CREATE TABLE IF NOT EXISTS trainer_progress (
    user_id     BIGINT NOT NULL REFERENCES accounts(id),
    puzzle_id   TEXT NOT NULL REFERENCES trainer_puzzles(id) ON DELETE CASCADE,
    completed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, puzzle_id)
);
CREATE INDEX IF NOT EXISTS idx_trainer_progress_user ON trainer_progress (user_id);

-- Opening trainer trees (Chessable-style move trees built from popularity + theory)
CREATE TABLE IF NOT EXISTS trainer_trees (
    id           TEXT PRIMARY KEY,           -- slug, e.g. "evans-gambit-white"
    name         TEXT NOT NULL,              -- display name, e.g. "Evans Gambit Accepted - White"
    color        TEXT NOT NULL,              -- "white" | "black"
    start_moves  TEXT NOT NULL DEFAULT '',   -- e.g. "e4 e5 Nf3 Nc6 Bc4 Bc5 b4 Bxb4"
    start_fen    TEXT NOT NULL,              -- root FEN (4-field)
    nodes_count  INTEGER NOT NULL,           -- post-prune node count
    tree         JSONB NOT NULL,             -- full tree JSON
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_trainer_trees_name ON trainer_trees (name);
ALTER TABLE trainer_trees ADD COLUMN IF NOT EXISTS lines_count INTEGER NOT NULL DEFAULT 0;

-- Per-user, per-tree learned moves (FEN, move) pairs
CREATE TABLE IF NOT EXISTS trainer_tree_progress (
    user_id    BIGINT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    tree_id    TEXT NOT NULL REFERENCES trainer_trees(id) ON DELETE CASCADE,
    fen        TEXT NOT NULL,
    move_san   TEXT NOT NULL,
    learned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, tree_id, fen, move_san)
);
CREATE INDEX IF NOT EXISTS idx_ttp_user_tree ON trainer_tree_progress (user_id, tree_id);

-- Precomputed opening mistakes (one row per mistake per game)
CREATE TABLE IF NOT EXISTS game_opening_mistakes (
    game_id     BIGINT NOT NULL REFERENCES user_games(id) ON DELETE CASCADE,
    user_id     BIGINT NOT NULL,
    ply         SMALLINT NOT NULL,
    move_san    TEXT NOT NULL,
    cp_loss     DOUBLE PRECISION NOT NULL,
    best_move   TEXT,
    color       TEXT NOT NULL,
    line        TEXT NOT NULL,
    PRIMARY KEY (game_id, ply)
);
CREATE INDEX IF NOT EXISTS idx_gom_user_id ON game_opening_mistakes (user_id);

-- Precomputed clean opening depth per game
CREATE TABLE IF NOT EXISTS game_opening_clean_plies (
    game_id         BIGINT PRIMARY KEY REFERENCES user_games(id) ON DELETE CASCADE,
    user_id         BIGINT NOT NULL,
    color           TEXT NOT NULL,
    clean_up_to     SMALLINT NOT NULL,
    clean_depth     SMALLINT NOT NULL,
    line            TEXT NOT NULL,
    avg_cp_loss     DOUBLE PRECISION NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_gocp_user_id ON game_opening_clean_plies (user_id);

-- Hard move anti-puzzles (single correct move positions)
CREATE TABLE IF NOT EXISTS trainer_hard_moves (
    id               TEXT PRIMARY KEY,
    opening_name     TEXT NOT NULL,
    fen              TEXT NOT NULL,
    sequence         TEXT NOT NULL,
    ply              SMALLINT NOT NULL,
    side             TEXT NOT NULL,
    games            INTEGER NOT NULL DEFAULT 0,
    best_move        TEXT NOT NULL,
    best_eval_cp     INTEGER NOT NULL,
    best_maia_pct    REAL,
    second_move      TEXT,
    second_eval_cp   INTEGER,
    gap_cp           INTEGER NOT NULL,
    mistake_move     TEXT NOT NULL,
    mistake_eval_cp  INTEGER NOT NULL,
    mistake_maia_pct REAL,
    eval_loss_cp     INTEGER NOT NULL,
    maia_top_3       JSONB,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_trainer_hard_moves_opening ON trainer_hard_moves (opening_name);

-- Per-user hard move completion tracking
CREATE TABLE IF NOT EXISTS trainer_hard_move_progress (
    user_id      BIGINT NOT NULL REFERENCES accounts(id),
    hard_move_id TEXT NOT NULL REFERENCES trainer_hard_moves(id) ON DELETE CASCADE,
    completed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, hard_move_id)
);
CREATE INDEX IF NOT EXISTS idx_trainer_hm_progress_user ON trainer_hard_move_progress (user_id);

-- Play-vs-Maia trainer card positions
CREATE TABLE IF NOT EXISTS trainer_maia_positions (
    id         TEXT PRIMARY KEY,
    title      TEXT NOT NULL,
    fen        TEXT NOT NULL,
    user_side  TEXT NOT NULL CHECK (user_side IN ('white', 'black')),
    notes      TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_trainer_maia_positions_title ON trainer_maia_positions (title);

-- opening_name on trees and maia positions (for catalog grouping)
ALTER TABLE trainer_trees ADD COLUMN IF NOT EXISTS opening_name TEXT;
ALTER TABLE trainer_maia_positions ADD COLUMN IF NOT EXISTS opening_name TEXT;

-- Opening metadata (root FEN + user color for catalog cards)
CREATE TABLE IF NOT EXISTS trainer_opening_meta (
    opening_name TEXT PRIMARY KEY,
    root_fen     TEXT NOT NULL,
    user_color   TEXT NOT NULL DEFAULT 'white'
);
ALTER TABLE trainer_opening_meta ADD COLUMN IF NOT EXISTS user_color TEXT NOT NULL DEFAULT 'white';
INSERT INTO trainer_opening_meta (opening_name, root_fen, user_color) VALUES
    ('Evans Gambit',              'r1bqk1nr/pppp1ppp/2n5/2b1p3/1PB1P3/5N2/P1PP1PPP/RNBQK2R b KQkq -', 'white'),
    ('Kings Gambit',              'rnbqkbnr/pppp1ppp/8/4p3/4PP2/8/PPPP2PP/RNBQKBNR b KQkq -',          'white'),
    ('Italian Game Knight Attack','r1bqkb1r/pppp1ppp/2n2n2/4p1N1/2B1P3/8/PPPP1PPP/RNBQK2R b KQkq -',  'white'),
    ('Knight Attack Traps',       'r1bqkb1r/pppp1ppp/2n2n2/4p1N1/2B1P3/8/PPPP1PPP/RNBQK2R b KQkq -',  'white'),
    ('Sicilian Dragon',           'rnbqkb1r/pp2pp1p/3p1np1/8/3NP3/2N5/PPP2PPP/R1BQKB1R w KQkq -',     'black'),
    ('Smith-Morra Gambit',        'rnbqkbnr/pp1ppppp/8/8/3pP3/2P5/PP3PPP/RNBQKBNR b KQkq -',           'white'),
    ('Stafford Gambit',           'r1bqkb1r/pppp1ppp/2n2n2/4N3/4P3/8/PPPP1PPP/RNBQKB1R w KQkq -',     'black'),
    ('Traxler Counterattack',     'r1bqk2r/pppp1ppp/2n2n2/2b1p1N1/2B1P3/8/PPPP1PPP/RNBQK2R w KQkq -', 'black')
ON CONFLICT (opening_name) DO UPDATE SET root_fen = EXCLUDED.root_fen, user_color = EXCLUDED.user_color;

-- Backfill: trees — strip " Accepted", " (White)", " (Black)" to get canonical opening name
UPDATE trainer_trees SET opening_name = TRIM(
    REGEXP_REPLACE(
        REGEXP_REPLACE(name, '\s*\((White|Black)\)\s*$', ''),
        '\s+Accepted\s*$', ''
    )
) WHERE opening_name IS NULL;

-- Backfill: maia — use title prefix before ":"
UPDATE trainer_maia_positions SET opening_name = TRIM(SPLIT_PART(title, ':', 1))
WHERE opening_name IS NULL AND title LIKE '%:%';
UPDATE trainer_maia_positions SET opening_name = title
WHERE opening_name IS NULL;
"#;
