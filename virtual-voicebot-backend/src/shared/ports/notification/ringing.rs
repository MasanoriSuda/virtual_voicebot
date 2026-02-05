use chrono::{DateTime, FixedOffset};

use super::NotificationFuture;

pub trait RingingNotifier: Send + Sync {
    fn notify_ringing(&self, from: String, timestamp: DateTime<FixedOffset>) -> NotificationFuture;
}
