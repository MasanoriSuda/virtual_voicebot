#![allow(clippy::module_inception)]

#[path = "session_coordinator.rs"]
mod session_coordinator;

pub use session_coordinator::{SessionCoordinator, SessionHandle};

/// Backwards-compatible alias until call sites move to SessionCoordinator.
pub type Session = SessionCoordinator;
