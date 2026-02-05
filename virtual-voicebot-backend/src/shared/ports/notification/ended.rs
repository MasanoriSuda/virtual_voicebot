use super::NotificationFuture;

pub trait CallEndedNotifier: Send + Sync {
    fn notify_ended(&self, from: String, duration_sec: u64) -> NotificationFuture;
}
