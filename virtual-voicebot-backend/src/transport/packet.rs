use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use log::{debug, info, warn};
use tokio::net::UdpSocket;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::rtp::parse_rtp_packet;
use crate::session::{SessionIn, SessionMap};
use crate::sip::tx::SipTransportRequest;

/// UDPで受けた「生パケット」
#[derive(Debug, Clone)]
pub struct RawPacket {
    pub src: SocketAddr,
    pub dst_port: u16,
    pub data: Vec<u8>,
}

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
    mut sip_send_rx: tokio::sync::mpsc::UnboundedReceiver<crate::sip::tx::SipTransportRequest>,
    session_map: SessionMap,
    rtp_port_map: RtpPortMap,
) -> std::io::Result<()> {
    let _sip_port = sip_sock.local_addr()?.port();
    let _rtp_port = rtp_sock.local_addr()?.port();

    let sip_task =
        tokio::spawn(async move { run_sip_udp_loop(sip_sock, sip_tx, &mut sip_send_rx).await });
    let rtp_task = tokio::spawn(run_rtp_udp_loop(rtp_sock, session_map, rtp_port_map));

    let (_r1, _r2) = tokio::join!(sip_task, rtp_task);
    Ok(())
}

/// SIP用 UDP ループ
async fn run_sip_udp_loop(
    sock: UdpSocket,
    sip_tx: UnboundedSender<SipInput>,
    sip_send_rx: &mut UnboundedReceiver<SipTransportRequest>,
) -> std::io::Result<()> {
    let mut buf = vec![0u8; 2048];

    loop {
        tokio::select! {
            recv_res = sock.recv_from(&mut buf) => {
                let (len, src) = recv_res?;
                let data = buf[..len].to_vec();

                info!("[sip recv] from {} len={}", src, data.len());

                // ここではSIP判定をせず「SIPポートで受けたUDP=全てSIP」とする
                let input = SipInput { src, data };
                if let Err(e) = sip_tx.send(input) {
                    eprintln!("[packet] failed to send to SIP handler: {:?}", e);
                }
            }
            Some(req) = sip_send_rx.recv() => {
                let _ = sock.send_to(&req.payload, req.dst).await.ok();
            }
        }
    }
}

/// RTP用 UDP ループ
///
/// 責務: UDPソケットからの受信・簡易RTPパース・sessionへの直接通知のみ。
/// ここでは rtp モジュールのストリーム管理/RTCP は未導入で、将来の委譲前提で現挙動を維持する。
async fn run_rtp_udp_loop(
    sock: UdpSocket,
    session_map: SessionMap,
    rtp_port_map: RtpPortMap,
) -> std::io::Result<()> {
    let local_port = sock.local_addr()?.port();
    println!("[packet] RTP socket bound on port {}", local_port);

    let mut buf = vec![0u8; 2048];

    loop {
        let (len, src) = sock.recv_from(&mut buf).await?;
        let data = buf[..len].to_vec();

        let raw = RawPacket {
            src,
            dst_port: local_port,
            data,
        };

        // テスト用途: local_port に対応する call_id を引く（rtp モジュール委譲前の暫定マップ）
        let call_id_opt = {
            let map = rtp_port_map.lock().unwrap();
            map.get(&raw.dst_port).cloned()
        };

        if let Some(call_id) = call_id_opt {
            // 対応するセッションを探して RTP入力イベントを投げる（rtp→session 経由は後続タスク）
            let sess_tx_opt = {
                let map = session_map.lock().unwrap();
                map.get(&call_id).cloned()
            };

            if let Some(sess_tx) = sess_tx_opt {
                match parse_rtp_packet(&raw.data) {
                    Ok(pkt) => {
                        debug!(
                            "[packet] RTP len={} from {} mapped to call_id={} pt={} seq={}",
                            len, raw.src, call_id, pkt.payload_type, pkt.sequence_number
                        );
                        let _ = sess_tx.send(SessionIn::RtpIn {
                            ts: pkt.timestamp,
                            payload: pkt.payload,
                        });
                    }
                    Err(e) => {
                        warn!(
                            "[packet] RTP parse error for call_id={} from {}: {:?}",
                            call_id, raw.src, e
                        );
                    }
                }
            } else {
                warn!(
                    "[packet] RTP for unknown session (call_id={}), from {}",
                    call_id, raw.src
                );
            }
        } else {
            // 未登録ポート → いまはログだけ
            warn!(
                "[packet] RTP on port {} without call_id mapping, from {}",
                raw.dst_port, raw.src
            );
        }
    }
}
