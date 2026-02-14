use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::shared::ports::routing_port::{
    CallActionRuleRow, IvrDestinationRow, IvrMenuRow, RegisteredNumberRow, RoutingFuture,
    RoutingPort, RoutingPortError, RoutingRuleRow,
};

pub struct RoutingRepoImpl {
    pool: PgPool,
}

impl RoutingRepoImpl {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl RoutingPort for RoutingRepoImpl {
    fn find_registered_number(
        &self,
        phone_number: &str,
    ) -> RoutingFuture<Option<RegisteredNumberRow>> {
        let pool = self.pool.clone();
        let phone_number = phone_number.to_string();
        Box::pin(async move {
            let row = sqlx::query(
                "SELECT action_code, ivr_flow_id, recording_enabled, announce_enabled
                 FROM registered_numbers
                 WHERE phone_number = $1 AND deleted_at IS NULL
                 LIMIT 1",
            )
            .bind(phone_number)
            .fetch_optional(&pool)
            .await
            .map_err(map_read_err)?;

            let Some(row) = row else {
                return Ok(None);
            };

            Ok(Some(RegisteredNumberRow {
                action_code: row.try_get("action_code").map_err(map_read_err)?,
                ivr_flow_id: row.try_get("ivr_flow_id").map_err(map_read_err)?,
                recording_enabled: row.try_get("recording_enabled").map_err(map_read_err)?,
                announce_enabled: row.try_get("announce_enabled").map_err(map_read_err)?,
                announcement_id: None,
            }))
        })
    }

    fn find_caller_group(&self, phone_number: &str) -> RoutingFuture<Option<Uuid>> {
        let pool = self.pool.clone();
        let phone_number = phone_number.to_string();
        Box::pin(async move {
            let row = sqlx::query(
                "SELECT group_id
                 FROM registered_numbers
                 WHERE phone_number = $1
                   AND group_id IS NOT NULL
                   AND deleted_at IS NULL
                 LIMIT 1",
            )
            .bind(phone_number)
            .fetch_optional(&pool)
            .await
            .map_err(map_read_err)?;

            let Some(row) = row else {
                return Ok(None);
            };

            row.try_get("group_id").map_err(map_read_err).map(Some)
        })
    }

    fn find_call_action_rule(&self, group_id: Uuid) -> RoutingFuture<Option<CallActionRuleRow>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let row = sqlx::query(
                "SELECT id, action_config
                 FROM call_action_rules
                 WHERE caller_group_id = $1 AND is_active = TRUE
                 ORDER BY priority ASC
                 LIMIT 1",
            )
            .bind(group_id)
            .fetch_optional(&pool)
            .await
            .map_err(map_read_err)?;

            let Some(row) = row else {
                return Ok(None);
            };

