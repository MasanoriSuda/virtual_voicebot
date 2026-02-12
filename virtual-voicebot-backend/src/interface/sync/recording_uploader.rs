use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;
use serde_json::json;
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct RecordingUploadRequest {
    pub call_log_id: Uuid,
    pub recording_id: Uuid,
    pub audio_path: PathBuf,
    pub meta_path: PathBuf,
}

#[derive(Debug, Error)]
pub enum RecordingUploadError {
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("file read failed: {0}")]
    FileReadFailed(String),
    #[error("transport failed: {0}")]
    TransportFailed(String),
    #[error("invalid response: {0}")]
    InvalidResponse(String),
}

#[derive(Clone)]
pub struct RecordingUploader {
    client: reqwest::Client,
    frontend_base_url: String,
}

#[derive(Deserialize)]
struct UploadResponse {
    #[serde(rename = "fileUrl")]
    file_url: String,
}

impl RecordingUploader {
    pub fn new(frontend_base_url: String, timeout: Duration) -> Result<Self, reqwest::Error> {
        let client = reqwest::Client::builder().timeout(timeout).build()?;
        Ok(Self {
            client,
            frontend_base_url: frontend_base_url.trim_end_matches('/').to_string(),
        })
    }

    pub async fn upload(
        &self,
        request: &RecordingUploadRequest,
    ) -> Result<String, RecordingUploadError> {
        if request.audio_path.as_os_str().is_empty() || request.meta_path.as_os_str().is_empty() {
            return Err(RecordingUploadError::InvalidRequest(
                "audio_path/meta_path must not be empty".to_string(),
            ));
        }

        let audio = tokio::fs::read(&request.audio_path)
            .await
            .map_err(|e| RecordingUploadError::FileReadFailed(e.to_string()))?;
        let meta = match tokio::fs::read_to_string(&request.meta_path).await {
            Ok(meta) => meta,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                log::warn!(
                    "[serversync] recording meta not found, generating fallback meta recording_id={} path={}",
                    request.recording_id,
                    request.meta_path.display()
                );
                fallback_meta_json(&request.audio_path)
            }
            Err(err) => return Err(RecordingUploadError::FileReadFailed(err.to_string())),
        };

        let audio_part = reqwest::multipart::Part::bytes(audio)
            .file_name("mixed.wav")
            .mime_str("application/octet-stream")
            .map_err(|e| RecordingUploadError::InvalidRequest(e.to_string()))?;
        let meta_part = reqwest::multipart::Part::text(meta)
            .file_name("meta.json")
            .mime_str("application/json")
            .map_err(|e| RecordingUploadError::InvalidRequest(e.to_string()))?;

        let form = reqwest::multipart::Form::new()
            .text("callLogId", request.call_log_id.to_string())
            .text("recordingId", request.recording_id.to_string())
            .part("audio", audio_part)
            .part("meta", meta_part);

        let url = format!("{}/api/ingest/recording-file", self.frontend_base_url);
        let response = self
            .client
            .post(url)
            .multipart(form)
            .send()
            .await
            .map_err(|e| RecordingUploadError::TransportFailed(e.to_string()))?
            .error_for_status()
            .map_err(|e| RecordingUploadError::TransportFailed(e.to_string()))?;

        let body: UploadResponse = response
            .json()
            .await
            .map_err(|e| RecordingUploadError::InvalidResponse(e.to_string()))?;
        if body.file_url.trim().is_empty() {
            return Err(RecordingUploadError::InvalidResponse(
                "fileUrl is empty".to_string(),
            ));
        }

        Ok(body.file_url)
    }
}

fn fallback_meta_json(audio_path: &Path) -> String {
    let call_id = audio_path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("unknown");
    json!({
        "callId": call_id,
        "recordingStartedAt": serde_json::Value::Null,
        "sampleRate": 8000,
        "channels": 2,
        "durationSec": 0.0,
        "files": {
            "mixed": "mixed.wav"
        }
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_meta_json_is_valid_json() {
        let meta = fallback_meta_json(Path::new("storage/recordings/call-a/mixed.wav"));
        let parsed: serde_json::Value = serde_json::from_str(&meta).expect("valid json");
        assert_eq!(parsed["callId"], "call-a");
        assert_eq!(parsed["files"]["mixed"], "mixed.wav");
        assert_eq!(parsed["sampleRate"], 8000);
        assert_eq!(parsed["channels"], 2);
    }
}
