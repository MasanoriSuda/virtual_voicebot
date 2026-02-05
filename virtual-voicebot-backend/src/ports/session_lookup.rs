use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::entities::CallId;
use crate::ports::rtp_sink::RtpEventSink;

pub type SessionLookupFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// Lookup interface to resolve per-call media sinks without depending on session internals.
pub trait SessionLookup: Send + Sync {
    fn rtp_sink(&self, call_id: CallId) -> SessionLookupFuture<Option<Arc<dyn RtpEventSink>>>;
}
