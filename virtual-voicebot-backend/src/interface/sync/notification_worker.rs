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

        if tokio::fs::try_exists(processing.as_path()).await? {
            self.flush_processing_file(processing.as_path()).await?;
        }

        if !tokio::fs::try_exists(self.queue_file.as_path()).await? {
            return Ok(());
        }

        match tokio::fs::rename(self.queue_file.as_path(), processing.as_path()).await {
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
        let content = tokio::fs::read_to_string(processing).await?;
        let lines = content
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>();

        for (index, line) in lines.iter().enumerate() {
            let line_no = index + 1;
            let payload: Value = match serde_json::from_str(line) {
                Ok(payload) => payload,
                Err(err) => {
                    let parse_error = NotificationWorkerError::InvalidPayloadLine(format!(
                        "line {line_no} ({err})"
                    ));
                    log::warn!(
                        "[serversync] skipping invalid notification payload: {}",
                        parse_error
                    );
                    Self::update_processing_checkpoint(processing, &lines[index + 1..]).await?;
                    continue;
                }
            };

            let call_id = match payload
                .get("call_id")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                Some(value) => value.to_string(),
                None => {
                    let missing_call_id_error = NotificationWorkerError::InvalidPayloadLine(
                        format!("line {line_no} missing call_id"),
                    );
                    log::warn!(
                        "[serversync] skipping invalid notification payload: {}",
                        missing_call_id_error
                    );
                    Self::update_processing_checkpoint(processing, &lines[index + 1..]).await?;
                    continue;
                }
            };

            if let Err(error) = self.send_notification(&payload, call_id.as_str()).await {
                log::warn!(
                    "[serversync] notification send failed (line_no={}, call_id={}): {}",
                    line_no,
                    call_id,
                    error
                );
                return Err(error);
            }
            Self::update_processing_checkpoint(processing, &lines[index + 1..]).await?;
        }

        Ok(())
    }

    async fn update_processing_checkpoint(
        processing: &Path,
        remaining: &[&str],
    ) -> Result<(), NotificationWorkerError> {
        if remaining.is_empty() {
            tokio::fs::remove_file(processing).await?;
        } else {
            let mut remaining_content = remaining.join("\n");
            remaining_content.push('\n');
            tokio::fs::write(processing, remaining_content).await?;
        }
        Ok(())
    }

    async fn send_notification(
        &self,
        payload: &Value,
        call_id: &str,
    ) -> Result<(), NotificationWorkerError> {
        let url = format!("{}/api/ingest/incoming-call", self.frontend_base_url);
        self.client
            .post(url)
            .header("Idempotency-Key", call_id)
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
        headers: String,
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
        let headers = head.to_ascii_lowercase();

        Ok(CapturedRequest {
            path,
            headers,
            body,
        })
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
            .send_notification(
                &json!({
                    "call_id": "test-send-notification",
                    "callerNumber": "09012345678",
                    "trigger": "direct",
                    "receivedAt": "2026-02-28T00:00:00Z"
                }),
                "test-send-notification",
            )
            .await
            .expect("notification send should succeed");

        let requests = captured.lock().expect("captured lock should be available");
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].path, "/api/ingest/incoming-call");
        assert!(requests[0]
            .headers
            .contains("idempotency-key: test-send-notification"));
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
                "call_id": "direct-success",
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
                "call_id": "existing-processing",
                "callerNumber": "from-processing",
                "trigger": "direct",
                "receivedAt": "2026-02-28T00:00:00Z"
            }),
        );
        write_json_line(
            queue_file.as_path(),
            &json!({
                "call_id": "pending-queue",
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
                "call_id": "retry-me",
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
    async fn process_once_keeps_only_unsent_lines_after_partial_http_error() {
        let temp = tempfile::tempdir().expect("tempdir should be creatable");
        let queue_file = temp.path().join("pending.jsonl");
        let processing_file = processing_file_path(queue_file.as_path());

        let first = json!({
            "call_id": "first-ok",
            "callerNumber": "first-ok",
            "trigger": "direct",
            "receivedAt": "2026-02-28T00:00:00Z"
        });
        let second = json!({
            "call_id": "second-retry",
            "callerNumber": "second-retry",
            "trigger": "direct",
            "receivedAt": "2026-02-28T00:00:01Z"
        });
        let content = format!(
            "{}\n{}\n",
            serde_json::to_string(&first).expect("first payload should serialize"),
            serde_json::to_string(&second).expect("second payload should serialize")
        );
        std::fs::write(queue_file.as_path(), content).expect("queue file should be writable");

        let (base_url, captured, server) = spawn_mock_server(vec![200, 500]).await;
        let worker = test_worker(queue_file.clone(), base_url);

        let result = worker.process_once().await;
        assert!(
            matches!(result, Err(NotificationWorkerError::Http(_))),
            "http error should be surfaced"
        );

        assert!(
            processing_file.exists(),
            "processing should remain for retry when second send fails"
        );
        let raw = std::fs::read_to_string(&processing_file).expect("processing file should remain");
        assert!(
            !raw.contains("\"callerNumber\":\"first-ok\""),
            "already-sent lines should be removed from processing"
        );
        assert!(
            raw.contains("\"callerNumber\":\"second-retry\""),
            "only unsent line should remain in processing"
        );

        let requests = captured.lock().expect("captured lock should be available");
        assert_eq!(requests.len(), 2);
        assert!(requests[0].body.contains("\"callerNumber\":\"first-ok\""));
        assert!(requests[1]
            .body
            .contains("\"callerNumber\":\"second-retry\""));
        drop(requests);
        server.abort();
    }

    #[tokio::test]
    async fn process_once_does_not_resend_already_sent_lines_after_partial_http_error() {
        let temp = tempfile::tempdir().expect("tempdir should be creatable");
        let queue_file = temp.path().join("pending.jsonl");
        let processing_file = processing_file_path(queue_file.as_path());

        let first = json!({
            "call_id": "first-once",
            "callerNumber": "first-once",
            "trigger": "direct",
            "receivedAt": "2026-02-28T00:00:00Z"
        });
        let second = json!({
            "call_id": "second-retry",
            "callerNumber": "second-retry",
            "trigger": "direct",
            "receivedAt": "2026-02-28T00:00:01Z"
        });
        let content = format!(
            "{}\n{}\n",
            serde_json::to_string(&first).expect("first payload should serialize"),
            serde_json::to_string(&second).expect("second payload should serialize")
        );
        std::fs::write(processing_file.as_path(), content)
            .expect("processing file should be writable");

        let (base_url, captured, server) = spawn_mock_server(vec![200, 500, 200]).await;
        let worker = test_worker(queue_file.clone(), base_url);

        let first_result = worker.process_once().await;
        assert!(
            matches!(first_result, Err(NotificationWorkerError::Http(_))),
            "first cycle should fail at second notification"
        );
        assert!(
            processing_file.exists(),
            "processing should remain after first cycle failure"
        );

        worker
            .process_once()
            .await
            .expect("second cycle should retry only unsent line");

        assert!(
            !processing_file.exists(),
            "processing should be removed after retry success"
        );
        assert!(!queue_file.exists(), "pending file should remain absent");

        let requests = captured.lock().expect("captured lock should be available");
        assert_eq!(requests.len(), 3);
        let first_count = requests
            .iter()
            .filter(|request| request.body.contains("\"callerNumber\":\"first-once\""))
            .count();
        let second_count = requests
            .iter()
            .filter(|request| request.body.contains("\"callerNumber\":\"second-retry\""))
            .count();
        assert_eq!(
            first_count, 1,
            "already-sent first line must not be retried on next cycle"
        );
        assert_eq!(
            second_count, 2,
            "second line should be attempted once, then retried once"
        );
        drop(requests);
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
                "call_id": "retry-success",
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
    async fn flush_processing_file_skips_invalid_payload_line_without_exposing_payload() {
        let temp = tempfile::tempdir().expect("tempdir should be creatable");
        let queue_file = temp.path().join("pending.jsonl");
        let processing_file = processing_file_path(queue_file.as_path());
        let valid = json!({
            "call_id": "valid-after-invalid",
            "callerNumber": "valid-after-invalid",
            "trigger": "direct",
            "receivedAt": "2026-02-28T00:00:00Z"
        });
        let content = format!(
            "invalid-json-line\n{}\n",
            serde_json::to_string(&valid).expect("valid payload should serialize")
        );
        std::fs::write(&processing_file, content).expect("processing file should be writable");
        let (base_url, captured, server) = spawn_mock_server(vec![200]).await;
        let worker = test_worker(queue_file, base_url);

        worker
            .flush_processing_file(processing_file.as_path())
            .await
            .expect("invalid line should be skipped and valid line should be sent");

        let requests = captured.lock().expect("captured lock should be available");
        assert_eq!(requests.len(), 1);
        assert!(requests[0]
            .body
            .contains("\"callerNumber\":\"valid-after-invalid\""));
        drop(requests);
        assert!(
            !processing_file.exists(),
            "processing file should be removed after skipping invalid and sending remaining"
        );
        server.abort();
    }

    #[tokio::test]
    async fn flush_processing_file_skips_line_missing_call_id() {
        let temp = tempfile::tempdir().expect("tempdir should be creatable");
        let queue_file = temp.path().join("pending.jsonl");
        let processing_file = processing_file_path(queue_file.as_path());
        let missing_call_id = json!({
            "callerNumber": "missing-call-id",
            "trigger": "direct",
            "receivedAt": "2026-02-28T00:00:00Z"
        });
        let valid = json!({
            "call_id": "valid-after-missing-call-id",
            "callerNumber": "valid-after-missing-call-id",
            "trigger": "direct",
            "receivedAt": "2026-02-28T00:00:01Z"
        });
        let content = format!(
            "{}\n{}\n",
            serde_json::to_string(&missing_call_id)
                .expect("missing-call-id payload should serialize"),
            serde_json::to_string(&valid).expect("valid payload should serialize")
        );
        std::fs::write(&processing_file, content).expect("processing file should be writable");
        let (base_url, captured, server) = spawn_mock_server(vec![200]).await;
        let worker = test_worker(queue_file, base_url);

        worker
            .flush_processing_file(processing_file.as_path())
            .await
            .expect("line missing call_id should be skipped and valid line should be sent");

        let requests = captured.lock().expect("captured lock should be available");
        assert_eq!(requests.len(), 1);
        assert!(requests[0]
            .body
            .contains("\"call_id\":\"valid-after-missing-call-id\""));
        assert!(requests[0]
            .body
            .contains("\"callerNumber\":\"valid-after-missing-call-id\""));
        drop(requests);
        assert!(
            !processing_file.exists(),
            "processing file should be removed after skipping missing-call-id line and sending remaining"
        );
        server.abort();
    }
}
