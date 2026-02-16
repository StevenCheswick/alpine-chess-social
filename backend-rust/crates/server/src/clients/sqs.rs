//! SQS + Batch client for queueing server-side analysis jobs

use aws_sdk_batch::Client as BatchClient;
use aws_sdk_sqs::Client as SqsClient;

use crate::config::Config;

/// Client for managing analysis jobs via SQS and AWS Batch
#[derive(Clone)]
pub struct AnalysisQueue {
    sqs: SqsClient,
    batch: BatchClient,
    queue_url: String,
    job_queue: String,
    job_definition: String,
}

impl AnalysisQueue {
    /// Create a new client from config.
    /// Returns None if SQS is not configured.
    pub async fn new(config: &Config) -> Option<Self> {
        let queue_url = config.sqs_queue_url.as_ref()?;

        let aws_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;

        let sqs = if let Some(endpoint) = &config.sqs_endpoint_url {
            // LocalStack or custom endpoint
            let sqs_config = aws_sdk_sqs::config::Builder::from(&aws_config)
                .endpoint_url(endpoint)
                .build();
            SqsClient::from_conf(sqs_config)
        } else {
            SqsClient::new(&aws_config)
        };

        let batch = BatchClient::new(&aws_config);

        Some(Self {
            sqs,
            batch,
            queue_url: queue_url.clone(),
            job_queue: config
                .batch_job_queue
                .clone()
                .unwrap_or_else(|| "alpine-chess-analysis-queue".to_string()),
            job_definition: config
                .batch_job_definition
                .clone()
                .unwrap_or_else(|| "alpine-chess-analysis-worker".to_string()),
        })
    }

    /// Queue a single game for server-side analysis
    pub async fn queue_game(&self, game_id: i64) -> Result<(), String> {
        self.sqs
            .send_message()
            .queue_url(&self.queue_url)
            .message_body(game_id.to_string())
            .send()
            .await
            .map_err(|e| format!("Failed to send SQS message: {e}"))?;

        Ok(())
    }

    /// Queue multiple games for server-side analysis and ensure a worker is running
    pub async fn queue_games(&self, game_ids: &[i64]) -> Result<usize, String> {
        if game_ids.is_empty() {
            return Ok(0);
        }

        // Queue all messages to SQS
        let mut total_queued = 0;

        for chunk in game_ids.chunks(10) {
            let entries: Vec<_> = chunk
                .iter()
                .enumerate()
                .map(|(i, id)| {
                    aws_sdk_sqs::types::SendMessageBatchRequestEntry::builder()
                        .id(i.to_string())
                        .message_body(id.to_string())
                        .build()
                        .unwrap()
                })
                .collect();

            let result = self
                .sqs
                .send_message_batch()
                .queue_url(&self.queue_url)
                .set_entries(Some(entries))
                .send()
                .await
                .map_err(|e| format!("Failed to send SQS batch: {e}"))?;

            total_queued += result.successful().len();

            if !result.failed().is_empty() {
                tracing::warn!(
                    "Some messages failed to queue: {:?}",
                    result.failed()
                );
            }
        }

        // Ensure a Batch worker is running to process the queue
        self.ensure_worker_running().await?;

        Ok(total_queued)
    }

    /// Check if a Batch worker is already running/pending, if not start one
    async fn ensure_worker_running(&self) -> Result<(), String> {
        // Check for existing running or pending jobs
        for status in ["RUNNING", "RUNNABLE", "STARTING", "SUBMITTED", "PENDING"] {
            let result = self
                .batch
                .list_jobs()
                .job_queue(&self.job_queue)
                .job_status(aws_sdk_batch::types::JobStatus::from(status))
                .max_results(1)
                .send()
                .await
                .map_err(|e| format!("Failed to list Batch jobs: {e}"))?;

            if !result.job_summary_list().is_empty() {
                tracing::info!(
                    status = status,
                    "Batch worker already exists, not starting new one"
                );
                return Ok(());
            }
        }

        // No active worker found, submit a new job
        let job_name = format!(
            "analysis-worker-{}",
            chrono::Utc::now().timestamp()
        );

        let result = self
            .batch
            .submit_job()
            .job_name(&job_name)
            .job_queue(&self.job_queue)
            .job_definition(&self.job_definition)
            .send()
            .await
            .map_err(|e| format!("Failed to submit Batch job: {e}"))?;

        tracing::info!(
            job_id = ?result.job_id(),
            job_name = job_name,
            "Submitted new Batch worker"
        );

        Ok(())
    }
}
