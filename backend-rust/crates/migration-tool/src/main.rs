//! SQLite to Postgres migration tool.
//! Reads from the existing SQLite database and writes to Postgres.

use anyhow::Result;
use rusqlite::Connection;
use sqlx::PgPool;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set (Postgres)");
    let sqlite_path =
        std::env::var("SQLITE_PATH").unwrap_or_else(|_| "../backend/data/chess.db".to_string());

    tracing::info!("Connecting to Postgres...");
    let pg_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    tracing::info!("Opening SQLite at {sqlite_path}...");
    let sqlite_conn = Connection::open(&sqlite_path)?;
    sqlite_conn.execute_batch("PRAGMA journal_mode=WAL;")?;

    // Migrate in order to preserve foreign keys
    migrate_accounts(&sqlite_conn, &pg_pool).await?;
    migrate_platform_users(&sqlite_conn, &pg_pool).await?;
    migrate_user_games(&sqlite_conn, &pg_pool).await?;
    migrate_game_tags(&sqlite_conn, &pg_pool).await?;
    migrate_game_analysis(&sqlite_conn, &pg_pool).await?;
    migrate_posts(&sqlite_conn, &pg_pool).await?;
    migrate_opening_trees(&sqlite_conn, &pg_pool).await?;

    // Reset sequences to max id + 1
    reset_sequences(&pg_pool).await?;

    // Validate row counts
    validate_counts(&sqlite_conn, &pg_pool).await?;

    tracing::info!("Migration complete!");
    Ok(())
}

async fn migrate_accounts(sqlite: &Connection, pg: &PgPool) -> Result<()> {
    tracing::info!("Migrating accounts...");

    let mut stmt = sqlite.prepare(
        "SELECT id, username, email, password_hash, display_name, chess_com_username, lichess_username, bio, avatar_url, created_at FROM accounts",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<String>>(5)?,
            row.get::<_, Option<String>>(6)?,
            row.get::<_, Option<String>>(7)?,
            row.get::<_, Option<String>>(8)?,
            row.get::<_, Option<String>>(9)?,
        ))
    })?;

    let mut count = 0i64;
    for row in rows {
        let (id, username, email, password_hash, display_name, chess_com_username, lichess_username, bio, avatar_url, created_at) = row?;

        sqlx::query(
            r#"INSERT INTO accounts (id, username, email, password_hash, display_name, chess_com_username, lichess_username, bio, avatar_url, created_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, COALESCE($10::timestamptz, NOW()))
               ON CONFLICT (id) DO NOTHING"#,
        )
        .bind(id)
        .bind(&username)
        .bind(&email)
        .bind(&password_hash)
        .bind(&display_name)
        .bind(&chess_com_username)
        .bind(&lichess_username)
        .bind(&bio)
        .bind(&avatar_url)
        .bind(created_at.as_deref())
        .execute(pg)
        .await?;
        count += 1;
    }

    tracing::info!("  Migrated {count} accounts");
    Ok(())
}

async fn migrate_platform_users(sqlite: &Connection, pg: &PgPool) -> Result<()> {
    tracing::info!("Migrating platform_users (from SQLite users table)...");

    let mut stmt =
        sqlite.prepare("SELECT id, chess_com_username, last_synced_at, created_at FROM users")?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
        ))
    })?;

    let mut count = 0i64;
    for row in rows {
        let (id, username, last_synced, created_at) = row?;

        sqlx::query(
            r#"INSERT INTO platform_users (id, chess_com_username, last_synced_at, created_at)
               VALUES ($1, $2, $3::timestamptz, COALESCE($4::timestamptz, NOW()))
               ON CONFLICT (id) DO NOTHING"#,
        )
        .bind(id)
        .bind(&username)
        .bind(last_synced.as_deref())
        .bind(created_at.as_deref())
        .execute(pg)
        .await?;
        count += 1;
    }

    tracing::info!("  Migrated {count} platform_users");
    Ok(())
}