            Ok(Some(CallActionRuleRow {
                id: row.try_get("id").map_err(map_read_err)?,
                action_config: row.try_get("action_config").map_err(map_read_err)?,
            }))
        })
    }

    fn is_spam(&self, phone_number: &str) -> RoutingFuture<bool> {
        let pool = self.pool.clone();
        let phone_number = phone_number.to_string();
        Box::pin(async move {
            sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(
                    SELECT 1
                    FROM spam_numbers
                    WHERE phone_number = $1 AND deleted_at IS NULL
                )",
            )
            .bind(phone_number)
            .fetch_one(&pool)
            .await
            .map_err(map_read_err)
        })
    }

    fn is_registered(&self, phone_number: &str) -> RoutingFuture<bool> {
        let pool = self.pool.clone();
        let phone_number = phone_number.to_string();
        Box::pin(async move {
            sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(
                    SELECT 1
                    FROM registered_numbers
                    WHERE phone_number = $1 AND deleted_at IS NULL
                )",
            )
            .bind(phone_number)
            .fetch_one(&pool)
            .await
            .map_err(map_read_err)
        })
    }

    fn find_routing_rule(&self, category: &str) -> RoutingFuture<Option<RoutingRuleRow>> {
        let pool = self.pool.clone();
        let category = category.to_string();
        Box::pin(async move {
            let row = sqlx::query(
                "SELECT id, action_code, ivr_flow_id
                 FROM routing_rules
                 WHERE caller_category = $1 AND is_active = TRUE
                 ORDER BY priority ASC
                 LIMIT 1",
            )
            .bind(category)
            .fetch_optional(&pool)
            .await
            .map_err(map_read_err)?;

            let Some(row) = row else {
                return Ok(None);
            };

            Ok(Some(RoutingRuleRow {
                id: row.try_get("id").map_err(map_read_err)?,
                action_code: row.try_get("action_code").map_err(map_read_err)?,
                ivr_flow_id: row.try_get("ivr_flow_id").map_err(map_read_err)?,
                announcement_id: None,
            }))
        })
    }

    fn get_system_settings_extra(&self) -> RoutingFuture<Option<Value>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let row = sqlx::query(
                "SELECT extra
                 FROM system_settings
                 WHERE id = 1
                 LIMIT 1",
            )
            .fetch_optional(&pool)
            .await
            .map_err(map_read_err)?;

            let Some(row) = row else {
                return Ok(None);
            };

            row.try_get("extra").map_err(map_read_err).map(Some)
        })
    }

    fn find_announcement_audio_file_url(
        &self,
        announcement_id: Uuid,
    ) -> RoutingFuture<Option<String>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let row = sqlx::query(
                "SELECT audio_file_url
                 FROM announcements
                 WHERE id = $1 AND is_active = TRUE
                 LIMIT 1",
            )
            .bind(announcement_id)
            .fetch_optional(&pool)
            .await
            .map_err(map_read_err)?;

            let Some(row) = row else {
                return Ok(None);
            };

            row.try_get("audio_file_url").map_err(map_read_err)
        })
    }

    fn find_ivr_menu(&self, flow_id: Uuid) -> RoutingFuture<Option<IvrMenuRow>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let row = sqlx::query(
                "SELECT root.id AS root_node_id,
                        root.audio_file_url AS root_audio_file_url,
                        keypad.id AS keypad_node_id,
                        keypad.timeout_sec AS keypad_timeout_sec,
                        keypad.max_retries AS keypad_max_retries
                 FROM ivr_flows flow
                 JOIN ivr_nodes root
                   ON root.flow_id = flow.id
                  AND root.parent_id IS NULL
                  AND root.node_type = 'ANNOUNCE'
                 JOIN ivr_nodes keypad
                   ON keypad.parent_id = root.id
                  AND keypad.node_type = 'KEYPAD'
                 WHERE flow.id = $1
                   AND flow.is_active = TRUE
                 ORDER BY root.created_at ASC, keypad.created_at ASC
                 LIMIT 1",
            )
            .bind(flow_id)
            .fetch_optional(&pool)
            .await
            .map_err(map_read_err)?;

            let Some(row) = row else {
                return Ok(None);
            };

            Ok(Some(IvrMenuRow {
                root_node_id: row.try_get("root_node_id").map_err(map_read_err)?,
                keypad_node_id: row.try_get("keypad_node_id").map_err(map_read_err)?,
                audio_file_url: row.try_get("root_audio_file_url").map_err(map_read_err)?,
                timeout_sec: row.try_get("keypad_timeout_sec").map_err(map_read_err)?,
                max_retries: row.try_get("keypad_max_retries").map_err(map_read_err)?,
            }))
        })
    }

    fn find_ivr_dtmf_destination(
        &self,
        keypad_node_id: Uuid,
        dtmf_key: &str,
    ) -> RoutingFuture<Option<IvrDestinationRow>> {
        let pool = self.pool.clone();
        let dtmf_key = dtmf_key.to_string();
        Box::pin(async move {
            let row = sqlx::query(
                "SELECT transition.id AS transition_id,
                        dest.id AS destination_node_id,
                        dest.action_code,
                        dest.audio_file_url,
                        dest.tts_text
                 FROM ivr_transitions transition
                 JOIN ivr_nodes dest
                   ON dest.id = transition.to_node_id
                 WHERE transition.from_node_id = $1
                   AND transition.input_type = 'DTMF'
                   AND transition.dtmf_key = $2
                 ORDER BY transition.created_at ASC
                 LIMIT 1",
            )
            .bind(keypad_node_id)
            .bind(dtmf_key)
            .fetch_optional(&pool)
            .await
            .map_err(map_read_err)?;

            map_destination_row(row)
        })
    }

    fn find_ivr_timeout_destination(
        &self,
        keypad_node_id: Uuid,
    ) -> RoutingFuture<Option<IvrDestinationRow>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let row = sqlx::query(
                "SELECT transition.id AS transition_id,
                        dest.id AS destination_node_id,
                        dest.action_code,
                        dest.audio_file_url,
                        dest.tts_text
                 FROM ivr_transitions transition
                 JOIN ivr_nodes dest
                   ON dest.id = transition.to_node_id
                 WHERE transition.from_node_id = $1
                   AND transition.input_type = 'TIMEOUT'
                 ORDER BY transition.created_at ASC
                 LIMIT 1",
            )
            .bind(keypad_node_id)
            .fetch_optional(&pool)
            .await
            .map_err(map_read_err)?;

            map_destination_row(row)
        })
    }

    fn find_ivr_invalid_destination(
        &self,
        keypad_node_id: Uuid,
    ) -> RoutingFuture<Option<IvrDestinationRow>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let row = sqlx::query(
                "SELECT transition.id AS transition_id,
                        dest.id AS destination_node_id,
                        dest.action_code,
                        dest.audio_file_url,
                        dest.tts_text
                 FROM ivr_transitions transition
                 JOIN ivr_nodes dest
                   ON dest.id = transition.to_node_id
                 WHERE transition.from_node_id = $1
                   AND transition.input_type = 'INVALID'
                 ORDER BY transition.created_at ASC
                 LIMIT 1",
            )
            .bind(keypad_node_id)
            .fetch_optional(&pool)
            .await
            .map_err(map_read_err)?;

            map_destination_row(row)
        })
    }
}

fn map_destination_row(
    row: Option<sqlx::postgres::PgRow>,
) -> Result<Option<IvrDestinationRow>, RoutingPortError> {
    let Some(row) = row else {
        return Ok(None);
    };
    Ok(Some(IvrDestinationRow {
        transition_id: row.try_get("transition_id").map_err(map_read_err)?,
        node_id: row.try_get("destination_node_id").map_err(map_read_err)?,
        action_code: row.try_get("action_code").map_err(map_read_err)?,
        audio_file_url: row.try_get("audio_file_url").map_err(map_read_err)?,
        metadata_json: row.try_get("tts_text").map_err(map_read_err)?,
    }))
}

fn map_read_err(err: sqlx::Error) -> RoutingPortError {
    RoutingPortError::ReadFailed(err.to_string())
}
