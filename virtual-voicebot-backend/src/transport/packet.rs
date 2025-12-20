use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::time::{Duration, Instant};

use crate::config;
use crate::rtp::rtcp::RtcpEventTx;
use crate::rtp::rx::{RawRtp, RtpReceiver};
use crate::session::SessionMap;
use crate::transport::{ConnId, TransportPeer, TransportSendRequest};

/// packet層 → SIP層 に渡す入力
#[derive(Debug, Clone)]
pub struct SipInput {
    pub peer: TransportPeer,
    pub data: Vec<u8>,
}

/// RTPポート → call_id のマップ
pub type RtpPortMap = Arc<Mutex<HashMap<u16, String>>>;

#[derive(Clone)]
struct TcpConn {
    peer: SocketAddr,
    tx: UnboundedSender<Vec<u8>>,
}

type TcpConnMap = Arc<Mutex<HashMap<ConnId, TcpConn>>>;

/// packet層のメインループ
///
/// - SIPソケット (5060) を受信して SipInput を送る
/// - RTPソケット (40000など) を受信して SessionIn::RtpIn を各セッションに送る
pub async fn run_packet_loop(
    sip_sock: UdpSocket,
    sip_tcp_listener: Option<TcpListener>,
    rtp_sock: UdpSocket,
    sip_tx: UnboundedSender<SipInput>,
    mut sip_send_rx: tokio::sync::mpsc::UnboundedReceiver<TransportSendRequest>,
    session_map: SessionMap,
    rtp_port_map: RtpPortMap,
    rtcp_tx: Option<RtcpEventTx>,
) -> std::io::Result<()> {
    let _sip_port = sip_sock.local_addr()?.port();
    let _rtp_port = rtp_sock.local_addr()?.port();

    let tcp_conns: TcpConnMap = Arc::new(Mutex::new(HashMap::new()));
    let conn_seq = Arc::new(AtomicU64::new(1));
    let tcp_idle = config::timeouts().sip_tcp_idle;

    let rtp_rx = RtpReceiver::new(session_map.clone(), rtp_port_map.clone(), rtcp_tx);

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

    let sip_task =
        tokio::spawn(async move {
            run_sip_udp_loop(sip_sock, sip_tx, &mut sip_send_rx, tcp_conns).await
        });
    let rtp_task = tokio::spawn(run_rtp_udp_loop(rtp_sock, rtp_rx));

    let (_r1, _r2) = tokio::join!(sip_task, rtp_task);
    Ok(())
}

/// SIP用 UDP ループ
async fn run_sip_udp_loop(
    sock: UdpSocket,
    sip_tx: UnboundedSender<SipInput>,
    sip_send_rx: &mut UnboundedReceiver<TransportSendRequest>,
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
                if let Err(e) = sip_tx.send(input) {
                    log::warn!("[packet] failed to send to SIP handler: {:?}", e);
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
                        let tx = tcp_conns
                            .lock()
                            .unwrap()
                            .get(&conn_id)
                            .map(|conn| conn.tx.clone());
                        if let Some(tx) = tx {
                            let _ = tx.send(req.payload);
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
        rtp_rx.handle_raw(raw);
    }
}

async fn run_sip_tcp_accept_loop(
    listener: TcpListener,
    sip_tx: UnboundedSender<SipInput>,
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
            if let Err(e) = handle_sip_tcp_conn(conn_id, peer, stream, sip_tx, tcp_conns, idle_timeout).await {
                log::warn!("[sip tcp] conn_id={} error: {:?}", conn_id, e);
            }
        });
    }
}

async fn handle_sip_tcp_conn(
    conn_id: ConnId,
    peer: SocketAddr,
    stream: TcpStream,
    sip_tx: UnboundedSender<SipInput>,
    tcp_conns: TcpConnMap,
    idle_timeout: Duration,
) -> std::io::Result<()> {
    let (mut reader, mut writer) = stream.into_split();
    let (write_tx, mut write_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
    tcp_conns
        .lock()
        .unwrap()
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
                log::info!("[sip tcp] idle timeout conn_id={} peer={}", conn_id, peer);
                break;
            }
            read_res = reader.read(&mut tmp) => {
                match read_res {
                    Ok(0) => {
                        log::info!("[sip tcp] conn_id={} closed by peer {}", conn_id, peer);
                        break;
                    }
                    Ok(n) => {
                        buf.extend_from_slice(&tmp[..n]);
                        idle_deadline = Instant::now() + idle_timeout;
                        if buf.len() > MAX_BUFFER {
                            log::warn!("[sip tcp] conn_id={} buffer overflow ({} bytes)", conn_id, buf.len());
                            break;
                        }
                        for msg in extract_sip_messages(&mut buf) {
                            log::info!(
                                "[sip <-] tcp conn_id={} peer={} len={}",
                                conn_id,
                                peer,
                                msg.len()
                            );
                            let input = SipInput {
                                peer: TransportPeer::Tcp(conn_id),
                                data: msg,
                            };
                            if let Err(e) = sip_tx.send(input) {
                                log::warn!("[sip tcp] conn_id={} send to sip handler failed: {:?}", conn_id, e);
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("[sip tcp] conn_id={} read error: {:?}", conn_id, e);
                        break;
                    }
                }
            }
            Some(payload) = write_rx.recv() => {
                if let Err(e) = writer.write_all(&payload).await {
                    log::warn!("[sip tcp] conn_id={} write error: {:?}", conn_id, e);
                    break;
                }
            }
            else => break,
        }
    }

    tcp_conns.lock().unwrap().remove(&conn_id);
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
