use std::future::Future;
use std::pin::Pin;

use chrono::{DateTime, Utc};
use serde_json::Value;
use thiserror::Error;

#[derive(Clone, Debug)]
pub struct SystemSettings {
    pub id: i32,
    pub recording_retention_days: i32,
    pub history_retention_days: i32,
    pub sync_endpoint_url: Option<String>,
    pub default_action_code: String,
    pub max_concurrent_calls: i32,
    pub extra: Value,
    pub version: i32,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("read failed: {0}")]
    ReadFailed(String),
    #[error("write failed: {0}")]
    WriteFailed(String),
}

pub type SettingsFuture<T> = Pin<Box<dyn Future<Output = Result<T, SettingsError>> + Send>>;

pub trait SettingsPort: Send + Sync {
    fn get(&self) -> SettingsFuture<SystemSettings>;
    fn update(&self, settings: SystemSettings) -> SettingsFuture<()>;
}
