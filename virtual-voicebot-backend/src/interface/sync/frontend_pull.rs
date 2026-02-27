use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use serde::Deserialize;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Postgres, Row, Transaction};
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::time::{interval, MissedTickBehavior};
use uuid::Uuid;

use crate::interface::sync::converters::{
    apply_frontend_snapshot, default_anonymous_action, default_default_action, CallActionsPayload,
    CallerGroup, ConverterError, FrontendAnnouncement, IvrFlowDefinition, StoredAction,
};
use crate::shared::config::{self, SyncConfig};
use crate::shared::utils::extract_url_path;

const FRONTEND_PULL_MAX_CONNECTIONS: u32 = 5;
const MAX_ANNOUNCEMENT_AUDIO_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Clone)]
pub struct FrontendPullWorker {
    pool: PgPool,
    http_client: reqwest::Client,
    frontend_base_url: String,
    poll_interval_sec: u64,
}

#[derive(Debug, Error)]
pub enum FrontendPullError {
    #[error("database failed: {0}")]
    DatabaseFailed(#[from] sqlx::Error),
    #[error("http failed: {0}")]
    HttpFailed(#[from] reqwest::Error),
    #[error("frontend response invalid: {0}")]
    InvalidResponse(String),
    #[error("converters failed: {0}")]
    ConverterFailed(#[from] ConverterError),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NumberGroupsResponse {
    ok: bool,
    #[serde(default)]
    caller_groups: Vec<CallerGroup>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CallActionsResponse {
    ok: bool,
    #[serde(default)]
    rules: Vec<crate::interface::sync::converters::IncomingRule>,
    anonymous_action: Option<StoredAction>,
    default_action: Option<StoredAction>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IvrFlowsResponse {
    ok: bool,
    #[serde(default)]
    flows: Vec<IvrFlowDefinition>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnnouncementsResponse {
    ok: bool,
    #[serde(default)]
    announcements: Vec<FrontendAnnouncement>,
    error: Option<String>,
}

#[derive(Clone, Debug)]
struct ExistingAnnouncementAudioState {
    audio_file_url: Option<String>,
    updated_at: DateTime<Utc>,
}

impl FrontendPullWorker {
    pub async fn new(database_url: String, config: SyncConfig) -> Result<Self, FrontendPullError> {
        let timeout = Duration::from_secs(config.timeout_sec);
        let http_client = reqwest::Client::builder().timeout(timeout).build()?;
        let pool = PgPoolOptions::new()
            .max_connections(FRONTEND_PULL_MAX_CONNECTIONS)
            .connect(&database_url)
            .await?;
        Ok(Self {
            pool,
            http_client,
            frontend_base_url: config.frontend_base_url.trim_end_matches('/').to_string(),
            poll_interval_sec: config.frontend_poll_interval_sec,
        })
    }

    pub async fn run(&self) {
        log::info!(
            "[serversync] frontend pull worker started (poll_interval={}s)",
            self.poll_interval_sec
        );
        let mut ticker = interval(Duration::from_secs(self.poll_interval_sec));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            ticker.tick().await;
            if let Err(error) = self.process_once().await {
                log::warn!("[serversync] frontend pull failed: {}", error);
            }
        }
    }

    pub async fn process_once(&self) -> Result<(), FrontendPullError> {
        log::info!("[serversync] frontend pull started");
        let groups = self.fetch_number_groups().await?;
        log::info!(
            "[serversync] GET /api/number-groups: success groups={}",
            groups.len()
        );
        let actions = self.fetch_call_actions().await?;
        log::info!(
            "[serversync] GET /api/call-actions: success rules={}",
            actions.rules.len()
        );
        let announcements = self.fetch_announcements().await?;
        log::info!(
            "[serversync] GET /api/announcements: success announcements={}",
            announcements.len()
        );
        let flows = self.fetch_ivr_flows_export().await?;
        log::info!(
            "[serversync] GET /api/ivr-flows/export: success flows={}",
            flows.len()
        );
        self.save_snapshot(&groups, &actions, &announcements, &flows)
            .await?;
        if let Err(error) = self.sync_announcement_audio_cache(&announcements).await {
            log::warn!(
                "[serversync] announcement audio cache sync failed: {}",
                error
            );
        }
        log::info!("[serversync] frontend pull saved");
        Ok(())
    }

    async fn fetch_number_groups(&self) -> Result<Vec<CallerGroup>, FrontendPullError> {
        let url = format!("{}/api/number-groups", self.frontend_base_url);
        let response = self.http_client.get(url).send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(FrontendPullError::InvalidResponse(format!(
                "GET /api/number-groups returned status {}",
                status
            )));
        }
        let body: NumberGroupsResponse = response.json().await?;
        if !body.ok {
            return Err(FrontendPullError::InvalidResponse(format!(
                "GET /api/number-groups returned ok=false{}",
                body.error
                    .as_ref()
                    .map(|value| format!(" ({value})"))
                    .unwrap_or_default()
            )));
        }
        Ok(body.caller_groups)
    }

    async fn fetch_call_actions(&self) -> Result<CallActionsPayload, FrontendPullError> {
        let url = format!("{}/api/call-actions", self.frontend_base_url);
        let response = self.http_client.get(url).send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(FrontendPullError::InvalidResponse(format!(
                "GET /api/call-actions returned status {}",
                status
            )));
        }
        let body: CallActionsResponse = response.json().await?;
        if !body.ok {
            return Err(FrontendPullError::InvalidResponse(format!(
                "GET /api/call-actions returned ok=false{}",
                body.error
                    .as_ref()
                    .map(|value| format!(" ({value})"))
                    .unwrap_or_default()
            )));
        }
        Ok(CallActionsPayload {
            rules: body.rules,
            anonymous_action: body
                .anonymous_action
                .unwrap_or_else(default_anonymous_action),
            default_action: body.default_action.unwrap_or_else(default_default_action),
        })
    }

    async fn fetch_ivr_flows_export(&self) -> Result<Vec<IvrFlowDefinition>, FrontendPullError> {
        let url = format!("{}/api/ivr-flows/export", self.frontend_base_url);
        let response = self.http_client.get(url).send().await?;
        let status = response.status();
        if status == StatusCode::NOT_FOUND {
            return Err(FrontendPullError::InvalidResponse(
                "GET /api/ivr-flows/export returned 404".to_string(),
            ));
        }
        if !status.is_success() {
            return Err(FrontendPullError::InvalidResponse(format!(
                "GET /api/ivr-flows/export returned status {}",
                status
            )));
        }
        let body: IvrFlowsResponse = response.json().await?;
        if !body.ok {
            return Err(FrontendPullError::InvalidResponse(format!(
                "GET /api/ivr-flows/export returned ok=false{}",
                body.error
                    .as_ref()
                    .map(|value| format!(" ({value})"))
                    .unwrap_or_default()
            )));
        }
        Ok(body.flows)
    }

    async fn fetch_announcements(&self) -> Result<Vec<FrontendAnnouncement>, FrontendPullError> {
        let url = format!("{}/api/announcements", self.frontend_base_url);
        let response = self.http_client.get(url).send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(FrontendPullError::InvalidResponse(format!(
                "GET /api/announcements returned status {}",
                status
            )));
        }
        let body: AnnouncementsResponse = response.json().await?;
        if !body.ok {
            return Err(FrontendPullError::InvalidResponse(format!(
                "GET /api/announcements returned ok=false{}",
                body.error
                    .as_ref()
                    .map(|value| format!(" ({value})"))
                    .unwrap_or_default()
            )));
        }
        Ok(body.announcements)
    }

    async fn save_snapshot(
        &self,
        groups: &[CallerGroup],
        actions: &CallActionsPayload,
        announcements: &[FrontendAnnouncement],
        flows: &[IvrFlowDefinition],
    ) -> Result<(), FrontendPullError> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        if let Err(error) =
            apply_frontend_snapshot(&mut tx, groups, actions, announcements, flows).await
        {
            tx.rollback().await?;
            return Err(FrontendPullError::ConverterFailed(error));
        }
        tx.commit().await?;
        Ok(())
    }

    async fn sync_announcement_audio_cache(
        &self,
        announcements: &[FrontendAnnouncement],
    ) -> Result<(), FrontendPullError> {
        let cfg = config::announcement_config();
        if cfg.frontend_base_url.is_none() {
            log::info!(
                "[serversync] announcement audio cache sync skipped: FRONTEND_BASE_URL not set"
            );
            return Ok(());
        }

        let existing = self.load_existing_announcement_audio_state().await?;
        let frontend_ids: HashSet<Uuid> = announcements
            .iter()
            .map(|announcement| announcement.id)
            .collect();
        let audio_dir = Path::new(cfg.audio_dir.as_str());

        for (id, state) in &existing {
            if frontend_ids.contains(id) {
                continue;
            }
            let Some(audio_file_url) = state.audio_file_url.as_deref() else {
                continue;
            };
            let url_path = extract_url_path(audio_file_url);
            if !is_safe_announcement_url_path(url_path.as_str()) {
                log::warn!(
                    "[serversync] skip deleting cached audio for removed announcement id={} invalid audio_file_url={}",
                    id,
                    audio_file_url
                );
                continue;
            }
            let local_path = announcement_cache_local_path(audio_dir, url_path.as_str());
            if let Err(err) = remove_file_if_exists(local_path.as_path()).await {
                log::warn!(
                    "[serversync] failed to delete cached audio for removed announcement id={} path={} err={}",
                    id,
                    local_path.display(),
                    err
                );
            }
        }

        for announcement in announcements {
            let Some(audio_file_url) = announcement.audio_file_url.as_deref() else {
                continue;
            };
            let url_path = extract_url_path(audio_file_url);
            if !is_safe_announcement_url_path(url_path.as_str()) {
                log::warn!(
                    "[serversync] skip cached audio sync for announcement id={} invalid audio_file_url={}",
                    announcement.id,
                    audio_file_url
                );
                continue;
            }

            let local_path = announcement_cache_local_path(audio_dir, url_path.as_str());
            if !should_download_announcement_audio(
                local_path.as_path(),
                announcement.updated_at.as_deref(),
                existing.get(&announcement.id),
            ) {
                continue;
            }

            if let Err(err) = download_audio_file(
                &self.http_client,
                self.frontend_base_url.as_str(),
                url_path.as_str(),
                local_path.as_path(),
            )
            .await
            {
                log::warn!(
                    "[serversync] failed to cache announcement audio id={} url={} path={} err={:?}",
                    announcement.id,
                    url_path,
                    local_path.display(),
                    err
                );
            }
        }

        Ok(())
    }

    async fn load_existing_announcement_audio_state(
        &self,
    ) -> Result<HashMap<Uuid, ExistingAnnouncementAudioState>, FrontendPullError> {
        let rows = sqlx::query("SELECT id, audio_file_url, updated_at FROM announcements")
            .fetch_all(&self.pool)
            .await?;
        let mut map = HashMap::with_capacity(rows.len());
        for row in rows {
            let id: Uuid = row.try_get("id")?;
            let audio_file_url: Option<String> = row.try_get("audio_file_url")?;
            let updated_at: DateTime<Utc> = row.try_get("updated_at")?;
            map.insert(
                id,
                ExistingAnnouncementAudioState {
                    audio_file_url,
                    updated_at,
                },
            );
        }
        Ok(map)
    }
}

fn should_download_announcement_audio(
    local_path: &Path,
    frontend_updated_at_raw: Option<&str>,
    existing: Option<&ExistingAnnouncementAudioState>,
) -> bool {
    if !local_path.exists() {
        return true;
    }

    let frontend_updated_at = frontend_updated_at_raw.and_then(parse_frontend_updated_at);

    match existing {
        None => true,
        Some(state) => match frontend_updated_at {
            Some(frontend_updated_at) => state.updated_at != frontend_updated_at,
            None => false,
        },
    }
}

fn parse_frontend_updated_at(raw: &str) -> Option<DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|timestamp| timestamp.with_timezone(&Utc))
}

