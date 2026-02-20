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
"#;
