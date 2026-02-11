use std::collections::HashMap;
use std::sync::RwLock;

use sqlx::PgPool;

use crate::error::AppError;

const VALID_TITLES: &[&str] = &[
    "GM", "IM", "FM", "CM", "NM", "WGM", "WIM", "WFM", "WCM", "WNM",
];

/// In-memory cache: lowercase username → title (e.g. "hikaru" → "GM")
static TITLED_CACHE: std::sync::LazyLock<RwLock<HashMap<String, String>>> =
    std::sync::LazyLock::new(|| RwLock::new(HashMap::new()));

/// Load all titled players from the database into the in-memory cache.
pub async fn load_cache(pool: &PgPool) -> Result<usize, AppError> {
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT username, title FROM titled_players")
            .fetch_all(pool)
            .await
            .map_err(AppError::Sqlx)?;

    let count = rows.len();
    let mut cache = TITLED_CACHE.write().unwrap();
    cache.clear();
    for (username, title) in rows {
        cache.insert(username.to_lowercase(), title);
    }

    tracing::info!("Titled players cache loaded: {} entries", count);
    Ok(count)
}

/// Lookup a username in the in-memory cache (case-insensitive).
pub fn lookup(username: &str) -> Option<String> {
    let cache = TITLED_CACHE.read().ok()?;
    cache.get(&username.to_lowercase()).cloned()
}

/// Check if a title string is one of the valid chess titles.
pub fn is_valid_title(title: &str) -> bool {
    VALID_TITLES.contains(&title)
}

/// Count entries currently in the cache.
pub fn cache_size() -> usize {
    TITLED_CACHE.read().map(|c| c.len()).unwrap_or(0)
}

/// Fetch titled players from Chess.com API for all valid titles,
/// upsert into the database, and reload the cache.
pub async fn seed_from_chesscom(pool: &PgPool) -> Result<usize, AppError> {
    let client = reqwest::Client::builder()
        .user_agent("AlpineChess/1.0")
        .build()
        .map_err(|e| AppError::Internal(format!("Failed to build HTTP client: {e}")))?;

    let mut total = 0usize;

    for title in VALID_TITLES {
        let url = format!("https://api.chess.com/pub/titled/{}", title);
        tracing::info!("Fetching titled players: {}", url);

        let resp = match client.get(&url).send().await {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("Failed to fetch {}: {}", url, e);
                continue;
            }
        };

        if !resp.status().is_success() {
            tracing::warn!("Non-success status for {}: {}", url, resp.status());
            continue;
        }

        let body: serde_json::Value = match resp.json().await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Failed to parse JSON for {}: {}", title, e);
                continue;
            }
        };

        let players = match body["players"].as_array() {
            Some(arr) => arr,
            None => {
                tracing::warn!("No 'players' array in response for {}", title);
                continue;
            }
        };

        let mut batch_count = 0;
        for player in players {
            if let Some(username) = player.as_str() {
                let lower = username.to_lowercase();
                sqlx::query(
                    "INSERT INTO titled_players (username, title) VALUES ($1, $2) ON CONFLICT (username) DO UPDATE SET title = $2",
                )
                .bind(&lower)
                .bind(*title)
                .execute(pool)
                .await
                .map_err(AppError::Sqlx)?;
                batch_count += 1;
            }
        }

        tracing::info!("  {} {}: {} players", title, "loaded", batch_count);
        total += batch_count;
    }

    // Reload cache from DB
    load_cache(pool).await?;

    Ok(total)
}

/// Insert titled tags for a batch of games. Takes a vec of (game_id, title) pairs.
/// Inserts both "titled" and the specific title (e.g. "GM") into game_tags.
pub async fn insert_title_tags(
    pool: &PgPool,
    game_title_pairs: &[(i64, String)],
) -> Result<usize, AppError> {
    let mut count = 0;
    for (game_id, title) in game_title_pairs {
        // Insert generic "titled" tag
        sqlx::query(
            "INSERT INTO game_tags (game_id, tag) VALUES ($1, 'titled') ON CONFLICT DO NOTHING",
        )
        .bind(game_id)
        .execute(pool)
        .await
        .map_err(AppError::Sqlx)?;

        // Insert specific title tag (e.g. "GM")
        sqlx::query(
            "INSERT INTO game_tags (game_id, tag) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(game_id)
        .bind(title)
        .execute(pool)
        .await
        .map_err(AppError::Sqlx)?;

        count += 1;
    }
    Ok(count)
}
