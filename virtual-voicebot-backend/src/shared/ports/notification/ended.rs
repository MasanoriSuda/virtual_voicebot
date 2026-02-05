use super::NotificationFuture;

pub trait CallEndedNotifier: Send + Sync {
    fn notify_ended(&self, call_id: &str, from: String, duration_sec: u64) -> NotificationFuture;
}
