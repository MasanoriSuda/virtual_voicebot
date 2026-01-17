use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::SeekFrom;
use std::path::{Component, Path, PathBuf};

use log::info;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::config;

pub mod ingest;

/// 録音ファイルを静的配信するシンプルなHTTPサーバ。
/// GET /recordings/<callId>/mixed.wav のようなパスだけを扱う。
pub async fn spawn_recording_server(bind: &str, base_dir: PathBuf) {
    let bind = bind.to_string();
    tokio::spawn(async move {
        if let Err(e) = run(&bind, base_dir).await {
            log::error!("[http] recording server error: {:?}", e);
        }
    });
}

#[allow(dead_code)]
pub async fn spawn_recording_server_with_listener(
    listener: TcpListener,
    base_dir: PathBuf,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if let Err(e) = run_with_listener(listener, base_dir).await {
            log::error!("[http] recording server error: {:?}", e);
        }
    })
}

async fn run(bind: &str, base_dir: PathBuf) -> std::io::Result<()> {
    let listener = TcpListener::bind(bind).await?;
    log::info!("[http] serving recordings on {}", bind);

    run_with_listener(listener, base_dir).await
}

async fn run_with_listener(listener: TcpListener, base_dir: PathBuf) -> std::io::Result<()> {
    loop {
        let (mut socket, _) = listener.accept().await?;
        let base_dir = base_dir.clone();
        tokio::spawn(async move {
            let _ = handle_conn(&mut socket, &base_dir).await;
        });
    }
}

async fn handle_conn(socket: &mut tokio::net::TcpStream, base_dir: &Path) -> std::io::Result<()> {
    let mut buf = vec![0u8; 4096];
    let mut read_len = 0usize;
    loop {
        let n = socket.read(&mut buf[read_len..]).await?;
        if n == 0 {
            return Ok(());
        }
        read_len += n;
        if read_len >= 4 && buf[..read_len].windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
        if read_len == buf.len() {
            buf.resize(buf.len() + 4096, 0);
        }
        if read_len > 64 * 1024 {
            return write_response(socket, 413, "Payload Too Large", b"").await;
        }
    }

    let request = String::from_utf8_lossy(&buf[..read_len]);
    let mut lines = request.lines();
    let first_line = if let Some(l) = lines.next() {
        l
    } else {
        return Ok(());
    };
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("");
    let mut headers = HashMap::new();
    for line in lines {
        if line.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(
                name.trim().to_ascii_lowercase(),
                value.trim().to_string(),
            );
        }
    }

    let req_path = path.to_string();
    let range_value = headers.get("range").map(|v| v.as_str());

    let is_get = method == "GET";
    let is_head = method == "HEAD";
    if (!is_get && !is_head) || !path.starts_with("/recordings/") {
        log_recording_response(404, None, &req_path, range_value);
        return write_response(socket, 404, "Not Found", b"").await;
    }

    let rel = match path.strip_prefix("/recordings/") {
        Some(rest) if !rest.is_empty() => rest,
        _ => {
            log_recording_response(404, None, &req_path, range_value);
            return write_response(socket, 404, "Not Found", b"").await;
        }
    };
    let rel = sanitize_path(rel);
    if rel.components().count() != 2 || rel.file_name() != Some(OsStr::new("mixed.wav")) {
        log_recording_response(404, None, &req_path, range_value);
        return write_response(socket, 404, "Not Found", b"").await;
    }
    let call_id = rel
        .components()
        .next()
        .and_then(|comp| comp.as_os_str().to_str())
        .map(|value| value.to_string());
    let file_path = base_dir.join(&rel);

    let meta = match tokio::time::timeout(config::timeouts().recording_io, tokio::fs::metadata(&file_path)).await {
        Ok(Ok(meta)) => meta,
        Ok(Err(_)) => {
            log_recording_response(404, call_id.as_deref(), &req_path, range_value);
            return write_response(socket, 404, "Not Found", b"").await;
        }
        Err(_) => {
            log_recording_response(504, call_id.as_deref(), &req_path, range_value);
            return write_response(socket, 504, "Gateway Timeout", b"").await;
        }
    };
    let total_len = meta.len();
    if total_len == 0 {
        log_recording_response(404, call_id.as_deref(), &req_path, range_value);
        return write_response(socket, 404, "Not Found", b"").await;
    }

    if let Some(range) = range_value {
        let (start, end) = match parse_range(range, total_len) {
            Ok(r) => r,
            Err(_) => {
                log_recording_response(416, call_id.as_deref(), &req_path, range_value);
                let mut resp = Vec::new();
                resp.extend_from_slice(b"HTTP/1.1 416 Range Not Satisfiable\r\n");
                resp.extend_from_slice(b"Content-Type: text/plain\r\n");
                resp.extend_from_slice(b"Accept-Ranges: bytes\r\n");
                resp.extend_from_slice(format!("Content-Range: bytes */{}\r\n", total_len).as_bytes());
                resp.extend_from_slice(b"Access-Control-Allow-Origin: *\r\n");
                resp.extend_from_slice(b"Content-Length: 0\r\n");
                resp.extend_from_slice(b"Connection: close\r\n\r\n");
                return socket.write_all(&resp).await;
            }
        };
        let chunk_len = end.saturating_sub(start).saturating_add(1);
        if chunk_len > usize::MAX as u64 {
            log_recording_response(416, call_id.as_deref(), &req_path, range_value);
            return write_response(socket, 416, "Range Not Satisfiable", b"").await;
        }
        if is_head {
            log_recording_response(206, call_id.as_deref(), &req_path, range_value);
            let headers = [
                ("Content-Type", "audio/wav".to_string()),
                ("Accept-Ranges", "bytes".to_string()),
                (
                    "Content-Range",
                    format!("bytes {}-{}/{}", start, end, total_len),
                ),
            ];
            return write_response_with_headers(socket, 206, "Partial Content", &headers, &[], chunk_len, false).await;
        }
        let read_res = tokio::time::timeout(
            config::timeouts().recording_io,
            async {
                let mut file = tokio::fs::File::open(&file_path).await?;
                file.seek(SeekFrom::Start(start)).await?;
                let mut buf = vec![0u8; chunk_len as usize];
                file.read_exact(&mut buf).await?;
                Ok::<_, std::io::Error>(buf)
            },
        )
        .await;
        match read_res {
            Ok(Ok(bytes)) => {
                log_recording_response(206, call_id.as_deref(), &req_path, range_value);
                let headers = [
                    ("Content-Type", "audio/wav".to_string()),
                    ("Accept-Ranges", "bytes".to_string()),
                    (
                        "Content-Range",
                        format!("bytes {}-{}/{}", start, end, total_len),
                    ),
                ];
                write_response_with_headers(
                    socket,
                    206,
                    "Partial Content",
                    &headers,
                    &bytes,
                    chunk_len,
                    true,
                )
                .await
            }
            Ok(Err(_)) => {
                log_recording_response(404, call_id.as_deref(), &req_path, range_value);
                write_response(socket, 404, "Not Found", b"").await
            }
            Err(_) => {
                log_recording_response(504, call_id.as_deref(), &req_path, range_value);
                write_response(socket, 504, "Gateway Timeout", b"").await
            }
        }
    } else if is_head {
        log_recording_response(200, call_id.as_deref(), &req_path, range_value);
        let headers = [
            ("Content-Type", "audio/wav".to_string()),
            ("Accept-Ranges", "bytes".to_string()),
        ];
        write_response_with_headers(socket, 200, "OK", &headers, &[], total_len, false).await
    } else {
        match tokio::time::timeout(config::timeouts().recording_io, tokio::fs::read(&file_path)).await {
            Ok(Ok(bytes)) => {
                log_recording_response(200, call_id.as_deref(), &req_path, range_value);
                let headers = [
                    ("Content-Type", "audio/wav".to_string()),
                    ("Accept-Ranges", "bytes".to_string()),
                ];
                write_response_with_headers(
                    socket,
                    200,
                    "OK",
                    &headers,
                    &bytes,
                    bytes.len() as u64,
                    true,
                )
                .await
            }
            Ok(Err(_)) => {
                log_recording_response(404, call_id.as_deref(), &req_path, range_value);
                write_response(socket, 404, "Not Found", b"").await
            }
            Err(_) => {
                log_recording_response(504, call_id.as_deref(), &req_path, range_value);
                write_response(socket, 504, "Gateway Timeout", b"").await
            }
        }
    }
}

