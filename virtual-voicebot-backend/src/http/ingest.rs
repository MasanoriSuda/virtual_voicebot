use std::time::Duration;

use crate::ports::ingest::{IngestError, IngestFuture, IngestPayload, IngestPort};

pub struct HttpIngestPort {
    timeout: Duration,
    client: reqwest::Client,
}

impl HttpIngestPort {
    pub fn new(timeout: Duration) -> Self {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .expect("failed to build ingest HTTP client");
        Self { timeout, client }
    }
}

impl IngestPort for HttpIngestPort {
    fn post(&self, url: String, payload: IngestPayload) -> IngestFuture<Result<(), IngestError>> {
        let timeout = self.timeout;
        let client = self.client.clone();
        Box::pin(async move {
            let recording = payload.recording.as_ref().map(|rec| {
                serde_json::json!({
                    "recordingUrl": rec.recording_url,
                    "durationSec": rec.duration_sec,
                    "sampleRate": rec.sample_rate,
                    "channels": rec.channels,
                })
            });
            let payload_json = serde_json::json!({
                "callId": payload.call_id.to_string(),
                "from": payload.from,
                "to": payload.to,
                "startedAt": humantime::format_rfc3339(payload.started_at).to_string(),
                "endedAt": humantime::format_rfc3339(payload.ended_at).to_string(),
                "status": payload.status,
                "summary": payload.summary,
                "durationSec": payload.duration_sec,
                "recording": recording,
            });
            client
                .post(url)
                .timeout(timeout)
                .json(&payload_json)
                .send()
                .await
                .map_err(|e| IngestError::Transport(e.to_string()))?;
            Ok(())
        })
    }
}
