use std::collections::HashMap;
use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::shared::ports::announcement_port::{
    Announcement, AnnouncementError, AnnouncementFuture, AnnouncementPort, UpsertAnnouncement,
};
use crate::shared::ports::folder_port::{
    Folder, FolderError, FolderFuture, FolderPort, UpsertFolder,
};
use crate::shared::ports::phone_lookup::{
    CallerCategory, PhoneLookupError, PhoneLookupFuture, PhoneLookupPort, PhoneLookupResult,
};
use crate::shared::ports::registered_number_port::{
    RegisteredNumber, RegisteredNumberError, RegisteredNumberFuture, RegisteredNumberPort,
    UpsertRegisteredNumber,
};
use crate::shared::ports::routing_rule_port::{
    RoutingRule, RoutingRuleError, RoutingRuleFuture, RoutingRulePort, UpsertRoutingRule,
};
use crate::shared::ports::schedule_port::{
    Schedule, ScheduleError, ScheduleFuture, SchedulePort, ScheduleTimeSlot, UpsertSchedule,
};
use crate::shared::ports::settings_port::{
    SettingsError, SettingsFuture, SettingsPort, SystemSettings,
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
            let ivr_flow_id: Option<Uuid> = row.try_get("ivr_flow_id").map_err(map_lookup_err)?;
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

    async fn list_folders_by_entity_type_inner(
        pool: &PgPool,
        entity_type: &str,
    ) -> Result<Vec<Folder>, FolderError> {
        let rows = sqlx::query(
            "SELECT id, parent_id, entity_type, name, description, sort_order, created_at, updated_at
             FROM folders
             WHERE entity_type = $1
             ORDER BY sort_order ASC, name ASC",
        )
        .bind(entity_type)
        .fetch_all(pool)
        .await
        .map_err(map_folder_read_err)?;

        let mut folders = Vec::with_capacity(rows.len());
        for row in rows {
            folders.push(Folder {
                id: row.try_get("id").map_err(map_folder_read_err)?,
                parent_id: row.try_get("parent_id").map_err(map_folder_read_err)?,
                entity_type: row.try_get("entity_type").map_err(map_folder_read_err)?,
                name: row.try_get("name").map_err(map_folder_read_err)?,
                description: row.try_get("description").map_err(map_folder_read_err)?,
                sort_order: row.try_get("sort_order").map_err(map_folder_read_err)?,
                created_at: row.try_get("created_at").map_err(map_folder_read_err)?,
                updated_at: row.try_get("updated_at").map_err(map_folder_read_err)?,
            });
        }

        Ok(folders)
    }

    async fn upsert_folder_inner(pool: &PgPool, folder: UpsertFolder) -> Result<(), FolderError> {
        sqlx::query(
            "INSERT INTO folders (id, parent_id, entity_type, name, description, sort_order)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (id) DO UPDATE SET
                 parent_id = EXCLUDED.parent_id,
                 entity_type = EXCLUDED.entity_type,
                 name = EXCLUDED.name,
                 description = EXCLUDED.description,
                 sort_order = EXCLUDED.sort_order,
                 updated_at = NOW()",
        )
        .bind(folder.id)
        .bind(folder.parent_id)
        .bind(folder.entity_type)
        .bind(folder.name)
        .bind(folder.description)
        .bind(folder.sort_order)
        .execute(pool)
        .await
        .map_err(map_folder_write_err)?;

        Ok(())
    }

    async fn list_active_schedules_inner(pool: &PgPool) -> Result<Vec<Schedule>, ScheduleError> {
        let rows = sqlx::query(
            "SELECT id, name, description, schedule_type, is_active, folder_id,
                    date_range_start, date_range_end,
                    action_type, action_target, action_code,
                    version, created_at, updated_at
             FROM schedules
             WHERE is_active = TRUE
             ORDER BY name ASC",
        )
        .fetch_all(pool)
        .await
        .map_err(map_schedule_read_err)?;

        let mut schedules = Vec::with_capacity(rows.len());
        let mut schedule_ids = Vec::with_capacity(rows.len());

        for row in rows {
            let id: Uuid = row.try_get("id").map_err(map_schedule_read_err)?;
            schedule_ids.push(id);

            schedules.push(Schedule {
                id,
                name: row.try_get("name").map_err(map_schedule_read_err)?,
                description: row.try_get("description").map_err(map_schedule_read_err)?,
                schedule_type: row
                    .try_get("schedule_type")
                    .map_err(map_schedule_read_err)?,
                is_active: row.try_get("is_active").map_err(map_schedule_read_err)?,
                folder_id: row.try_get("folder_id").map_err(map_schedule_read_err)?,
                date_range_start: row
                    .try_get("date_range_start")
                    .map_err(map_schedule_read_err)?,
                date_range_end: row
                    .try_get("date_range_end")
                    .map_err(map_schedule_read_err)?,
                action_type: row.try_get("action_type").map_err(map_schedule_read_err)?,
                action_target: row
                    .try_get("action_target")
                    .map_err(map_schedule_read_err)?,
                action_code: row.try_get("action_code").map_err(map_schedule_read_err)?,
                version: row.try_get("version").map_err(map_schedule_read_err)?,
                created_at: row.try_get("created_at").map_err(map_schedule_read_err)?,
                updated_at: row.try_get("updated_at").map_err(map_schedule_read_err)?,
                time_slots: Vec::new(),
            });
        }

        if schedule_ids.is_empty() {
            return Ok(schedules);
        }

        let slot_rows = sqlx::query(
            "SELECT id, schedule_id, day_of_week, start_time, end_time, created_at
             FROM schedule_time_slots
             WHERE schedule_id = ANY($1)
             ORDER BY start_time ASC",
        )
        .bind(&schedule_ids)
        .fetch_all(pool)
        .await
        .map_err(map_schedule_read_err)?;

        let mut slots_by_schedule: HashMap<Uuid, Vec<ScheduleTimeSlot>> = HashMap::new();
        for row in slot_rows {
            let schedule_id: Uuid = row.try_get("schedule_id").map_err(map_schedule_read_err)?;
            let slot = ScheduleTimeSlot {
                id: row.try_get("id").map_err(map_schedule_read_err)?,
                schedule_id,
                day_of_week: row.try_get("day_of_week").map_err(map_schedule_read_err)?,
                start_time: row.try_get("start_time").map_err(map_schedule_read_err)?,
                end_time: row.try_get("end_time").map_err(map_schedule_read_err)?,
                created_at: row.try_get("created_at").map_err(map_schedule_read_err)?,
            };
            slots_by_schedule.entry(schedule_id).or_default().push(slot);
        }

        for schedule in &mut schedules {
            if let Some(slots) = slots_by_schedule.remove(&schedule.id) {
                schedule.time_slots = slots;
            }
        }

        Ok(schedules)
    }

    async fn upsert_schedule_inner(
        pool: &PgPool,
        schedule: UpsertSchedule,
    ) -> Result<(), ScheduleError> {
        let UpsertSchedule {
            id,
            name,
            description,
            schedule_type,
            is_active,
            folder_id,
            date_range_start,
            date_range_end,
            action_type,
            action_target,
            action_code,
            version,
            time_slots,
        } = schedule;

        let mut tx = pool.begin().await.map_err(map_schedule_write_err)?;

        let upsert_result = sqlx::query(
            "INSERT INTO schedules (
                 id, name, description, schedule_type, is_active, folder_id,
                 date_range_start, date_range_end,
                 action_type, action_target, action_code, version
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
             ON CONFLICT (id) DO UPDATE SET
                 name = EXCLUDED.name,
                 description = EXCLUDED.description,
                 schedule_type = EXCLUDED.schedule_type,
                 is_active = EXCLUDED.is_active,
                 folder_id = EXCLUDED.folder_id,
                 date_range_start = EXCLUDED.date_range_start,
                 date_range_end = EXCLUDED.date_range_end,
                 action_type = EXCLUDED.action_type,
                 action_target = EXCLUDED.action_target,
                 action_code = EXCLUDED.action_code,
                 version = EXCLUDED.version,
                 updated_at = NOW()
             WHERE schedules.version = EXCLUDED.version - 1",
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(schedule_type)
        .bind(is_active)
        .bind(folder_id)
        .bind(date_range_start)
        .bind(date_range_end)
        .bind(action_type)
        .bind(action_target)
        .bind(action_code)
        .bind(version)
        .execute(&mut *tx)
        .await
        .map_err(map_schedule_write_err)?;

        if upsert_result.rows_affected() == 0 {
            return Err(ScheduleError::WriteFailed(
                "optimistic lock conflict on schedules".to_string(),
            ));
        }

        sqlx::query("DELETE FROM schedule_time_slots WHERE schedule_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(map_schedule_write_err)?;

        for slot in time_slots {
            sqlx::query(
                "INSERT INTO schedule_time_slots (
                     id, schedule_id, day_of_week, start_time, end_time
                 )
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(slot.id)
            .bind(id)
            .bind(slot.day_of_week)
            .bind(slot.start_time)
            .bind(slot.end_time)
            .execute(&mut *tx)
            .await
            .map_err(map_schedule_write_err)?;
        }

        tx.commit().await.map_err(map_schedule_write_err)?;
        Ok(())
    }

    async fn list_active_announcements_inner(
        pool: &PgPool,
    ) -> Result<Vec<Announcement>, AnnouncementError> {
        let rows = sqlx::query(
            "SELECT id, name, description, announcement_type, is_active, folder_id,
                    audio_file_url, tts_text, duration_sec, language,
                    version, created_at, updated_at
             FROM announcements
             WHERE is_active = TRUE
             ORDER BY name ASC",
        )
        .fetch_all(pool)
        .await
        .map_err(map_announcement_read_err)?;

        let mut announcements = Vec::with_capacity(rows.len());
        for row in rows {
            announcements.push(Announcement {
                id: row.try_get("id").map_err(map_announcement_read_err)?,
                name: row.try_get("name").map_err(map_announcement_read_err)?,
                description: row
                    .try_get("description")
                    .map_err(map_announcement_read_err)?,
                announcement_type: row
                    .try_get("announcement_type")
                    .map_err(map_announcement_read_err)?,
                is_active: row
                    .try_get("is_active")
                    .map_err(map_announcement_read_err)?,
                folder_id: row
                    .try_get("folder_id")
                    .map_err(map_announcement_read_err)?,
                audio_file_url: row
                    .try_get("audio_file_url")
                    .map_err(map_announcement_read_err)?,
                tts_text: row.try_get("tts_text").map_err(map_announcement_read_err)?,
                duration_sec: row
                    .try_get("duration_sec")
                    .map_err(map_announcement_read_err)?,
                language: row.try_get("language").map_err(map_announcement_read_err)?,
                version: row.try_get("version").map_err(map_announcement_read_err)?,
                created_at: row
                    .try_get("created_at")
                    .map_err(map_announcement_read_err)?,
                updated_at: row
                    .try_get("updated_at")
                    .map_err(map_announcement_read_err)?,
            });
        }

        Ok(announcements)
    }

    async fn upsert_announcement_inner(
        pool: &PgPool,
        announcement: UpsertAnnouncement,
    ) -> Result<(), AnnouncementError> {
        let upsert_result = sqlx::query(
            "INSERT INTO announcements (
                 id, name, description, announcement_type, is_active, folder_id,
                 audio_file_url, tts_text, duration_sec, language, version
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
             ON CONFLICT (id) DO UPDATE SET
                 name = EXCLUDED.name,
                 description = EXCLUDED.description,
                 announcement_type = EXCLUDED.announcement_type,
                 is_active = EXCLUDED.is_active,
                 folder_id = EXCLUDED.folder_id,
                 audio_file_url = EXCLUDED.audio_file_url,
                 tts_text = EXCLUDED.tts_text,
                 duration_sec = EXCLUDED.duration_sec,
                 language = EXCLUDED.language,
                 version = EXCLUDED.version,
                 updated_at = NOW()
             WHERE announcements.version = EXCLUDED.version - 1",
        )
        .bind(announcement.id)
        .bind(announcement.name)
        .bind(announcement.description)
        .bind(announcement.announcement_type)
        .bind(announcement.is_active)
        .bind(announcement.folder_id)
        .bind(announcement.audio_file_url)
        .bind(announcement.tts_text)
        .bind(announcement.duration_sec)
        .bind(announcement.language)
        .bind(announcement.version)
        .execute(pool)
        .await
        .map_err(map_announcement_write_err)?;

        if upsert_result.rows_affected() == 0 {
            return Err(AnnouncementError::WriteFailed(
                "optimistic lock conflict on announcements".to_string(),
            ));
        }

        Ok(())
    }

    async fn get_system_settings_inner(pool: &PgPool) -> Result<SystemSettings, SettingsError> {
        let row = sqlx::query(
            "SELECT id, recording_retention_days, history_retention_days,
                    sync_endpoint_url, default_action_code, max_concurrent_calls,
                    extra, version, updated_at
             FROM system_settings
             WHERE id = 1",
        )
        .fetch_one(pool)
        .await
        .map_err(map_settings_read_err)?;

        Ok(SystemSettings {
            id: row.try_get("id").map_err(map_settings_read_err)?,
            recording_retention_days: row
                .try_get("recording_retention_days")
                .map_err(map_settings_read_err)?,
            history_retention_days: row
                .try_get("history_retention_days")
                .map_err(map_settings_read_err)?,
            sync_endpoint_url: row
                .try_get("sync_endpoint_url")
                .map_err(map_settings_read_err)?,
            default_action_code: row
                .try_get("default_action_code")
                .map_err(map_settings_read_err)?,
            max_concurrent_calls: row
                .try_get("max_concurrent_calls")
                .map_err(map_settings_read_err)?,
            extra: row.try_get("extra").map_err(map_settings_read_err)?,
            version: row.try_get("version").map_err(map_settings_read_err)?,
            updated_at: row.try_get("updated_at").map_err(map_settings_read_err)?,
        })
    }

    async fn update_system_settings_inner(
        pool: &PgPool,
        settings: SystemSettings,
    ) -> Result<(), SettingsError> {
        let upsert_result = sqlx::query(
            "INSERT INTO system_settings (
                 id, recording_retention_days, history_retention_days,
                 sync_endpoint_url, default_action_code, max_concurrent_calls,
                 extra, version
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (id) DO UPDATE SET
                 recording_retention_days = EXCLUDED.recording_retention_days,
                 history_retention_days = EXCLUDED.history_retention_days,
                 sync_endpoint_url = EXCLUDED.sync_endpoint_url,
                 default_action_code = EXCLUDED.default_action_code,
                 max_concurrent_calls = EXCLUDED.max_concurrent_calls,
                 extra = EXCLUDED.extra,
                 version = EXCLUDED.version,
                 updated_at = NOW()
             WHERE system_settings.version = EXCLUDED.version - 1",
        )
        .bind(settings.id)
        .bind(settings.recording_retention_days)
        .bind(settings.history_retention_days)
        .bind(settings.sync_endpoint_url)
        .bind(settings.default_action_code)
        .bind(settings.max_concurrent_calls)
        .bind(settings.extra)
        .bind(settings.version)
        .execute(pool)
        .await
        .map_err(map_settings_write_err)?;

        if upsert_result.rows_affected() == 0 {
            return Err(SettingsError::WriteFailed(
                "optimistic lock conflict on system_settings".to_string(),
            ));
        }

        Ok(())
    }

    async fn list_active_registered_numbers_inner(
        pool: &PgPool,
    ) -> Result<Vec<RegisteredNumber>, RegisteredNumberError> {
        let rows = sqlx::query(
            "SELECT id, phone_number, name, category, action_code, ivr_flow_id,
                    recording_enabled, announce_enabled, notes, folder_id,
                    version, deleted_at, created_at, updated_at
             FROM registered_numbers
             WHERE deleted_at IS NULL
             ORDER BY phone_number ASC",
        )
        .fetch_all(pool)
        .await
        .map_err(map_registered_number_read_err)?;

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            items.push(RegisteredNumber {
                id: row.try_get("id").map_err(map_registered_number_read_err)?,
                phone_number: row
                    .try_get("phone_number")
                    .map_err(map_registered_number_read_err)?,
                name: row
                    .try_get("name")
                    .map_err(map_registered_number_read_err)?,
                category: row
                    .try_get("category")
                    .map_err(map_registered_number_read_err)?,
                action_code: row
                    .try_get("action_code")
                    .map_err(map_registered_number_read_err)?,
                ivr_flow_id: row
                    .try_get("ivr_flow_id")
                    .map_err(map_registered_number_read_err)?,
                recording_enabled: row
                    .try_get("recording_enabled")
                    .map_err(map_registered_number_read_err)?,
                announce_enabled: row
                    .try_get("announce_enabled")
                    .map_err(map_registered_number_read_err)?,
                notes: row
                    .try_get("notes")
                    .map_err(map_registered_number_read_err)?,
                folder_id: row
                    .try_get("folder_id")
                    .map_err(map_registered_number_read_err)?,
                version: row
                    .try_get("version")
                    .map_err(map_registered_number_read_err)?,
                deleted_at: row
                    .try_get("deleted_at")
                    .map_err(map_registered_number_read_err)?,
                created_at: row
                    .try_get("created_at")
                    .map_err(map_registered_number_read_err)?,
                updated_at: row
                    .try_get("updated_at")
                    .map_err(map_registered_number_read_err)?,
            });
        }

        Ok(items)
    }

    async fn upsert_registered_number_inner(
        pool: &PgPool,
        input: UpsertRegisteredNumber,
    ) -> Result<(), RegisteredNumberError> {
        let upsert_result = sqlx::query(
            "INSERT INTO registered_numbers (
                 id, phone_number, name, category, action_code, ivr_flow_id,
                 recording_enabled, announce_enabled, notes, folder_id, version, deleted_at
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NULL)
             ON CONFLICT (id) DO UPDATE SET
                 phone_number = EXCLUDED.phone_number,
                 name = EXCLUDED.name,
                 category = EXCLUDED.category,
                 action_code = EXCLUDED.action_code,
                 ivr_flow_id = EXCLUDED.ivr_flow_id,
                 recording_enabled = EXCLUDED.recording_enabled,
                 announce_enabled = EXCLUDED.announce_enabled,
                 notes = EXCLUDED.notes,
                 folder_id = EXCLUDED.folder_id,
                 version = EXCLUDED.version,
                 deleted_at = NULL,
                 updated_at = NOW()
             WHERE registered_numbers.version = EXCLUDED.version - 1",
        )
        .bind(input.id)
        .bind(input.phone_number)
        .bind(input.name)
        .bind(input.category)
        .bind(input.action_code)
        .bind(input.ivr_flow_id)
        .bind(input.recording_enabled)
        .bind(input.announce_enabled)
        .bind(input.notes)
        .bind(input.folder_id)
        .bind(input.version)
        .execute(pool)
        .await
        .map_err(map_registered_number_write_err)?;

        if upsert_result.rows_affected() == 0 {
            return Err(RegisteredNumberError::WriteFailed(
                "optimistic lock conflict on registered_numbers".to_string(),
            ));
        }

        Ok(())
    }

    async fn list_active_routing_rules_inner(
        pool: &PgPool,
    ) -> Result<Vec<RoutingRule>, RoutingRuleError> {
        let rows = sqlx::query(
            "SELECT id, caller_category, action_code, ivr_flow_id, priority,
                    is_active, folder_id, version, created_at, updated_at
             FROM routing_rules
             ORDER BY priority ASC, created_at ASC",
        )
        .fetch_all(pool)
        .await
        .map_err(map_routing_rule_read_err)?;

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            items.push(RoutingRule {
                id: row.try_get("id").map_err(map_routing_rule_read_err)?,
                caller_category: row
                    .try_get("caller_category")
                    .map_err(map_routing_rule_read_err)?,
                action_code: row
                    .try_get("action_code")
                    .map_err(map_routing_rule_read_err)?,
                ivr_flow_id: row
                    .try_get("ivr_flow_id")
                    .map_err(map_routing_rule_read_err)?,
                priority: row.try_get("priority").map_err(map_routing_rule_read_err)?,
                is_active: row
                    .try_get("is_active")
                    .map_err(map_routing_rule_read_err)?,
                folder_id: row
                    .try_get("folder_id")
                    .map_err(map_routing_rule_read_err)?,
                version: row.try_get("version").map_err(map_routing_rule_read_err)?,
                created_at: row
                    .try_get("created_at")
                    .map_err(map_routing_rule_read_err)?,
                updated_at: row
                    .try_get("updated_at")
                    .map_err(map_routing_rule_read_err)?,
            });
        }

        Ok(items)
    }

    async fn upsert_routing_rule_inner(
        pool: &PgPool,
        input: UpsertRoutingRule,
    ) -> Result<(), RoutingRuleError> {
        let upsert_result = sqlx::query(
            "INSERT INTO routing_rules (
                 id, caller_category, action_code, ivr_flow_id, priority,
                 is_active, folder_id, version
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (id) DO UPDATE SET
                 caller_category = EXCLUDED.caller_category,
                 action_code = EXCLUDED.action_code,
                 ivr_flow_id = EXCLUDED.ivr_flow_id,
                 priority = EXCLUDED.priority,
                 is_active = EXCLUDED.is_active,
                 folder_id = EXCLUDED.folder_id,
                 version = EXCLUDED.version,
                 updated_at = NOW()
             WHERE routing_rules.version = EXCLUDED.version - 1",
        )
        .bind(input.id)
        .bind(input.caller_category)
        .bind(input.action_code)
        .bind(input.ivr_flow_id)
        .bind(input.priority)
        .bind(input.is_active)
        .bind(input.folder_id)
        .bind(input.version)
        .execute(pool)
        .await
        .map_err(map_routing_rule_write_err)?;

        if upsert_result.rows_affected() == 0 {
            return Err(RoutingRuleError::WriteFailed(
                "optimistic lock conflict on routing_rules".to_string(),
            ));
        }

        Ok(())
    }
}

