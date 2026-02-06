use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};

use crate::shared::ports::phone_lookup::{
    CallerCategory, PhoneLookupError, PhoneLookupFuture, PhoneLookupPort, PhoneLookupResult,
};

const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_CONNECTIONS: u32 = 5;

pub struct PostgresAdapter {
    pool: PgPool,
}

impl PostgresAdapter {
    pub async fn new(database_url: String) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(MAX_CONNECTIONS)
            .acquire_timeout(ACQUIRE_TIMEOUT)
            .connect(&database_url)
            .await?;
        Ok(Self { pool })
    }

    async fn lookup_phone_inner(
        pool: &PgPool,
        phone_number: &str,
    ) -> Result<Option<PhoneLookupResult>, PhoneLookupError> {
        if is_spam_number(pool, phone_number).await? {
            let (action_code, ivr_flow_id) = lookup_rule_action(pool, "spam", "RJ").await?;
            return Ok(Some(PhoneLookupResult {
                phone_number: phone_number.to_string(),
                caller_category: CallerCategory::Spam,
                action_code,
                ivr_flow_id,
                recording_enabled: false,
                announce_enabled: true,
            }));
        }

        if let Some(row) = sqlx::query(
            "SELECT action_code, ivr_flow_id, recording_enabled, announce_enabled
             FROM registered_numbers
             WHERE phone_number = $1 AND deleted_at IS NULL
             LIMIT 1",
        )
        .bind(phone_number)
        .fetch_optional(pool)
        .await
        .map_err(map_lookup_err)?
        {
            let action_code: String = row.try_get("action_code").map_err(map_lookup_err)?;
            let ivr_flow_id: Option<uuid::Uuid> =
                row.try_get("ivr_flow_id").map_err(map_lookup_err)?;
            let recording_enabled: bool =
                row.try_get("recording_enabled").map_err(map_lookup_err)?;
            let announce_enabled: bool = row.try_get("announce_enabled").map_err(map_lookup_err)?;
            return Ok(Some(PhoneLookupResult {
                phone_number: phone_number.to_string(),
                caller_category: CallerCategory::Registered,
                action_code,
                ivr_flow_id,
                recording_enabled,
                announce_enabled,
            }));
        }

        let (action_code, ivr_flow_id) = lookup_rule_action(pool, "unknown", "IV").await?;
        Ok(Some(PhoneLookupResult {
            phone_number: phone_number.to_string(),
            caller_category: CallerCategory::Unknown,
            action_code,
            ivr_flow_id,
            recording_enabled: true,
            announce_enabled: true,
        }))
    }
}

impl PhoneLookupPort for PostgresAdapter {
    fn lookup_phone(&self, phone_number: String) -> PhoneLookupFuture {
        let pool = self.pool.clone();
        Box::pin(async move { PostgresAdapter::lookup_phone_inner(&pool, &phone_number).await })
    }
}

async fn is_spam_number(pool: &PgPool, phone_number: &str) -> Result<bool, PhoneLookupError> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
            SELECT 1
            FROM spam_numbers
            WHERE phone_number = $1 AND deleted_at IS NULL
        )",
    )
    .bind(phone_number)
    .fetch_one(pool)
    .await
    .map_err(map_lookup_err)?;
    Ok(exists)
}

async fn lookup_rule_action(
    pool: &PgPool,
    category: &str,
    default_action: &str,
) -> Result<(String, Option<uuid::Uuid>), PhoneLookupError> {
    if let Some(row) = sqlx::query(
        "SELECT action_code, ivr_flow_id
         FROM routing_rules
         WHERE caller_category = $1 AND is_active = TRUE
         ORDER BY priority ASC
         LIMIT 1",
    )
    .bind(category)
    .fetch_optional(pool)
    .await
    .map_err(map_lookup_err)?
    {
        let action_code: String = row.try_get("action_code").map_err(map_lookup_err)?;
        let ivr_flow_id: Option<uuid::Uuid> = row.try_get("ivr_flow_id").map_err(map_lookup_err)?;
        return Ok((action_code, ivr_flow_id));
    }
    Ok((default_action.to_string(), None))
}

fn map_lookup_err(err: sqlx::Error) -> PhoneLookupError {
    PhoneLookupError::LookupFailed(err.to_string())
}
