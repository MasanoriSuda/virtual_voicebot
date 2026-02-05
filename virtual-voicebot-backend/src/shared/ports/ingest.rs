use std::future::Future;
use std::pin::Pin;
use std::time::SystemTime;

use thiserror::Error;

use crate::shared::entities::identifiers::CallId;

pub type IngestFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

#[derive(Debug, Clone)]
pub struct IngestRecording {
    pub recording_url: String,
    pub duration_sec: u64,
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Debug, Clone)]
pub struct IngestPayload {
    pub call_id: CallId,
    pub from: String,
    pub to: String,
    pub started_at: SystemTime,
    pub ended_at: SystemTime,
    pub status: String,
    pub summary: String,
    pub duration_sec: u64,
    pub recording: Option<IngestRecording>,
}

#[derive(Debug, Error)]
pub enum IngestError {
    #[error("http error: {0}")]
    Transport(String),
    #[error("serialization error: {0}")]
    Serialize(String),
}

pub trait IngestPort: Send + Sync {
    fn post(&self, url: String, payload: IngestPayload) -> IngestFuture<Result<(), IngestError>>;
}