impl PhoneLookupPort for PostgresAdapter {
    fn lookup_phone(&self, phone_number: String) -> PhoneLookupFuture {
        let pool = self.pool.clone();
        Box::pin(async move { PostgresAdapter::lookup_phone_inner(&pool, &phone_number).await })
    }
}

impl FolderPort for PostgresAdapter {
    fn list_by_entity_type(&self, entity_type: String) -> FolderFuture<Vec<Folder>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            PostgresAdapter::list_folders_by_entity_type_inner(&pool, &entity_type).await
        })
    }

    fn upsert_folder(&self, folder: UpsertFolder) -> FolderFuture<()> {
        let pool = self.pool.clone();
        Box::pin(async move { PostgresAdapter::upsert_folder_inner(&pool, folder).await })
    }
}

impl SchedulePort for PostgresAdapter {
    fn list_active(&self) -> ScheduleFuture<Vec<Schedule>> {
        let pool = self.pool.clone();
        Box::pin(async move { PostgresAdapter::list_active_schedules_inner(&pool).await })
    }

    fn upsert_schedule(&self, schedule: UpsertSchedule) -> ScheduleFuture<()> {
        let pool = self.pool.clone();
        Box::pin(async move { PostgresAdapter::upsert_schedule_inner(&pool, schedule).await })
    }
}

