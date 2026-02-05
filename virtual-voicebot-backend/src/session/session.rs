#![allow(clippy::module_inception)]

pub use super::coordinator::SessionCoordinator;
pub use super::types::SessionHandle;

/// Backwards-compatible alias until call sites move to SessionCoordinator.
pub type Session = SessionCoordinator;
