use axum::{extract::Path, extract::Query, Extension, Json};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use sqlx::PgPool;
use std::collections::HashMap;

use crate::auth::middleware::AuthUser;
use crate::clients;
use crate::db::{analysis, games, opening_tree as ot_db, users};
use crate::error::AppError;

#[derive(Deserialize)]
pub struct StoredGamesQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub tags: Option<String>,
    pub platform: Option<String>,
}

#[derive(Deserialize)]
pub struct TagsQuery {
    pub selected_tags: Option<String>,
}

#[derive(Deserialize)]
pub struct AnalyzeQuery {
    pub limit: Option<i64>,
    pub platform: Option<String>,
}

/// GET /api/games/stored
pub async fn get_stored_games(
    Extension(pool): Extension<PgPool>,
    Query(q): Query<StoredGamesQuery>,
    user: AuthUser,
) -> Result<Json<JsonValue>, AppError> {
    let limit = q.limit.unwrap_or(50).min(200);
    let offset = q.offset.unwrap_or(0).max(0);
    let account_id = user.id;

    let source = q.platform.as_deref().filter(|p| *p == "chess_com" || *p == "lichess");

    let tags_list: Option<Vec<String>> = q.tags.as_ref().map(|t| {
        t.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    });

    let games_list = games::get_user_games_paginated(
        &pool,
        account_id,
        limit,
        offset,
        tags_list.as_deref(),
        source,
    )
    .await?;

    let total = games::get_user_games_count_filtered(
        &pool,
        account_id,
        tags_list.as_deref(),
        source,
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

    let pgn_tcn_pairs = if is_first_sync {
        tracing::info!("First sync for {} — fetching up to {} games", chess_com_username, max_games);
        let mut all_pairs = Vec::new();
        let mut year = now.year();
        let mut month = now.month();

        loop {
            match client.fetch_user_games(&chess_com_username, Some(year), Some(month), true).await {
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

            if month == 1 {
                month = 12;
                year -= 1;
            } else {
                month -= 1;
            }

            if year < 2010 {
                break;
            }
        }
        all_pairs
    } else {
        tracing::info!("Re-sync for {} — fetching current month", chess_com_username);
        client
            .fetch_user_games(&chess_com_username, Some(now.year()), Some(now.month()), true)
            .await
            .unwrap_or_default()
    };

    let synced_count = if !pgn_tcn_pairs.is_empty() {
        let game_records = build_game_records(&pgn_tcn_pairs, &chess_com_username);
        let count = games::upsert_games(&pool, account_id, &game_records, "chess_com").await?;

        // Invalidate opening tree cache
        ot_db::invalidate_opening_trees(&pool, account_id).await?;

        // Analyze newly synced games
        analyze_user_games_internal(&pool, account_id, &chess_com_username, "chess_com", count as usize).await?;

        count
    } else {
        0
    };

    users::update_last_synced(&pool, account_id, "chess_com").await?;
    let total_games = games::get_user_games_count(&pool, account_id, None).await?;

    Ok(Json(serde_json::json!({
        "username": chess_com_username,
        "synced": synced_count,
        "total": total_games,
        "lastSyncedAt": users::get_last_synced(&pool, account_id, "chess_com").await?.map(|t| t.to_rfc3339()),
        "isFirstSync": is_first_sync,
    })))
}

/// POST /api/games/sync/lichess
pub async fn sync_lichess_games(
    Extension(pool): Extension<PgPool>,
    user: AuthUser,
) -> Result<Json<JsonValue>, AppError> {
    let lichess_username = user
        .lichess_username
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or(AppError::BadRequest(
            "No Lichess username linked to account".into(),
        ))?
        .to_string();

    let account_id = user.id;
    let last_synced = users::get_last_synced(&pool, account_id, "lichess").await?;
    let is_first_sync = last_synced.is_none();

    let client = clients::lichess::LichessClient::new();
    let max_games = if is_first_sync { Some(1000) } else { None };

    let pgn_pairs = client
        .fetch_user_games(&lichess_username, max_games)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch Lichess games: {e}")))?;

    let synced_count = if !pgn_pairs.is_empty() {
        let game_records = build_lichess_game_records(&pgn_pairs, &lichess_username);
        let count = games::upsert_games(&pool, account_id, &game_records, "lichess").await?;

        analyze_user_games_internal(&pool, account_id, &lichess_username, "lichess", count as usize).await?;

        count
    } else {
        0
    };

    users::update_last_synced(&pool, account_id, "lichess").await?;
    let total_games = games::get_user_games_count(&pool, account_id, Some("lichess")).await?;

    Ok(Json(serde_json::json!({
        "username": lichess_username,
        "synced": synced_count,
        "total": total_games,
        "lastSyncedAt": users::get_last_synced(&pool, account_id, "lichess").await?.map(|t| t.to_rfc3339()),
        "isFirstSync": is_first_sync,
    })))
}

/// POST /api/games/analyze
pub async fn analyze_games(
    Extension(pool): Extension<PgPool>,
    Query(q): Query<AnalyzeQuery>,
    user: AuthUser,
) -> Result<Json<JsonValue>, AppError> {
    let platform = q.platform.as_deref().unwrap_or("chess_com");
    let limit = q.limit.unwrap_or(1000).min(5000) as usize;

    let platform_username = if platform == "lichess" {
        user.lichess_username
            .as_deref()
            .filter(|s| !s.is_empty())
            .ok_or(AppError::BadRequest("No Lichess username linked".into()))?
            .to_string()
    } else {
        user.chess_com_username
            .as_deref()
            .filter(|s| !s.is_empty())
            .ok_or(AppError::BadRequest("No Chess.com username linked".into()))?
            .to_string()
    };

    let source = if platform == "lichess" { "lichess" } else { "chess_com" };
    let account_id = user.id;

    let (analyzed, skipped) =
        analyze_user_games_internal(&pool, account_id, &platform_username, source, limit).await?;

    let remaining = games::get_unanalyzed_games_count(&pool, account_id).await?;
    let total = games::get_user_games_count(&pool, account_id, Some(source)).await?;

    Ok(Json(serde_json::json!({
        "analyzed": analyzed,
        "remaining": remaining,
        "total": total,
        "skippedNoPgn": skipped,
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

fn build_lichess_game_records(
    pairs: &[(String, String)],
    username: &str,
) -> Vec<JsonValue> {
    pairs
        .iter()
        .filter_map(|(pgn, game_id)| {
            let game = chess_core::pgn::parse_pgn(pgn, None)?;
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
                "id": game_id,
                "opponent": opponent,
                "opponentRating": opponent_elo,
                "userRating": user_elo,
                "result": result,
                "timeControl": game.metadata.time_control,
                "date": date,
                "userColor": if user_is_white { "white" } else { "black" },
                "moves": game.moves,
                "tcn": game.tcn,
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

/// Run batch analysis on unanalyzed games. Returns (analyzed, skipped).
async fn analyze_user_games_internal(
    pool: &PgPool,
    account_id: i64,
    platform_username: &str,
    source: &str,
    limit: usize,
) -> Result<(i64, i64), AppError> {
    let batch_size = 100i64;
    let mut total_analyzed = 0i64;
    let mut total_skipped = 0i64;

    while (total_analyzed as usize) < limit {
        let remaining = (limit - total_analyzed as usize).min(batch_size as usize);

        let unanalyzed =
            games::get_unanalyzed_games(pool, account_id, remaining as i64, Some(source)).await?;

        if unanalyzed.is_empty() {
            break;
        }

        tracing::info!(
            "Analyzing batch of {} {} games for {}",
            unanalyzed.len(),
            source,
            platform_username
        );

        // Convert to GameData and run analyzers
        let mut game_data_list = Vec::new();
        let mut game_id_map: HashMap<String, i64> = HashMap::new();

        for g in &unanalyzed {
            let db_id = g["id"].as_i64().unwrap_or(0);
            let game_link = g["chessComGameId"].as_str().unwrap_or("");

            // Build GameData for analyzers
            let moves: Vec<String> = g["moves"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            let tcn = g["tcn"].as_str().map(|s| s.to_string());

            if moves.is_empty() && tcn.is_none() {
                total_skipped += 1;
                continue;
            }

            let user_color = g["userColor"].as_str().unwrap_or("white");
            let result_code = g["result"].as_str().unwrap_or("D");
            let chess_result = match result_code {
                "W" => {
                    if user_color == "white" {
                        "1-0"
                    } else {
                        "0-1"
                    }
                }
                "L" => {
                    if user_color == "white" {
                        "0-1"
                    } else {
                        "1-0"
                    }
                }
                _ => "1/2-1/2",
            };

            let gd = chess_core::game_data::GameData {
                metadata: chess_core::game_data::GameMetadata {
                    white: if user_color == "white" {
                        platform_username.to_string()
                    } else {
                        g["opponent"].as_str().unwrap_or("").to_string()
                    },
                    black: if user_color == "black" {
                        platform_username.to_string()
                    } else {
                        g["opponent"].as_str().unwrap_or("").to_string()
                    },
                    result: chess_result.to_string(),
                    date: g["date"].as_str().map(|s| s.to_string()),
                    time_control: g["timeControl"].as_str().map(|s| s.to_string()),
                    eco: None,
                    event: None,
                    link: Some(game_link.to_string()),
                },
                moves,
                pgn: String::new(),
                tcn,
            };

            game_data_list.push(gd);
            game_id_map.insert(game_link.to_string(), db_id);
        }

        if game_data_list.is_empty() {
            // Mark games with no moves as analyzed
            let no_move_ids: Vec<i64> = unanalyzed
                .iter()
                .filter(|g| {
                    let moves_empty = g["moves"]
                        .as_array()
                        .map(|a| a.is_empty())
                        .unwrap_or(true);
                    let tcn_empty = g["tcn"].as_str().map(|s| s.is_empty()).unwrap_or(true);
                    moves_empty && tcn_empty
                })
                .filter_map(|g| g["id"].as_i64())
                .collect();

            if !no_move_ids.is_empty() {
                let tags_map: HashMap<i64, Vec<String>> =
                    no_move_ids.iter().map(|id| (*id, vec![])).collect();
                games::mark_games_analyzed(pool, &tags_map).await?;
            }
            continue;
        }

        // Run analyzers
        let game_tags = chess_analyzers::analyze_batch(platform_username, &game_data_list);

        // Map to database IDs
        let mut tags_map: HashMap<i64, Vec<String>> = HashMap::new();
        for gd in &game_data_list {
            if let Some(ref link) = gd.metadata.link {
                if let Some(&db_id) = game_id_map.get(link) {
                    let tags = game_tags.get(link).cloned().unwrap_or_default();
                    tags_map.insert(db_id, tags);
                }
            }
        }

        let updated = games::mark_games_analyzed(pool, &tags_map).await?;
        total_analyzed += updated;

        tracing::info!("Batch complete: {} games tagged (total: {})", updated, total_analyzed);
    }

    Ok((total_analyzed, total_skipped))
}
