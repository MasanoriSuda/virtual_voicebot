use std::env;
use std::fs;
use std::io::Write;

use reqwest::header::RANGE;
use reqwest::StatusCode;
use tempfile::tempdir;
use tokio::net::TcpListener;

use virtual_voicebot_backend::interface::http;
use virtual_voicebot_backend::shared::logging;

struct ServerGuard(tokio::task::JoinHandle<()>);

impl Drop for ServerGuard {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[tokio::test]
async fn recording_http_e2e() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempdir()?;
    let base_dir = temp.path().join("storage/recordings");
    let log_dir = match env::var("E2E_LOG_DIR") {
        Ok(dir) => std::path::PathBuf::from(dir),
        Err(_) => temp.path().join("logs"),
    };
    let call_id = "test_call";
    let call_dir = base_dir.join(call_id);
    fs::create_dir_all(&call_dir)?;

    let total_len = 4096usize;
    let mut file = fs::File::create(call_dir.join("mixed.wav"))?;
    file.write_all(&vec![0u8; total_len])?;
    drop(file);
    fs::write(call_dir.join("caller.wav"), vec![1u8; 16])?;

    env::set_var("LOG_MODE", "file");
    env::set_var("LOG_DIR", log_dir.to_string_lossy().as_ref());
    env::set_var("LOG_FORMAT", "text");
    env::set_var("RUST_LOG", "info");
    logging::init();

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let handle = http::spawn_recording_server_with_listener(listener, base_dir).await;
    let _guard = ServerGuard(handle);

    let base_url = format!("http://{}", addr);
    let recording_url = format!("{}/recordings/{}/mixed.wav", base_url, call_id);
    let client = reqwest::Client::new();

    let res = client.head(&recording_url).send().await?;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers()
            .get("accept-ranges")
            .and_then(|v| v.to_str().ok()),
        Some("bytes")
    );
    let content_len: u64 = res
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .ok_or("missing content-length")?
        .parse()?;
    assert_eq!(content_len, total_len as u64);

    let res = client
        .get(&recording_url)
        .header(RANGE, "bytes=0-1023")
        .send()
        .await?;
    assert_eq!(res.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        res.headers()
            .get("accept-ranges")
            .and_then(|v| v.to_str().ok()),
        Some("bytes")
    );
    let content_len: u64 = res
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .ok_or("missing content-length")?
        .parse()?;
    assert_eq!(content_len, 1024);
    let expected_range = format!("bytes 0-1023/{}", total_len);
    assert_eq!(
        res.headers()
            .get("content-range")
            .and_then(|v| v.to_str().ok()),
        Some(expected_range.as_str())
    );

    let res = client
        .get(&recording_url)
        .header(RANGE, "bytes=0-")
        .send()
        .await?;
    assert!(res.status() == StatusCode::PARTIAL_CONTENT || res.status() == StatusCode::OK);
    if res.status() == StatusCode::PARTIAL_CONTENT {
        let expected_range = format!("bytes 0-{}/{}", total_len - 1, total_len);
        assert_eq!(
            res.headers()
                .get("content-range")
                .and_then(|v| v.to_str().ok()),
            Some(expected_range.as_str())
        );
    }

    let res = client
        .get(&recording_url)
        .header(RANGE, "bytes=999999999-1000000000")
        .send()
        .await?;
    assert_eq!(res.status(), StatusCode::RANGE_NOT_SATISFIABLE);
    let expected_range = format!("bytes */{}", total_len);
    assert_eq!(
        res.headers()
            .get("content-range")
            .and_then(|v| v.to_str().ok()),
        Some(expected_range.as_str())
    );

    let missing_url = format!(
        "{}/recordings/THIS_CALL_ID_DOES_NOT_EXIST/mixed.wav",
        base_url
    );
    let res = client.get(&missing_url).send().await?;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    let other_url = format!("{}/recordings/{}/caller.wav", base_url, call_id);
    let res = client.get(&other_url).send().await?;
    assert!(res.status() == StatusCode::NOT_FOUND || res.status() == StatusCode::FORBIDDEN);

    log::logger().flush();
    let log_path = log_dir.join("app.log");
    let logs = fs::read_to_string(&log_path)?;
    assert!(logs.contains("status=206"));
    assert!(logs.contains("status=404"));
    assert!(logs.contains("status=416"));
    assert!(logs.contains("call_id=test_call"));
    assert!(logs.contains("path=/recordings/test_call/mixed.wav"));
    assert!(logs.contains("path=/recordings/THIS_CALL_ID_DOES_NOT_EXIST/mixed.wav"));
    assert!(logs.contains("range=bytes=0-1023"));
    assert!(logs.contains("range=bytes=999999999-1000000000"));

    Ok(())
}
