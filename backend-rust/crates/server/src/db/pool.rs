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
    lichess_username  TEXT,
    bio           TEXT,
    avatar_url    TEXT,
    chess_com_last_synced_at TIMESTAMPTZ,
    lichess_last_synced_at   TIMESTAMPTZ,
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
    pgn               TEXT,
    moves             JSONB,
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
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_game_analysis_game_id
    ON game_analysis (game_id);

-- Posts
CREATE TABLE IF NOT EXISTS posts (
    id                 BIGSERIAL PRIMARY KEY,
    account_id         BIGINT NOT NULL REFERENCES accounts(id),
    post_type          TEXT NOT NULL,
    content            TEXT NOT NULL,
    game_id            BIGINT REFERENCES user_games(id),
    key_position_index INTEGER DEFAULT 0,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_posts_account_id  ON posts (account_id);
CREATE INDEX IF NOT EXISTS idx_posts_created_at  ON posts (created_at DESC);

-- Opening tree cache
CREATE TABLE IF NOT EXISTS user_opening_trees (
    id          BIGSERIAL PRIMARY KEY,
    user_id     BIGINT NOT NULL REFERENCES accounts(id),
    color       TEXT NOT NULL,
    tree_json   JSONB NOT NULL,
    total_games INTEGER DEFAULT 0,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, color)
);
"#;