async fn download_audio_file(
    http_client: &reqwest::Client,
    frontend_base_url: &str,
    url_path: &str,
    local_path: &Path,
) -> anyhow::Result<()> {
    debug_assert!(is_safe_announcement_url_path(url_path));

    let full_url = format!("{}{}", frontend_base_url.trim_end_matches('/'), url_path);
    let mut resp = http_client
        .get(&full_url)
        .send()
        .await?
        .error_for_status()?;
    if let Some(content_length) = resp
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
    {
        if content_length > MAX_ANNOUNCEMENT_AUDIO_BYTES {
            return Err(anyhow::anyhow!(
                "audio response too large: content-length={} max={} url={}",
                content_length,
                MAX_ANNOUNCEMENT_AUDIO_BYTES,
                full_url
            ));
        }
    }

    let parent = local_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("local_path has no parent: {}", local_path.display()))?;
    tokio::fs::create_dir_all(parent).await?;

    let tmp_path = local_path.with_extension("tmp");
    let write_result: anyhow::Result<()> = async {
        let mut file = tokio::fs::File::create(&tmp_path).await?;
        let mut total_bytes = 0_u64;
        while let Some(chunk) = resp.chunk().await? {
            total_bytes = total_bytes
                .checked_add(chunk.len() as u64)
                .ok_or_else(|| anyhow::anyhow!("audio response size overflow url={}", full_url))?;
            if total_bytes > MAX_ANNOUNCEMENT_AUDIO_BYTES {
                return Err(anyhow::anyhow!(
                    "audio response exceeded max size while streaming: size={} max={} url={}",
                    total_bytes,
                    MAX_ANNOUNCEMENT_AUDIO_BYTES,
                    full_url
                ));
            }
            file.write_all(&chunk).await?;
        }
        file.flush().await?;
        Ok(())
    }
    .await;

    if let Err(err) = write_result {
        tokio::fs::remove_file(&tmp_path).await.ok();
        return Err(err);
    }

    if let Err(err) = tokio::fs::rename(&tmp_path, local_path).await {
        tokio::fs::remove_file(&tmp_path).await.ok();
        return Err(err.into());
    }
    Ok(())
}

