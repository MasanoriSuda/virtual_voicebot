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
    use super::{processing_file_path, NotificationWorker, NotificationWorkerError};
    use serde_json::{json, Value};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};

    #[derive(Clone, Debug)]
    struct CapturedRequest {
        path: String,
        body: String,
    }

    fn test_worker(queue_file: PathBuf, frontend_base_url: String) -> NotificationWorker {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .expect("test client should be buildable");
        NotificationWorker {
            queue_file,
            frontend_base_url,
            client,
        }
    }

    async fn read_http_request(socket: &mut TcpStream) -> std::io::Result<CapturedRequest> {
        let mut buf = [0u8; 1024];
        let mut request = Vec::new();
        let mut header_end = None;
        let mut content_length = 0usize;

        loop {
            let n = socket.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            request.extend_from_slice(&buf[..n]);

            if header_end.is_none() {
                if let Some(pos) = request.windows(4).position(|window| window == b"\r\n\r\n") {
                    let end = pos + 4;
                    header_end = Some(end);
                    let headers = String::from_utf8_lossy(&request[..end]).to_ascii_lowercase();
                    for line in headers.lines() {
                        if let Some(value) = line.strip_prefix("content-length:") {
                            content_length = value.trim().parse::<usize>().unwrap_or(0);
                        }
                    }
                }
            }

            if let Some(end) = header_end {
                if request.len() >= end + content_length {
                    break;
                }
            }
        }

        let header_end = header_end.unwrap_or(request.len());
        let request_text = String::from_utf8_lossy(&request).into_owned();
        let head = &request_text[..header_end];
        let body = request_text[header_end..].to_string();
        let path = head
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap_or("")
            .to_string();

        Ok(CapturedRequest { path, body })
    }

    async fn spawn_mock_server(
        statuses: Vec<u16>,
    ) -> (
        String,
        Arc<Mutex<Vec<CapturedRequest>>>,
        tokio::task::JoinHandle<()>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("mock server bind should succeed");
        let addr = listener
            .local_addr()
            .expect("mock server local_addr should be available");
        let captured = Arc::new(Mutex::new(Vec::<CapturedRequest>::new()));
        let captured_clone = captured.clone();
        let statuses = Arc::new(statuses);
        let idx = Arc::new(AtomicUsize::new(0));
        let idx_clone = idx.clone();

        let handle = tokio::spawn(async move {
            loop {
                let (mut socket, _) = match listener.accept().await {
                    Ok(value) => value,
                    Err(_) => break,
                };

                let request = match read_http_request(&mut socket).await {
                    Ok(value) => value,
                    Err(_) => break,
                };
                captured_clone
                    .lock()
                    .expect("captured lock should be available")
                    .push(request);

                let current = idx_clone.fetch_add(1, Ordering::SeqCst);
                let status = statuses
                    .get(current)
                    .copied()
                    .or_else(|| statuses.last().copied())
                    .unwrap_or(200);
                let reason = if (200..300).contains(&status) {
                    "OK"
                } else {
                    "ERROR"
                };
                let response = format!(
                    "HTTP/1.1 {} {}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    status, reason
                );
                let _ = socket.write_all(response.as_bytes()).await;
            }
        });

        (format!("http://{}", addr), captured, handle)
    }

    fn write_json_line(path: &Path, payload: &Value) {
        let content = format!(
            "{}\n",
            serde_json::to_string(payload).expect("payload should serialize")
        );
        std::fs::write(path, content).expect("jsonl should be writable");
    }

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

    #[tokio::test]
    async fn send_notification_posts_to_expected_endpoint() {
        let temp = tempfile::tempdir().expect("tempdir should be creatable");
        let queue_file = temp.path().join("pending.jsonl");
        let (base_url, captured, server) = spawn_mock_server(vec![200]).await;
        let worker = test_worker(queue_file, base_url);

        worker
            .send_notification(&json!({
                "callerNumber": "09012345678",
                "trigger": "direct",
                "receivedAt": "2026-02-28T00:00:00Z"
            }))
            .await
            .expect("notification send should succeed");

        let requests = captured.lock().expect("captured lock should be available");
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].path, "/api/ingest/incoming-call");
        assert!(requests[0]
            .body
            .contains("\"callerNumber\":\"09012345678\""));
        drop(requests);
        server.abort();
    }

    #[tokio::test]
    async fn process_once_renames_pending_flushes_and_removes_processing() {
        let temp = tempfile::tempdir().expect("tempdir should be creatable");
        let queue_file = temp.path().join("pending.jsonl");
        write_json_line(
            queue_file.as_path(),
            &json!({
                "callerNumber": "09011111111",
                "trigger": "direct",
                "receivedAt": "2026-02-28T00:00:00Z"
            }),
        );
        let processing_file = processing_file_path(queue_file.as_path());
        let (base_url, captured, server) = spawn_mock_server(vec![200]).await;
        let worker = test_worker(queue_file.clone(), base_url);

        worker
            .process_once()
            .await
            .expect("process_once should succeed");

        assert!(!queue_file.exists(), "pending file should be consumed");
        assert!(
            !processing_file.exists(),
            "processing file should be removed after success"
        );
        let requests = captured.lock().expect("captured lock should be available");
        assert_eq!(requests.len(), 1);
        assert!(requests[0]
            .body
            .contains("\"callerNumber\":\"09011111111\""));
        drop(requests);
        server.abort();
    }

    #[tokio::test]
    async fn process_once_processes_existing_processing_before_pending() {
        let temp = tempfile::tempdir().expect("tempdir should be creatable");
        let queue_file = temp.path().join("pending.jsonl");
        let processing_file = processing_file_path(queue_file.as_path());

        write_json_line(
            processing_file.as_path(),
            &json!({
                "callerNumber": "from-processing",
                "trigger": "direct",
                "receivedAt": "2026-02-28T00:00:00Z"
            }),
        );
        write_json_line(
            queue_file.as_path(),
            &json!({
                "callerNumber": "from-pending",
                "trigger": "direct",
                "receivedAt": "2026-02-28T00:00:01Z"
            }),
        );

        let (base_url, captured, server) = spawn_mock_server(vec![200, 200]).await;
        let worker = test_worker(queue_file.clone(), base_url);

        worker
            .process_once()
            .await
            .expect("process_once should succeed");

        assert!(!queue_file.exists(), "pending file should be consumed");
        assert!(
            !processing_file.exists(),
            "processing file should be removed after success"
        );
        let requests = captured.lock().expect("captured lock should be available");
        assert_eq!(requests.len(), 2);
        assert!(
            requests[0]
                .body
                .contains("\"callerNumber\":\"from-processing\""),
            "existing .processing should be sent first"
        );
        assert!(
            requests[1]
                .body
                .contains("\"callerNumber\":\"from-pending\""),
            "pending should be sent second"
        );
        drop(requests);
        server.abort();
    }

    #[tokio::test]
    async fn process_once_keeps_processing_file_on_http_error() {
        let temp = tempfile::tempdir().expect("tempdir should be creatable");
        let queue_file = temp.path().join("pending.jsonl");
        write_json_line(
            queue_file.as_path(),
            &json!({
                "callerNumber": "retry-me",
                "trigger": "direct",
                "receivedAt": "2026-02-28T00:00:00Z"
            }),
        );
        let processing_file = processing_file_path(queue_file.as_path());
        let (base_url, _captured, server) = spawn_mock_server(vec![500]).await;
        let worker = test_worker(queue_file.clone(), base_url);

        let result = worker.process_once().await;
        assert!(
            matches!(result, Err(NotificationWorkerError::Http(_))),
            "http error should be surfaced"
        );
        assert!(
            !queue_file.exists(),
            "pending should be renamed before send attempt"
        );
        assert!(
            processing_file.exists(),
            "processing should remain for retry when send fails"
        );
        let raw = std::fs::read_to_string(&processing_file).expect("processing file should remain");
        assert!(raw.contains("\"callerNumber\":\"retry-me\""));
        server.abort();
    }

    #[tokio::test]
    async fn process_once_retries_existing_processing_file_on_next_cycle() {
        let temp = tempfile::tempdir().expect("tempdir should be creatable");
        let queue_file = temp.path().join("pending.jsonl");
        let processing_file = processing_file_path(queue_file.as_path());
        write_json_line(
            processing_file.as_path(),
            &json!({
                "callerNumber": "retry-success",
                "trigger": "direct",
                "receivedAt": "2026-02-28T00:00:00Z"
            }),
        );

        let (base_url, captured, server) = spawn_mock_server(vec![200]).await;
        let worker = test_worker(queue_file.clone(), base_url);

        worker
            .process_once()
            .await
            .expect("existing processing should be retried successfully");

        assert!(
            !processing_file.exists(),
            "processing should be removed after retry success"
        );
        assert!(!queue_file.exists(), "pending should stay absent");
        let requests = captured.lock().expect("captured lock should be available");
        assert_eq!(requests.len(), 1);
        assert!(requests[0]
            .body
            .contains("\"callerNumber\":\"retry-success\""));
        drop(requests);
        server.abort();
    }

    #[tokio::test]
    async fn flush_processing_file_returns_invalid_payload_and_keeps_file() {
        let temp = tempfile::tempdir().expect("tempdir should be creatable");
        let queue_file = temp.path().join("pending.jsonl");
        let processing_file = processing_file_path(queue_file.as_path());
        std::fs::write(&processing_file, "invalid-json-line\n")
            .expect("processing file should be writable");
        let worker = test_worker(queue_file, "http://127.0.0.1:9".to_string());

        let result = worker
            .flush_processing_file(processing_file.as_path())
            .await;
        match result {
            Err(NotificationWorkerError::InvalidPayloadLine(message)) => {
                assert!(message.contains("invalid-json-line"));
            }
            other => panic!("expected InvalidPayloadLine, got {:?}", other),
        }

        assert!(
            processing_file.exists(),
            "invalid payload should not delete processing file"
        );
    }
}
