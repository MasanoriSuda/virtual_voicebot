pub mod interface;
pub mod protocol;
pub mod service;
pub mod shared;

// Backward-compatible re-exports (transitional).
pub use interface::{db, http, notification};
pub use protocol::{rtp, session, sip, transport};
pub use service::{ai, call_control as app, recording};
pub use shared::{config, entities, error, logging, media, ports, utils};
