use std::future::Future;
use std::pin::Pin;

use chrono::{DateTime, Utc};
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct RoutingRule {
    pub id: Uuid,
    pub caller_category: String,
    pub action_code: String,
    pub ivr_flow_id: Option<Uuid>,
    pub priority: i32,
    pub is_active: bool,
    pub folder_id: Option<Uuid>,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct UpsertRoutingRule {
    pub id: Uuid,
    pub caller_category: String,
    pub action_code: String,
    pub ivr_flow_id: Option<Uuid>,
    pub priority: i32,
    pub is_active: bool,
    pub folder_id: Option<Uuid>,
    pub version: i32,
}

#[derive(Debug, Error)]
pub enum RoutingRuleError {
    #[error("read failed: {0}")]
    ReadFailed(String),
    #[error("write failed: {0}")]
    WriteFailed(String),
}

pub type RoutingRuleFuture<T> = Pin<Box<dyn Future<Output = Result<T, RoutingRuleError>> + Send>>;

pub trait RoutingRulePort: Send + Sync {
    fn list_active(&self) -> RoutingRuleFuture<Vec<RoutingRule>>;
    fn upsert_routing_rule(&self, input: UpsertRoutingRule) -> RoutingRuleFuture<()>;
}