async fn migrate_user_games(sqlite: &Connection, pg: &PgPool) -> Result<()> {
    tracing::info!("Migrating user_games...");

    let mut stmt = sqlite.prepare(
        "SELECT id, user_id, chess_com_game_id, opponent, opponent_rating, user_rating, result, user_color, time_control, date, pgn, moves, tags, tcn, source, analyzed_at, created_at, updated_at FROM user_games",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<i32>>(4)?,
            row.get::<_, Option<i32>>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, Option<String>>(8)?,
            row.get::<_, Option<String>>(9)?,
            row.get::<_, Option<String>>(10)?,
            row.get::<_, Option<String>>(11)?,
            row.get::<_, Option<String>>(12)?,
            row.get::<_, Option<String>>(13)?,
            row.get::<_, Option<String>>(14)?,
            row.get::<_, Option<String>>(15)?,
            row.get::<_, Option<String>>(16)?,
            row.get::<_, Option<String>>(17)?,
        ))
    })?;

    let mut count = 0i64;
    for row in rows {
        let (id, user_id, game_id, opponent, opp_rating, user_rating, result, user_color, time_control, date, _pgn, moves_json, tags_json, tcn, source, analyzed_at, created_at, updated_at) = row?;

        let source = source.unwrap_or_else(|| "chess_com".to_string());

        // Parse JSON fields
        let tags: serde_json::Value = tags_json
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or(serde_json::json!([]));

        let moves: serde_json::Value = moves_json
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or(serde_json::Value::Null);

        sqlx::query(
            r#"INSERT INTO user_games (id, user_id, chess_com_game_id, opponent, opponent_rating, user_rating, result, user_color, time_control, date, moves, tags, tcn, source, analyzed_at, created_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15::timestamptz, COALESCE($16::timestamptz, NOW()), COALESCE($17::timestamptz, NOW()))
               ON CONFLICT (id) DO NOTHING"#,
        )
        .bind(id)
        .bind(user_id)
        .bind(&game_id)
        .bind(&opponent)
        .bind(opp_rating)
        .bind(user_rating)
        .bind(&result)
        .bind(&user_color)
        .bind(time_control.as_deref())
        .bind(date.as_deref())
        .bind(&moves)
        .bind(&tags)
        .bind(tcn.as_deref())
        .bind(&source)
        .bind(analyzed_at.as_deref())
        .bind(created_at.as_deref())
        .bind(updated_at.as_deref())
        .execute(pg)
        .await?;
        count += 1;

        if count % 1000 == 0 {
            tracing::info!("  Migrated {count} games...");
        }
    }

    tracing::info!("  Migrated {count} user_games");
    Ok(())
}

async fn migrate_game_tags(sqlite: &Connection, pg: &PgPool) -> Result<()> {
    tracing::info!("Migrating game_tags...");

    let mut stmt = sqlite.prepare("SELECT id, game_id, tag FROM game_tags")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;

    let mut count = 0i64;
    for row in rows {
        let (id, game_id, tag) = row?;
        sqlx::query(
            "INSERT INTO game_tags (id, game_id, tag) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        )
        .bind(id)
        .bind(game_id)
        .bind(&tag)
        .execute(pg)
        .await?;
        count += 1;
    }

    tracing::info!("  Migrated {count} game_tags");
    Ok(())
}

async fn migrate_game_analysis(sqlite: &Connection, pg: &PgPool) -> Result<()> {
    tracing::info!("Migrating game_analysis...");

    let mut stmt = sqlite.prepare(
        "SELECT id, game_id, white_accuracy, black_accuracy, white_avg_cp_loss, black_avg_cp_loss, white_classifications, black_classifications, moves, phase_accuracy, first_inaccuracy_move, created_at FROM game_analysis",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, f64>(2)?,
            row.get::<_, f64>(3)?,
            row.get::<_, f64>(4)?,
            row.get::<_, f64>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, String>(8)?,
            row.get::<_, Option<String>>(9)?,
            row.get::<_, Option<String>>(10)?,
            row.get::<_, Option<String>>(11)?,
        ))
    })?;

    let mut count = 0i64;
    for row in rows {
        let (id, game_id, wa, ba, wacl, bacl, wc, bc, moves, pa, fim, created_at) = row?;

        let wc_json: serde_json::Value = serde_json::from_str(&wc)?;
        let bc_json: serde_json::Value = serde_json::from_str(&bc)?;
        let moves_json: serde_json::Value = serde_json::from_str(&moves)?;
        let pa_json: Option<serde_json::Value> = pa.as_deref().and_then(|s| serde_json::from_str(s).ok());
        let fim_json: Option<serde_json::Value> = fim.as_deref().and_then(|s| serde_json::from_str(s).ok());

        sqlx::query(
            r#"INSERT INTO game_analysis (id, game_id, white_accuracy, black_accuracy, white_avg_cp_loss, black_avg_cp_loss, white_classifications, black_classifications, moves, phase_accuracy, first_inaccuracy_move, created_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, COALESCE($12::timestamptz, NOW()))
               ON CONFLICT (id) DO NOTHING"#,
        )
        .bind(id)
        .bind(game_id)
        .bind(wa)
        .bind(ba)
        .bind(wacl)
        .bind(bacl)
        .bind(&wc_json)
        .bind(&bc_json)
        .bind(&moves_json)
        .bind(&pa_json)
        .bind(&fim_json)
        .bind(created_at.as_deref())
        .execute(pg)
        .await?;
        count += 1;
    }

    tracing::info!("  Migrated {count} game_analysis rows");
    Ok(())
}

