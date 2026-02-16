use std::sync::Arc;

use log::{info, warn};
use serde::Deserialize;
use thiserror::Error;
use uuid::Uuid;

use super::normalize_phone_number_e164;
use crate::shared::ports::routing_port::{
    RegisteredNumberRow, RoutingPort, RoutingPortError, RoutingRuleRow,
};

#[derive(Debug, Clone)]
pub struct ActionConfig {
    pub action_code: String,
    pub caller_category: String,
    pub ivr_flow_id: Option<Uuid>,
    pub recording_enabled: bool,
    pub announce_enabled: bool,
    pub recording_announcement_id: Option<Uuid>,
    pub announcement_id: Option<Uuid>,
    pub announcement_audio_file_url: Option<String>,
    pub scenario_id: Option<String>,
    pub include_announcement: Option<bool>,
}

impl ActionConfig {
    pub fn default_vr() -> Self {
        Self {
            action_code: "VR".to_string(),
            caller_category: "unknown".to_string(),
            ivr_flow_id: None,
            recording_enabled: true,
            announce_enabled: false,
            recording_announcement_id: None,
            announcement_id: None,
            announcement_audio_file_url: None,
            scenario_id: None,
            include_announcement: None,
        }
    }

    pub fn default_bz() -> Self {
        Self {
            action_code: "BZ".to_string(),
            caller_category: "unknown".to_string(),
            ivr_flow_id: None,
            recording_enabled: false,
            announce_enabled: false,
            recording_announcement_id: None,
            announcement_id: None,
            announcement_audio_file_url: None,
            scenario_id: None,
            include_announcement: None,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActionConfigDto {
    action_code: String,
    #[serde(default)]
    ivr_flow_id: Option<Uuid>,
    #[serde(default = "default_true")]
    recording_enabled: bool,
    #[serde(default = "default_false")]
    announce_enabled: bool,
    #[serde(default)]
    recording_announcement_id: Option<Uuid>,
    #[serde(default)]
    announcement_id: Option<Uuid>,
    #[serde(default)]
    welcome_announcement_id: Option<Uuid>,
    #[serde(default)]
    scenario_id: Option<String>,
    #[serde(default)]
    include_announcement: Option<bool>,
}

impl From<ActionConfigDto> for ActionConfig {
    fn from(dto: ActionConfigDto) -> Self {
        let announcement_id = dto.welcome_announcement_id.or(dto.announcement_id);
        Self {
            action_code: dto.action_code,
            caller_category: "unknown".to_string(),
            ivr_flow_id: dto.ivr_flow_id,
            recording_enabled: dto.recording_enabled,
            announce_enabled: dto.announce_enabled,
            recording_announcement_id: dto.recording_announcement_id,
            announcement_id,
            announcement_audio_file_url: None,
            scenario_id: dto.scenario_id,
            include_announcement: dto.include_announcement,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum CallerCategory {
    Spam,
    Registered,
    Unknown,
}

impl CallerCategory {
    fn as_str(self) -> &'static str {
        match self {
            Self::Spam => "spam",
            Self::Registered => "registered",
            Self::Unknown => "unknown",
        }
    }
}

enum CallerGroupMatch {
    Matched(ActionConfig),
    NoGroup,
    NoActiveRule { group_id: Uuid },
}

#[derive(Debug, Error)]
pub enum RoutingError {
    #[error("invalid phone number: {0}")]
    InvalidPhoneNumber(String),
    #[error("database failed: {0}")]
    DatabaseFailed(String),
    #[error("json failed: {0}")]
    JsonFailed(String),
}

impl From<RoutingPortError> for RoutingError {
    fn from(value: RoutingPortError) -> Self {
        Self::DatabaseFailed(value.to_string())
    }
}

impl From<serde_json::Error> for RoutingError {
    fn from(value: serde_json::Error) -> Self {
        Self::JsonFailed(value.to_string())
    }
}

pub struct RuleEvaluator {
    routing_port: Arc<dyn RoutingPort>,
}

impl RuleEvaluator {
    pub fn new(routing_port: Arc<dyn RoutingPort>) -> Self {
        Self { routing_port }
    }

    pub async fn evaluate(
        &self,
        caller_id: &str,
        call_id: &str,
    ) -> Result<ActionConfig, RoutingError> {
        info!(
            "[RuleEvaluator] call_id={} evaluating caller_id={}",
            call_id, caller_id
        );

        if is_anonymous(caller_id) {
            info!(
                "[RuleEvaluator] call_id={} anonymous caller detected caller_id={}",
                call_id, caller_id
            );
            return self.get_anonymous_action(call_id).await;
        }

        let normalized_caller_id = match normalize_phone_number_e164(caller_id) {
            Ok(normalized) => normalized,
            Err(err) => {
                warn!(
                    "[RuleEvaluator] call_id={} phone normalization failed: {}, fallback to defaultAction",
                    call_id, err
                );
                return self.get_default_action(call_id).await;
            }
        };
        info!(
            "[RuleEvaluator] call_id={} normalized caller={} -> {}",
            call_id, caller_id, normalized_caller_id
        );

        if let Some(action) = self
            .match_registered_number(&normalized_caller_id, call_id)
            .await?
        {
            info!(
                "[RuleEvaluator] call_id={} hit stage=1 source=registered_numbers",
                call_id
            );
            return Ok(action);
        }
        info!(
            "[RuleEvaluator] call_id={} miss stage=1 source=registered_numbers",
            call_id
        );

        match self
            .match_caller_group(&normalized_caller_id, call_id)
            .await?
        {
            CallerGroupMatch::Matched(action) => {
                info!(
                    "[RuleEvaluator] call_id={} hit stage=2 source=call_action_rules",
                    call_id
                );
                return Ok(action);
            }
            CallerGroupMatch::NoGroup => {
                info!(
                    "[RuleEvaluator] call_id={} miss stage=2 source=call_action_rules",
                    call_id
                );
            }
            CallerGroupMatch::NoActiveRule { group_id } => {
                info!(
                    "[RuleEvaluator] call_id={} stage=2 group_id={} has no active rule, fallback to defaultAction",
                    call_id, group_id
                );
                return self.get_default_action(call_id).await;
            }
        }

        let category = self.classify_caller(&normalized_caller_id, call_id).await?;
        if matches!(category, CallerCategory::Unknown) {
            info!(
                "[RuleEvaluator] call_id={} category=unknown uses defaultAction",
                call_id
            );
            return self.get_default_action(call_id).await;
        }

        if let Some(action) = self.match_routing_rule(category, call_id).await? {
            info!(
                "[RuleEvaluator] call_id={} hit stage=3 source=routing_rules category={}",
                call_id,
                category.as_str()
            );
            return Ok(action);
        }
        info!(
            "[RuleEvaluator] call_id={} miss stage=3 source=routing_rules category={}",
            call_id,
            category.as_str()
        );

        info!(
            "[RuleEvaluator] call_id={} fallback stage=4 source=system_settings.defaultAction",
            call_id
        );
        self.get_default_action(call_id).await
    }

    async fn match_registered_number(
        &self,
        caller_id: &str,
        call_id: &str,
    ) -> Result<Option<ActionConfig>, RoutingError> {
        let row = self.routing_port.find_registered_number(caller_id).await?;
        let Some(row) = row else {
            return Ok(None);
        };

        if let Some(group_id) = row.group_id {
            info!(
                "[RuleEvaluator] call_id={} stage=1 group_id found, defer to stage=2 phone_number={} group_id={}",
                call_id, caller_id, group_id
            );
            return Ok(None);
        }

        let action = to_action_config_from_registered(row);
        let mut action = action;
        action.caller_category = "registered".to_string();
        info!(
            "[RuleEvaluator] call_id={} stage=1 action_code={}",
            call_id, action.action_code
        );
        Ok(Some(action))
    }

    async fn match_caller_group(
        &self,
        caller_id: &str,
        call_id: &str,
    ) -> Result<CallerGroupMatch, RoutingError> {
        let group_id = self.routing_port.find_caller_group(caller_id).await?;
        let Some(group_id) = group_id else {
            return Ok(CallerGroupMatch::NoGroup);
        };

        let row = self.routing_port.find_call_action_rule(group_id).await?;
        let Some(row) = row else {
            return Ok(CallerGroupMatch::NoActiveRule { group_id });
        };

        let dto: ActionConfigDto = serde_json::from_value(row.action_config)?;
        let mut action: ActionConfig = dto.into();
        action.caller_category = "registered".to_string();
        info!(
            "[RuleEvaluator] call_id={} stage=2 rule_id={} action_code={}",
            call_id, row.id, action.action_code
        );
        Ok(CallerGroupMatch::Matched(action))
    }

    async fn classify_caller(
        &self,
        caller_id: &str,
        call_id: &str,
    ) -> Result<CallerCategory, RoutingError> {
        if self.routing_port.is_spam(caller_id).await? {
            info!(
                "[RuleEvaluator] call_id={} classified category=spam",
                call_id
            );
            return Ok(CallerCategory::Spam);
        }

        if self.routing_port.is_registered(caller_id).await? {
            info!(
                "[RuleEvaluator] call_id={} classified category=registered",
                call_id
            );
            return Ok(CallerCategory::Registered);
        }

        info!(
            "[RuleEvaluator] call_id={} classified category=unknown",
            call_id
        );
        Ok(CallerCategory::Unknown)
    }

    async fn match_routing_rule(
        &self,
        category: CallerCategory,
        call_id: &str,
    ) -> Result<Option<ActionConfig>, RoutingError> {
        let row = self
            .routing_port
            .find_routing_rule(category.as_str())
            .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let rule_id = row.id;
        let mut action = to_action_config_from_routing_rule(row);
        action.caller_category = category.as_str().to_string();
        info!(
            "[RuleEvaluator] call_id={} stage=3 rule_id={} action_code={}",
            call_id, rule_id, action.action_code
        );
        Ok(Some(action))
    }

    async fn get_default_action(&self, call_id: &str) -> Result<ActionConfig, RoutingError> {
        let mut action = self
            .get_action_from_settings_or_fallback(
                "defaultAction",
                ActionConfig::default_vr(),
                call_id,
            )
            .await?;
        action.caller_category = "unknown".to_string();
        Ok(action)
    }

    async fn get_anonymous_action(&self, call_id: &str) -> Result<ActionConfig, RoutingError> {
        let mut action = self
            .get_action_from_settings_or_fallback(
                "anonymousAction",
                ActionConfig::default_bz(),
                call_id,
            )
            .await?;
        action.caller_category = "anonymous".to_string();
        Ok(action)
    }

    async fn get_action_from_settings_or_fallback(
        &self,
        field_name: &str,
        fallback: ActionConfig,
        call_id: &str,
    ) -> Result<ActionConfig, RoutingError> {
        let extra = self.routing_port.get_system_settings_extra().await?;
        let Some(extra) = extra else {
            warn!(
                "[RuleEvaluator] call_id={} system_settings.extra not found, fallback action_code={}",
                call_id, fallback.action_code
            );
            return Ok(fallback);
        };

        let Some(raw_action) = extra.get(field_name) else {
            warn!(
                "[RuleEvaluator] call_id={} {} missing in system_settings.extra, fallback action_code={}",
                call_id,
                field_name,
                fallback.action_code
            );
            return Ok(fallback);
        };

        let raw_config = raw_action
            .get("actionConfig")
            .cloned()
            .unwrap_or_else(|| raw_action.clone());
        match serde_json::from_value::<ActionConfigDto>(raw_config) {
            Ok(dto) => Ok(dto.into()),
            Err(err) => {
                warn!(
                    "[RuleEvaluator] call_id={} failed to parse {}: {}, fallback action_code={}",
                    call_id, field_name, err, fallback.action_code
                );
                Ok(fallback)
            }
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

fn is_anonymous(caller_id: &str) -> bool {
    let trimmed = caller_id.trim();
    trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("anonymous")
        || trimmed.eq_ignore_ascii_case("withheld")
}

fn to_action_config_from_registered(row: RegisteredNumberRow) -> ActionConfig {
    ActionConfig {
        action_code: row.action_code,
        caller_category: "registered".to_string(),
        ivr_flow_id: row.ivr_flow_id,
        recording_enabled: row.recording_enabled,
        announce_enabled: row.announce_enabled,
        recording_announcement_id: None,
        announcement_id: row.announcement_id,
        announcement_audio_file_url: None,
        scenario_id: None,
        include_announcement: None,
    }
}

fn to_action_config_from_routing_rule(row: RoutingRuleRow) -> ActionConfig {
    ActionConfig {
        action_code: row.action_code,
        caller_category: "unknown".to_string(),
        ivr_flow_id: row.ivr_flow_id,
        recording_enabled: true,
        announce_enabled: true,
        recording_announcement_id: None,
        announcement_id: row.announcement_id,
        announcement_audio_file_url: None,
        scenario_id: None,
        include_announcement: None,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{ActionConfig, ActionConfigDto, RuleEvaluator};
    use crate::shared::ports::routing_port::{
        CallActionRuleRow, IvrDestinationRow, IvrMenuRow, NoopRoutingPort, RegisteredNumberRow,
        RoutingFuture, RoutingPort, RoutingRuleRow,
    };
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn action_config_dto_parses_announcement_id() {
        let announcement_id = Uuid::now_v7();
        let dto: ActionConfigDto = serde_json::from_value(json!({
            "actionCode": "AN",
            "announcementId": announcement_id
        }))
        .expect("dto should parse");
        let config: ActionConfig = dto.into();
        assert_eq!(config.action_code, "AN");
        assert_eq!(config.announcement_id, Some(announcement_id));
    }

    #[test]
    fn action_config_dto_parses_recording_announcement_id() {
        let recording_announcement_id = Uuid::now_v7();
        let dto: ActionConfigDto = serde_json::from_value(json!({
            "actionCode": "VR",
            "announceEnabled": true,
            "recordingAnnouncementId": recording_announcement_id
        }))
        .expect("dto should parse");
        let config: ActionConfig = dto.into();
        assert_eq!(config.action_code, "VR");
        assert_eq!(
            config.recording_announcement_id,
            Some(recording_announcement_id)
        );
    }

    #[test]
    fn action_config_dto_defaults_without_announcement_id() {
        let dto: ActionConfigDto = serde_json::from_value(json!({
            "actionCode": "NR"
        }))
        .expect("dto should parse");
        let config: ActionConfig = dto.into();
        assert_eq!(config.action_code, "NR");
        assert_eq!(config.recording_announcement_id, None);
        assert_eq!(config.announcement_id, None);
    }

    #[test]
    fn action_config_dto_maps_welcome_announcement_id() {
        let welcome_announcement_id = Uuid::now_v7();
        let dto: ActionConfigDto = serde_json::from_value(json!({
            "actionCode": "VB",
            "welcomeAnnouncementId": welcome_announcement_id
        }))
        .expect("dto should parse");
        let config: ActionConfig = dto.into();
        assert_eq!(config.action_code, "VB");
        assert_eq!(config.announcement_id, Some(welcome_announcement_id));
    }

    #[test]
    fn action_config_dto_parses_include_announcement() {
        let dto: ActionConfigDto = serde_json::from_value(json!({
            "actionCode": "VR",
            "includeAnnouncement": true
        }))
        .expect("dto should parse");
        let config: ActionConfig = dto.into();
        assert_eq!(config.action_code, "VR");
        assert_eq!(config.include_announcement, Some(true));
    }

    #[tokio::test]
    async fn evaluate_returns_default_action_when_normalization_fails() {
        let evaluator = RuleEvaluator::new(Arc::new(NoopRoutingPort::new()));
        let action = evaluator
            .evaluate("abc", "call-123")
            .await
            .expect("normalization failure should fallback to default action");

        assert_eq!(action.action_code, "VR");
        assert_eq!(action.caller_category, "unknown");
    }

    struct UnknownRoutingRulePort {
        default_action_announcement_id: Uuid,
        noop: NoopRoutingPort,
    }

    impl UnknownRoutingRulePort {
        fn new(default_action_announcement_id: Uuid) -> Self {
            Self {
                default_action_announcement_id,
                noop: NoopRoutingPort::new(),
            }
        }
    }

    impl RoutingPort for UnknownRoutingRulePort {
        fn find_registered_number(
            &self,
            phone_number: &str,
        ) -> RoutingFuture<Option<RegisteredNumberRow>> {
            self.noop.find_registered_number(phone_number)
        }

        fn find_caller_group(&self, phone_number: &str) -> RoutingFuture<Option<Uuid>> {
            self.noop.find_caller_group(phone_number)
        }

        fn find_call_action_rule(
            &self,
            group_id: Uuid,
        ) -> RoutingFuture<Option<CallActionRuleRow>> {
            self.noop.find_call_action_rule(group_id)
        }

        fn is_spam(&self, phone_number: &str) -> RoutingFuture<bool> {
            self.noop.is_spam(phone_number)
        }

        fn is_registered(&self, phone_number: &str) -> RoutingFuture<bool> {
            self.noop.is_registered(phone_number)
        }

        fn find_routing_rule(&self, category: &str) -> RoutingFuture<Option<RoutingRuleRow>> {
            let row = if category == "unknown" {
                Some(RoutingRuleRow {
                    id: Uuid::now_v7(),
                    action_code: "IV".to_string(),
                    ivr_flow_id: None,
                    announcement_id: None,
                })
            } else {
                None
            };
            Box::pin(async move { Ok(row) })
        }

        fn get_system_settings_extra(&self) -> RoutingFuture<Option<serde_json::Value>> {
            let announcement_id = self.default_action_announcement_id;
            let value = json!({
                "defaultAction": {
                    "actionType": "deny",
                    "actionConfig": {
                        "actionCode": "AN",
                        "announcementId": announcement_id,
                    }
                }
            });
            Box::pin(async move { Ok(Some(value)) })
        }

        fn find_announcement_audio_file_url(
            &self,
            announcement_id: Uuid,
        ) -> RoutingFuture<Option<String>> {
            self.noop.find_announcement_audio_file_url(announcement_id)
        }

        fn find_ivr_menu(&self, flow_id: Uuid) -> RoutingFuture<Option<IvrMenuRow>> {
            self.noop.find_ivr_menu(flow_id)
        }

        fn find_ivr_dtmf_destination(
            &self,
            keypad_node_id: Uuid,
            dtmf_key: &str,
        ) -> RoutingFuture<Option<IvrDestinationRow>> {
            self.noop
                .find_ivr_dtmf_destination(keypad_node_id, dtmf_key)
        }

        fn find_ivr_dtmf_destination_by_flow(
            &self,
            flow_id: Uuid,
            dtmf_key: &str,
        ) -> RoutingFuture<Option<IvrDestinationRow>> {
            self.noop
                .find_ivr_dtmf_destination_by_flow(flow_id, dtmf_key)
        }

        fn find_ivr_timeout_destination(
            &self,
            keypad_node_id: Uuid,
        ) -> RoutingFuture<Option<IvrDestinationRow>> {
            self.noop.find_ivr_timeout_destination(keypad_node_id)
        }

        fn find_ivr_timeout_destination_by_flow(
            &self,
            flow_id: Uuid,
        ) -> RoutingFuture<Option<IvrDestinationRow>> {
            self.noop.find_ivr_timeout_destination_by_flow(flow_id)
        }

        fn find_ivr_invalid_destination(
            &self,
            keypad_node_id: Uuid,
        ) -> RoutingFuture<Option<IvrDestinationRow>> {
            self.noop.find_ivr_invalid_destination(keypad_node_id)
        }

        fn find_ivr_invalid_destination_by_flow(
            &self,
            flow_id: Uuid,
        ) -> RoutingFuture<Option<IvrDestinationRow>> {
            self.noop.find_ivr_invalid_destination_by_flow(flow_id)
        }
    }

    struct GroupPriorityRoutingPort {
        group_id: Uuid,
        noop: NoopRoutingPort,
    }

    impl GroupPriorityRoutingPort {
        fn new(group_id: Uuid) -> Self {
            Self {
                group_id,
                noop: NoopRoutingPort::new(),
            }
        }
    }

    impl RoutingPort for GroupPriorityRoutingPort {
        fn find_registered_number(
            &self,
            _phone_number: &str,
        ) -> RoutingFuture<Option<RegisteredNumberRow>> {
            let row = RegisteredNumberRow {
                action_code: "VR".to_string(),
                ivr_flow_id: None,
                recording_enabled: true,
                announce_enabled: true,
                announcement_id: None,
                group_id: Some(self.group_id),
            };
            Box::pin(async move { Ok(Some(row)) })
        }

        fn find_caller_group(&self, _phone_number: &str) -> RoutingFuture<Option<Uuid>> {
            let group_id = self.group_id;
            Box::pin(async move { Ok(Some(group_id)) })
        }

        fn find_call_action_rule(
            &self,
            _group_id: Uuid,
        ) -> RoutingFuture<Option<CallActionRuleRow>> {
            let row = CallActionRuleRow {
                id: Uuid::now_v7(),
                action_config: json!({
                    "actionCode": "BZ"
                }),
            };
            Box::pin(async move { Ok(Some(row)) })
        }

        fn is_spam(&self, phone_number: &str) -> RoutingFuture<bool> {
            self.noop.is_spam(phone_number)
        }

        fn is_registered(&self, phone_number: &str) -> RoutingFuture<bool> {
            self.noop.is_registered(phone_number)
        }

        fn find_routing_rule(&self, category: &str) -> RoutingFuture<Option<RoutingRuleRow>> {
            self.noop.find_routing_rule(category)
        }

        fn get_system_settings_extra(&self) -> RoutingFuture<Option<serde_json::Value>> {
            self.noop.get_system_settings_extra()
        }

        fn find_announcement_audio_file_url(
            &self,
            announcement_id: Uuid,
        ) -> RoutingFuture<Option<String>> {
            self.noop.find_announcement_audio_file_url(announcement_id)
        }

        fn find_ivr_menu(&self, flow_id: Uuid) -> RoutingFuture<Option<IvrMenuRow>> {
            self.noop.find_ivr_menu(flow_id)
        }

        fn find_ivr_dtmf_destination(
            &self,
            keypad_node_id: Uuid,
            dtmf_key: &str,
        ) -> RoutingFuture<Option<IvrDestinationRow>> {
            self.noop
                .find_ivr_dtmf_destination(keypad_node_id, dtmf_key)
        }

        fn find_ivr_dtmf_destination_by_flow(
            &self,
            flow_id: Uuid,
            dtmf_key: &str,
        ) -> RoutingFuture<Option<IvrDestinationRow>> {
            self.noop
                .find_ivr_dtmf_destination_by_flow(flow_id, dtmf_key)
        }

        fn find_ivr_timeout_destination(
            &self,
            keypad_node_id: Uuid,
        ) -> RoutingFuture<Option<IvrDestinationRow>> {
            self.noop.find_ivr_timeout_destination(keypad_node_id)
        }

        fn find_ivr_timeout_destination_by_flow(
            &self,
            flow_id: Uuid,
        ) -> RoutingFuture<Option<IvrDestinationRow>> {
            self.noop.find_ivr_timeout_destination_by_flow(flow_id)
        }

        fn find_ivr_invalid_destination(
            &self,
            keypad_node_id: Uuid,
        ) -> RoutingFuture<Option<IvrDestinationRow>> {
            self.noop.find_ivr_invalid_destination(keypad_node_id)
        }

        fn find_ivr_invalid_destination_by_flow(
            &self,
            flow_id: Uuid,
        ) -> RoutingFuture<Option<IvrDestinationRow>> {
            self.noop.find_ivr_invalid_destination_by_flow(flow_id)
        }
    }

    struct GroupWithoutActiveRulePort {
        group_id: Uuid,
        default_action_announcement_id: Uuid,
        noop: NoopRoutingPort,
    }

    impl GroupWithoutActiveRulePort {
        fn new(group_id: Uuid, default_action_announcement_id: Uuid) -> Self {
            Self {
                group_id,
                default_action_announcement_id,
                noop: NoopRoutingPort::new(),
            }
        }
    }

    impl RoutingPort for GroupWithoutActiveRulePort {
        fn find_registered_number(
            &self,
            _phone_number: &str,
        ) -> RoutingFuture<Option<RegisteredNumberRow>> {
            let row = RegisteredNumberRow {
                action_code: "VR".to_string(),
                ivr_flow_id: None,
                recording_enabled: true,
                announce_enabled: true,
                announcement_id: None,
                group_id: Some(self.group_id),
            };
            Box::pin(async move { Ok(Some(row)) })
        }

        fn find_caller_group(&self, _phone_number: &str) -> RoutingFuture<Option<Uuid>> {
            let group_id = self.group_id;
            Box::pin(async move { Ok(Some(group_id)) })
        }

        fn find_call_action_rule(
            &self,
            _group_id: Uuid,
        ) -> RoutingFuture<Option<CallActionRuleRow>> {
            Box::pin(async move { Ok(None) })
        }

        fn is_spam(&self, phone_number: &str) -> RoutingFuture<bool> {
            self.noop.is_spam(phone_number)
        }

        fn is_registered(&self, phone_number: &str) -> RoutingFuture<bool> {
            self.noop.is_registered(phone_number)
        }

        fn find_routing_rule(&self, category: &str) -> RoutingFuture<Option<RoutingRuleRow>> {
            let row = if category == "registered" {
                Some(RoutingRuleRow {
                    id: Uuid::now_v7(),
                    action_code: "VR".to_string(),
                    ivr_flow_id: None,
                    announcement_id: None,
                })
            } else {
                None
            };
            Box::pin(async move { Ok(row) })
        }

        fn get_system_settings_extra(&self) -> RoutingFuture<Option<serde_json::Value>> {
            let announcement_id = self.default_action_announcement_id;
            let value = json!({
                "defaultAction": {
                    "actionType": "deny",
                    "actionConfig": {
                        "actionCode": "AN",
                        "announcementId": announcement_id,
                    }
                }
            });
            Box::pin(async move { Ok(Some(value)) })
        }

        fn find_announcement_audio_file_url(
            &self,
            announcement_id: Uuid,
        ) -> RoutingFuture<Option<String>> {
            self.noop.find_announcement_audio_file_url(announcement_id)
        }

        fn find_ivr_menu(&self, flow_id: Uuid) -> RoutingFuture<Option<IvrMenuRow>> {
            self.noop.find_ivr_menu(flow_id)
        }

        fn find_ivr_dtmf_destination(
            &self,
            keypad_node_id: Uuid,
            dtmf_key: &str,
        ) -> RoutingFuture<Option<IvrDestinationRow>> {
            self.noop
                .find_ivr_dtmf_destination(keypad_node_id, dtmf_key)
        }

        fn find_ivr_dtmf_destination_by_flow(
            &self,
            flow_id: Uuid,
            dtmf_key: &str,
        ) -> RoutingFuture<Option<IvrDestinationRow>> {
            self.noop
                .find_ivr_dtmf_destination_by_flow(flow_id, dtmf_key)
        }

        fn find_ivr_timeout_destination(
            &self,
            keypad_node_id: Uuid,
        ) -> RoutingFuture<Option<IvrDestinationRow>> {
            self.noop.find_ivr_timeout_destination(keypad_node_id)
        }

        fn find_ivr_timeout_destination_by_flow(
            &self,
            flow_id: Uuid,
        ) -> RoutingFuture<Option<IvrDestinationRow>> {
            self.noop.find_ivr_timeout_destination_by_flow(flow_id)
        }

        fn find_ivr_invalid_destination(
            &self,
            keypad_node_id: Uuid,
        ) -> RoutingFuture<Option<IvrDestinationRow>> {
            self.noop.find_ivr_invalid_destination(keypad_node_id)
        }

        fn find_ivr_invalid_destination_by_flow(
            &self,
            flow_id: Uuid,
        ) -> RoutingFuture<Option<IvrDestinationRow>> {
            self.noop.find_ivr_invalid_destination_by_flow(flow_id)
        }
    }

    #[tokio::test]
    async fn evaluate_prefers_default_action_over_unknown_routing_rule() {
        let announcement_id = Uuid::now_v7();
        let evaluator = RuleEvaluator::new(Arc::new(UnknownRoutingRulePort::new(announcement_id)));

        let action = evaluator
            .evaluate("+81568686236", "call-unknown")
            .await
            .expect("unknown should use defaultAction");

        assert_eq!(action.action_code, "AN");
        assert_eq!(action.caller_category, "unknown");
        assert_eq!(action.announcement_id, Some(announcement_id));
    }

    #[tokio::test]
    async fn evaluate_prefers_caller_group_rule_when_registered_number_has_group() {
        let group_id = Uuid::now_v7();
        let evaluator = RuleEvaluator::new(Arc::new(GroupPriorityRoutingPort::new(group_id)));

        let action = evaluator
            .evaluate("+819012345678", "call-group-priority")
            .await
            .expect("group rule should be evaluated before registered number action");

        assert_eq!(action.action_code, "BZ");
        assert_eq!(action.caller_category, "registered");
    }

    #[tokio::test]
    async fn evaluate_falls_back_to_default_action_when_group_has_no_active_rule() {
        let group_id = Uuid::now_v7();
        let announcement_id = Uuid::now_v7();
        let evaluator = RuleEvaluator::new(Arc::new(GroupWithoutActiveRulePort::new(
            group_id,
            announcement_id,
        )));

        let action = evaluator
            .evaluate("+819012345678", "call-group-disabled")
            .await
            .expect("missing active group rule should fallback to defaultAction");

        assert_eq!(action.action_code, "AN");
        assert_eq!(action.caller_category, "unknown");
        assert_eq!(action.announcement_id, Some(announcement_id));
    }
}
