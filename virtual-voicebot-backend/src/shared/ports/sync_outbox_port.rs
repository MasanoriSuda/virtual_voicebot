use std::future::Future;
use std::pin::Pin;

use chrono::{DateTime, Utc};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct NewOutboxEntry {
    pub entity_type: String,
    pub entity_id: Uuid,
    pub payload: Value,
}

#[derive(Clone, Debug)]
pub struct PendingOutboxEntry {
    pub id: i64,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub payload: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Error)]
pub enum SyncOutboxError {
    #[error("write failed: {0}")]
    WriteFailed(String),
    #[error("read failed: {0}")]
    ReadFailed(String),
}

pub type SyncOutboxFuture<T> = Pin<Box<dyn Future<Output = Result<T, SyncOutboxError>> + Send>>;

pub trait SyncOutboxPort: Send + Sync {
    fn enqueue(&self, entry: NewOutboxEntry) -> SyncOutboxFuture<i64>;
    fn fetch_pending(&self, limit: i64) -> SyncOutboxFuture<Vec<PendingOutboxEntry>>;
    fn mark_processed(&self, id: i64, processed_at: DateTime<Utc>) -> SyncOutboxFuture<()>;
    fn mark_recording_uploaded(&self, recording_id: Uuid, file_url: String)
        -> SyncOutboxFuture<()>;
}