async fn remove_file_if_exists(path: &Path) -> std::io::Result<()> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

fn is_safe_announcement_url_path(url_path: &str) -> bool {
    let Some(rest) = url_path.strip_prefix("/audio/announcements/") else {
        return false;
    };
    if rest.is_empty() {
        return false;
    }
    !rest.contains('/')
        && rest != "."
        && rest != ".."
        && !rest.contains('%')
        && !rest.contains('\\')
}

fn announcement_cache_local_path(audio_dir: &Path, url_path: &str) -> PathBuf {
    let filename = url_path
        .rsplit('/')
        .next()
        .filter(|segment| !segment.is_empty())
        .unwrap_or("invalid.wav");
    audio_dir.join(filename)
}

#[cfg(test)]
mod tests {
    use super::{
        is_safe_announcement_url_path, AnnouncementsResponse, CallActionsResponse,
        NumberGroupsResponse,
    };
    use crate::shared::utils::extract_url_path;

    #[test]
    fn number_groups_response_accepts_camel_case_fields() {
        let raw = r#"{"ok":true,"callerGroups":[{"id":"11111111-1111-4111-8111-111111111111","name":"grp","phoneNumbers":["+819012345678"]}]}"#;
        let parsed: NumberGroupsResponse = serde_json::from_str(raw).expect("valid response");
        assert!(parsed.ok);
        assert_eq!(parsed.caller_groups.len(), 1);
    }

