use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use tokio::net::UdpSocket;
use tokio::sync::mpsc::UnboundedSender;

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
        local_ip,
        sip_port,
        advertised_rtp_port,
    ));
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
    local_ip: String,
    sip_port: u16,
    advertised_rtp_port: u16,
) -> std::io::Result<()> {
    let mut buf = vec![0u8; 2048];

    loop {
        let (len, src) = sock.recv_from(&mut buf).await?;
        let data = buf[..len].to_vec();

        // INVITE を受信したら 100/180 を即返信する（main側の処理とは独立）
        if let Ok(text) = String::from_utf8(data.clone()) {
            if let Ok(SipMessage::Request(req)) = parse_sip_message(&text) {
                if matches!(req.method, SipMethod::Invite) {
                    let sdp_ip = if local_ip == "0.0.0.0" {
                        src.ip().to_string()
                    } else {
                        local_ip.clone()
                    };

                    if let Some(resp) =
                        build_provisional_response(&req, 100, "Trying")
                    {
                        let _ = sock.send_to(resp.as_bytes(), src).await.ok();
                    }
                    if let Some(resp) =
                        build_provisional_response(&req, 180, "Ringing")
                    {
                        let _ = sock.send_to(resp.as_bytes(), src).await.ok();
                    }
                    if let Some(resp) =
                        build_final_response(&req, 200, "OK", &sdp_ip, sip_port, advertised_rtp_port)
                    {
                        let _ = sock.send_to(resp.as_bytes(), src).await.ok();
                    }
                } else if matches!(req.method, SipMethod::Bye) {
                    if let Some(resp) = build_simple_response(&req, 200, "OK") {
                        let _ = sock.send_to(resp.as_bytes(), src).await.ok();
                    }
                }
            }
        }

        // ここではSIP判定をせず「SIPポートで受けたUDP=全てSIP」とする
        let input = SipInput { src, data };
        if let Err(e) = sip_tx.send(input) {
            eprintln!("[packet] failed to send to SIP handler: {:?}", e);
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

    let content_length = sdp.as_bytes().len();

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

fn build_simple_response(
    req: &crate::sip::SipRequest,
    code: u16,
    reason: &str,
) -> Option<String> {
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
