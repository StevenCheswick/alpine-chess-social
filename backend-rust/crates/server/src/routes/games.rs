use axum::{extract::Path, extract::Query, Extension, Json};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use sqlx::PgPool;

use crate::auth::middleware::AuthUser;
use crate::clients;
use crate::clients::sqs::AnalysisQueue;
use crate::db::{analysis, games, opening_moves, titled_players, users};
use crate::error::AppError;

#[derive(Deserialize)]
pub struct StoredGamesQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub tags: Option<String>,
    pub platform: Option<String>,
    pub analyzed: Option<bool>,
}

#[derive(Deserialize)]
pub struct TagsQuery {
    pub selected_tags: Option<String>,
}

/// GET /api/games/stored
pub async fn get_stored_games(
    Extension(pool): Extension<PgPool>,
    Query(q): Query<StoredGamesQuery>,
    user: AuthUser,
) -> Result<Json<JsonValue>, AppError> {
    let raw_limit = q.limit.unwrap_or(50);
    let offset = q.offset.unwrap_or(0).max(0);
    let account_id = user.id;

    let source = q.platform.as_deref().filter(|p| *p == "chess_com");

    let tags_list: Option<Vec<String>> = q.tags.as_ref().map(|t| {
        t.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    });

    // limit=0 means "just give me the count" — skip fetching/decoding rows
    if raw_limit == 0 {
        let total = games::get_user_games_count_filtered(
            &pool,
            account_id,
            tags_list.as_deref(),
            source,
            q.analyzed,
        )
        .await?;

        return Ok(Json(serde_json::json!({
            "platform": q.platform,
            "games": [],
            "total": total,
            "limit": 0,
            "offset": offset,
            "tags": tags_list,
            "hasMore": false,
        })));
    }

    let limit = raw_limit.min(10000);

    let games_list = games::get_user_games_paginated(
        &pool,
        account_id,
        limit,
        offset,
        tags_list.as_deref(),
        source,
        q.analyzed,
    )
    .await?;

    let total = games::get_user_games_count_filtered(
        &pool,
        account_id,
        tags_list.as_deref(),
        source,
        q.analyzed,
    )
    .await?;

    Ok(Json(serde_json::json!({
        "platform": q.platform,
        "games": games_list,
        "total": total,
        "limit": limit,
        "offset": offset,
        "tags": tags_list,
        "hasMore": offset + games_list.len() as i64 > total,
    })))
}

/// GET /api/games/tags
pub async fn get_game_tags(
    Extension(pool): Extension<PgPool>,
    Query(q): Query<TagsQuery>,
    user: AuthUser,
) -> Result<Json<JsonValue>, AppError> {
    let account_id = user.id;

    let tag_counts = if let Some(ref selected) = q.selected_tags {
        let tags_list: Vec<String> = selected
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if tags_list.is_empty() {
            games::get_user_tag_counts(&pool, account_id).await?
        } else {
            games::get_user_tag_counts_filtered(&pool, account_id, &tags_list).await?
        }
    } else {
        games::get_user_tag_counts(&pool, account_id).await?
    };

    Ok(Json(serde_json::json!({
        "tags": tag_counts,
        "selectedTags": q.selected_tags.as_ref().map(|s| s.split(',').collect::<Vec<_>>()).unwrap_or_default(),
    })))
}

/// GET /api/games/{game_id}
pub async fn get_game_by_id(
    Extension(pool): Extension<PgPool>,
    Path(game_id): Path<i64>,
    user: AuthUser,
) -> Result<Json<JsonValue>, AppError> {
    let game = games::get_game_by_id(&pool, user.id, game_id)
        .await?
        .ok_or(AppError::NotFound("Game not found".into()))?;

    Ok(Json(game))
}

