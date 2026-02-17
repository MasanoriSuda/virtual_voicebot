use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use serde_json::{json, Value};
use thiserror::Error;
use tokio::time::{interval, MissedTickBehavior};
use uuid::Uuid;

use crate::interface::sync::recording_uploader::{
    RecordingUploadError, RecordingUploadRequest, RecordingUploader,
};
use crate::shared::config::SyncConfig;
use crate::shared::ports::sync_outbox_port::{PendingOutboxEntry, SyncOutboxPort};

#[derive(Debug, Error)]
pub enum SyncWorkerError {
    #[error("outbox failed: {0}")]
    OutboxFailed(String),
    #[error("http failed: {0}")]
    HttpFailed(String),
    #[error("payload invalid: {0}")]
    InvalidPayload(String),
    #[error("recording upload failed: {0}")]
    RecordingUploadFailed(#[from] RecordingUploadError),
    #[error("file cleanup failed: {0}")]
    FileCleanupFailed(String),
}

const FILE_READ_RETRY_GRACE_SECONDS: i64 = 300;

#[derive(Clone)]
pub struct OutboxWorker {
    outbox_repo: Arc<dyn SyncOutboxPort>,
    http_client: reqwest::Client,
    config: SyncConfig,
    recording_uploader: RecordingUploader,
}

impl OutboxWorker {
    pub fn new(
        outbox_repo: Arc<dyn SyncOutboxPort>,
        config: SyncConfig,
    ) -> Result<Self, reqwest::Error> {
        let timeout = Duration::from_secs(config.timeout_sec);
        let http_client = reqwest::Client::builder().timeout(timeout).build()?;
        let recording_uploader = RecordingUploader::new(config.frontend_base_url.clone(), timeout)?;
        Ok(Self {
            outbox_repo,
            http_client,
            config,
            recording_uploader,
        })
    }

    pub async fn run(&self) {
        let mut ticker = interval(Duration::from_secs(self.config.poll_interval_sec));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            ticker.tick().await;
            if let Err(error) = self.process_batch().await {
                log::warn!("[serversync] batch processing failed: {}", error);
            }
        }
    }

    pub async fn process_batch(&self) -> Result<(), SyncWorkerError> {
        let entries = self
            .outbox_repo
            .fetch_pending(self.config.batch_size)
            .await
            .map_err(|e| SyncWorkerError::OutboxFailed(e.to_string()))?;

        for entry in entries {
            if let Err(error) = self.send_entry(&entry).await {
                log::warn!(
                    "[serversync] failed to send outbox entry {} (entity_type={}): {}",
                    entry.id,
                    entry.entity_type,
                    error
                );
                if should_mark_processed_on_error(&entry, &error) {
                    log::warn!(
                        "[serversync] marking outbox entry {} as processed due to non-retryable error",
                        entry.id
                    );
                    self.outbox_repo
                        .mark_processed(entry.id, Utc::now())
                        .await
                        .map_err(|e| SyncWorkerError::OutboxFailed(e.to_string()))?;
                }
                continue;
            }
            self.outbox_repo
                .mark_processed(entry.id, Utc::now())
                .await
                .map_err(|e| SyncWorkerError::OutboxFailed(e.to_string()))?;
        }

        Ok(())
    }

    async fn send_entry(&self, entry: &PendingOutboxEntry) -> Result<(), SyncWorkerError> {
        match entry.entity_type.as_str() {
            "recording_file" => self.send_recording_file(entry).await,
            _ => self.send_json_payload(entry).await,
        }
    }

