use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use log::{debug, info, warn};
use tokio::net::UdpSocket;
use tokio::sync::mpsc::UnboundedSender;

use crate::rtp::parse_rtp_packet;
use crate::session::{SessionIn, SessionMap};
use crate::sip::{parse_sip_message, SipMessage, SipMethod};

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
    local_ip: String,
    advertised_rtp_port: u16,
) -> std::io::Result<()> {
    let sip_port = sip_sock.local_addr()?.port();
    let _rtp_port = rtp_sock.local_addr()?.port();

    let sip_task = tokio::spawn(run_sip_udp_loop(
        sip_sock,
        sip_tx,
        local_ip, // used for SIPレスポンスのSDP/Contact生成。将来的にsip側へ移譲予定。
        sip_port,
        advertised_rtp_port,
    ));
    let rtp_task = tokio::spawn(run_rtp_udp_loop(rtp_sock, session_map, rtp_port_map));

    let (_r1, _r2) = tokio::join!(sip_task, rtp_task);
    Ok(())
}

/// SIP用 UDP ループ
async fn run_sip_udp_loop(
    sock: UdpSocket,
    sip_tx: UnboundedSender<SipInput>,
    local_ip: String,
    sip_port: u16,
    advertised_rtp_port: u16,
) -> std::io::Result<()> {
    let mut buf = vec![0u8; 2048];

    loop {
        let (len, src) = sock.recv_from(&mut buf).await?;
        let data = buf[..len].to_vec();

        maybe_send_immediate_sip_response(
            &sock,
            src,
            &data,
            &local_ip,
            sip_port,
            advertised_rtp_port,
        )
        .await;

        // ここではSIP判定をせず「SIPポートで受けたUDP=全てSIP」とする
        let input = SipInput { src, data };
        if let Err(e) = sip_tx.send(input) {
            eprintln!("[packet] failed to send to SIP handler: {:?}", e);
        }
    }
}

// 現状の即時返信処理をまとめたヘルパ（後で sip/session へ移譲予定）
async fn maybe_send_immediate_sip_response(
    sock: &UdpSocket,
    src: SocketAddr,
    data: &[u8],
    local_ip: &str,
    sip_port: u16,
    advertised_rtp_port: u16,
) {
    if let Ok(text) = String::from_utf8(data.to_vec()) {
        info!("[sip recv] from {} len={}:\n{}", src, data.len(), text);
        if let Ok(SipMessage::Request(req)) = parse_sip_message(&text) {
            if matches!(req.method, SipMethod::Invite) {
                let sdp_ip = if local_ip == "0.0.0.0" {
                    src.ip().to_string()
                } else {
                    local_ip.to_string()
                };

                if let Some(resp) = build_provisional_response(&req, 100, "Trying") {
                    let _ = sock.send_to(resp.as_bytes(), src).await.ok();
                }
                if let Some(resp) = build_provisional_response(&req, 180, "Ringing") {
                    let _ = sock.send_to(resp.as_bytes(), src).await.ok();
                }
                if let Some(resp) =
                    build_final_response(&req, 200, "OK", &sdp_ip, sip_port, advertised_rtp_port)
                {
                    info!("[packet] Sending 200 OK with SDP:\n{}", resp);
                    let _ = sock.send_to(resp.as_bytes(), src).await.ok();
                }
            } else if matches!(req.method, SipMethod::Bye) {
                if let Some(resp) = build_simple_response(&req, 200, "OK") {
                    let _ = sock.send_to(resp.as_bytes(), src).await.ok();
                }
            } else if matches!(req.method, SipMethod::Register) {
                if let Some(resp) = build_simple_response(&req, 200, "OK") {
                    info!("[packet] Sending 200 OK for REGISTER to {}", src);
                    let _ = sock.send_to(resp.as_bytes(), src).await.ok();
                }
            }
        }
    }
}

fn build_provisional_response(
    req: &crate::sip::SipRequest,
    code: u16,
    reason: &str,
) -> Option<String> {
    let via = req.header_value("Via")?;
    let from = req.header_value("From")?;
    let mut to = req.header_value("To")?.to_string();
    let call_id = req.header_value("Call-ID")?;
    let cseq = req.header_value("CSeq")?;

    // provisional/2xx には To-tag を付けるのが無難
    if !to.to_ascii_lowercase().contains("tag=") {
        to = format!("{to};tag=rustbot");
    }

    Some(format!(
        "SIP/2.0 {code} {reason}\r\n\
Via: {via}\r\n\
From: {from}\r\n\
To: {to}\r\n\
Call-ID: {call_id}\r\n\
CSeq: {cseq}\r\n\
Content-Length: 0\r\n\r\n"
    ))
}

fn build_final_response(
    req: &crate::sip::SipRequest,
    code: u16,
    reason: &str,
    local_ip: &str,
    sip_port: u16,
    rtp_port: u16,
) -> Option<String> {
    let via = req.header_value("Via")?;
    let from = req.header_value("From")?;
    let mut to = req.header_value("To")?.to_string();
    let call_id = req.header_value("Call-ID")?;
    let cseq = req.header_value("CSeq")?;

    if !to.to_ascii_lowercase().contains("tag=") {
        to = format!("{to};tag=rustbot");
    }

    let sdp = format!(
        concat!(
            "v=0\r\n",
            "o=rustbot 1 1 IN IP4 {ip}\r\n",
            "s=Rust PCMU Bot\r\n",
            "c=IN IP4 {ip}\r\n",
            "t=0 0\r\n",
            "m=audio {rtp} RTP/AVP 0\r\n",
            "a=rtpmap:0 PCMU/8000\r\n",
            "a=sendrecv\r\n",
        ),
        ip = local_ip,
        rtp = rtp_port
    );

    let content_length = sdp.len();

    Some(format!(
        "SIP/2.0 {code} {reason}\r\n\
Via: {via}\r\n\
From: {from}\r\n\
To: {to}\r\n\
Call-ID: {call_id}\r\n\
CSeq: {cseq}\r\n\
Contact: <sip:rustbot@{ip}:{sport}>\r\n\
Content-Type: application/sdp\r\n\
Content-Length: {len}\r\n\r\n\
{sdp}",
        ip = local_ip,
        sport = sip_port,
        len = content_length,
        sdp = sdp
    ))
}

fn build_simple_response(req: &crate::sip::SipRequest, code: u16, reason: &str) -> Option<String> {
    let via = req.header_value("Via")?;
    let from = req.header_value("From")?;
    let mut to = req.header_value("To")?.to_string();
    let call_id = req.header_value("Call-ID")?;
    let cseq = req.header_value("CSeq")?;

    if !to.to_ascii_lowercase().contains("tag=") {
        to = format!("{to};tag=rustbot");
    }

    Some(format!(
        "SIP/2.0 {code} {reason}\r\n\
Via: {via}\r\n\
From: {from}\r\n\
To: {to}\r\n\
Call-ID: {call_id}\r\n\
CSeq: {cseq}\r\n\
Content-Length: 0\r\n\r\n"
    ))
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
