use anyhow::Result;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

pub type IngestFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

pub trait IngestPort: Send + Sync {
    fn post(&self, url: String, payload: Value) -> IngestFuture<Result<()>>;
}

pub struct HttpIngestPort {
    timeout: Duration,
}

impl HttpIngestPort {
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }
}

impl IngestPort for HttpIngestPort {
    fn post(&self, url: String, payload: Value) -> IngestFuture<Result<()>> {
        let timeout = self.timeout;
        Box::pin(async move {
            let client = reqwest::Client::builder().timeout(timeout).build()?;
            client.post(url).json(&payload).send().await?;
            Ok(())
        })
    }
}
