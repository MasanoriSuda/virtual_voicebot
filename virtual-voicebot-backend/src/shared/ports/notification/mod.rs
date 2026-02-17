use std::future::Future;
use std::pin::Pin;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum NotificationError {
    #[error("notification failed: {0}")]
    Failed(String),
}

pub type NotificationFuture = Pin<Box<dyn Future<Output = Result<(), NotificationError>> + Send>>;

pub mod ended;
pub mod missed;
pub mod ringing;

pub use ended::CallEndedNotifier;
pub use missed::MissedCallNotifier;
pub use ringing::RingingNotifier;

pub trait NotificationService: RingingNotifier + MissedCallNotifier + CallEndedNotifier {}

impl<T> NotificationService for T where T: RingingNotifier + MissedCallNotifier + CallEndedNotifier {}