    #[test]
    fn call_actions_response_can_omit_fallback_actions() {
        let raw = r#"{"ok":true,"rules":[]}"#;
        let parsed: CallActionsResponse = serde_json::from_str(raw).expect("valid response");
        assert!(parsed.ok);
        assert!(parsed.anonymous_action.is_none());
        assert!(parsed.default_action.is_none());
    }

    #[test]
    fn announcements_response_accepts_announcement_list() {
        let raw = r#"{"ok":true,"announcements":[{"id":"11111111-1111-4111-8111-111111111111","name":"greeting","announcementType":"custom","isActive":true,"audioFileUrl":"/audio/announcements/a.wav","updatedAt":"2026-02-24T12:34:56.000Z"}]}"#;
        let parsed: AnnouncementsResponse = serde_json::from_str(raw).expect("valid response");
        assert!(parsed.ok);
        assert_eq!(parsed.announcements.len(), 1);
        assert!(parsed.announcements[0].updated_at.is_some());
    }

    #[test]
    fn extract_url_path_strips_query_and_fragment() {
        let path = extract_url_path("http://localhost:3000/audio/announcements/a.wav?v=2#section");
        assert_eq!(path, "/audio/announcements/a.wav");
    }

    #[test]
    fn safe_announcement_url_path_rejects_subdirectories() {
        assert!(!is_safe_announcement_url_path(
            "/audio/announcements/morning/greeting.wav"
        ));
    }
}