async fn migrate_posts(sqlite: &Connection, pg: &PgPool) -> Result<()> {
    tracing::info!("Migrating posts...");

    let mut stmt = sqlite.prepare(
        "SELECT id, account_id, post_type, content, game_id, key_position_index, created_at, updated_at FROM posts",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<i64>>(4)?,
            row.get::<_, Option<i32>>(5)?,
            row.get::<_, Option<String>>(6)?,
            row.get::<_, Option<String>>(7)?,
        ))
    })?;

    let mut count = 0i64;
    for row in rows {
        let (id, account_id, post_type, content, game_id, kpi, created_at, updated_at) = row?;

        sqlx::query(
            r#"INSERT INTO posts (id, account_id, post_type, content, game_id, key_position_index, created_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, $6, COALESCE($7::timestamptz, NOW()), COALESCE($8::timestamptz, NOW()))
               ON CONFLICT (id) DO NOTHING"#,
        )
        .bind(id)
        .bind(account_id)
        .bind(&post_type)
        .bind(&content)
        .bind(game_id)
        .bind(kpi)
        .bind(created_at.as_deref())
        .bind(updated_at.as_deref())
        .execute(pg)
        .await?;
        count += 1;
    }

    tracing::info!("  Migrated {count} posts");
    Ok(())
}

async fn migrate_opening_trees(sqlite: &Connection, pg: &PgPool) -> Result<()> {
    tracing::info!("Migrating opening trees...");

    let mut stmt = sqlite.prepare(
        "SELECT id, user_id, color, tree_json, total_games, updated_at FROM user_opening_trees",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<i32>>(4)?,
            row.get::<_, Option<String>>(5)?,
        ))
    })?;

    let mut count = 0i64;
    for row in rows {
        let (id, user_id, color, tree_json, total_games, updated_at) = row?;

        let tree: serde_json::Value = serde_json::from_str(&tree_json)?;

        sqlx::query(
            r#"INSERT INTO user_opening_trees (id, user_id, color, tree_json, total_games, updated_at)
               VALUES ($1, $2, $3, $4, $5, COALESCE($6::timestamptz, NOW()))
               ON CONFLICT (id) DO NOTHING"#,
        )
        .bind(id)
        .bind(user_id)
        .bind(&color)
        .bind(&tree)
        .bind(total_games.unwrap_or(0))
        .bind(updated_at.as_deref())
        .execute(pg)
        .await?;
        count += 1;
    }

    tracing::info!("  Migrated {count} opening trees");
    Ok(())
}

async fn reset_sequences(pg: &PgPool) -> Result<()> {
    tracing::info!("Resetting Postgres sequences...");

    let tables = [
        ("accounts", "accounts_id_seq"),
        ("platform_users", "platform_users_id_seq"),
        ("user_games", "user_games_id_seq"),
        ("game_tags", "game_tags_id_seq"),
        ("game_analysis", "game_analysis_id_seq"),
        ("posts", "posts_id_seq"),
        ("user_opening_trees", "user_opening_trees_id_seq"),
    ];

    for (table, seq) in tables {
        let query = format!(
            "SELECT setval('{}', COALESCE((SELECT MAX(id) FROM {}), 0) + 1, false)",
            seq, table
        );
        sqlx::query(&query).execute(pg).await?;
    }

    tracing::info!("  Sequences reset");
    Ok(())
}

async fn validate_counts(sqlite: &Connection, pg: &PgPool) -> Result<()> {
    tracing::info!("Validating row counts...");

    let tables = [
        ("accounts", "accounts"),
        ("users", "platform_users"),
        ("user_games", "user_games"),
        ("game_tags", "game_tags"),
        ("game_analysis", "game_analysis"),
        ("posts", "posts"),
        ("user_opening_trees", "user_opening_trees"),
    ];

    for (sqlite_table, pg_table) in tables {
        let sqlite_count: i64 = sqlite.query_row(
            &format!("SELECT COUNT(*) FROM {sqlite_table}"),
            [],
            |row| row.get(0),
        )?;

        let pg_count: (i64,) =
            sqlx::query_as(&format!("SELECT COUNT(*) FROM {pg_table}"))
                .fetch_one(pg)
                .await?;

        let status = if sqlite_count == pg_count.0 {
            "OK"
        } else {
            "MISMATCH"
        };

        tracing::info!(
            "  {}: SQLite={}, Postgres={} [{}]",
            pg_table,
            sqlite_count,
            pg_count.0,
            status
        );
    }

    Ok(())
}