fn sanitize_path(p: &str) -> PathBuf {
    let mut clean = PathBuf::new();
    for comp in Path::new(p).components() {
        match comp {
            Component::Normal(c) => clean.push(c),
            _ => {}
        }
    }
    clean
}

async fn write_response(
    socket: &mut tokio::net::TcpStream,
    status: u16,
    reason: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let content_type = if status == 200 { "audio/wav" } else { "text/plain" };
    let headers = [
        ("Content-Type", content_type.to_string()),
        ("Accept-Ranges", "bytes".to_string()),
    ];
    write_response_with_headers(
        socket,
        status,
        reason,
        &headers,
        body,
        body.len() as u64,
        true,
    )
    .await
}

async fn write_response_with_headers(
    socket: &mut tokio::net::TcpStream,
    status: u16,
    reason: &str,
    headers: &[(&str, String)],
    body: &[u8],
    content_len: u64,
    write_body: bool,
) -> std::io::Result<()> {
    let mut resp = Vec::new();
    resp.extend_from_slice(format!("HTTP/1.1 {} {}\r\n", status, reason).as_bytes());
    for (name, value) in headers {
        resp.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
    }
    resp.extend_from_slice(b"Access-Control-Allow-Origin: *\r\n");
    resp.extend_from_slice(format!("Content-Length: {content_len}\r\n").as_bytes());
    resp.extend_from_slice(b"Connection: close\r\n\r\n");
    if write_body {
        resp.extend_from_slice(body);
    }
    socket.write_all(&resp).await
}

fn parse_range(value: &str, total_len: u64) -> Result<(u64, u64), ()> {
    let value = value.trim();
    let prefix = "bytes=";
    if !value.starts_with(prefix) {
        return Err(());
    }
    let range_set = &value[prefix.len()..];
    if range_set.contains(',') {
        return Err(());
    }
    let mut parts = range_set.splitn(2, '-');
    let start_str = parts.next().unwrap_or("");
    let end_str = parts.next().unwrap_or("");
    if start_str.is_empty() {
        return Err(());
    }
    let start: u64 = start_str.parse().map_err(|_| ())?;
    if start >= total_len {
        return Err(());
    }
    let mut end = if end_str.is_empty() {
        total_len.saturating_sub(1)
    } else {
        end_str.parse().map_err(|_| ())?
    };
    if end < start {
        return Err(());
    }
    if end >= total_len {
        end = total_len - 1;
    }
    Ok((start, end))
}

fn log_recording_response(status: u16, call_id: Option<&str>, path: &str, range: Option<&str>) {
    let call_id = call_id.unwrap_or("-");
    let range = range.unwrap_or("-");
    info!(
        "recording_access status={} call_id={} path={} range={}",
        status, call_id, path, range
    );
}
