#![allow(clippy::module_inception)]

pub use super::coordinator::{SessionCoordinator, SessionHandle};

/// Backwards-compatible alias until call sites move to SessionCoordinator.
pub type Session = SessionCoordinator;