impl AnnouncementPort for PostgresAdapter {
    fn list_active(&self) -> AnnouncementFuture<Vec<Announcement>> {
        let pool = self.pool.clone();
        Box::pin(async move { PostgresAdapter::list_active_announcements_inner(&pool).await })
    }

    fn upsert_announcement(&self, announcement: UpsertAnnouncement) -> AnnouncementFuture<()> {
        let pool = self.pool.clone();
        Box::pin(
            async move { PostgresAdapter::upsert_announcement_inner(&pool, announcement).await },
        )
    }
}

impl SettingsPort for PostgresAdapter {
    fn get(&self) -> SettingsFuture<SystemSettings> {
        let pool = self.pool.clone();
        Box::pin(async move { PostgresAdapter::get_system_settings_inner(&pool).await })
    }

    fn update(&self, settings: SystemSettings) -> SettingsFuture<()> {
        let pool = self.pool.clone();
        Box::pin(
            async move { PostgresAdapter::update_system_settings_inner(&pool, settings).await },
        )
    }
}

impl RegisteredNumberPort for PostgresAdapter {
    fn list_active(&self) -> RegisteredNumberFuture<Vec<RegisteredNumber>> {
        let pool = self.pool.clone();
        Box::pin(async move { PostgresAdapter::list_active_registered_numbers_inner(&pool).await })
    }

