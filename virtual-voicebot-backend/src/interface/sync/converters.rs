use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sqlx::{Postgres, Row, Transaction};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ConverterError {
    #[error("database failed: {0}")]
    DatabaseFailed(#[from] sqlx::Error),
    #[error("json failed: {0}")]
    JsonFailed(#[from] serde_json::Error),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CallerGroup {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub phone_numbers: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IncomingRule {
    pub id: Uuid,
    pub name: String,
    pub caller_group_id: Option<Uuid>,
    pub action_type: String,
    #[serde(default)]
    pub action_config: Value,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredAction {
    pub action_type: String,
    #[serde(default)]
    pub action_config: Value,
}

#[derive(Clone, Debug)]
pub struct CallActionsPayload {
    pub rules: Vec<IncomingRule>,
    pub anonymous_action: StoredAction,
    pub default_action: StoredAction,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IvrFlowDefinition {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub announcement_id: Option<Uuid>,
    #[serde(default = "default_timeout_sec")]
    pub timeout_sec: i32,
    #[serde(default = "default_max_retries")]
    pub max_retries: i32,
    #[serde(default)]
    pub routes: Vec<IvrRoute>,
    #[serde(default = "default_ivr_destination")]
    pub fallback_action: IvrActionDestination,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IvrRoute {
    pub dtmf_key: String,
    pub label: String,
    pub destination: IvrActionDestination,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IvrActionDestination {
    pub action_code: String,
    pub announcement_id: Option<Uuid>,
    pub ivr_flow_id: Option<Uuid>,
    pub scenario_id: Option<String>,
    pub welcome_announcement_id: Option<Uuid>,
    pub recording_enabled: Option<bool>,
    pub include_announcement: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendAnnouncement {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    #[serde(default = "default_announcement_type")]
    pub announcement_type: String,
    #[serde(default = "default_true")]
    pub is_active: bool,
    pub audio_file_url: Option<String>,
    pub tts_text: Option<String>,
    pub duration_sec: Option<f64>,
    pub language: Option<String>,
}

pub fn default_anonymous_action() -> StoredAction {
    StoredAction {
        action_type: "deny".to_string(),
        action_config: json!({ "actionCode": "BZ" }),
    }
}

pub fn default_default_action() -> StoredAction {
    StoredAction {
        action_type: "allow".to_string(),
        action_config: json!({ "actionCode": "VR" }),
    }
}

pub async fn apply_frontend_snapshot(
    tx: &mut Transaction<'_, Postgres>,
    groups: &[CallerGroup],
    actions: &CallActionsPayload,
    announcements: &[FrontendAnnouncement],
    flows: &[IvrFlowDefinition],
) -> Result<(), ConverterError> {
    convert_caller_groups(tx, groups).await?;
    convert_incoming_rules(tx, &actions.rules).await?;
    save_call_actions_settings(tx, actions).await?;
    convert_announcements(tx, announcements).await?;
    convert_ivr_flows(tx, flows).await?;
    Ok(())
}

async fn convert_caller_groups(
    tx: &mut Transaction<'_, Postgres>,
    groups: &[CallerGroup],
) -> Result<(), ConverterError> {
    let mut all_phone_numbers: Vec<String> = Vec::new();
    for group in groups {
        for phone_number in &group.phone_numbers {
            let normalized = normalize_phone_number(phone_number);
            if !normalized.is_empty() {
                all_phone_numbers.push(normalized);
            }
        }
    }

    let unique_phone_numbers: Vec<String> = all_phone_numbers
        .iter()
        .cloned()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    if !unique_phone_numbers.is_empty() {
        sqlx::query(
            "UPDATE registered_numbers
             SET deleted_at = NOW(),
                 group_id = NULL,
                 group_name = NULL,
                 updated_at = NOW()
             WHERE phone_number != ALL($1)
               AND deleted_at IS NULL",
        )
        .bind(&unique_phone_numbers)
        .execute(&mut **tx)
        .await?;
    } else {
        sqlx::query(
            "UPDATE registered_numbers
             SET deleted_at = NOW(),
                 group_id = NULL,
                 group_name = NULL,
                 updated_at = NOW()
             WHERE deleted_at IS NULL",
        )
        .execute(&mut **tx)
        .await?;
    }

    for group in groups {
        for phone_number in &group.phone_numbers {
            let normalized = normalize_phone_number(phone_number);
            if normalized.is_empty() {
                continue;
            }

            let update_active = sqlx::query(
                "UPDATE registered_numbers
                 SET group_id = $2,
                     group_name = $3,
                     deleted_at = NULL,
                     updated_at = NOW()
                 WHERE phone_number = $1
                   AND deleted_at IS NULL",
            )
            .bind(&normalized)
            .bind(group.id)
            .bind(group.name.clone())
            .execute(&mut **tx)
            .await?;
            if update_active.rows_affected() > 0 {
                continue;
            }

            let revive_deleted = sqlx::query(
                "UPDATE registered_numbers
                 SET group_id = $2,
                     group_name = $3,
                     deleted_at = NULL,
                     updated_at = NOW()
                 WHERE id = (
                     SELECT id
                     FROM registered_numbers
                     WHERE phone_number = $1
                       AND deleted_at IS NOT NULL
                     ORDER BY updated_at DESC
                     LIMIT 1
                 )",
            )
            .bind(&normalized)
            .bind(group.id)
            .bind(group.name.clone())
            .execute(&mut **tx)
            .await?;
            if revive_deleted.rows_affected() > 0 {
                continue;
            }

            sqlx::query(
                "INSERT INTO registered_numbers
                    (id, phone_number, group_id, group_name, category, action_code, recording_enabled, announce_enabled, created_at, updated_at)
                 VALUES
                    (gen_random_uuid(), $1, $2, $3, 'general', 'VR', TRUE, TRUE, NOW(), NOW())
                 ON CONFLICT (phone_number) WHERE deleted_at IS NULL
                 DO UPDATE SET
                    group_id = $2,
                    group_name = $3,
                    updated_at = NOW()",
            )
            .bind(normalized)
            .bind(group.id)
            .bind(group.name.clone())
            .execute(&mut **tx)
            .await?;
        }
    }

    Ok(())
}

async fn convert_incoming_rules(
    tx: &mut Transaction<'_, Postgres>,
    rules: &[IncomingRule],
) -> Result<(), ConverterError> {
    sqlx::query("DELETE FROM call_action_rules")
        .execute(&mut **tx)
        .await?;

    for (index, rule) in rules.iter().enumerate() {
        let action_type = normalize_action_type(&rule.action_type);
        sqlx::query(
            "INSERT INTO call_action_rules
                (id, name, caller_group_id, action_type, action_config, priority, is_active, created_at, updated_at)
             VALUES
                ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())",
        )
        .bind(rule.id)
        .bind(rule.name.trim())
        .bind(rule.caller_group_id)
        .bind(action_type)
        .bind(rule.action_config.clone())
        .bind(index as i32)
        .bind(rule.is_active)
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}

async fn save_call_actions_settings(
    tx: &mut Transaction<'_, Postgres>,
    actions: &CallActionsPayload,
) -> Result<(), ConverterError> {
    let existing_extra = sqlx::query("SELECT extra FROM system_settings WHERE id = 1")
        .fetch_optional(&mut **tx)
        .await?;

    let mut extra = match existing_extra {
        Some(row) => row
            .try_get::<Value, _>("extra")
            .unwrap_or_else(|_| json!({})),
        None => json!({}),
    };
    if !extra.is_object() {
        extra = json!({});
    }

    let anonymous_action = serde_json::to_value(&actions.anonymous_action)?;
    let default_action = serde_json::to_value(&actions.default_action)?;
    if let Some(map) = extra.as_object_mut() {
        map.insert("anonymousAction".to_string(), anonymous_action);
        map.insert("defaultAction".to_string(), default_action);
    }

    sqlx::query(
        "INSERT INTO system_settings (id, extra, updated_at)
         VALUES (1, $1, NOW())
         ON CONFLICT (id) DO UPDATE SET
            extra = $1,
            updated_at = NOW()",
    )
    .bind(extra)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn convert_announcements(
    tx: &mut Transaction<'_, Postgres>,
    announcements: &[FrontendAnnouncement],
) -> Result<(), ConverterError> {
    let frontend_ids: Vec<Uuid> = announcements
        .iter()
        .map(|announcement| announcement.id)
        .collect();
    if !frontend_ids.is_empty() {
        sqlx::query("DELETE FROM announcements WHERE NOT (id = ANY($1))")
            .bind(&frontend_ids)
            .execute(&mut **tx)
            .await?;
    } else {
        sqlx::query("DELETE FROM announcements")
            .execute(&mut **tx)
            .await?;
    }

    for announcement in announcements {
        let audio_file_url = normalize_optional_text(announcement.audio_file_url.as_deref());
        let tts_text = normalize_optional_text(announcement.tts_text.as_deref());
        if audio_file_url.is_none() && tts_text.is_none() {
            log::warn!(
                "[serversync] announcement has neither audio_file_url nor tts_text, skipping id={}",
                announcement.id
            );
            continue;
        }
        let duration_sec = announcement
            .duration_sec
            .filter(|value| value.is_finite() && *value >= 0.0)
            .map(|value| value.round() as i32);
        let language = announcement
            .language
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("ja");
        sqlx::query(
            "INSERT INTO announcements (
                id, name, description, announcement_type, is_active, folder_id,
                audio_file_url, tts_text, duration_sec, language, version, created_at, updated_at
             )
             VALUES ($1, $2, $3, $4, $5, NULL, $6, $7, $8, $9, 1, NOW(), NOW())
             ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                description = EXCLUDED.description,
                announcement_type = EXCLUDED.announcement_type,
                is_active = EXCLUDED.is_active,
                folder_id = NULL,
                audio_file_url = EXCLUDED.audio_file_url,
                tts_text = EXCLUDED.tts_text,
                duration_sec = EXCLUDED.duration_sec,
                language = EXCLUDED.language,
                updated_at = NOW()",
        )
        .bind(announcement.id)
        .bind(announcement.name.trim())
        .bind(announcement.description.as_deref().map(str::trim))
        .bind(normalize_announcement_type(&announcement.announcement_type))
        .bind(announcement.is_active)
        .bind(audio_file_url)
        .bind(tts_text)
        .bind(duration_sec)
        .bind(language)
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}

async fn convert_ivr_flows(
    tx: &mut Transaction<'_, Postgres>,
    flows: &[IvrFlowDefinition],
) -> Result<(), ConverterError> {
    let frontend_flow_ids: Vec<Uuid> = flows.iter().map(|flow| flow.id).collect();
    if !frontend_flow_ids.is_empty() {
        sqlx::query("DELETE FROM ivr_flows WHERE NOT (id = ANY($1))")
            .bind(&frontend_flow_ids)
            .execute(&mut **tx)
            .await?;
    } else {
        sqlx::query("DELETE FROM ivr_flows")
            .execute(&mut **tx)
            .await?;
    }

    for flow in flows {
        let timeout_sec = if flow.timeout_sec <= 0 {
            default_timeout_sec()
        } else {
            flow.timeout_sec
        };
        let max_retries = if flow.max_retries < 0 {
            default_max_retries()
        } else {
            flow.max_retries
        };

        sqlx::query(
            "INSERT INTO ivr_flows (id, name, description, is_active, created_at, updated_at)
             VALUES ($1, $2, $3, $4, NOW(), NOW())
             ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                description = EXCLUDED.description,
                is_active = EXCLUDED.is_active,
                updated_at = NOW()",
        )
        .bind(flow.id)
        .bind(flow.name.trim())
        .bind(flow.description.clone())
        .bind(flow.is_active)
        .execute(&mut **tx)
        .await?;

        sqlx::query("DELETE FROM ivr_nodes WHERE flow_id = $1")
            .bind(flow.id)
            .execute(&mut **tx)
            .await?;

        let root_node_id = Uuid::new_v4();
        let root_audio_file_url = match flow.announcement_id {
            Some(announcement_id) => resolve_announcement_url(tx, announcement_id).await?,
            None => None,
        };
        sqlx::query(
            "INSERT INTO ivr_nodes
                (id, flow_id, parent_id, node_type, audio_file_url, depth, timeout_sec, max_retries, created_at, updated_at)
             VALUES
                ($1, $2, NULL, 'ANNOUNCE', $3, 0, $4, $5, NOW(), NOW())",
        )
        .bind(root_node_id)
        .bind(flow.id)
        .bind(root_audio_file_url)
        .bind(timeout_sec)
        .bind(max_retries)
        .execute(&mut **tx)
        .await?;

        let keypad_node_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ivr_nodes
                (id, flow_id, parent_id, node_type, depth, timeout_sec, max_retries, created_at, updated_at)
             VALUES
                ($1, $2, $3, 'KEYPAD', 1, $4, $5, NOW(), NOW())",
        )
        .bind(keypad_node_id)
        .bind(flow.id)
        .bind(root_node_id)
        .bind(timeout_sec)
        .bind(max_retries)
        .execute(&mut **tx)
        .await?;

        for route in &flow.routes {
            if route.dtmf_key.trim().is_empty() {
                continue;
            }
            let destination_node_id = create_destination_node(
                tx,
                flow.id,
                keypad_node_id,
                &route.destination,
                2,
                timeout_sec,
                max_retries,
            )
            .await?;

            sqlx::query(
                "INSERT INTO ivr_transitions
                    (id, from_node_id, input_type, dtmf_key, to_node_id, created_at)
                 VALUES
                    ($1, $2, 'DTMF', $3, $4, NOW())",
            )
            .bind(Uuid::new_v4())
            .bind(keypad_node_id)
            .bind(route.dtmf_key.trim())
            .bind(destination_node_id)
            .execute(&mut **tx)
            .await?;
        }

        let fallback_node_id = create_destination_node(
            tx,
            flow.id,
            keypad_node_id,
            &flow.fallback_action,
            2,
            timeout_sec,
            max_retries,
        )
        .await?;
        for input_type in ["TIMEOUT", "INVALID"] {
            sqlx::query(
                "INSERT INTO ivr_transitions
                    (id, from_node_id, input_type, dtmf_key, to_node_id, created_at)
                 VALUES
                    ($1, $2, $3, NULL, $4, NOW())",
            )
            .bind(Uuid::new_v4())
            .bind(keypad_node_id)
            .bind(input_type)
            .bind(fallback_node_id)
            .execute(&mut **tx)
            .await?;
        }
    }

    Ok(())
}

async fn create_destination_node(
    tx: &mut Transaction<'_, Postgres>,
    flow_id: Uuid,
    parent_id: Uuid,
    destination: &IvrActionDestination,
    depth: i16,
    timeout_sec: i32,
    max_retries: i32,
) -> Result<Uuid, ConverterError> {
    let node_id = Uuid::new_v4();
    let action_code = normalize_action_code(&destination.action_code);
    let announcement_id = match action_code.as_str() {
        "AN" | "VM" => destination.announcement_id,
        "VB" => destination
            .welcome_announcement_id
            .or(destination.announcement_id),
        _ => None,
    };
    let audio_file_url = match announcement_id {
        Some(id) => resolve_announcement_url(tx, id).await?,
        None => None,
    };
    let metadata = build_destination_metadata(destination);

    sqlx::query(
        "INSERT INTO ivr_nodes
            (id, flow_id, parent_id, node_type, action_code, audio_file_url, tts_text, depth, timeout_sec, max_retries, created_at, updated_at)
         VALUES
            ($1, $2, $3, 'EXIT', $4, $5, $6, $7, $8, $9, NOW(), NOW())",
    )
    .bind(node_id)
    .bind(flow_id)
    .bind(parent_id)
    .bind(action_code)
    .bind(audio_file_url)
    .bind(metadata)
    .bind(depth)
    .bind(timeout_sec)
    .bind(max_retries)
    .execute(&mut **tx)
    .await?;

    Ok(node_id)
}

async fn resolve_announcement_url(
    tx: &mut Transaction<'_, Postgres>,
    announcement_id: Uuid,
) -> Result<Option<String>, ConverterError> {
    let row = sqlx::query("SELECT audio_file_url FROM announcements WHERE id = $1")
        .bind(announcement_id)
        .fetch_optional(&mut **tx)
        .await?;

    let Some(row) = row else {
        log::warn!(
            "[serversync] announcement not found while converting IVR flow: {}",
            announcement_id
        );
        return Ok(None);
    };

    let audio_file_url: Option<String> = row.try_get("audio_file_url")?;
    if audio_file_url.is_none() {
        log::warn!(
            "[serversync] announcement has null audio_file_url while converting IVR flow: {}",
            announcement_id
        );
    }
    Ok(audio_file_url)
}

fn build_destination_metadata(destination: &IvrActionDestination) -> Option<String> {
    let mut metadata = Map::new();
    if let Some(ivr_flow_id) = destination.ivr_flow_id {
        metadata.insert(
            "ivrFlowId".to_string(),
            Value::String(ivr_flow_id.to_string()),
        );
    }
    if let Some(scenario_id) = destination
        .scenario_id
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        metadata.insert(
            "scenarioId".to_string(),
            Value::String(scenario_id.to_string()),
        );
    }
    if let Some(value) = destination.recording_enabled {
        metadata.insert("recordingEnabled".to_string(), Value::Bool(value));
    }
    if let Some(value) = destination.include_announcement {
        metadata.insert("includeAnnouncement".to_string(), Value::Bool(value));
    }
    if metadata.is_empty() {
        None
    } else {
        Some(Value::Object(metadata).to_string())
    }
}

fn normalize_phone_number(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .filter(|ch| !matches!(ch, '-' | ' ' | '\t' | '\n' | '\r' | '(' | ')' | '（' | '）'))
        .collect();

    if cleaned.starts_with('+') {
        cleaned
    } else if let Some(domestic) = cleaned.strip_prefix('0') {
        format!("+81{}", domestic)
    } else {
        cleaned
    }
}

fn normalize_action_type(raw: &str) -> &'static str {
    if raw.eq_ignore_ascii_case("deny") {
        "deny"
    } else {
        "allow"
    }
}

fn normalize_announcement_type(raw: &str) -> &'static str {
    match raw.trim().to_ascii_lowercase().as_str() {
        "greeting" => "greeting",
        "hold" => "hold",
        "ivr" => "ivr",
        "closed" => "closed",
        "recording_notice" => "recording_notice",
        "custom" => "custom",
        _ => "custom",
    }
}

fn normalize_optional_text(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn normalize_action_code(raw: &str) -> String {
    raw.trim().to_ascii_uppercase()
}

fn default_true() -> bool {
    true
}

fn default_timeout_sec() -> i32 {
    10
}

fn default_max_retries() -> i32 {
    2
}

fn default_announcement_type() -> String {
    "custom".to_string()
}

fn default_ivr_destination() -> IvrActionDestination {
    IvrActionDestination {
        action_code: "VR".to_string(),
        announcement_id: None,
        ivr_flow_id: None,
        scenario_id: None,
        welcome_announcement_id: None,
        recording_enabled: None,
        include_announcement: None,
    }
}

#[cfg(test)]
mod tests {
    use super::{default_anonymous_action, default_default_action, normalize_phone_number};

    #[test]
    fn phone_number_normalization_removes_delimiters() {
        let normalized = normalize_phone_number(" +81 (90)-1234 5678 ");
        assert_eq!(normalized, "+819012345678");
    }

    #[test]
    fn phone_number_normalization_converts_domestic_to_e164() {
        let normalized = normalize_phone_number("080-1234-5678");
        assert_eq!(normalized, "+818012345678");
    }

    #[test]
    fn default_actions_match_contract() {
        let anonymous = default_anonymous_action();
        let default_action = default_default_action();
        assert_eq!(anonymous.action_type, "deny");
        assert_eq!(default_action.action_type, "allow");
        assert_eq!(anonymous.action_config["actionCode"], "BZ");
        assert_eq!(default_action.action_config["actionCode"], "VR");
    }
}
