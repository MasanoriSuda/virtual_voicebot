use anyhow::Result;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

pub type IngestFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

pub trait IngestPort: Send + Sync {
    fn post(&self, url: String, payload: Value) -> IngestFuture<Result<()>>;
}
