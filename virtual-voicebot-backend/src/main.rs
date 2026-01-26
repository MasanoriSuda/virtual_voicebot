mod ai;
mod app;
mod config;
mod db;
mod http;
mod logging;
mod media;
mod ports;
mod recording;
mod rtp;
mod session;
mod sip;
mod transport;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::net::{TcpListener, UdpSocket};
use tokio::sync::mpsc::unbounded_channel;

use crate::db::{NoopPhoneLookup, PhoneLookupPort, TsurugiAdapter};
use crate::rtp::tx::RtpTxHandle;
use crate::session::{
    spawn_session, MediaConfig, SessionIn, SessionMap, SessionOut, SessionRegistry,
};
use crate::sip::{b2bua_bridge, SipConfig, SipCore, SipEvent};
use crate::transport::{run_packet_loop, RtpPortMap, SipInput, TransportSendRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logging::init();
    ai::llm::init_system_prompt();

    let cfg = config::Config::from_env()?;
    let sip_bind_ip = cfg.sip_bind_ip;
    let sip_port = cfg.sip_port;
    let rtp_port_cfg = cfg.rtp_port;
    let advertised_ip = cfg.advertised_ip;
    let advertised_rtp_port = cfg.advertised_rtp_port;
    let recording_http_addr = cfg.recording_http_addr;
    let ingest_call_url = cfg.ingest_call_url;
    let recording_base_url = cfg.recording_base_url;

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
    b2bua_bridge::init(sip_send_tx.clone(), sip_port);

    // --- ソケット準備 (SIP/RTPポートは環境変数で指定) ---
    let sip_sock = UdpSocket::bind((sip_bind_ip.as_str(), sip_port)).await?;
    let sip_tcp_listener = TcpListener::bind((sip_bind_ip.as_str(), sip_port)).await?;
    let rtp_sock = UdpSocket::bind(("0.0.0.0", rtp_port_cfg)).await?;
    let rtp_port = rtp_sock.local_addr()?.port();
    log::info!(
        "Listening SIP UDP on {}, SIP TCP on {}, RTP on {}",
        sip_sock.local_addr()?,
        sip_tcp_listener.local_addr()?,
        rtp_sock.local_addr()?
    );
    log::info!("[recording] static HTTP on {}", recording_http_addr);

    // 録音配信の簡易HTTPサーバ（/recordings/<callId>/... を静的配信）
    {
        let base_dir = std::env::current_dir()?.join(recording::RECORDINGS_DIR);
        http::spawn_recording_server(&recording_http_addr, base_dir).await;
    }

    // packetループ起動（UDP受信 → SIP/RTP振り分け → セッションへ）
    {
        let session_map_for_packet = session_map.clone();
        let rtp_port_map_for_packet = rtp_port_map.clone();
        tokio::spawn(async move {
            if let Err(e) = run_packet_loop(
                sip_sock,
                Some(sip_tcp_listener),
                rtp_sock,
                sip_tx,
                sip_send_rx,
                session_map_for_packet,
                rtp_port_map_for_packet,
                None,
            )
            .await
            {
                log::error!("[packet] loop error: {:?}", e);
            }
        });
    }

    // --- SIP処理ループ: packet層からのSIP入力をセッションへ結線 ---
    let ai_port = Arc::new(ai::DefaultAiPort::new());
    let phone_lookup: Arc<dyn PhoneLookupPort> = if config::phone_lookup_enabled() {
        if let Some(endpoint) = config::tsurugi_endpoint() {
            Arc::new(TsurugiAdapter::new(endpoint))
        } else {
            Arc::new(NoopPhoneLookup::new())
        }
    } else {
        Arc::new(NoopPhoneLookup::new())
    };
    let ingest_port = Arc::new(http::ingest::HttpIngestPort::new(
        config::timeouts().ingest_http,
    ));
    let storage_port = Arc::new(recording::storage::FileStoragePort::new());
    let mut sip_core = SipCore::new(
        SipConfig {
            advertised_ip: advertised_ip.clone(),
            sip_port,
            advertised_rtp_port,
        },
        sip_send_tx,
    );
    let shutdown = tokio::signal::ctrl_c();
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            res = &mut shutdown => {
                if let Err(err) = res {
                    log::warn!("[main] shutdown signal error: {:?}", err);
                }
                sip_core.shutdown();
                break;
            }
            Some(input) = sip_rx.recv() => {
                let events = sip_core.handle_input(&input);
                for ev in events {
                    match ev {
                        SipEvent::IncomingInvite {
                            call_id,
                            from,
                            to,
                            offer,
                            session_timer,
                        } => {
                            log::info!("[main] new INVITE, call_id={}", call_id);

                            let rtp_handle = RtpTxHandle::new();
                            let ingest_url = ingest_call_url.clone();
                            let recording_base_url = recording_base_url.clone();

                            let app_tx = app::spawn_app_worker(
                                call_id.clone(),
                                session_out_tx.clone(),
                                ai_port.clone(),
                                phone_lookup.clone(),
                            );
                            let sess_tx = spawn_session(
                                call_id.clone(),
                                from.clone(),
                                to.clone(),
                                session_registry.clone(),
                                MediaConfig::pcmu(advertised_ip.clone(), rtp_port),
                                session_out_tx.clone(),
                                app_tx,
                                rtp_handle.clone(),
                                ingest_url,
                                recording_base_url,
                                ingest_port.clone(),
                                storage_port.clone(),
                            );

                            {
                                let mut map = rtp_port_map.lock().unwrap();
                                map.insert(rtp_port, call_id.clone());
                            }

                            rtp_handles.insert(call_id.clone(), rtp_handle);
                            let _ = sess_tx.send(SessionIn::SipInvite {
                                call_id,
                                from,
                                to,
                                offer,
                                session_timer,
                            });
                        }
                        SipEvent::ReInvite {
                            call_id,
                            offer,
                            session_timer,
                        } => {
                            if let Some(sess_tx) = session_registry.get(&call_id) {
                                let _ = sess_tx.send(SessionIn::SipReInvite {
                                    offer,
                                    session_timer,
                                });
                            }
                        }
                        SipEvent::Ack { call_id } => {
                            log::info!("[main] ACK for call_id={}", call_id);
                            if let Some(sess_tx) = session_registry.get(&call_id) {
                                let _ = sess_tx.send(SessionIn::SipAck);
                            }
                        }
                        SipEvent::Cancel { call_id } => {
                            log::info!("[main] CANCEL for call_id={}", call_id);
                            if let Some(sess_tx) = session_registry.get(&call_id) {
                                let _ = sess_tx.send(SessionIn::SipCancel);
                            }
                        }
                        SipEvent::Bye { call_id } => {
                            log::info!("[main] BYE for call_id={}", call_id);
                            if let Some(sess_tx) = session_registry.get(&call_id) {
                                let _ = sess_tx.send(SessionIn::SipBye);
                            }
                        }
                        SipEvent::SessionRefresh { call_id, timer } => {
                            if let Some(sess_tx) = session_registry.get(&call_id) {
                                let _ = sess_tx.send(SessionIn::SipSessionExpires { timer });
                            }
                        }
                        SipEvent::TransactionTimeout { call_id } => {
                            log::warn!("[main] TransactionTimeout for call_id={}", call_id);
                            if let Some(sess_tx) = session_registry.get(&call_id) {
                                let _ =
                                    sess_tx.send(SessionIn::SipTransactionTimeout { call_id });
                            }
                        }
                        SipEvent::Unknown => {
                            log::debug!("[main] Unknown / unsupported SIP message");
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
                    SessionOut::Metrics { name, value } => {
                        if name == "rtp_in" {
                            log::debug!(
                                "[metrics] name={} value={} call_id={}",
                                name,
                                value,
                                call_id
                            );
                        } else {
                            log::info!(
                                "[metrics] name={} value={} call_id={}",
                                name,
                                value,
                                call_id
                            );
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
