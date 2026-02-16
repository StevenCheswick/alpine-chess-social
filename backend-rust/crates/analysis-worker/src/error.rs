//! Worker error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum WorkerError {
    #[error("Configuration error: {0}")]
    Config(&'static str),

    #[error("SQS error: {0}")]
    Sqs(String),

    #[error("Secrets Manager error: {0}")]
    SecretsManager(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Stockfish error: {0}")]
    Stockfish(String),

    #[error("Analysis error: {0}")]
    Analysis(String),

    #[error("Game not found: {0}")]
    GameNotFound(i64),

    #[error("TCN decode error: {0}")]
    TcnDecode(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
