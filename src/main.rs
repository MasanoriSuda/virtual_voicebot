mod packet;
mod session;
mod sip;
mod rtp;
mod bot;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::net::UdpSocket;
use tokio::sync::mpsc::unbounded_channel;

use crate::packet::{run_packet_loop, RtpPortMap, SipInput};
use crate::session::{spawn_session, MediaConfig, SessionIn, SessionMap};
use crate::sip::{process_sip_datagram, SipEvent};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let sip_bind_ip =
        std::env::var("SIP_BIND_IP").unwrap_or_else(|_| "0.0.0.0".to_string());
    let sip_port = std::env::var("SIP_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5060);
    let rtp_port_cfg = std::env::var("RTP_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10000);

    // --- セッションとRTPポート管理の共有マップ ---
    let session_map: SessionMap = Arc::new(Mutex::new(HashMap::new()));
    let rtp_port_map: RtpPortMap = Arc::new(Mutex::new(HashMap::new()));

    // packet層 → SIP処理ループ へのチャネル
    let (sip_tx, mut sip_rx) = unbounded_channel::<SipInput>();

    // --- ソケット準備 (SIP/RTPポートは環境変数で指定) ---
    let sip_sock = UdpSocket::bind((sip_bind_ip.as_str(), sip_port)).await?;
    let rtp_sock = UdpSocket::bind(("0.0.0.0", rtp_port_cfg)).await?;
    let rtp_port = rtp_sock.local_addr()?.port();
    let local_ip = std::env::var("LOCAL_IP").unwrap_or_else(|_| "0.0.0.0".to_string());
    let advertised_ip = std::env::var("ADVERTISED_IP").unwrap_or_else(|_| local_ip.clone());
    let advertised_rtp_port = std::env::var("ADVERTISED_RTP_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(rtp_port);
    let local_ip_for_packet = advertised_ip.clone();

    println!(
        "Listening SIP on {}, RTP on {}",
        sip_sock.local_addr()?,
        rtp_sock.local_addr()?
    );

    // packetループ起動（UDP受信 → SIP/RTP振り分け → セッションへ）
    {
        let session_map_for_packet = session_map.clone();
        let rtp_port_map_for_packet = rtp_port_map.clone();
        tokio::spawn(async move {
            if let Err(e) = run_packet_loop(
                sip_sock,
                rtp_sock,
                sip_tx,
                session_map_for_packet,
                rtp_port_map_for_packet,
                local_ip_for_packet,
                advertised_rtp_port,
            )
            .await
            {
                eprintln!("[packet] loop error: {:?}", e);
            }
        });
    }

    // --- SIP処理ループ: packet層からのSIP入力をセッションへ結線 ---
    while let Some(input) = sip_rx.recv().await {
        let events = process_sip_datagram(&input);
        for ev in events {
            match ev {
                SipEvent::IncomingInvite {
                    call_id,
                    from,
                    to,
                    offer,
                } => {
                    println!("[main] new INVITE, call_id={}", call_id);

                    // セッション生成
                    let sess_tx = spawn_session(
                        call_id.clone(),
                        session_map.clone(),
                        MediaConfig::pcmu(local_ip.clone(), rtp_port),
                    );

                    // RTPポートとcall_idを紐付け
                    {
                        let mut map = rtp_port_map.lock().unwrap();
                        map.insert(rtp_port, call_id.clone());
                    }

                    // セッションに Invite イベントを送る
                    let _ = sess_tx.send(SessionIn::Invite {
                        call_id,
                        from,
                        to,
                        offer,
                    });
                }
                SipEvent::Ack { call_id } => {
                    println!("[main] ACK for call_id={}", call_id);
                    if let Some(sess_tx) = session_map.lock().unwrap().get(&call_id).cloned() {
                        let _ = sess_tx.send(SessionIn::Ack);
                    }
                }
                SipEvent::Bye { call_id } => {
                    println!("[main] BYE for call_id={}", call_id);
                    if let Some(sess_tx) = session_map.lock().unwrap().get(&call_id).cloned() {
                        let _ = sess_tx.send(SessionIn::Bye);
                    }
                }
                SipEvent::Unknown => {
                    println!("[main] Unknown / unsupported SIP message");
                }
            }
        }
    }

    Ok(())
}
