use std::path::PathBuf;

use chrono::{DateTime, Utc};

use crate::entities::identifiers::{CallId, RecordingId};

#[derive(Debug, Clone)]
pub struct Recording {
    pub id: RecordingId,
    pub call_id: CallId,
    pub path: PathBuf,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct RecordingRef {
    pub id: RecordingId,
    pub path: PathBuf,
}