/// POST /api/games/sync
pub async fn sync_games(
    Extension(pool): Extension<PgPool>,
    user: AuthUser,
) -> Result<Json<JsonValue>, AppError> {
    let chess_com_username = user
        .chess_com_username
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or(AppError::BadRequest(
            "No Chess.com username linked to account".into(),
        ))?
        .to_string();

    let account_id = user.id;
    let last_synced = users::get_last_synced(&pool, account_id, "chess_com").await?;
    let is_first_sync = last_synced.is_none();

    let client = clients::chess_com::ChessComClient::new();
    let now = chrono::Utc::now();
    let max_games: usize = 1000;

    // Track the last archive month we processed (for backfill cursor)
    let mut stopped_at_month: Option<(i32, u32)> = None;
    let mut all_archives_consumed = false;

    let pgn_tcn_pairs = if is_first_sync {
        tracing::info!("First sync for {} — fetching up to {} games", chess_com_username, max_games);
        let mut all_pairs = Vec::new();

        // Use the archives endpoint to only fetch months that have games
        let archive_months = client.fetch_archives(&chess_com_username).await.unwrap_or_default();
        tracing::info!("  Found {} monthly archives", archive_months.len());

        let mut hit_limit = false;
        for (i, (year, month)) in archive_months.iter().enumerate() {
            match client.fetch_user_games(&chess_com_username, Some(*year), Some(*month), true).await {
                Ok(pairs) => {
                    if !pairs.is_empty() {
                        tracing::info!("  {}/{:02}: {} games", year, month, pairs.len());
                        all_pairs.extend(pairs);
                        if all_pairs.len() >= max_games {
                            all_pairs.truncate(max_games);
                            stopped_at_month = Some((*year, *month));
                            hit_limit = true;
                            break;
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("  {}/{:02}: Error - {}", year, month, e);
                }
            }
            // If this is the last archive and we didn't hit the limit
            if i == archive_months.len() - 1 {
                all_archives_consumed = true;
            }
        }
        if !hit_limit && archive_months.is_empty() {
            all_archives_consumed = true;
        }
        all_pairs
    } else {
        // Re-sync: fetch all months since last sync (not just current month)
        let last = last_synced.unwrap(); // safe: is_first_sync is false
        let since_year = last.year();
        let since_month = last.month();
        tracing::info!(
            "Re-sync for {} — fetching archives from {}/{:02} to {}/{:02}",
            chess_com_username, since_year, since_month, now.year(), now.month()
        );

        let archive_months = client.fetch_archives(&chess_com_username).await.unwrap_or_default();
        let mut all_pairs = Vec::new();

        for (year, month) in &archive_months {
            // Skip months before last sync
            if (*year, *month) < (since_year, since_month) {
                continue;
            }
            match client.fetch_user_games(&chess_com_username, Some(*year), Some(*month), true).await {
                Ok(pairs) => {
                    if !pairs.is_empty() {
                        tracing::info!("  {}/{:02}: {} games", year, month, pairs.len());
                        all_pairs.extend(pairs);
                        if all_pairs.len() >= max_games {
                            all_pairs.truncate(max_games);
                            break;
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("  {}/{:02}: Error - {}", year, month, e);
                }
            }
        }
        all_pairs
    };

    let synced_count = if !pgn_tcn_pairs.is_empty() {
        let game_records = build_game_records(&pgn_tcn_pairs, &chess_com_username);
        let count = games::upsert_games(&pool, account_id, &game_records, "chess_com").await?;

        // Incrementally populate opening stats for newly synced games
        opening_moves::populate_opening_stats(&pool, account_id).await?;

        // Tag titled opponents (Chess.com: lookup in-memory cache)
        let source_ids: Vec<String> = game_records
            .iter()
            .filter_map(|g| g["id"].as_str().map(|s| s.to_string()))
            .collect();
        let db_games = games::get_game_ids_and_opponents(&pool, account_id, "chess_com", &source_ids).await?;
        let title_pairs: Vec<(i64, String)> = db_games
            .iter()
            .filter_map(|(db_id, _source_id, opponent)| {
                titled_players::lookup(opponent).map(|title| (*db_id, title))
            })
            .collect();
        if !title_pairs.is_empty() {
            let tagged = titled_players::insert_title_tags(&pool, &title_pairs).await?;
            tracing::info!("Tagged {} Chess.com games with titled opponent tags", tagged);
        }

        count
    } else {
        0
    };

    users::update_last_synced(&pool, account_id, "chess_com").await?;

    // Set backfill cursor on first sync
    let (oldest_synced_month, has_more_history) = if is_first_sync {
        if all_archives_consumed {
            users::update_oldest_synced_month(&pool, account_id, "complete").await?;
            (Some("complete".to_string()), false)
        } else if let Some((y, m)) = stopped_at_month {
            let cursor = format!("{}-{:02}", y, m);
            users::update_oldest_synced_month(&pool, account_id, &cursor).await?;
            (Some(cursor), true)
        } else {
            (None, false)
        }
    } else {
        // For re-syncs, read existing cursor
        let cursor = users::get_oldest_synced_month(&pool, account_id).await?;
        let more = cursor.as_deref().map(|c| c != "complete").unwrap_or(false);
        (cursor, more)
    };

    let total_games = games::get_user_games_count(&pool, account_id, None).await?;

    Ok(Json(serde_json::json!({
        "username": chess_com_username,
        "synced": synced_count,
        "total": total_games,
        "lastSyncedAt": users::get_last_synced(&pool, account_id, "chess_com").await?.map(|t| t.to_rfc3339()),
        "isFirstSync": is_first_sync,
        "oldestSyncedMonth": oldest_synced_month,
        "hasMoreHistory": has_more_history,
    })))
}

/// GET /api/games/{game_id}/analysis
pub async fn get_game_analysis(
    Extension(pool): Extension<PgPool>,
    Path(game_id): Path<i64>,
    user: AuthUser,
) -> Result<Json<JsonValue>, AppError> {
    // Verify game belongs to user
    let _game = games::get_game_by_id(&pool, user.id, game_id)
        .await?
        .ok_or(AppError::NotFound("Game not found".into()))?;

    let result = analysis::get_game_analysis(&pool, game_id).await?;
    Ok(Json(result.unwrap_or(JsonValue::Null)))
}

/// POST /api/games/{game_id}/analysis
pub async fn save_game_analysis(
    Extension(pool): Extension<PgPool>,
    Path(game_id): Path<i64>,
    user: AuthUser,
    Json(body): Json<JsonValue>,
) -> Result<Json<JsonValue>, AppError> {
    let _game = games::get_game_by_id(&pool, user.id, game_id)
        .await?
        .ok_or(AppError::NotFound("Game not found".into()))?;

    analysis::save_game_analysis(&pool, game_id, &body).await?;
    super::dashboard::invalidate_stats_cache();

    Ok(Json(serde_json::json!({"success": true})))
}

#[derive(Deserialize)]
pub struct AnalyzeServerRequest {
    /// Specific game IDs to analyze
    pub game_ids: Option<Vec<i64>>,
    /// Or analyze all unanalyzed games (with optional filters)
    pub all_unanalyzed: Option<bool>,
    pub platform: Option<String>,
    pub tags: Option<String>,
    /// Max number of games to queue (for testing)
    pub limit: Option<usize>,
}

/// POST /api/games/analyze-server
/// Queue games for server-side analysis via AWS Batch
pub async fn analyze_server(
    Extension(pool): Extension<PgPool>,
    Extension(queue): Extension<Option<AnalysisQueue>>,
    user: AuthUser,
    Json(body): Json<AnalyzeServerRequest>,
) -> Result<Json<JsonValue>, AppError> {
    let queue = queue.ok_or_else(|| {
        AppError::BadRequest("Server-side analysis is not configured".into())
    })?;

    let game_ids = if let Some(ids) = body.game_ids {
        // Verify all games belong to user
        let verified = games::verify_game_ownership(&pool, user.id, &ids).await?;
        if verified.len() != ids.len() {
            return Err(AppError::BadRequest(
                "Some game IDs do not belong to this user".into(),
            ));
        }
        verified
    } else if body.all_unanalyzed == Some(true) {
        // Get all unanalyzed games for user
        let tags_list: Option<Vec<String>> = body.tags.as_ref().map(|t| {
            t.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        });
        let source = body.platform.as_deref().filter(|p| *p == "chess_com");

        let mut ids = games::get_unanalyzed_game_ids(&pool, user.id, tags_list.as_deref(), source).await?;
        // Apply limit if specified
        if let Some(limit) = body.limit {
            ids.truncate(limit);
        }
        ids
    } else {
        return Err(AppError::BadRequest(
            "Must provide game_ids or set all_unanalyzed=true".into(),
        ));
    };

    if game_ids.is_empty() {
        return Ok(Json(serde_json::json!({
            "queued": 0,
            "message": "No games to analyze"
        })));
    }

    let queued = queue
        .queue_games(&game_ids)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    tracing::info!(
        user_id = user.id,
        queued = queued,
        "Queued games for server-side analysis"
    );

    Ok(Json(serde_json::json!({
        "queued": queued,
        "total_requested": game_ids.len(),
    })))
}

/// GET /api/users/me/games
pub async fn get_my_games(
    Extension(pool): Extension<PgPool>,
    Query(q): Query<LimitQuery>,
    user: AuthUser,
) -> Result<Json<JsonValue>, AppError> {
    let limit = q.limit.unwrap_or(50).min(100);
    let games_list = games::get_user_games_paginated(
        &pool,
        user.id,
        limit,
        0,
        None,
        None,
        None,
    )
    .await?;

    Ok(Json(serde_json::json!({
        "games": games_list,
        "total": games_list.len(),
    })))
}

#[derive(Deserialize)]
pub struct LimitQuery {
    pub limit: Option<i64>,
}

/// GET /api/games/backfill/status
/// Returns whether the user has more history to load and the current cursor.
pub async fn backfill_status(
    Extension(pool): Extension<PgPool>,
    user: AuthUser,
) -> Result<Json<JsonValue>, AppError> {
    let account_id = user.id;
    let cursor = users::get_oldest_synced_month(&pool, account_id).await?;

    // For legacy users who synced before this feature, infer from earliest game
    let (oldest_synced_month, has_more_history) = match cursor {
        Some(ref c) if c == "complete" => (Some("complete".to_string()), false),
        Some(ref c) => (Some(c.clone()), true),
        None => {
            // Check if user has any games (i.e. they synced before this feature)
            let last_synced = users::get_last_synced(&pool, account_id, "chess_com").await?;
            if last_synced.is_some() {
                // Legacy user: infer cursor from earliest game date
                let earliest = games::get_earliest_game_date(&pool, account_id, "chess_com").await?;
                match earliest {
                    Some(month) => (Some(month), true),
                    None => (None, false),
                }
            } else {
                (None, false)
            }
        }
    };

    Ok(Json(serde_json::json!({
        "oldestSyncedMonth": oldest_synced_month,
        "hasMoreHistory": has_more_history,
    })))
}

/// POST /api/games/backfill
/// Fetch the next batch of older games, working backwards through Chess.com archives.
pub async fn backfill_games(
    Extension(pool): Extension<PgPool>,
    user: AuthUser,
) -> Result<Json<JsonValue>, AppError> {
    let chess_com_username = user
        .chess_com_username
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or(AppError::BadRequest(
            "No Chess.com username linked to account".into(),
        ))?
        .to_string();

    let account_id = user.id;
    let max_games: usize = 1000;

    // Determine the cursor — which month to start before
    let cursor = users::get_oldest_synced_month(&pool, account_id).await?;

    let cursor_month: Option<String> = match cursor {
        Some(ref c) if c == "complete" => {
            return Ok(Json(serde_json::json!({
                "synced": 0,
                "total": games::get_user_games_count(&pool, account_id, None).await?,
                "oldestSyncedMonth": "complete",
                "hasMoreHistory": false,
                "message": "All history already loaded",
            })));
        }
        Some(c) => Some(c),
        None => {
            // Legacy user: infer from earliest game
            let earliest = games::get_earliest_game_date(&pool, account_id, "chess_com").await?;
            earliest
        }
    };

    let client = clients::chess_com::ChessComClient::new();
    let archive_months = client.fetch_archives(&chess_com_username).await.map_err(|e| {
        AppError::Internal(e.into())
    })?;

    // Filter archives to only months strictly older than the cursor
    let older_archives: Vec<&(i32, u32)> = if let Some(ref cursor_str) = cursor_month {
        // Parse "YYYY-MM" into (year, month)
        let parts: Vec<&str> = cursor_str.split('-').collect();
        if parts.len() >= 2 {
            let cy: i32 = parts[0].parse().unwrap_or(9999);
            let cm: u32 = parts[1].parse().unwrap_or(12);
            archive_months
                .iter()
                .filter(|(y, m)| (*y, *m) < (cy, cm))
                .collect()
        } else {
            archive_months.iter().collect()
        }
    } else {
        // No cursor at all — fetch everything (shouldn't normally happen)
        archive_months.iter().collect()
    };

    tracing::info!(
        "Backfill for {} — {} older archives to process (cursor: {:?})",
        chess_com_username,
        older_archives.len(),
        cursor_month
    );

    if older_archives.is_empty() {
        // No more archives — mark complete
        users::update_oldest_synced_month(&pool, account_id, "complete").await?;
        return Ok(Json(serde_json::json!({
            "synced": 0,
            "total": games::get_user_games_count(&pool, account_id, None).await?,
            "oldestSyncedMonth": "complete",
            "hasMoreHistory": false,
            "message": "All history loaded",
        })));
    }

    let mut all_pairs = Vec::new();
    let mut last_processed_month: Option<(i32, u32)> = None;
    let mut all_consumed = false;

    for (i, &&(year, month)) in older_archives.iter().enumerate() {
        match client.fetch_user_games(&chess_com_username, Some(year), Some(month), true).await {
            Ok(pairs) => {
                if !pairs.is_empty() {
                    tracing::info!("  backfill {}/{:02}: {} games", year, month, pairs.len());
                    all_pairs.extend(pairs);
                    if all_pairs.len() >= max_games {
                        all_pairs.truncate(max_games);
                        last_processed_month = Some((year, month));
                        break;
                    }
                }
            }
            Err(e) => {
                tracing::warn!("  backfill {}/{:02}: Error - {}", year, month, e);
            }
        }
        if i == older_archives.len() - 1 {
            all_consumed = true;
        }
    }

    let synced_count = if !all_pairs.is_empty() {
        let game_records = build_game_records(&all_pairs, &chess_com_username);
        let count = games::upsert_games(&pool, account_id, &game_records, "chess_com").await?;

        // Incrementally populate opening stats
        opening_moves::populate_opening_stats(&pool, account_id).await?;

        // Tag titled opponents
        let source_ids: Vec<String> = game_records
            .iter()
            .filter_map(|g| g["id"].as_str().map(|s| s.to_string()))
            .collect();
        let db_games = games::get_game_ids_and_opponents(&pool, account_id, "chess_com", &source_ids).await?;
        let title_pairs: Vec<(i64, String)> = db_games
            .iter()
            .filter_map(|(db_id, _source_id, opponent)| {
                titled_players::lookup(opponent).map(|title| (*db_id, title))
            })
            .collect();
        if !title_pairs.is_empty() {
            let tagged = titled_players::insert_title_tags(&pool, &title_pairs).await?;
            tracing::info!("Backfill: tagged {} games with titled opponent tags", tagged);
        }

        count
    } else {
        0
    };

    // Update cursor
    let (new_cursor, has_more) = if all_consumed {
        users::update_oldest_synced_month(&pool, account_id, "complete").await?;
        ("complete".to_string(), false)
    } else if let Some((y, m)) = last_processed_month {
        let c = format!("{}-{:02}", y, m);
        users::update_oldest_synced_month(&pool, account_id, &c).await?;
        (c, true)
    } else {
        // All archives had errors or empty — mark complete
        users::update_oldest_synced_month(&pool, account_id, "complete").await?;
        ("complete".to_string(), false)
    };

    let total_games = games::get_user_games_count(&pool, account_id, None).await?;

    Ok(Json(serde_json::json!({
        "synced": synced_count,
        "total": total_games,
        "oldestSyncedMonth": new_cursor,
        "hasMoreHistory": has_more,
    })))
}

// ---- Internal helpers ----

use chrono::Datelike;

fn build_game_records(pairs: &[(String, Option<String>)], username: &str) -> Vec<JsonValue> {
    pairs
        .iter()
        .filter_map(|(pgn, tcn)| {
            let game = chess_core::pgn::parse_pgn(pgn, tcn.as_deref())?;
            let user_is_white = game.metadata.white.eq_ignore_ascii_case(username);
            let opponent = if user_is_white {
                &game.metadata.black
            } else {
                &game.metadata.white
            };

            let opponent_elo =
                chess_core::pgn::extract_header_int(pgn, if user_is_white { "BlackElo" } else { "WhiteElo" });
            let user_elo =
                chess_core::pgn::extract_header_int(pgn, if user_is_white { "WhiteElo" } else { "BlackElo" });

            let result = get_result_code(&game.metadata.result, user_is_white);
            let date = game.metadata.date.map(|d| d.replace('.', "-"));

            Some(serde_json::json!({
                "id": game.metadata.link.unwrap_or_default(),
                "opponent": opponent,
                "opponentRating": opponent_elo,
                "userRating": user_elo,
                "result": result,
                "timeControl": game.metadata.time_control,
                "date": date,
                "userColor": if user_is_white { "white" } else { "black" },
                "moves": game.moves,
                "tcn": tcn,
                "tags": [],
            }))
        })
        .collect()
}

fn get_result_code(result: &str, user_is_white: bool) -> &'static str {
    match result {
        "1-0" => {
            if user_is_white {
                "W"
            } else {
                "L"
            }
        }
        "0-1" => {
            if user_is_white {
                "L"
            } else {
                "W"
            }
        }
        _ => "D",
    }
}
