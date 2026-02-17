use std::future::Future;
use std::pin::Pin;

use chrono::{DateTime, Utc};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct EndedRecording {
    pub id: Uuid,
    pub file_path: String,
    pub duration_sec: Option<i32>,
    pub format: String,
    pub file_size_bytes: Option<i64>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug)]
pub struct EndedCallLog {
    pub id: Uuid,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub duration_sec: Option<i32>,
    pub external_call_id: String,
    pub sip_call_id: String,
    pub caller_number: Option<String>,
    pub caller_category: String,
    pub action_code: String,
    pub ivr_flow_id: Option<Uuid>,
    pub answered_at: Option<DateTime<Utc>>,
    pub end_reason: String,
    pub status: String,
    pub call_disposition: String,
    pub final_action: Option<String>,
    pub transfer_status: String,
    pub transfer_started_at: Option<DateTime<Utc>>,
    pub transfer_answered_at: Option<DateTime<Utc>>,
    pub transfer_ended_at: Option<DateTime<Utc>>,
    pub ivr_events: Vec<EndedIvrSessionEvent>,
    pub recording: Option<EndedRecording>,
}

#[derive(Clone, Debug)]
pub struct EndedIvrSessionEvent {
    pub id: Uuid,
    pub sequence: i32,
    pub event_type: String,
    pub occurred_at: DateTime<Utc>,
    pub node_id: Option<Uuid>,
    pub dtmf_key: Option<String>,
    pub transition_id: Option<Uuid>,
    pub exit_action: Option<String>,
    pub exit_reason: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Error)]
pub enum CallLogPortError {
    #[error("write failed: {0}")]
    WriteFailed(String),
}

pub type CallLogFuture<T> = Pin<Box<dyn Future<Output = Result<T, CallLogPortError>> + Send>>;

pub trait CallLogPort: Send + Sync {
    fn persist_call_ended(&self, call_log: EndedCallLog) -> CallLogFuture<()>;
}

#[derive(Default)]
pub struct NoopCallLogPort;

impl NoopCallLogPort {
    pub fn new() -> Self {
        Self
    }
}

impl CallLogPort for NoopCallLogPort {
    fn persist_call_ended(&self, _call_log: EndedCallLog) -> CallLogFuture<()> {
        Box::pin(async { Ok(()) })
    }
}
