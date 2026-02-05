use chrono::{DateTime, FixedOffset};

use super::NotificationFuture;

pub trait MissedCallNotifier: Send + Sync {
    fn notify_missed(&self, from: String, timestamp: DateTime<FixedOffset>) -> NotificationFuture;
}
