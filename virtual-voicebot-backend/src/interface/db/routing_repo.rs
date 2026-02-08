use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::shared::ports::routing_port::{
    CallActionRuleRow, RegisteredNumberRow, RoutingFuture, RoutingPort, RoutingPortError,
    RoutingRuleRow,
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
}

fn map_read_err(err: sqlx::Error) -> RoutingPortError {
    RoutingPortError::ReadFailed(err.to_string())
}