    async fn send_json_payload(&self, entry: &PendingOutboxEntry) -> Result<(), SyncWorkerError> {
        let url = format!("{}/api/ingest/sync", self.config.frontend_base_url);
        let body = json!({
            "entries": [{
                "entityType": entry.entity_type,
                "entityId": entry.entity_id.to_string(),
                "payload": entry.payload,
                "createdAt": entry.created_at.to_rfc3339(),
            }]
        });
        self.http_client
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| SyncWorkerError::HttpFailed(e.to_string()))?
            .error_for_status()
            .map_err(|e| SyncWorkerError::HttpFailed(e.to_string()))?;
        Ok(())
    }

    async fn send_recording_file(&self, entry: &PendingOutboxEntry) -> Result<(), SyncWorkerError> {
        let payload = RecordingFilePayload::from_value(&entry.payload)?;
        let file_url = self
            .recording_uploader
            .upload(&RecordingUploadRequest {
                call_log_id: payload.call_log_id,
                recording_id: payload.recording_id,
                audio_path: payload.audio_path.clone(),
                meta_path: payload.meta_path.clone(),
            })
            .await?;

        self.outbox_repo
            .mark_recording_uploaded(payload.recording_id, file_url)
            .await
            .map_err(|e| SyncWorkerError::OutboxFailed(e.to_string()))?;

        cleanup_recording_dir(&payload.recording_dir, &payload.call_id).await?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct RecordingFilePayload {
    call_log_id: Uuid,
    recording_id: Uuid,
    call_id: String,
    audio_path: PathBuf,
    meta_path: PathBuf,
    recording_dir: PathBuf,
}

impl RecordingFilePayload {
    fn from_value(value: &Value) -> Result<Self, SyncWorkerError> {
        let call_log_id = parse_uuid(value, "callLogId", "call_log_id")?;
        let recording_id = parse_uuid(value, "recordingId", "recording_id")?;

        let call_id = value
            .get("callId")
            .or_else(|| value.get("call_id"))
            .and_then(Value::as_str)
            .map(ToString::to_string);

        let audio_path = value
            .get("filePath")
            .or_else(|| value.get("file_path"))
            .and_then(Value::as_str)
            .map(PathBuf::from)
            .or_else(|| {
                call_id
                    .as_ref()
                    .map(|id| PathBuf::from(format!("storage/recordings/{id}/mixed.wav")))
            })
            .ok_or_else(|| {
                SyncWorkerError::InvalidPayload(
                    "recording_file payload requires filePath/file_path or callId/call_id"
                        .to_string(),
                )
            })?;
        let audio_path = resolve_recording_path(audio_path);

        let recording_dir = audio_path.parent().map(Path::to_path_buf).ok_or_else(|| {
            SyncWorkerError::InvalidPayload("audio_path has no parent directory".to_string())
        })?;

        let resolved_call_id = call_id.unwrap_or_else(|| {
            recording_dir
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_string()
        });
        if resolved_call_id.is_empty() {
            return Err(SyncWorkerError::InvalidPayload(
                "callId/call_id is missing and could not be inferred".to_string(),
            ));
        }

        let meta_path = value
            .get("metaPath")
            .or_else(|| value.get("meta_path"))
            .and_then(Value::as_str)
            .map(PathBuf::from)
            .map(resolve_recording_path)
            .unwrap_or_else(|| recording_dir.join("meta.json"));

        Ok(Self {
            call_log_id,
            recording_id,
            call_id: resolved_call_id,
            audio_path,
            meta_path,
            recording_dir,
        })
    }
}

fn parse_uuid(value: &Value, camel: &str, snake: &str) -> Result<Uuid, SyncWorkerError> {
    let raw = value
        .get(camel)
        .or_else(|| value.get(snake))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            SyncWorkerError::InvalidPayload(format!("{camel}/{snake} is missing in payload"))
        })?;

    Uuid::parse_str(raw)
        .map_err(|e| SyncWorkerError::InvalidPayload(format!("{camel}/{snake} is invalid: {e}")))
}

fn resolve_recording_path(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        return path;
    }

    let mut candidates = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join(&path));
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    candidates.push(manifest_dir.join(&path));
    if let Some(repo_root) = manifest_dir.parent() {
        candidates.push(repo_root.join(&path));
    }

    for candidate in candidates {
        if candidate.exists() {
            return candidate;
        }
    }

    path
}