    fn upsert_registered_number(
        &self,
        input: UpsertRegisteredNumber,
    ) -> RegisteredNumberFuture<()> {
        let pool = self.pool.clone();
        Box::pin(async move { PostgresAdapter::upsert_registered_number_inner(&pool, input).await })
    }
}

impl RoutingRulePort for PostgresAdapter {
    fn list_active(&self) -> RoutingRuleFuture<Vec<RoutingRule>> {
        let pool = self.pool.clone();
        Box::pin(async move { PostgresAdapter::list_active_routing_rules_inner(&pool).await })
    }

    fn upsert_routing_rule(&self, input: UpsertRoutingRule) -> RoutingRuleFuture<()> {
        let pool = self.pool.clone();
        Box::pin(async move { PostgresAdapter::upsert_routing_rule_inner(&pool, input).await })
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
) -> Result<(String, Option<Uuid>), PhoneLookupError> {
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
        let ivr_flow_id: Option<Uuid> = row.try_get("ivr_flow_id").map_err(map_lookup_err)?;
        return Ok((action_code, ivr_flow_id));
    }
    Ok((default_action.to_string(), None))
}

fn map_lookup_err(err: sqlx::Error) -> PhoneLookupError {
    PhoneLookupError::LookupFailed(err.to_string())
}

