use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::{mpsc, Mutex};
use tokio::time::{Duration, Instant};
use tokio_rustls::TlsAcceptor;

use crate::config::{self, RtpConfig};
use crate::rtp::rtcp::RtcpEventTx;
use crate::rtp::rx::{RawRtp, RtpReceiver};
use crate::entities::CallId;
use crate::ports::session_lookup::SessionLookup;
use crate::transport::{tls, ConnId, TransportPeer, TransportSendRequest};

/// packet層 → SIP層 に渡す入力
#[derive(Debug, Clone)]
pub struct SipInput {
    pub peer: TransportPeer,
    pub data: Vec<u8>,
}

/// RTP送信元アドレス → call_id のマップ（同時通話対応）
pub type RtpPortMap = Arc<Mutex<HashMap<SocketAddr, CallId>>>;

#[derive(Clone)]
struct TcpConn {
    peer: SocketAddr,
    tx: mpsc::Sender<Vec<u8>>,
}

const TCP_WRITE_CHANNEL_CAPACITY: usize = 256;

type TcpConnMap = Arc<Mutex<HashMap<ConnId, TcpConn>>>;

/// Run the packet I/O loop handling SIP and RTP transport.
///
/// This function binds the provided sockets and spawns background tasks to:
/// - receive SIP messages over UDP and forward them as `SipInput` to `sip_tx`,
/// - accept and handle optional SIP TCP and TLS connections, and
/// - receive RTP packets and dispatch them to the RTP receiver.
///
/// The function returns when the main SIP UDP and RTP UDP tasks complete or on error during setup.
///
/// # Examples
///
/// ```no_run
/// use tokio::net::{UdpSocket, TcpListener};
/// use tokio::sync::mpsc;
/// use std::net::SocketAddr;
///
/// #[tokio::test]
/// async fn spawn_packet_loop_smoke() {
///     let sip_sock = UdpSocket::bind(("127.0.0.1", 0)).await.unwrap();
///     let rtp_sock = UdpSocket::bind(("127.0.0.1", 0)).await.unwrap();
///     let (sip_tx, _sip_rx) = mpsc::channel(16);
///     let (send_tx, send_rx) = mpsc::channel(16);
///     // Minimal placeholders for session_lookup, rtp_port_map, rtcp_tx
///     let session_lookup: std::sync::Arc<dyn crate::ports::session_lookup::SessionLookup> =
///         std::sync::Arc::new(crate::session::SessionRegistry::new());
///     let rtp_port_map = std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
///     let rtcp_tx = None;
///     let rtp_cfg = crate::config::rtp_config().clone();
///     let tcp_idle = crate::config::timeouts().sip_tcp_idle;
///
///     // Run the loop in background; this will return quickly in tests when sockets are dropped.
///     let _handle = tokio::spawn(async move {
///         let _ = crate::packet::run_packet_loop(
///             sip_sock,
///             None::<TcpListener>,
///             rtp_sock,
///             sip_tx,
///             send_rx,
///             session_lookup,
///             rtp_port_map,
///             rtcp_tx,
///             rtp_cfg,
///             tcp_idle,
///         )
///         .await;
///     });
/// }
/// ```
pub async fn run_packet_loop(
    sip_sock: UdpSocket,
    sip_tcp_listener: Option<TcpListener>,
    rtp_sock: UdpSocket,
    sip_tx: mpsc::Sender<SipInput>,
    mut sip_send_rx: tokio::sync::mpsc::Receiver<TransportSendRequest>,
    session_lookup: Arc<dyn SessionLookup>,
    rtp_port_map: RtpPortMap,
    rtcp_tx: Option<RtcpEventTx>,
    rtp_cfg: RtpConfig,
    tcp_idle: Duration,
) -> std::io::Result<()> {
    let _sip_port = sip_sock.local_addr()?.port();
    let _rtp_port = rtp_sock.local_addr()?.port();

    let tcp_conns: TcpConnMap = Arc::new(Mutex::new(HashMap::new()));
    let conn_seq = Arc::new(AtomicU64::new(1));
    let rtp_rx = RtpReceiver::new(session_lookup.clone(), rtp_port_map.clone(), rtcp_tx, rtp_cfg);

    if let Some(listener) = sip_tcp_listener {
        let sip_tx = sip_tx.clone();
        let tcp_conns = tcp_conns.clone();
        let conn_seq = conn_seq.clone();
        tokio::spawn(async move {
            if let Err(e) =
                run_sip_tcp_accept_loop(listener, sip_tx, tcp_conns, conn_seq, tcp_idle).await
            {
                log::error!("[packet] SIP TCP loop error: {:?}", e);
            }
        });
    }

    if let Some(settings) = config::tls_settings() {
        let acceptor = tls::build_tls_acceptor(settings)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let listener = TcpListener::bind((settings.bind_ip.as_str(), settings.port)).await?;
        let sip_tx = sip_tx.clone();
        let tcp_conns = tcp_conns.clone();
        let conn_seq = conn_seq.clone();
        tokio::spawn(async move {
            if let Err(e) =
                run_sip_tls_accept_loop(listener, acceptor, sip_tx, tcp_conns, conn_seq, tcp_idle)
                    .await
            {
                log::error!("[packet] SIP TLS loop error: {:?}", e);
            }
        });
    }

    let sip_task = tokio::spawn(async move {
        run_sip_udp_loop(sip_sock, sip_tx, &mut sip_send_rx, tcp_conns).await
    });
    let rtp_task = tokio::spawn(run_rtp_udp_loop(rtp_sock, rtp_rx));

    let (_r1, _r2) = tokio::join!(sip_task, rtp_task);
    Ok(())
}

