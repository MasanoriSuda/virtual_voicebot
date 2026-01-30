use anyhow::Result;
use std::future::Future;
use std::pin::Pin;

#[derive(Clone, Debug)]
pub struct PhoneLookupResult {
    pub phone_number: String,
    pub ivr_enabled: bool,
}

pub type PhoneLookupFuture =
    Pin<Box<dyn Future<Output = Result<Option<PhoneLookupResult>>> + Send>>;

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