fn map_folder_read_err(err: sqlx::Error) -> FolderError {
    FolderError::ReadFailed(err.to_string())
}

fn map_folder_write_err(err: sqlx::Error) -> FolderError {
    FolderError::WriteFailed(err.to_string())
}

fn map_schedule_read_err(err: sqlx::Error) -> ScheduleError {
    ScheduleError::ReadFailed(err.to_string())
}

fn map_schedule_write_err(err: sqlx::Error) -> ScheduleError {
    ScheduleError::WriteFailed(err.to_string())
}

fn map_announcement_read_err(err: sqlx::Error) -> AnnouncementError {
    AnnouncementError::ReadFailed(err.to_string())
}

fn map_announcement_write_err(err: sqlx::Error) -> AnnouncementError {
    AnnouncementError::WriteFailed(err.to_string())
}

fn map_settings_read_err(err: sqlx::Error) -> SettingsError {
    SettingsError::ReadFailed(err.to_string())
}

fn map_settings_write_err(err: sqlx::Error) -> SettingsError {
    SettingsError::WriteFailed(err.to_string())
}

fn map_registered_number_read_err(err: sqlx::Error) -> RegisteredNumberError {
    RegisteredNumberError::ReadFailed(err.to_string())
}

fn map_registered_number_write_err(err: sqlx::Error) -> RegisteredNumberError {
    RegisteredNumberError::WriteFailed(err.to_string())
}

fn map_routing_rule_read_err(err: sqlx::Error) -> RoutingRuleError {
    RoutingRuleError::ReadFailed(err.to_string())
}

fn map_routing_rule_write_err(err: sqlx::Error) -> RoutingRuleError {
    RoutingRuleError::WriteFailed(err.to_string())
}