/// SIP用 UDP ループ
async fn run_sip_udp_loop(
    sock: UdpSocket,
    sip_tx: mpsc::Sender<SipInput>,
    sip_send_rx: &mut mpsc::Receiver<TransportSendRequest>,
    tcp_conns: TcpConnMap,
) -> std::io::Result<()> {
    let local_addr = sock.local_addr()?;
    let local_port = local_addr.port();
    let mut buf = vec![0u8; 2048];

    loop {
        tokio::select! {
            recv_res = sock.recv_from(&mut buf) => {
                let (len, src) = recv_res?;
                let data = buf[..len].to_vec();

                log::info!("[sip <-] {} -> {} len={}", src, local_addr, data.len());

                // ここではSIP判定をせず「SIPポートで受けたUDP=全てSIP」とする
                let input = SipInput {
                    peer: TransportPeer::Udp(src),
                    data,
                };
                if let Err(e) = sip_tx.try_send(input) {
                    log::warn!("[packet] SIP input dropped (channel full): {:?}", e);
                }
            }
            Some(req) = sip_send_rx.recv() => {
                match req.peer {
                    TransportPeer::Udp(dst) => {
                        // 現状は単一ソケット運用のため src_port は informational
                        if req.src_port != local_port {
                            log::debug!("[sip send] requested src_port {} differs from bound {}", req.src_port, local_port);
                        }
                        log::info!(
                            "[sip ->] {} -> {} len={}",
                            local_addr,
                            dst,
                            req.payload.len()
                        );
                        let _ = sock.send_to(&req.payload, dst).await.ok();
                    }
                    TransportPeer::Tcp(conn_id) => {
                        let tx = {
                            let map = tcp_conns.lock().await;
                            map.get(&conn_id).map(|conn| conn.tx.clone())
                        };
                        if let Some(tx) = tx {
                            let _ = tx.try_send(req.payload);
                        } else {
                            log::warn!("[sip send] unknown tcp conn_id={}", conn_id);
                        }
                    }
                }
            }
        }
    }
}

/// RTP用 UDP ループ
///
/// 責務: UDPソケットからの受信・簡易RTPパース・sessionへの直接通知のみ。
/// ここでは rtp モジュールのストリーム管理/RTCP は未導入で、将来の委譲前提で現挙動を維持する。
async fn run_rtp_udp_loop(sock: UdpSocket, rtp_rx: RtpReceiver) -> std::io::Result<()> {
    let local_port = sock.local_addr()?.port();
    log::info!("[packet] RTP socket bound on port {}", local_port);

    let mut buf = vec![0u8; 2048];

    loop {
        let (len, src) = sock.recv_from(&mut buf).await?;
        let data = buf[..len].to_vec();

        let raw = RawRtp {
            src,
            dst_port: local_port,
            data,
        };

        // rtp レイヤへ委譲（解析と session への転送）
        rtp_rx.handle_raw(raw).await;
    }
}

