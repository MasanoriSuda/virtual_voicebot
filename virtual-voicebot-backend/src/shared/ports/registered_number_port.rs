use std::future::Future;
use std::pin::Pin;

use chrono::{DateTime, Utc};
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct RegisteredNumber {
    pub id: Uuid,
    pub phone_number: String,
    pub name: Option<String>,
    pub category: String,
    pub action_code: String,
    pub ivr_flow_id: Option<Uuid>,
    pub recording_enabled: bool,
    pub announce_enabled: bool,
    pub notes: Option<String>,
    pub folder_id: Option<Uuid>,
    pub version: i32,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct UpsertRegisteredNumber {
    pub id: Uuid,
    pub phone_number: String,
    pub name: Option<String>,
    pub category: String,
    pub action_code: String,
    pub ivr_flow_id: Option<Uuid>,
    pub recording_enabled: bool,
    pub announce_enabled: bool,
    pub notes: Option<String>,
    pub folder_id: Option<Uuid>,
    pub version: i32,
}

#[derive(Debug, Error)]
pub enum RegisteredNumberError {
    #[error("read failed: {0}")]
    ReadFailed(String),
    #[error("write failed: {0}")]
    WriteFailed(String),
}

pub type RegisteredNumberFuture<T> =
    Pin<Box<dyn Future<Output = Result<T, RegisteredNumberError>> + Send>>;

pub trait RegisteredNumberPort: Send + Sync {
    fn list_active(&self) -> RegisteredNumberFuture<Vec<RegisteredNumber>>;
    fn upsert_registered_number(&self, input: UpsertRegisteredNumber)
        -> RegisteredNumberFuture<()>;
}
