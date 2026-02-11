use axum::{Extension, Json};
use serde_json::Value as JsonValue;
use shakmaty::{Chess, Position, uci::UciMove, san::San};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::RwLock;

use crate::auth::middleware::AuthUser;
use crate::db::{analysis, opening_moves};
use crate::error::AppError;

// Simple in-process cache
static STATS_CACHE: std::sync::LazyLock<RwLock<HashMap<i64, JsonValue>>> =
    std::sync::LazyLock::new(|| RwLock::new(HashMap::new()));

pub fn invalidate_stats_cache() {
    if let Ok(mut cache) = STATS_CACHE.write() {
        cache.clear();
    }
}

const ROLLING_WINDOW: usize = 100;
const MAX_CHART_POINTS: usize = 50;

/// GET /api/games/stats
pub async fn get_game_stats(
    Extension(pool): Extension<PgPool>,
    user: AuthUser,
) -> Result<Json<JsonValue>, AppError> {
    let account_id = user.id;

    // Check cache
    if let Ok(cache) = STATS_CACHE.read() {
        if let Some(cached) = cache.get(&account_id) {
            return Ok(Json(cached.clone()));
        }
    }

    let stats = build_game_stats(&pool, account_id).await?;

    // Store in cache
    if let Ok(mut cache) = STATS_CACHE.write() {
        cache.insert(account_id, stats.clone());
    }

    Ok(Json(stats))
}

