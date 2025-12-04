use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

#![allow(dead_code)]

use tokio::net::UdpSocket;
use tokio::sync::mpsc::UnboundedSender;

use crate::session::{SessionIn, SessionMap};

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
    session_map: SessionMap,
    rtp_port_map: RtpPortMap,
) -> std::io::Result<()> {
    let sip_task = tokio::spawn(run_sip_udp_loop(sip_sock, sip_tx));
    let rtp_task = tokio::spawn(run_rtp_udp_loop(
        rtp_sock,
        session_map,
        rtp_port_map,
    ));

    let (_r1, _r2) = tokio::join!(sip_task, rtp_task);
    Ok(())
}

/// SIP用 UDP ループ
async fn run_sip_udp_loop(
    sock: UdpSocket,
    sip_tx: UnboundedSender<SipInput>,
) -> std::io::Result<()> {
    let mut buf = vec![0u8; 2048];

    loop {
        let (len, src) = sock.recv_from(&mut buf).await?;
        let data = buf[..len].to_vec();

        // ここではSIP判定をせず「SIPポートで受けたUDP=全てSIP」とする
        let input = SipInput { src, data };
        if let Err(e) = sip_tx.send(input) {
            eprintln!("[packet] failed to send to SIP handler: {:?}", e);
        }
    }
}

/// RTP用 UDP ループ
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

        // テスト用途: local_port に対応する call_id を引く
        let call_id_opt = {
            let map = rtp_port_map.lock().unwrap();
            map.get(&raw.dst_port).cloned()
        };

        if let Some(call_id) = call_id_opt {
            // 対応するセッションを探して RTP入力イベントを投げる
            let sess_tx_opt = {
                let map = session_map.lock().unwrap();
                map.get(&call_id).cloned()
            };

            if let Some(sess_tx) = sess_tx_opt {
                // ここではヘッダをパースせず、生データを丸ごと渡すだけ
                let _ = sess_tx.send(SessionIn::RtpIn {
                    ts: 0,
                    payload: raw.data.clone(),
                });
            } else {
                eprintln!(
                    "[packet] RTP for unknown session (call_id={}), from {}",
                    call_id, raw.src
                );
            }
        } else {
            // 未登録ポート → いまはログだけ
            eprintln!(
                "[packet] RTP on port {} without call_id mapping, from {}",
                raw.dst_port, raw.src
            );
        }
    }
}
