use chrono::{DateTime, Utc};

use crate::shared::entities::identifiers::{CallId, SessionId};

#[derive(Debug, Clone)]
pub struct Session {
    pub id: SessionId,
    pub call_id: CallId,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

impl Session {
    pub fn new(call_id: CallId) -> Self {
        Self {
            id: SessionId::new(),
            call_id,
            started_at: Utc::now(),
            ended_at: None,
        }
    }
}
