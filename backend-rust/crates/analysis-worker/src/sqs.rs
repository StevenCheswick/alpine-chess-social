//! SQS client wrapper for analysis job queue

use aws_sdk_sqs::Client;
use tracing::debug;

use crate::config::WorkerConfig;
use crate::error::WorkerError;

/// A message received from SQS
#[derive(Debug, Clone)]
pub struct SqsMessage {
    /// Message body (contains game ID)
    pub body: String,
    /// Receipt handle for deletion/visibility extension
    pub receipt_handle: String,
}

/// SQS client for receiving and managing analysis jobs
#[derive(Clone)]
pub struct SqsClient {
    client: Client,
    queue_url: String,
    visibility_timeout: i32,
}

impl SqsClient {
    /// Create a new SQS client
    pub async fn new(config: &WorkerConfig) -> Result<Self, WorkerError> {
        let aws_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;

        // For LocalStack/local dev, use custom endpoint
        let client = if let Some(endpoint) = &config.sqs_endpoint_url {
            let sqs_config = aws_sdk_sqs::config::Builder::from(&aws_config)
                .endpoint_url(endpoint)
                .build();
            Client::from_conf(sqs_config)
        } else {
            Client::new(&aws_config)
        };

        Ok(Self {
            client,
            queue_url: config.sqs_queue_url.clone(),
            visibility_timeout: config.visibility_timeout_secs as i32,
        })
    }

    /// Receive messages from the queue with long polling
    pub async fn receive_messages(&self) -> Result<Vec<SqsMessage>, WorkerError> {
        let response = self
            .client
            .receive_message()
            .queue_url(&self.queue_url)
            .max_number_of_messages(10)
            .wait_time_seconds(20) // Long polling
            .visibility_timeout(self.visibility_timeout)
            .send()
            .await
            .map_err(|e| WorkerError::Sqs(format!("Failed to receive messages: {e}")))?;

        let messages = response
            .messages()
            .iter()
            .filter_map(|msg| {
                let body = msg.body()?;
                let receipt = msg.receipt_handle()?;
                Some(SqsMessage {
                    body: body.to_string(),
                    receipt_handle: receipt.to_string(),
                })
            })
            .collect();

        debug!(count = response.messages().len(), "Received messages");
        Ok(messages)
    }

    /// Delete a message from the queue (after successful processing)
    pub async fn delete_message(&self, receipt_handle: &str) -> Result<(), WorkerError> {
        self.client
            .delete_message()
            .queue_url(&self.queue_url)
            .receipt_handle(receipt_handle)
            .send()
            .await
            .map_err(|e| WorkerError::Sqs(format!("Failed to delete message: {e}")))?;

        debug!("Deleted message");
        Ok(())
    }

    /// Extend visibility timeout for a message (for long-running analysis)
    pub async fn extend_visibility(
        &self,
        receipt_handle: &str,
        timeout_seconds: i32,
    ) -> Result<(), WorkerError> {
        self.client
            .change_message_visibility()
            .queue_url(&self.queue_url)
            .receipt_handle(receipt_handle)
            .visibility_timeout(timeout_seconds)
            .send()
            .await
            .map_err(|e| WorkerError::Sqs(format!("Failed to extend visibility: {e}")))?;

        debug!(timeout_seconds, "Extended visibility");
        Ok(())
    }
}
