use anyhow::Result;
use serde_json::Value;
use std::time::Duration;

use crate::ports::ingest::{IngestFuture, IngestPort};

pub struct HttpIngestPort {
    timeout: Duration,
    client: reqwest::Client,
}

impl HttpIngestPort {
    pub fn new(timeout: Duration) -> Self {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .expect("failed to build ingest HTTP client");
        Self { timeout, client }
    }
}

impl IngestPort for HttpIngestPort {
    fn post(&self, url: String, payload: Value) -> IngestFuture<Result<()>> {
        let timeout = self.timeout;
        let client = self.client.clone();
        Box::pin(async move {
            client.post(url).timeout(timeout).json(&payload).send().await?;
            Ok(())
        })
    }
}