/// Accepts incoming SIP TCP connections and spawns a per-connection handler for each.
///
/// The function retrieves the listener's local address for logging, then repeatedly accepts
/// new TCP streams. Each accepted connection is assigned a unique connection id and handled
/// in a dedicated tokio task that runs the connection lifecycle and forwards complete SIP
/// messages to the SIP processing channel.
///
/// # Returns
///
/// `Ok(())` when the listener local address is obtained and the accept loop is running; returns
/// an `Err(std::io::Error)` if obtaining the local address or accepting a connection fails.
///
/// # Examples
///
/// ```
/// # use tokio::net::TcpListener;
/// # use tokio::sync::mpsc;
/// # use std::sync::{Arc, atomic::AtomicU64};
/// # use std::time::Duration;
/// # async fn example() -> std::io::Result<()> {
/// let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
/// let (sip_tx, _sip_rx) = mpsc::channel(16);
/// // Placeholder for the real TcpConnMap type used by the library:
/// let tcp_conns = Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
/// let conn_seq = Arc::new(AtomicU64::new(1));
/// tokio::spawn(run_sip_tcp_accept_loop(
///     listener,
///     sip_tx,
///     tcp_conns,
///     conn_seq,
///     Duration::from_secs(60),
/// ));
/// Ok(())
/// # }
/// ```
async fn run_sip_tcp_accept_loop(
    listener: TcpListener,
    sip_tx: mpsc::Sender<SipInput>,
    tcp_conns: TcpConnMap,
    conn_seq: Arc<AtomicU64>,
    idle_timeout: Duration,
) -> std::io::Result<()> {
    let local_addr = listener.local_addr()?;
    log::info!("[packet] SIP TCP listener bound on {}", local_addr);

    loop {
        let (stream, peer) = listener.accept().await?;
        let conn_id = conn_seq.fetch_add(1, Ordering::Relaxed);
        log::info!("[sip tcp] accepted conn_id={} peer={}", conn_id, peer);

        let sip_tx = sip_tx.clone();
        let tcp_conns = tcp_conns.clone();
        tokio::spawn(async move {
            if let Err(e) =
                handle_sip_tcp_conn(conn_id, peer, stream, sip_tx, tcp_conns, idle_timeout).await
            {
                log::warn!("[sip tcp] conn_id={} error: {:?}", conn_id, e);
            }
        });
    }
}

async fn handle_sip_tcp_conn(
    conn_id: ConnId,
    peer: SocketAddr,
    stream: TcpStream,
    sip_tx: mpsc::Sender<SipInput>,
    tcp_conns: TcpConnMap,
    idle_timeout: Duration,
) -> std::io::Result<()> {
    handle_sip_stream_conn(
        conn_id,
        peer,
        stream,
        sip_tx,
        tcp_conns,
        idle_timeout,
        "tcp",
    )
    .await
}

async fn run_sip_tls_accept_loop(
    listener: TcpListener,
    acceptor: TlsAcceptor,
    sip_tx: mpsc::Sender<SipInput>,
    tcp_conns: TcpConnMap,
    conn_seq: Arc<AtomicU64>,
    idle_timeout: Duration,
) -> std::io::Result<()> {
    let local_addr = listener.local_addr()?;
    log::info!("[packet] SIP TLS listener bound on {}", local_addr);

    loop {
        let (stream, peer) = listener.accept().await?;
        let conn_id = conn_seq.fetch_add(1, Ordering::Relaxed);
        log::info!("[sip tls] accepted conn_id={} peer={}", conn_id, peer);

        let sip_tx = sip_tx.clone();
        let tcp_conns = tcp_conns.clone();
        let acceptor = acceptor.clone();
        tokio::spawn(async move {
            match acceptor.accept(stream).await {
                Ok(tls_stream) => {
                    if let Err(e) = handle_sip_stream_conn(
                        conn_id,
                        peer,
                        tls_stream,
                        sip_tx,
                        tcp_conns,
                        idle_timeout,
                        "tls",
                    )
                    .await
                    {
                        log::warn!("[sip tls] conn_id={} error: {:?}", conn_id, e);
                    }
                }
                Err(e) => {
                    log::warn!("[sip tls] conn_id={} handshake error: {:?}", conn_id, e);
                }
            }
        });
    }
}

