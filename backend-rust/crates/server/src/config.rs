use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_expire_hours: i64,
    pub host: String,
    pub port: u16,
    /// SQS queue URL for server-side analysis (optional)
    pub sqs_queue_url: Option<String>,
    /// Custom SQS endpoint (for LocalStack)
    pub sqs_endpoint_url: Option<String>,
    /// AWS Batch job queue name
    pub batch_job_queue: Option<String>,
    /// AWS Batch job definition name
    pub batch_job_definition: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: env::var("DATABASE_URL")
                .expect("DATABASE_URL must be set"),
            jwt_secret: env::var("JWT_SECRET_KEY")
                .unwrap_or_else(|_| "dev-secret-key-change-in-production".to_string()),
            jwt_expire_hours: env::var("JWT_EXPIRE_HOURS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(168), // 7 days
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8000),
            sqs_queue_url: env::var("SQS_QUEUE_URL").ok(),
            sqs_endpoint_url: env::var("SQS_ENDPOINT_URL").ok(),
            batch_job_queue: env::var("BATCH_JOB_QUEUE").ok(),
            batch_job_definition: env::var("BATCH_JOB_DEFINITION").ok(),
        }
    }
}