async fn cleanup_recording_dir(dir: &Path, call_id: &str) -> Result<(), SyncWorkerError> {
    let Some(parent) = dir.parent() else {
        return Err(SyncWorkerError::FileCleanupFailed(
            "recording directory has no parent".to_string(),
        ));
    };
    if parent.file_name().and_then(|name| name.to_str()) != Some("recordings") {
        return Err(SyncWorkerError::FileCleanupFailed(format!(
            "unexpected recording directory: {}",
            dir.display()
        )));
    }
    if dir.file_name().and_then(|name| name.to_str()) != Some(call_id) {
        return Err(SyncWorkerError::FileCleanupFailed(format!(
            "call_id mismatch for directory: {}",
            dir.display()
        )));
    }

    match tokio::fs::remove_dir_all(dir).await {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(SyncWorkerError::FileCleanupFailed(err.to_string())),
    }
}

fn should_mark_processed_on_error(entry: &PendingOutboxEntry, error: &SyncWorkerError) -> bool {
    if entry.entity_type != "recording_file" {
        return false;
    }
    match error {
        SyncWorkerError::InvalidPayload(_) => true,
        SyncWorkerError::RecordingUploadFailed(RecordingUploadError::FileReadFailed(message)) => {
            let lower = message.to_ascii_lowercase();
            let is_not_found =
                message.contains("No such file or directory") || lower.contains("not found");
            if !is_not_found {
                return false;
            }
            // File creation can lag slightly behind outbox enqueue.
            // Keep retrying for a grace period before considering it non-retryable.
            let age_sec = (Utc::now() - entry.created_at).num_seconds();
            age_sec >= FILE_READ_RETRY_GRACE_SECONDS
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recording_payload_accepts_camel_case() {
        let payload = json!({
            "callLogId": "11111111-1111-1111-1111-111111111111",
            "recordingId": "22222222-2222-2222-2222-222222222222",
            "callId": "call-a",
            "filePath": "storage/recordings/call-a/mixed.wav",
        });

        let parsed = RecordingFilePayload::from_value(&payload).expect("valid payload");
        assert_eq!(parsed.call_id, "call-a");
        assert_eq!(
            parsed.audio_path,
            PathBuf::from("storage/recordings/call-a/mixed.wav")
        );
        assert_eq!(
            parsed.meta_path,
            PathBuf::from("storage/recordings/call-a/meta.json")
        );
    }

    #[test]
    fn recording_payload_accepts_snake_case_and_infers_call_id() {
        let payload = json!({
            "call_log_id": "33333333-3333-3333-3333-333333333333",
            "recording_id": "44444444-4444-4444-4444-444444444444",
            "file_path": "storage/recordings/call-b/mixed.wav",
            "meta_path": "storage/recordings/call-b/meta.json",
        });

        let parsed = RecordingFilePayload::from_value(&payload).expect("valid payload");
        assert_eq!(parsed.call_id, "call-b");
        assert_eq!(
            parsed.meta_path,
            PathBuf::from("storage/recordings/call-b/meta.json")
        );
    }

    fn sample_pending_recording_entry() -> PendingOutboxEntry {
        PendingOutboxEntry {
            id: 1,
            entity_type: "recording_file".to_string(),
            entity_id: Uuid::nil(),
            payload: json!({}),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn missing_file_error_is_retryable_within_grace_period() {
        let entry = sample_pending_recording_entry();
        let error = SyncWorkerError::RecordingUploadFailed(RecordingUploadError::FileReadFailed(
            "No such file or directory (os error 2)".to_string(),
        ));
        assert!(!should_mark_processed_on_error(&entry, &error));
    }

    #[test]
    fn missing_file_error_is_marked_processed_after_grace_period() {
        let mut entry = sample_pending_recording_entry();
        entry.created_at =
            Utc::now() - chrono::Duration::seconds(FILE_READ_RETRY_GRACE_SECONDS + 1);
        let error = SyncWorkerError::RecordingUploadFailed(RecordingUploadError::FileReadFailed(
            "No such file or directory (os error 2)".to_string(),
        ));
        assert!(should_mark_processed_on_error(&entry, &error));
    }

    #[test]
    fn retryable_transport_error_is_not_marked_processed() {
        let entry = sample_pending_recording_entry();
        let error = SyncWorkerError::RecordingUploadFailed(RecordingUploadError::TransportFailed(
            "timeout".to_string(),
        ));
        assert!(!should_mark_processed_on_error(&entry, &error));
    }
}
