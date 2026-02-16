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

use std::sync::Arc;

use tokio::sync::{Mutex, Semaphore};
use tracing::{error, info, warn};

use crate::config::WorkerConfig;
use crate::error::WorkerError;
use crate::sqs::SqsClient;
use crate::stockfish::StockfishEngine;

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

    // Load config (fetches DB URL from Secrets Manager in prod)
    let config = WorkerConfig::load().await?;
    info!(
        stockfish_path = %config.stockfish_path,
        nodes = config.nodes_per_position,
        "Worker config loaded"
    );

    // Create database pool
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await?;
    info!("Database connection established");

    // Check if GAME_ID is passed directly (from Lambda via Batch env override)
    if let Ok(game_id_str) = std::env::var("GAME_ID") {
        info!(game_id = %game_id_str, "Processing single game from GAME_ID env var");

        let game_id: i64 = game_id_str.parse().map_err(|_| {
            anyhow::anyhow!("Invalid GAME_ID format: {}", game_id_str)
        })?;

        // Create single Stockfish engine
        let mut engine = StockfishEngine::new(&config.stockfish_path)
            .expect("Failed to spawn Stockfish");
        info!("Stockfish engine ready");

        // Analyze the single game
        match analyzer::analyze_game(&mut engine, &pool, &config, game_id).await {
            Ok(()) => {
                info!(game_id, "Analysis complete");
            }
            Err(WorkerError::GameNotFound(_)) => {
                warn!(game_id, "Game not found");
            }
            Err(e) => {
                error!(game_id, error = %e, "Analysis failed");
                return Err(e.into());
            }
        }

        engine.quit();
        return Ok(());
    }

    // No GAME_ID - fall back to SQS polling mode (for local dev or legacy)
    info!("No GAME_ID env var, falling back to SQS polling mode");

    // Create engine pool (one Stockfish process per CPU)
    let num_workers = num_cpus::get();
    info!(num_workers, "Creating Stockfish engine pool");

    let engines: Vec<Arc<Mutex<StockfishEngine>>> = (0..num_workers)
        .map(|i| {
            let engine = StockfishEngine::new(&config.stockfish_path)
                .expect("Failed to spawn Stockfish");
            info!(engine_id = i, "Stockfish engine ready");
            Arc::new(Mutex::new(engine))
        })
        .collect();

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

                            for (i, msg) in messages.into_iter().enumerate() {
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

                    for (i, msg) in messages.into_iter().enumerate() {
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
        engine.quit();
    }

    Ok(())
}
