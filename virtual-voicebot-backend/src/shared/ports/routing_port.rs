use std::future::Future;
use std::pin::Pin;

use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct RegisteredNumberRow {
    pub action_code: String,
    pub ivr_flow_id: Option<Uuid>,
    pub recording_enabled: bool,
    pub announce_enabled: bool,
    pub announcement_id: Option<Uuid>,
}

#[derive(Clone, Debug)]
pub struct CallActionRuleRow {
    pub id: Uuid,
    pub action_config: Value,
}

#[derive(Clone, Debug)]
pub struct RoutingRuleRow {
    pub id: Uuid,
    pub action_code: String,
    pub ivr_flow_id: Option<Uuid>,
    pub announcement_id: Option<Uuid>,
}

#[derive(Clone, Debug)]
pub struct IvrMenuRow {
    pub root_node_id: Uuid,
    pub keypad_node_id: Uuid,
    pub audio_file_url: Option<String>,
    pub timeout_sec: i32,
    pub max_retries: i32,
}

#[derive(Clone, Debug)]
pub struct IvrDestinationRow {
    pub transition_id: Uuid,
    pub node_id: Uuid,
    pub action_code: String,
    pub audio_file_url: Option<String>,
    pub metadata_json: Option<String>,
}

#[derive(Debug, Error)]
pub enum RoutingPortError {
    #[error("read failed: {0}")]
    ReadFailed(String),
}

pub type RoutingFuture<T> = Pin<Box<dyn Future<Output = Result<T, RoutingPortError>> + Send>>;

pub trait RoutingPort: Send + Sync {
    fn find_registered_number(
        &self,
        phone_number: &str,
    ) -> RoutingFuture<Option<RegisteredNumberRow>>;
    fn find_caller_group(&self, phone_number: &str) -> RoutingFuture<Option<Uuid>>;
    fn find_call_action_rule(&self, group_id: Uuid) -> RoutingFuture<Option<CallActionRuleRow>>;
    fn is_spam(&self, phone_number: &str) -> RoutingFuture<bool>;
    fn is_registered(&self, phone_number: &str) -> RoutingFuture<bool>;
    fn find_routing_rule(&self, category: &str) -> RoutingFuture<Option<RoutingRuleRow>>;
    fn get_system_settings_extra(&self) -> RoutingFuture<Option<Value>>;
    fn find_announcement_audio_file_url(
        &self,
        announcement_id: Uuid,
    ) -> RoutingFuture<Option<String>>;
    fn find_ivr_menu(&self, flow_id: Uuid) -> RoutingFuture<Option<IvrMenuRow>>;
    fn find_ivr_dtmf_destination(
        &self,
        keypad_node_id: Uuid,
        dtmf_key: &str,
    ) -> RoutingFuture<Option<IvrDestinationRow>>;
    fn find_ivr_timeout_destination(
        &self,
        keypad_node_id: Uuid,
    ) -> RoutingFuture<Option<IvrDestinationRow>>;
    fn find_ivr_invalid_destination(
        &self,
        keypad_node_id: Uuid,
    ) -> RoutingFuture<Option<IvrDestinationRow>>;
}

#[derive(Default)]
pub struct NoopRoutingPort;

impl NoopRoutingPort {
    pub fn new() -> Self {
        Self
    }
}

impl RoutingPort for NoopRoutingPort {
    fn find_registered_number(
        &self,
        _phone_number: &str,
    ) -> RoutingFuture<Option<RegisteredNumberRow>> {
        Box::pin(async { Ok(None) })
    }

    fn find_caller_group(&self, _phone_number: &str) -> RoutingFuture<Option<Uuid>> {
        Box::pin(async { Ok(None) })
    }

    fn find_call_action_rule(&self, _group_id: Uuid) -> RoutingFuture<Option<CallActionRuleRow>> {
        Box::pin(async { Ok(None) })
    }

    fn is_spam(&self, _phone_number: &str) -> RoutingFuture<bool> {
        Box::pin(async { Ok(false) })
    }

    fn is_registered(&self, _phone_number: &str) -> RoutingFuture<bool> {
        Box::pin(async { Ok(false) })
    }

    fn find_routing_rule(&self, _category: &str) -> RoutingFuture<Option<RoutingRuleRow>> {
        Box::pin(async { Ok(None) })
    }

    fn get_system_settings_extra(&self) -> RoutingFuture<Option<Value>> {
        Box::pin(async { Ok(None) })
    }

    fn find_announcement_audio_file_url(
        &self,
        _announcement_id: Uuid,
    ) -> RoutingFuture<Option<String>> {
        Box::pin(async { Ok(None) })
    }

    fn find_ivr_menu(&self, _flow_id: Uuid) -> RoutingFuture<Option<IvrMenuRow>> {
        Box::pin(async { Ok(None) })
    }

    fn find_ivr_dtmf_destination(
        &self,
        _keypad_node_id: Uuid,
        _dtmf_key: &str,
    ) -> RoutingFuture<Option<IvrDestinationRow>> {
        Box::pin(async { Ok(None) })
    }

    fn find_ivr_timeout_destination(
        &self,
        _keypad_node_id: Uuid,
    ) -> RoutingFuture<Option<IvrDestinationRow>> {
        Box::pin(async { Ok(None) })
    }

    fn find_ivr_invalid_destination(
        &self,
        _keypad_node_id: Uuid,
    ) -> RoutingFuture<Option<IvrDestinationRow>> {
        Box::pin(async { Ok(None) })
    }
}
