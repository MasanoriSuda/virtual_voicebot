mod ai;
mod app;
mod rtp;
mod session;
mod sip;
mod transport;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::net::UdpSocket;
use tokio::sync::mpsc::unbounded_channel;

use crate::rtp::tx::RtpTxHandle;
use crate::session::{
    spawn_session, MediaConfig, SessionIn, SessionMap, SessionOut, SessionRegistry,
};
use crate::sip::{SipConfig, SipCore, SipEvent};
use crate::transport::{run_packet_loop, RtpPortMap, SipInput, TransportSendRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let sip_bind_ip = std::env::var("SIP_BIND_IP").unwrap_or_else(|_| "0.0.0.0".to_string());
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
    let session_registry = SessionRegistry::new(session_map.clone());
    let rtp_port_map: RtpPortMap = Arc::new(Mutex::new(HashMap::new()));
    let mut rtp_handles: HashMap<String, RtpTxHandle> = HashMap::new();

    // packet層 → SIP処理ループ へのチャネル
    let (sip_tx, mut sip_rx) = unbounded_channel::<SipInput>();
    // sip → transport 送信指示
    let (sip_send_tx, sip_send_rx) = unbounded_channel::<TransportSendRequest>();
    // session → sip 指示
    let (session_out_tx, mut session_out_rx) =
        unbounded_channel::<(crate::session::types::CallId, SessionOut)>();

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
                sip_send_rx,
                session_map_for_packet,
                rtp_port_map_for_packet,
                None,
            )
            .await
            {
                eprintln!("[packet] loop error: {:?}", e);
            }
        });
    }

    // --- SIP処理ループ: packet層からのSIP入力をセッションへ結線 ---
    let mut sip_core = SipCore::new(
        SipConfig {
            advertised_ip: advertised_ip.clone(),
            sip_port,
            advertised_rtp_port,
        },
        sip_send_tx,
    );
    loop {
        tokio::select! {
            Some(input) = sip_rx.recv() => {
                let events = sip_core.handle_input(&input);
                for ev in events {
                    match ev {
                        SipEvent::IncomingInvite { call_id, from, to, offer } => {
                            println!("[main] new INVITE, call_id={}", call_id);

                            let rtp_handle = RtpTxHandle::new();
                            let sess_tx = spawn_session(
                                call_id.clone(),
                                session_registry.clone(),
                                MediaConfig::pcmu(advertised_ip.clone(), rtp_port),
                                session_out_tx.clone(),
                                rtp_handle.clone(),
                            );

                            {
                                let mut map = rtp_port_map.lock().unwrap();
                                map.insert(rtp_port, call_id.clone());
                            }

                            rtp_handles.insert(call_id.clone(), rtp_handle);
                            let _ =
                                sess_tx.send(SessionIn::SipInvite { call_id, from, to, offer });
                        }
                        SipEvent::Ack { call_id } => {
                            println!("[main] ACK for call_id={}", call_id);
                            if let Some(sess_tx) = session_registry.get(&call_id) {
                                let _ = sess_tx.send(SessionIn::SipAck);
                            }
                        }
                        SipEvent::Bye { call_id } => {
                            println!("[main] BYE for call_id={}", call_id);
                            if let Some(sess_tx) = session_registry.get(&call_id) {
                                let _ = sess_tx.send(SessionIn::SipBye);
                            }
                        }
                        SipEvent::TransactionTimeout { call_id } => {
                            println!("[main] TransactionTimeout for call_id={}", call_id);
                            if let Some(sess_tx) = session_registry.get(&call_id) {
                                let _ =
                                    sess_tx.send(SessionIn::SipTransactionTimeout { call_id });
                            }
                        }
                SipEvent::Unknown => {
                    println!("[main] Unknown / unsupported SIP message");
                }
            }
        }
            }
            Some((call_id, out)) = session_out_rx.recv() => {
                match out {
                    SessionOut::RtpStartTx { dst_ip, dst_port, .. } => {
                        // rtpハンドルの開始は session 側で実施済み。ここではログのみ。
                        log::debug!("[main] RtpStartTx for call_id={} dst={}:{}", call_id, dst_ip, dst_port);
                    }
                    SessionOut::RtpStopTx => {
                        if let Some(handle) = rtp_handles.remove(&call_id) {
                            handle.stop(&call_id);
                        }
                    }
                    SessionOut::AppSessionTimeout => {
                        log::warn!("[main] session timer fired for call_id={}", call_id);
                        if let Some(handle) = rtp_handles.remove(&call_id) {
                            handle.stop(&call_id);
                        }
                    }
                    SessionOut::AppSendBotAudioFile { path } => {
                        if let Some(sess_tx) = session_registry.get(&call_id) {
                            let _ = sess_tx.send(SessionIn::AppBotAudioFile { path });
                        }
                    }
                    SessionOut::AppRequestHangup => {
                        if let Some(sess_tx) = session_registry.get(&call_id) {
                            let _ = sess_tx.send(SessionIn::AppHangup);
                        }
                    }
                    other => {
                        sip_core.handle_session_out(&call_id, other);
                    }
                }
            }
            else => break,
        }
    }

    Ok(())
}
