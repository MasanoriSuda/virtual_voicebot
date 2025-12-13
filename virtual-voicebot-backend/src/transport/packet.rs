use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use log::info;
use tokio::net::UdpSocket;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::rtp::rtcp::RtcpEventTx;
use crate::rtp::rx::{RawRtp, RtpReceiver};
use crate::session::SessionMap;
use crate::transport::TransportSendRequest;

/// packet層 → SIP層 に渡す入力
#[derive(Debug, Clone)]
pub struct SipInput {
    pub src: SocketAddr,
    pub data: Vec<u8>,
}

/// RTPポート → call_id のマップ
pub type RtpPortMap = Arc<Mutex<HashMap<u16, String>>>;

/// packet層のメインループ
///
/// - SIPソケット (5060) を受信して SipInput を送る
/// - RTPソケット (40000など) を受信して SessionIn::RtpIn を各セッションに送る
pub async fn run_packet_loop(
    sip_sock: UdpSocket,
    rtp_sock: UdpSocket,
    sip_tx: UnboundedSender<SipInput>,
    mut sip_send_rx: tokio::sync::mpsc::UnboundedReceiver<TransportSendRequest>,
    session_map: SessionMap,
    rtp_port_map: RtpPortMap,
    rtcp_tx: Option<RtcpEventTx>,
) -> std::io::Result<()> {
    let _sip_port = sip_sock.local_addr()?.port();
    let _rtp_port = rtp_sock.local_addr()?.port();

    let rtp_rx = RtpReceiver::new(session_map.clone(), rtp_port_map.clone(), rtcp_tx);

    let sip_task =
        tokio::spawn(async move { run_sip_udp_loop(sip_sock, sip_tx, &mut sip_send_rx).await });
    let rtp_task = tokio::spawn(run_rtp_udp_loop(rtp_sock, rtp_rx));

    let (_r1, _r2) = tokio::join!(sip_task, rtp_task);
    Ok(())
}

/// SIP用 UDP ループ
async fn run_sip_udp_loop(
    sock: UdpSocket,
    sip_tx: UnboundedSender<SipInput>,
    sip_send_rx: &mut UnboundedReceiver<TransportSendRequest>,
) -> std::io::Result<()> {
    let local_port = sock.local_addr()?.port();
    let mut buf = vec![0u8; 2048];

    loop {
        tokio::select! {
            recv_res = sock.recv_from(&mut buf) => {
                let (len, src) = recv_res?;
                let data = buf[..len].to_vec();

                info!("[sip <-] from {} len={}", src, data.len());

                // ここではSIP判定をせず「SIPポートで受けたUDP=全てSIP」とする
                let input = SipInput { src, data };
                if let Err(e) = sip_tx.send(input) {
                    eprintln!("[packet] failed to send to SIP handler: {:?}", e);
                }
            }
            Some(req) = sip_send_rx.recv() => {
                // 現状は単一ソケット運用のため src_port は informational
                if req.src_port != local_port {
                    log::debug!("[sip send] requested src_port {} differs from bound {}", req.src_port, local_port);
                }
                let _ = sock.send_to(&req.payload, req.dst).await.ok();
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
    println!("[packet] RTP socket bound on port {}", local_port);

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
