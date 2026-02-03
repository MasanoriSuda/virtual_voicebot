#![allow(clippy::module_inception)]

pub mod b2bua;
mod capture;
pub mod coordinator;
pub mod ingest_manager;
pub mod recording_manager;
pub mod rtp_stream_manager;
pub mod session;
pub mod state_machine;
mod timers;
pub mod types;
pub mod writing;

#[allow(unused_imports)]
pub use session::{Session, SessionCoordinator, SessionHandle};
pub use state_machine::SessionStateMachine;
pub use types::SessionRegistry;
#[allow(unused_imports)]
pub use types::{MediaConfig, Sdp, SessionIn, SessionOut};
#[allow(unused_imports)]
pub use writing::{spawn_call, spawn_session};
