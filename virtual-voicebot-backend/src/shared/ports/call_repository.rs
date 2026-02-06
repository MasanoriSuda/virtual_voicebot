use std::future::Future;
use std::pin::Pin;

use chrono::{DateTime, Utc};
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct NewCallLog {
    pub id: Uuid,
    pub started_at: DateTime<Utc>,
    pub external_call_id: String,
    pub sip_call_id: Option<String>,
    pub caller_number: Option<String>,
    pub caller_category: String,
    pub action_code: String,
    pub ivr_flow_id: Option<Uuid>,
}

#[derive(Clone, Debug)]
pub struct CallEndUpdate {
    pub id: Uuid,
    pub started_at: DateTime<Utc>,
    pub answered_at: Option<DateTime<Utc>>,
    pub ended_at: DateTime<Utc>,
    pub duration_sec: Option<i32>,
    pub end_reason: String,
}

#[derive(Debug, Error)]
pub enum CallRepositoryError {
    #[error("write failed: {0}")]
    WriteFailed(String),
    #[error("not found: {0}")]
    NotFound(String),
}

pub type CallRepositoryFuture<T> =
    Pin<Box<dyn Future<Output = Result<T, CallRepositoryError>> + Send>>;

pub trait CallRepositoryPort: Send + Sync {
    fn insert_call_started(&self, record: NewCallLog) -> CallRepositoryFuture<()>;
    fn mark_call_ended(&self, update: CallEndUpdate) -> CallRepositoryFuture<()>;
    fn mark_call_synced(&self, id: Uuid, started_at: DateTime<Utc>) -> CallRepositoryFuture<()>;
}