async fn handle_sip_stream_conn<S>(
    conn_id: ConnId,
    peer: SocketAddr,
    stream: S,
    sip_tx: mpsc::Sender<SipInput>,
    tcp_conns: TcpConnMap,
    idle_timeout: Duration,
    label: &'static str,
) -> std::io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let (mut reader, mut writer) = tokio::io::split(stream);
    let (write_tx, mut write_rx) = mpsc::channel::<Vec<u8>>(TCP_WRITE_CHANNEL_CAPACITY);
    tcp_conns
        .lock()
        .await
        .insert(conn_id, TcpConn { peer, tx: write_tx });

    const MAX_BUFFER: usize = 256 * 1024;
    let mut buf: Vec<u8> = Vec::new();
    let mut tmp = vec![0u8; 4096];
    let mut idle_deadline = Instant::now() + idle_timeout;

    loop {
        let idle_sleep = tokio::time::sleep_until(idle_deadline);
        tokio::pin!(idle_sleep);
        tokio::select! {
            _ = &mut idle_sleep => {
                log::info!("[sip {}] idle timeout conn_id={} peer={}", label, conn_id, peer);
                break;
            }
            read_res = reader.read(&mut tmp) => {
                match read_res {
                    Ok(0) => {
                        log::info!("[sip {}] conn_id={} closed by peer {}", label, conn_id, peer);
                        break;
                    }
                    Ok(n) => {
                        buf.extend_from_slice(&tmp[..n]);
                        idle_deadline = Instant::now() + idle_timeout;
                        if buf.len() > MAX_BUFFER {
                            log::warn!(
                                "[sip {}] conn_id={} buffer overflow ({} bytes)",
                                label,
                                conn_id,
                                buf.len()
                            );
                            break;
                        }
                        for msg in extract_sip_messages(&mut buf) {
                            log::info!(
                                "[sip <-] {} conn_id={} peer={} len={}",
                                label,
                                conn_id,
                                peer,
                                msg.len()
                            );
                            let input = SipInput {
                                peer: TransportPeer::Tcp(conn_id),
                                data: msg,
                            };
                            if let Err(e) = sip_tx.try_send(input) {
                                log::warn!(
                                    "[sip {}] conn_id={} sip input dropped (channel full): {:?}",
                                    label,
                                    conn_id,
                                    e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!(
                            "[sip {}] conn_id={} read error: {:?}",
                            label,
                            conn_id,
                            e
                        );
                        break;
                    }
                }
            }
            Some(payload) = write_rx.recv() => {
                if let Err(e) = writer.write_all(&payload).await {
                    log::warn!(
                        "[sip {}] conn_id={} write error: {:?}",
                        label,
                        conn_id,
                        e
                    );
                    break;
                }
            }
            else => break,
        }
    }

    tcp_conns.lock().await.remove(&conn_id);
    Ok(())
}

fn extract_sip_messages(buf: &mut Vec<u8>) -> Vec<Vec<u8>> {
    let mut messages = Vec::new();
    loop {
        let Some(header_end) = find_header_end(buf) else {
            break;
        };
        let header_len = header_end + 4;
        let content_len = parse_content_length(&buf[..header_len]).unwrap_or(0);
        let total_len = header_len.saturating_add(content_len);
        if buf.len() < total_len {
            break;
        }
        let msg = buf.drain(..total_len).collect::<Vec<u8>>();
        messages.push(msg);
    }
    messages
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

fn parse_content_length(header_bytes: &[u8]) -> Option<usize> {
    let text = std::str::from_utf8(header_bytes).ok()?;
    for line in text.split("\r\n") {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.trim().eq_ignore_ascii_case("Content-Length") {
            return value.trim().parse::<usize>().ok();
        }
    }
    Some(0)
}
