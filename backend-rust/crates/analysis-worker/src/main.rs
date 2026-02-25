//! AWS Analysis Worker
//!
//! Processes game analysis jobs from SQS queue using native Stockfish.
//! Designed for AWS Batch on ARM Graviton instances.

mod analyzer;
mod book_cache;
mod config;
mod db;
mod error;
mod sqs;
mod stockfish;

// Re-declare lib modules so the binary can use them via `crate::`
mod analysis;
mod board_utils;
mod endgame;
mod puzzle;
mod queen_sac;
mod smothered_mate;
mod tactics;

use std::sync::Arc;

use tokio::sync::{Mutex, Semaphore};
use tracing::{error, info, warn};

use crate::config::WorkerConfig;
use crate::error::WorkerError;
use crate::sqs::SqsClient;
use crate::stockfish::StockfishEngine;

/// Parse --test-games 123,456,789 from CLI args
fn parse_test_games() -> Option<Vec<i64>> {
    let args: Vec<String> = std::env::args().collect();
    for i in 0..args.len() {
        if args[i] == "--test-games" {
            if let Some(ids_str) = args.get(i + 1) {
                let ids: Vec<i64> = ids_str
                    .split(',')
                    .filter_map(|s| s.trim().parse().ok())
                    .collect();
                if !ids.is_empty() {
                    return Some(ids);
                }
            }
        }
    }
    None
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    // Load .env file for local dev
    let _ = dotenvy::dotenv();

    // --test-games mode: analyze specific game IDs locally, skip SQS
    if let Some(game_ids) = parse_test_games() {
        // Force local dev mode so config doesn't need AWS secrets
        std::env::set_var("LOCAL_DEV", "1");
        std::env::set_var("SQS_QUEUE_URL", "unused");

        let config = WorkerConfig::load().await?;
        info!(stockfish_path = %config.stockfish_path, "Test mode config loaded");

        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(2)
            .connect(&config.database_url)
            .await?;

        let mut engine = StockfishEngine::new(&config.stockfish_path)
            .await
            .expect("Failed to spawn Stockfish");

        let mut passed = 0u32;
        let mut failed = 0u32;

        for game_id in &game_ids {
            println!("\n{}", "=".repeat(60));
            println!("Analyzing game_id={game_id}...");

            match analyzer::analyze_game(&mut engine, &pool, &config, *game_id).await {
                Ok(()) => {
                    let row: Option<(i64,)> = sqlx::query_as(
                        "SELECT game_id FROM game_tags WHERE game_id = $1 AND tag = 'queen_sacrifice'"
                    )
                    .bind(game_id)
                    .fetch_optional(&pool)
                    .await?;

                    if row.is_some() {
                        println!("  PASS: queen_sacrifice tag detected");
                        passed += 1;
                    } else {
                        let tags: Vec<(String,)> = sqlx::query_as(
                            "SELECT tag FROM game_tags WHERE game_id = $1"
                        )
                        .bind(game_id)
                        .fetch_all(&pool)
                        .await?;
                        let tag_list: Vec<&str> = tags.iter().map(|t| t.0.as_str()).collect();
                        println!("  FAIL: no queen_sacrifice tag");
                        println!("  Tags found: {tag_list:?}");
                        failed += 1;
                    }
                }
                Err(e) => {
                    println!("  ERROR: {e}");
                    failed += 1;
                }
            }
        }

        engine.quit().await;
        println!("\n{}", "=".repeat(60));
        println!("Results: {passed} passed, {failed} failed out of {} games", game_ids.len());
        return Ok(());
    }

    // Load config (fetches DB URL from Secrets Manager in prod)
    let config = WorkerConfig::load().await?;
    info!(
        stockfish_path = %config.stockfish_path,
        nodes = config.nodes_per_position,
        "Worker config loaded"
    );

    // Create engine pool (one Stockfish process per CPU)
    let num_workers = num_cpus::get();

    // Create database pool scaled to worker count
    let pool_size = (num_workers + 2) as u32; // headroom for overlapping saves
    let pool = sqlx::postgres::PgPoolOptions::new()
        .min_connections(pool_size)
        .max_connections(pool_size)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .idle_timeout(std::time::Duration::from_secs(300))
        .connect(&config.database_url)
        .await?;
    info!(pool_size, "Database connection pool established");
    info!(num_workers, "Creating Stockfish engine pool");

    let mut engines: Vec<Arc<Mutex<StockfishEngine>>> = Vec::with_capacity(num_workers);
    for i in 0..num_workers {
        let engine = StockfishEngine::new(&config.stockfish_path)
            .await
            .expect("Failed to spawn Stockfish");
        info!(engine_id = i, "Stockfish engine ready");
        engines.push(Arc::new(Mutex::new(engine)));
    }

    // Create SQS client
    let sqs = SqsClient::new(&config).await?;
    info!(queue_url = %config.sqs_queue_url, "SQS client ready");

    // Create semaphore for parallel processing
    let semaphore = Arc::new(Semaphore::new(num_workers));

    // Track consecutive empty receives for graceful exit
    let mut empty_receives = 0;

    // Set up SIGTERM handler for spot interruptions (Unix only)
    #[cfg(unix)]
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;

    info!("Starting main loop");

    loop {
        // Check for shutdown signals
        #[cfg(unix)]
        {
            tokio::select! {
                _ = sigterm.recv() => {
                    info!("Received SIGTERM, waiting for in-flight work...");
                    // Acquire all permits = wait for all tasks to complete
                    for _ in 0..num_workers {
                        let _ = semaphore.acquire().await;
                    }
                    info!("Graceful shutdown complete");
                    break;
                }
                result = sqs.receive_messages() => {
                    match result {
                        Ok(messages) => {
                            if messages.is_empty() {
                                empty_receives += 1;
                                if empty_receives >= config.max_empty_receives {
                                    info!("No messages after {} polls, exiting", config.max_empty_receives);
                                    break;
                                }
                                continue;
                            }
                            empty_receives = 0;

                            // Collect all available messages (SQS may distribute across servers)
                            let mut all_messages = messages;
                            while all_messages.len() < num_workers {
                                if let Ok(more) = sqs.receive_messages_nowait().await {
                                    if more.is_empty() {
                                        break;
                                    }
                                    all_messages.extend(more);
                                } else {
                                    break;
                                }
                            }

                            for (i, msg) in all_messages.into_iter().enumerate() {
                                let game_id: i64 = match msg.body.parse() {
                                    Ok(id) => id,
                                    Err(_) => {
                                        warn!(body = %msg.body, "Invalid game ID format, deleting message");
                                        let _ = sqs.delete_message(&msg.receipt_handle).await;
                                        continue;
                                    }
                                };

                                let permit = semaphore.clone().acquire_owned().await?;
                                let engine = engines[i % num_workers].clone();
                                let pool = pool.clone();
                                let sqs = sqs.clone();
                                let receipt = msg.receipt_handle.clone();
                                let config = config.clone();

                                tokio::spawn(async move {
                                    let _permit = permit; // Hold until done
                                    let mut engine = engine.lock().await;

                                    match analyzer::analyze_game(&mut engine, &pool, &config, game_id).await {
                                        Ok(()) => {
                                            info!(game_id, "Analysis complete");
                                            let _ = sqs.delete_message(&receipt).await;
                                        }
                                        Err(WorkerError::GameNotFound(_)) => {
                                            warn!(game_id, "Game not found, deleting message");
                                            let _ = sqs.delete_message(&receipt).await;
                                        }
                                        Err(e) => {
                                            error!(game_id, error = %e, "Analysis failed");
                                            // Don't delete - will retry via visibility timeout
                                        }
                                    }
                                });
                            }
                        }
                        Err(e) => {
                            error!(error = %e, "Failed to receive messages");
                            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        }
                    }
                }
            }
        }

        // Windows fallback (no SIGTERM handling)
        #[cfg(not(unix))]
        {
            match sqs.receive_messages().await {
                Ok(messages) => {
                    if messages.is_empty() {
                        empty_receives += 1;
                        if empty_receives >= config.max_empty_receives {
                            info!(
                                "No messages after {} polls, exiting",
                                config.max_empty_receives
                            );
                            break;
                        }
                        continue;
                    }
                    empty_receives = 0;

                    // Collect all available messages (SQS may distribute across servers)
                    let mut all_messages = messages;
                    while all_messages.len() < num_workers {
                        if let Ok(more) = sqs.receive_messages_nowait().await {
                            if more.is_empty() {
                                break;
                            }
                            all_messages.extend(more);
                        } else {
                            break;
                        }
                    }

                    for (i, msg) in all_messages.into_iter().enumerate() {
                        let game_id: i64 = match msg.body.parse() {
                            Ok(id) => id,
                            Err(_) => {
                                warn!(body = %msg.body, "Invalid game ID format, deleting message");
                                let _ = sqs.delete_message(&msg.receipt_handle).await;
                                continue;
                            }
                        };

                        let permit = semaphore.clone().acquire_owned().await?;
                        let engine = engines[i % num_workers].clone();
                        let pool = pool.clone();
                        let sqs = sqs.clone();
                        let receipt = msg.receipt_handle.clone();
                        let config = config.clone();

                        tokio::spawn(async move {
                            let _permit = permit;
                            let mut engine = engine.lock().await;

                            match analyzer::analyze_game(&mut engine, &pool, &config, game_id).await
                            {
                                Ok(()) => {
                                    info!(game_id, "Analysis complete");
                                    let _ = sqs.delete_message(&receipt).await;
                                }
                                Err(WorkerError::GameNotFound(_)) => {
                                    warn!(game_id, "Game not found, deleting message");
                                    let _ = sqs.delete_message(&receipt).await;
                                }
                                Err(e) => {
                                    error!(game_id, error = %e, "Analysis failed");
                                }
                            }
                        });
                    }
                }
                Err(e) => {
                    error!(error = %e, "Failed to receive messages");
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            }
        }
    }

    // Clean up engines
    info!("Shutting down Stockfish engines");
    for engine in engines {
        let mut engine = engine.lock().await;
        engine.quit().await;
    }

    Ok(())
}
