use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::Value;
use thiserror::Error;
use tokio::time::{interval, MissedTickBehavior};

use crate::shared::config::{self, SyncConfig};

const NOTIFICATION_POLL_INTERVAL_SEC: u64 = 1;

#[derive(Clone)]
pub struct NotificationWorker {
    queue_file: PathBuf,
    frontend_base_url: String,
    client: reqwest::Client,
}

#[derive(Debug, Error)]
pub enum NotificationWorkerError {
    #[error("io failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("http failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("invalid queue payload line: {0}")]
    InvalidPayloadLine(String),
}

impl NotificationWorker {
    pub fn new(config: SyncConfig) -> Result<Self, reqwest::Error> {
        let timeout = Duration::from_secs(config.timeout_sec);
        let client = reqwest::Client::builder().timeout(timeout).build()?;
        Ok(Self {
            queue_file: PathBuf::from(config::notification_queue_file()),
            frontend_base_url: config.frontend_base_url.trim_end_matches('/').to_string(),
            client,
        })
    }

    pub async fn run(&self) {
        log::info!(
            "[serversync] notification worker started (poll_interval={}s, queue_file={})",
            NOTIFICATION_POLL_INTERVAL_SEC,
            self.queue_file.display()
        );
        let mut ticker = interval(Duration::from_secs(NOTIFICATION_POLL_INTERVAL_SEC));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            ticker.tick().await;
            if let Err(error) = self.process_once().await {
                log::warn!("[serversync] notification worker failed: {}", error);
            }
        }
    }

    pub async fn process_once(&self) -> Result<(), NotificationWorkerError> {
        let processing = processing_file_path(self.queue_file.as_path());

        if processing.exists() {
            self.flush_processing_file(processing.as_path()).await?;
        }

        if !self.queue_file.exists() {
            return Ok(());
        }

        match std::fs::rename(self.queue_file.as_path(), processing.as_path()) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(err) => return Err(NotificationWorkerError::Io(err)),
        }

        self.flush_processing_file(processing.as_path()).await
    }

    async fn flush_processing_file(
        &self,
        processing: &Path,
    ) -> Result<(), NotificationWorkerError> {
        let content = std::fs::read_to_string(processing)?;
        for line in content
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
        {
            let payload: Value = serde_json::from_str(line).map_err(|err| {
                NotificationWorkerError::InvalidPayloadLine(format!("{line} ({err})"))
            })?;
            self.send_notification(&payload).await?;
        }
        std::fs::remove_file(processing)?;
        Ok(())
    }

    async fn send_notification(&self, payload: &Value) -> Result<(), NotificationWorkerError> {
        let url = format!("{}/api/ingest/incoming-call", self.frontend_base_url);
        self.client
            .post(url)
            .json(payload)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}

fn processing_file_path(queue_file: &Path) -> PathBuf {
    let processing_extension = match queue_file.extension().and_then(|value| value.to_str()) {
        Some(ext) if !ext.is_empty() => format!("{ext}.processing"),
        _ => "processing".to_string(),
    };
    queue_file.with_extension(processing_extension)
}

#[cfg(test)]
mod tests {
    use super::processing_file_path;
    use std::path::PathBuf;

    #[test]
    fn processing_file_path_uses_jsonl_processing_extension() {
        let queue = PathBuf::from("storage/notifications/pending.jsonl");
        assert_eq!(
            processing_file_path(queue.as_path()),
            PathBuf::from("storage/notifications/pending.jsonl.processing")
        );
    }

    #[test]
    fn processing_file_path_falls_back_for_extensionless_file() {
        let queue = PathBuf::from("storage/notifications/pending");
        assert_eq!(
            processing_file_path(queue.as_path()),
            PathBuf::from("storage/notifications/pending.processing")
        );
    }
}
