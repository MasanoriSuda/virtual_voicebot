use std::path::{Component, Path, PathBuf};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// 録音ファイルを静的配信するシンプルなHTTPサーバ。
/// GET /recordings/<callId>/mixed.wav のようなパスだけを扱う。
pub async fn spawn_recording_server(bind: &str, base_dir: PathBuf) {
    let bind = bind.to_string();
    tokio::spawn(async move {
        if let Err(e) = run(&bind, base_dir).await {
            eprintln!("[http] recording server error: {:?}", e);
        }
    });
}

async fn run(bind: &str, base_dir: PathBuf) -> std::io::Result<()> {
    let listener = TcpListener::bind(bind).await?;
    println!("[http] serving recordings on {}", bind);

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

    if method != "GET" || !path.starts_with("/recordings/") {
        return write_response(socket, 404, "Not Found", b"").await;
    }

    let rel = sanitize_path(path.trim_start_matches('/'));
    let file_path = base_dir.join(rel);

    match tokio::fs::read(&file_path).await {
        Ok(bytes) => write_response(socket, 200, "OK", &bytes).await,
        Err(_) => write_response(socket, 404, "Not Found", b"").await,
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
    let mut resp = Vec::new();
    resp.extend_from_slice(format!("HTTP/1.1 {} {}\r\n", status, reason).as_bytes());
    if status == 200 {
        resp.extend_from_slice(b"Content-Type: audio/wav\r\n");
    } else {
        resp.extend_from_slice(b"Content-Type: text/plain\r\n");
    }
    resp.extend_from_slice(b"Access-Control-Allow-Origin: *\r\n");
    resp.extend_from_slice(format!("Content-Length: {}\r\n", body.len()).as_bytes());
    resp.extend_from_slice(b"Connection: close\r\n\r\n");
    resp.extend_from_slice(body);
    socket.write_all(&resp).await
}