async fn build_game_stats(pool: &PgPool, user_id: i64) -> Result<JsonValue, AppError> {
    let stats = analysis::get_user_game_stats(pool, user_id).await?;

    let mut accuracy_over_time = Vec::new();
    let mut phase_accuracy_over_time = Vec::new();
    let mut first_inaccuracy_over_time = Vec::new();
    let mut rating_over_time = Vec::new();
    let mut move_quality_breakdown: HashMap<String, i64> = HashMap::new();

    for key in ["best", "excellent", "good", "inaccuracy", "mistake", "blunder"] {
        move_quality_breakdown.insert(key.to_string(), 0);
    }

    let mut raw_accuracy = Vec::new();
    let mut raw_opening = Vec::new();
    let mut raw_middlegame = Vec::new();
    let mut raw_endgame = Vec::new();
    let mut raw_inaccuracy: Vec<f64> = Vec::new();

    for game in &stats {
        let date = game["date"].as_str().unwrap_or("");
        let game_id = game["id"].as_i64().unwrap_or(0);
        let accuracy = game["accuracy"].as_f64().unwrap_or(0.0);

        accuracy_over_time.push(serde_json::json!({"date": date, "gameId": game_id}));
        raw_accuracy.push((accuracy * 10.0).round() / 10.0);

        let pa = &game["phase_accuracy"];
        phase_accuracy_over_time.push(serde_json::json!({"date": date, "gameId": game_id}));
        raw_opening.push(pa.get("opening").and_then(|v| v.as_f64()).map(|v| (v * 10.0).round() / 10.0));
        raw_middlegame.push(pa.get("middlegame").and_then(|v| v.as_f64()).map(|v| (v * 10.0).round() / 10.0));
        raw_endgame.push(pa.get("endgame").and_then(|v| v.as_f64()).map(|v| (v * 10.0).round() / 10.0));

        first_inaccuracy_over_time.push(serde_json::json!({"date": date, "gameId": game_id}));
        raw_inaccuracy.push(game["first_inaccuracy"].as_f64().unwrap_or(0.0));

        if let Some(rating) = game["user_rating"].as_i64() {
            rating_over_time.push(serde_json::json!({"date": date, "rating": rating, "gameId": game_id}));
        }

        let classifications = &game["classifications"];
        for key in ["best", "excellent", "good", "inaccuracy", "mistake", "blunder"] {
            if let Some(count) = classifications.get(key).and_then(|v| v.as_i64()) {
                *move_quality_breakdown.entry(key.to_string()).or_insert(0) += count;
            }
        }
    }

    // Apply rolling averages
    let smoothed_acc = rolling_avg(&raw_accuracy);
    let smoothed_inacc = rolling_avg(&raw_inaccuracy);

    for i in 0..accuracy_over_time.len() {
        accuracy_over_time[i]["accuracy"] = serde_json::json!(smoothed_acc[i]);
        first_inaccuracy_over_time[i]["moveNumber"] = serde_json::json!(smoothed_inacc[i]);
    }

    // Phase accuracy rolling average
    for (key, raw_vals) in [("opening", &raw_opening), ("middlegame", &raw_middlegame), ("endgame", &raw_endgame)] {
        let filled = rolling_avg_optional(raw_vals);
        for i in 0..phase_accuracy_over_time.len() {
            phase_accuracy_over_time[i][key] = match filled[i] {
                Some(v) => serde_json::json!(v),
                None => JsonValue::Null,
            };
        }
    }

    // Downsample
    let accuracy_over_time = downsample(accuracy_over_time);
    let phase_accuracy_over_time = downsample(phase_accuracy_over_time);
    let first_inaccuracy_over_time = downsample(first_inaccuracy_over_time);
    let rating_over_time = downsample(rating_over_time);

    // Most/least accurate
    let mut eligible: Vec<&JsonValue> = stats
        .iter()
        .filter(|g| {
            let acc = g["accuracy"].as_f64().unwrap_or(0.0);
            let total_moves: i64 = ["best", "excellent", "good", "inaccuracy", "mistake", "blunder"]
                .iter()
                .filter_map(|k| g["classifications"].get(k).and_then(|v| v.as_i64()))
                .sum();
            acc < 100.0 && total_moves >= 25
        })
        .collect();

    eligible.sort_by(|a, b| {
        b["accuracy"]
            .as_f64()
            .unwrap_or(0.0)
            .partial_cmp(&a["accuracy"].as_f64().unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let most_accurate: Vec<JsonValue> = eligible.iter().take(5).map(|g| game_summary(g)).collect();

    let mut least: Vec<&JsonValue> = stats.iter().collect();
    least.sort_by(|a, b| {
        a["accuracy"]
            .as_f64()
            .unwrap_or(0.0)
            .partial_cmp(&b["accuracy"].as_f64().unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let least_accurate: Vec<JsonValue> = least.iter().take(5).map(|g| game_summary(g)).collect();

    // Opening blunders: most repeated mistakes (cp_loss >= 50 = half a pawn)
    let blunder_rows = opening_moves::get_opening_blunders(pool, user_id, 50.0, 5).await?;
    let opening_blunders: Vec<JsonValue> = blunder_rows
        .iter()
        .map(|b| {
            let san = uci_line_to_san(&b.line);
            let best_move_san = b.best_move.as_deref()
                .and_then(|uci| uci_to_san(&san.pre_last_pos, uci));
            let mut obj = serde_json::json!({
                "line": san.formatted,
                "moves": san.moves,
                "ply": b.ply,
                "color": b.color,
                "mistakeCount": b.mistake_count,
                "avgCpLoss": b.avg_cp_loss,
                "sampleGameId": b.sample_game_id,
            });
            if let Some(best) = best_move_san {
                obj["bestMove"] = serde_json::json!(best);
            }
            obj
        })
        .collect();

    // Cleanest opening lines: deepest lines played with no inaccuracies (cp_loss < 50)
    let clean_rows = opening_moves::get_cleanest_lines(pool, user_id, 50.0, 5, 5).await?;
    let cleanest_lines: Vec<JsonValue> = clean_rows
        .iter()
        .map(|c| {
            let san = uci_line_to_san(&c.line);
            serde_json::json!({
                "line": san.formatted,
                "moves": san.moves,
                "color": c.color,
                "cleanDepth": c.clean_depth,
                "gameCount": c.game_count,
                "avgCpLoss": c.avg_cp_loss,
                "sampleGameId": c.sample_game_id,
            })
        })
        .collect();

    Ok(serde_json::json!({
        "totalAnalyzedGames": stats.len(),
        "accuracyOverTime": accuracy_over_time,
        "phaseAccuracyOverTime": phase_accuracy_over_time,
        "firstInaccuracyOverTime": first_inaccuracy_over_time,
        "ratingOverTime": rating_over_time,
        "moveQualityBreakdown": move_quality_breakdown,
        "mostAccurateGames": most_accurate,
        "leastAccurateGames": least_accurate,
        "openingBlunders": opening_blunders,
        "cleanestLines": cleanest_lines,
    }))
}

fn game_summary(g: &JsonValue) -> JsonValue {
    serde_json::json!({
        "gameId": g["id"],
        "date": g["date"],
        "accuracy": ((g["accuracy"].as_f64().unwrap_or(0.0) * 10.0).round() / 10.0),
        "opponent": g["opponent"],
        "opponentRating": g["opponent_rating"],
        "result": g["result"],
        "userColor": g["user_color"],
    })
}

fn clamp_outliers(values: &[f64], pct: f64) -> Vec<f64> {
    if values.len() < 10 {
        return values.to_vec();
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let lo = sorted[(sorted.len() as f64 * pct / 100.0) as usize];
    let hi = sorted[(sorted.len() as f64 * (100.0 - pct) / 100.0) as usize - 1];
    values.iter().map(|v| v.max(lo).min(hi)).collect()
}

fn rolling_avg(values: &[f64]) -> Vec<f64> {
    let clamped = clamp_outliers(values, 5.0);
    let mut result = Vec::with_capacity(clamped.len());
    let mut running = 0.0;
    for (i, v) in clamped.iter().enumerate() {
        running += v;
        if i >= ROLLING_WINDOW {
            running -= clamped[i - ROLLING_WINDOW];
        }
        let count = (i + 1).min(ROLLING_WINDOW);
        result.push((running / count as f64 * 10.0).round() / 10.0);
    }
    result
}

fn rolling_avg_optional(values: &[Option<f64>]) -> Vec<Option<f64>> {
    // Clamp non-null values
    let non_null: Vec<f64> = values.iter().filter_map(|v| *v).collect();
    let clamped_lookup = if non_null.len() >= 10 {
        clamp_outliers(&non_null, 5.0)
    } else {
        non_null.clone()
    };

    let mut ci = 0;
    let mut clamped_vals: Vec<Option<f64>> = Vec::with_capacity(values.len());
    for v in values {
        if v.is_some() {
            clamped_vals.push(Some(clamped_lookup[ci]));
            ci += 1;
        } else {
            clamped_vals.push(None);
        }
    }

    let mut result = Vec::with_capacity(values.len());
    let mut window = std::collections::VecDeque::new();
    let mut running = 0.0;

    for v in &clamped_vals {
        if let Some(val) = v {
            window.push_back(*val);
            running += val;
            if window.len() > ROLLING_WINDOW {
                running -= window.pop_front().unwrap();
            }
            result.push(Some((running / window.len() as f64 * 10.0).round() / 10.0));
        } else {
            result.push(None);
        }
    }

    result
}

fn downsample(data: Vec<JsonValue>) -> Vec<JsonValue> {
    let n = data.len();
    if n <= MAX_CHART_POINTS {
        return data;
    }
    let step = (n - 1) as f64 / (MAX_CHART_POINTS - 1) as f64;
    let mut indices = std::collections::BTreeSet::new();
    indices.insert(0);
    indices.insert(n - 1);
    for i in 1..(MAX_CHART_POINTS - 1) {
        indices.insert((i as f64 * step).round() as usize);
    }
    indices.into_iter().map(|i| data[i].clone()).collect()
}

/// Result of converting a UCI line to SAN.
struct SanLine {
    formatted: String,
    moves: Vec<String>,
    /// Position just before the last move was played (for converting best_move).
    pre_last_pos: Chess,
}

/// Convert a UCI move line to SAN formatted line + moves array.
/// e.g. "e2e4 e7e5 g1f3 f7f6" → ("1. e4 e5 2. Nf3 f6", ["e4", "e5", "Nf3", "f6"])
fn uci_line_to_san(uci_line: &str) -> SanLine {
    let tokens: Vec<&str> = uci_line.split_whitespace().collect();
    let mut pos = Chess::default();
    let mut formatted = String::new();
    let mut moves = Vec::new();
    let mut pre_last_pos = pos.clone();

    for (i, uci_str) in tokens.iter().enumerate() {
        let uci_move: UciMove = match uci_str.parse() {
            Ok(m) => m,
            Err(_) => return SanLine {
                formatted: uci_line.to_string(),
                moves: tokens.iter().map(|s| s.to_string()).collect(),
                pre_last_pos: pos,
            },
        };
        let legal_move = match uci_move.to_move(&pos) {
            Ok(m) => m,
            Err(_) => return SanLine {
                formatted: uci_line.to_string(),
                moves: tokens.iter().map(|s| s.to_string()).collect(),
                pre_last_pos: pos,
            },
        };
        let san = San::from_move(&pos, &legal_move);
        let san_str = san.to_string();
        moves.push(san_str.clone());

        let move_num = (i / 2) + 1;
        if i % 2 == 0 {
            if !formatted.is_empty() {
                formatted.push(' ');
            }
            formatted.push_str(&format!("{}. {}", move_num, san_str));
        } else {
            formatted.push_str(&format!(" {}", san_str));
        }

        pre_last_pos = pos.clone();
        pos.play_unchecked(&legal_move);
    }

    SanLine { formatted, moves, pre_last_pos }
}

/// Convert a single UCI move to SAN at a given position.
fn uci_to_san(pos: &Chess, uci_str: &str) -> Option<String> {
    let uci_move: UciMove = uci_str.parse().ok()?;
    let legal_move = uci_move.to_move(pos).ok()?;
    Some(San::from_move(pos, &legal_move).to_string())
}
