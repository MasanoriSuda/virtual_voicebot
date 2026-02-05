use crate::shared::entities::CallId;
use chrono::{DateTime, FixedOffset};

use super::NotificationFuture;

pub trait RingingNotifier: Send + Sync {
    fn notify_ringing(
        &self,
        call_id: CallId,
        from: String,
        timestamp: DateTime<FixedOffset>,
    ) -> NotificationFuture;
}
