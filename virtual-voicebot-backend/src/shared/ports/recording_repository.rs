use std::future::Future;
use std::pin::Pin;

use chrono::{DateTime, Utc};
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct NewRecording {
    pub id: Uuid,
    pub call_log_id: Uuid,
    pub recording_type: String,
    pub sequence_number: i16,
    pub file_path: String,
    pub s3_url: Option<String>,
    pub upload_status: String,
    pub duration_sec: Option<i32>,
    pub format: String,
    pub file_size_bytes: Option<i64>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Error)]
pub enum RecordingRepositoryError {
    #[error("write failed: {0}")]
    WriteFailed(String),
    #[error("not found: {0}")]
    NotFound(String),
}

pub type RecordingRepositoryFuture<T> =
    Pin<Box<dyn Future<Output = Result<T, RecordingRepositoryError>> + Send>>;

pub trait RecordingRepositoryPort: Send + Sync {
    fn append_recording(&self, recording: NewRecording) -> RecordingRepositoryFuture<()>;
    fn update_upload_status(
        &self,
        id: Uuid,
        upload_status: String,
        s3_url: Option<String>,
    ) -> RecordingRepositoryFuture<()>;
    fn mark_recording_synced(
        &self,
        id: Uuid,
        synced_at: DateTime<Utc>,
    ) -> RecordingRepositoryFuture<()>;
}
