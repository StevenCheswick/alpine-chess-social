//! SQS client for queueing server-side analysis jobs

use aws_sdk_sqs::Client;

use crate::config::Config;

/// SQS client for sending analysis jobs
#[derive(Clone)]
pub struct AnalysisQueue {
    client: Client,
    queue_url: String,
}

impl AnalysisQueue {
    /// Create a new SQS client from config.
    /// Returns None if SQS is not configured.
    pub async fn new(config: &Config) -> Option<Self> {
        let queue_url = config.sqs_queue_url.as_ref()?;

        let aws_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;

        let client = if let Some(endpoint) = &config.sqs_endpoint_url {
            // LocalStack or custom endpoint
            let sqs_config = aws_sdk_sqs::config::Builder::from(&aws_config)
                .endpoint_url(endpoint)
                .build();
            Client::from_conf(sqs_config)
        } else {
            Client::new(&aws_config)
        };

        Some(Self {
            client,
            queue_url: queue_url.clone(),
        })
    }

    /// Queue a single game for server-side analysis
    pub async fn queue_game(&self, game_id: i64) -> Result<(), String> {
        self.client
            .send_message()
            .queue_url(&self.queue_url)
            .message_body(game_id.to_string())
            .send()
            .await
            .map_err(|e| format!("Failed to send SQS message: {e}"))?;

        Ok(())
    }

    /// Queue multiple games for server-side analysis (batch)
    pub async fn queue_games(&self, game_ids: &[i64]) -> Result<usize, String> {
        if game_ids.is_empty() {
            return Ok(0);
        }

        // SQS supports max 10 messages per batch
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
                .client
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

        Ok(total_queued)
    }
}
