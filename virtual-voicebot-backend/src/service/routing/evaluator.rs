use std::sync::Arc;

use log::{info, warn};
use serde::Deserialize;
use thiserror::Error;
use uuid::Uuid;

use crate::shared::ports::routing_port::{
    RegisteredNumberRow, RoutingPort, RoutingPortError, RoutingRuleRow,
};

#[derive(Debug, Clone)]
pub struct ActionConfig {
    pub action_code: String,
    pub ivr_flow_id: Option<Uuid>,
    pub recording_enabled: bool,
    pub announce_enabled: bool,
    pub announcement_id: Option<Uuid>,
    pub announcement_audio_file_url: Option<String>,
    pub scenario_id: Option<String>,
    pub include_announcement: Option<bool>,
}

impl ActionConfig {
    pub fn default_vr() -> Self {
        Self {
            action_code: "VR".to_string(),
            ivr_flow_id: None,
            recording_enabled: true,
            announce_enabled: false,
            announcement_id: None,
            announcement_audio_file_url: None,
            scenario_id: None,
            include_announcement: None,
        }
    }

    pub fn default_bz() -> Self {
        Self {
            action_code: "BZ".to_string(),
            ivr_flow_id: None,
            recording_enabled: false,
            announce_enabled: false,
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
    announcement_id: Option<Uuid>,
    #[serde(default)]
    scenario_id: Option<String>,
    #[serde(default)]
    include_announcement: Option<bool>,
}

impl From<ActionConfigDto> for ActionConfig {
    fn from(dto: ActionConfigDto) -> Self {
        Self {
            action_code: dto.action_code,
            ivr_flow_id: dto.ivr_flow_id,
            recording_enabled: dto.recording_enabled,
            announce_enabled: dto.announce_enabled,
            announcement_id: dto.announcement_id,
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

        let normalized_caller_id = normalize_phone_number(caller_id)?;
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

        if let Some(action) = self
            .match_caller_group(&normalized_caller_id, call_id)
            .await?
        {
            info!(
                "[RuleEvaluator] call_id={} hit stage=2 source=call_action_rules",
                call_id
            );
            return Ok(action);
        }
        info!(
            "[RuleEvaluator] call_id={} miss stage=2 source=call_action_rules",
            call_id
        );

        let category = self.classify_caller(&normalized_caller_id, call_id).await?;
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
        let action = to_action_config_from_registered(row);
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
    ) -> Result<Option<ActionConfig>, RoutingError> {
        let group_id = self.routing_port.find_caller_group(caller_id).await?;
        let Some(group_id) = group_id else {
            return Ok(None);
        };

        let row = self.routing_port.find_call_action_rule(group_id).await?;
        let Some(row) = row else {
            return Ok(None);
        };

        let dto: ActionConfigDto = serde_json::from_value(row.action_config)?;
        let action: ActionConfig = dto.into();
        info!(
            "[RuleEvaluator] call_id={} stage=2 rule_id={} action_code={}",
            call_id, row.id, action.action_code
        );
        Ok(Some(action))
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
        let action = to_action_config_from_routing_rule(row);
        info!(
            "[RuleEvaluator] call_id={} stage=3 rule_id={} action_code={}",
            call_id, rule_id, action.action_code
        );
        Ok(Some(action))
    }

    async fn get_default_action(&self, call_id: &str) -> Result<ActionConfig, RoutingError> {
        self.get_action_from_settings_or_fallback(
            "defaultAction",
            ActionConfig::default_vr(),
            call_id,
        )
        .await
    }

    async fn get_anonymous_action(&self, call_id: &str) -> Result<ActionConfig, RoutingError> {
        self.get_action_from_settings_or_fallback(
            "anonymousAction",
            ActionConfig::default_bz(),
            call_id,
        )
        .await
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

fn normalize_phone_number(phone_number: &str) -> Result<String, RoutingError> {
    let cleaned: String = phone_number
        .chars()
        .filter(|ch| !matches!(ch, '-' | ' ' | '\t' | '\n' | '\r' | '(' | ')'))
        .collect();
    if !(cleaned.starts_with('+') && (8..=16).contains(&cleaned.len())) {
        return Err(RoutingError::InvalidPhoneNumber(phone_number.to_string()));
    }
    if !cleaned[1..].chars().all(|ch| ch.is_ascii_digit()) {
        return Err(RoutingError::InvalidPhoneNumber(phone_number.to_string()));
    }
    Ok(cleaned)
}

fn to_action_config_from_registered(row: RegisteredNumberRow) -> ActionConfig {
    ActionConfig {
        action_code: row.action_code,
        ivr_flow_id: row.ivr_flow_id,
        recording_enabled: row.recording_enabled,
        announce_enabled: row.announce_enabled,
        announcement_id: row.announcement_id,
        announcement_audio_file_url: None,
        scenario_id: None,
        include_announcement: None,
    }
}

fn to_action_config_from_routing_rule(row: RoutingRuleRow) -> ActionConfig {
    ActionConfig {
        action_code: row.action_code,
        ivr_flow_id: row.ivr_flow_id,
        recording_enabled: true,
        announce_enabled: true,
        announcement_id: row.announcement_id,
        announcement_audio_file_url: None,
        scenario_id: None,
        include_announcement: None,
    }
}

#[cfg(test)]
mod tests {
    use super::{ActionConfig, ActionConfigDto};
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
    fn action_config_dto_defaults_without_announcement_id() {
        let dto: ActionConfigDto = serde_json::from_value(json!({
            "actionCode": "NR"
        }))
        .expect("dto should parse");
        let config: ActionConfig = dto.into();
        assert_eq!(config.action_code, "NR");
        assert_eq!(config.announcement_id, None);
    }
}
