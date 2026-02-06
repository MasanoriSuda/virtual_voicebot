use std::future::Future;
use std::pin::Pin;

use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallerCategory {
    Spam,
    Registered,
    Unknown,
    Anonymous,
}

#[derive(Clone, Debug)]
pub struct PhoneLookupResult {
    pub phone_number: String,
    pub caller_category: CallerCategory,
    pub action_code: String,
    pub ivr_flow_id: Option<Uuid>,
    pub recording_enabled: bool,
    pub announce_enabled: bool,
}

#[derive(Debug, Error)]
pub enum PhoneLookupError {
    #[error("lookup failed: {0}")]
    LookupFailed(String),
}

pub type PhoneLookupFuture =
    Pin<Box<dyn Future<Output = Result<Option<PhoneLookupResult>, PhoneLookupError>> + Send>>;

pub trait PhoneLookupPort: Send + Sync {
    fn lookup_phone(&self, phone_number: String) -> PhoneLookupFuture;
}

#[derive(Clone, Debug, Default)]
pub struct NoopPhoneLookup;

impl NoopPhoneLookup {
    pub fn new() -> Self {
        Self
    }
}

impl PhoneLookupPort for NoopPhoneLookup {
    fn lookup_phone(&self, _phone_number: String) -> PhoneLookupFuture {
        Box::pin(async move { Ok(None) })
    }
}
