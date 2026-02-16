//! Worker configuration from environment variables and AWS Secrets Manager

use std::env;

use aws_sdk_secretsmanager::Client as SecretsClient;
use tracing::info;

use crate::error::WorkerError;

#[derive(Clone, Debug)]
pub struct WorkerConfig {
    /// Database connection URL (fetched from Secrets Manager in prod)
    pub database_url: String,

    /// SQS queue URL for analysis jobs
    pub sqs_queue_url: String,

    /// Custom SQS endpoint URL (for LocalStack)
    pub sqs_endpoint_url: Option<String>,

    /// Path to Stockfish binary
    pub stockfish_path: String,

    /// Nodes per position for Stockfish analysis
    pub nodes_per_position: u32,

    /// Consecutive empty SQS receives before exiting
    pub max_empty_receives: u32,

    /// SQS visibility timeout in seconds
    pub visibility_timeout_secs: u32,
}

impl WorkerConfig {
    /// Load configuration from environment variables.
    /// In production, fetches DATABASE_URL from AWS Secrets Manager.
    pub async fn load() -> Result<Self, WorkerError> {
        let sqs_queue_url =
            env::var("SQS_QUEUE_URL").map_err(|_| WorkerError::Config("SQS_QUEUE_URL not set"))?;

        // Custom endpoint for LocalStack
        let sqs_endpoint_url = env::var("SQS_ENDPOINT_URL").ok();

        let stockfish_path = env::var("STOCKFISH_PATH")
            .unwrap_or_else(|_| "/usr/local/bin/stockfish".to_string());

        let nodes_per_position = env::var("NODES_PER_POSITION")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(100_000);

        let max_empty_receives = env::var("MAX_EMPTY_RECEIVES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5);

        let visibility_timeout_secs = env::var("VISIBILITY_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(300);

        // Determine database URL
        let database_url = if env::var("LOCAL_DEV").is_ok() {
            // Local development: use DATABASE_URL directly
            info!("Local dev mode: using DATABASE_URL from environment");
            env::var("DATABASE_URL")
                .map_err(|_| WorkerError::Config("DATABASE_URL not set (LOCAL_DEV mode)"))?
        } else {
            // Production: fetch from Secrets Manager
            let secret_arn = env::var("DB_SECRET_ARN")
                .map_err(|_| WorkerError::Config("DB_SECRET_ARN not set"))?;

            info!(secret_arn = %secret_arn, "Fetching database URL from Secrets Manager");
            fetch_database_url_from_secrets(&secret_arn).await?
        };

        Ok(Self {
            database_url,
            sqs_queue_url,
            sqs_endpoint_url,
            stockfish_path,
            nodes_per_position,
            max_empty_receives,
            visibility_timeout_secs,
        })
    }
}

/// Fetch database URL from AWS Secrets Manager
async fn fetch_database_url_from_secrets(secret_arn: &str) -> Result<String, WorkerError> {
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let client = SecretsClient::new(&config);

    let response = client
        .get_secret_value()
        .secret_id(secret_arn)
        .send()
        .await
        .map_err(|e| WorkerError::SecretsManager(e.to_string()))?;

    let secret_string = response
        .secret_string()
        .ok_or_else(|| WorkerError::SecretsManager("Secret has no string value".into()))?;

    // Secret can be either:
    // 1. A plain connection string
    // 2. A JSON object with connection details
    if secret_string.starts_with("postgresql://") || secret_string.starts_with("postgres://") {
        Ok(secret_string.to_string())
    } else {
        // Parse as JSON
        let secret: serde_json::Value = serde_json::from_str(secret_string)
            .map_err(|e| WorkerError::SecretsManager(format!("Failed to parse secret JSON: {e}")))?;

        // Try common field names
        if let Some(url) = secret.get("url").or(secret.get("DATABASE_URL")) {
            return url
                .as_str()
                .map(String::from)
                .ok_or_else(|| WorkerError::SecretsManager("Database URL is not a string".into()));
        }

        // Build connection string from components
        let host = secret
            .get("host")
            .and_then(|v| v.as_str())
            .ok_or_else(|| WorkerError::SecretsManager("Missing 'host' in secret".into()))?;
        let port = secret
            .get("port")
            .and_then(|v| v.as_u64())
            .unwrap_or(5432);
        let username = secret
            .get("username")
            .and_then(|v| v.as_str())
            .ok_or_else(|| WorkerError::SecretsManager("Missing 'username' in secret".into()))?;
        let password = secret
            .get("password")
            .and_then(|v| v.as_str())
            .ok_or_else(|| WorkerError::SecretsManager("Missing 'password' in secret".into()))?;
        let database = secret
            .get("dbname")
            .or(secret.get("database"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| WorkerError::SecretsManager("Missing 'dbname' in secret".into()))?;

        Ok(format!(
            "postgresql://{username}:{password}@{host}:{port}/{database}"
        ))
    }
}
