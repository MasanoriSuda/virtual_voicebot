use std::path::PathBuf;
use std::time::Duration;

use serde::Deserialize;
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
        let meta = tokio::fs::read_to_string(&request.meta_path)
            .await
            .map_err(|e| RecordingUploadError::FileReadFailed(e.to_string()))?;

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
