mod interface;
mod protocol;
mod service;
mod shared;

use std::collections::HashMap;
use std::sync::Arc;

use tokio::net::{TcpListener, UdpSocket};
use tokio::sync::{mpsc, Mutex};

use crate::interface::db::TsurugiAdapter;
use crate::interface::http;
use crate::interface::notification::{LineAdapter, NoopNotification};
use crate::protocol::rtp::tx::RtpTxHandle;
use crate::protocol::session::types::CallId;
use crate::protocol::session::{spawn_session, MediaConfig, SessionControlIn, SessionOut, SessionRegistry};
use crate::protocol::sip::{b2bua_bridge, SipCommand, SipConfig, SipCore, SipEvent};
use crate::protocol::transport::{run_packet_loop, RtpPortMap, SipInput, TransportSendRequest};
use crate::service::ai;
use crate::service::call_control as app;
use crate::service::recording;
use crate::service::call_control::AppNotificationPort;
use crate::shared::{config, logging};
use crate::shared::ports::phone_lookup::{NoopPhoneLookup, PhoneLookupPort};
use crate::shared::ports::session_lookup::SessionLookup;

const SIP_INPUT_CHANNEL_CAPACITY: usize = 256;
const SIP_SEND_CHANNEL_CAPACITY: usize = 256;
const SESSION_OUT_CHANNEL_CAPACITY: usize = 128;

/// Starts the SIP/RTP server, initializes services and shared state, and runs the main event loop.
///
/// This initializes logging and AI prompts, binds SIP (UDP/TCP) and RTP sockets, spawns the
/// packet processing loop and a simple HTTP server for recordings, and wires SIP events to
/// per-call session workers and application workers. The function runs until a shutdown signal
/// is received.
///
/// # Returns
///
/// `Ok(())` on graceful shutdown, or an error if initialization fails.
///
/// # Examples
///
/// ```no_run
/// // Starts the server; the call blocks until a shutdown signal is received.
/// async fn start_server() {
///     let _ = crate::main().await;
/// }
/// ```
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logging::init();
    ai::llm::init_system_prompt();
    ai::intent::init_intent_prompt();

    let cfg = config::Config::from_env()?;
    let timeouts = config::timeouts().clone();
    let rtp_cfg = config::rtp_config().clone();
    let app_cfg = config::AppRuntimeConfig::from_env();
    let session_cfg = Arc::new(config::SessionRuntimeConfig::from_env(&cfg));
    let sip_bind_ip = cfg.sip_bind_ip;
    let sip_port = cfg.sip_port;
    let rtp_port_cfg = cfg.rtp_port;
    let advertised_ip = cfg.advertised_ip;
    let advertised_rtp_port = cfg.advertised_rtp_port;
    let recording_http_addr = cfg.recording_http_addr;
    let ingest_call_url = cfg.ingest_call_url;
    let recording_base_url = cfg.recording_base_url;

    // --- セッションとRTP送信元管理の共有マップ ---
    let session_registry = SessionRegistry::new();
    let rtp_port_map: RtpPortMap = Arc::new(Mutex::new(HashMap::new()));
    let mut rtp_handles: HashMap<CallId, RtpTxHandle> = HashMap::new();
    let mut rtp_peers: HashMap<CallId, std::net::SocketAddr> = HashMap::new();

    // packet層 → SIP処理ループ へのチャネル（過負荷時はtransport側でdrop）
    let (sip_tx, mut sip_rx) = mpsc::channel::<SipInput>(SIP_INPUT_CHANNEL_CAPACITY);
    // sip → transport 送信指示（過負荷時はsip側でdrop）
    let (sip_send_tx, sip_send_rx) =
        mpsc::channel::<TransportSendRequest>(SIP_SEND_CHANNEL_CAPACITY);
    // session → sip 指示（boundedでバックプレッシャ）
    let (session_out_tx, mut session_out_rx) =
        mpsc::channel::<(CallId, SessionOut)>(SESSION_OUT_CHANNEL_CAPACITY);
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
        let rtp_port_map_for_packet = rtp_port_map.clone();
        let session_lookup_for_packet: Arc<dyn SessionLookup> = Arc::new(session_registry.clone());
        let rtp_cfg_for_packet = rtp_cfg.clone();
        tokio::spawn(async move {
            if let Err(e) = run_packet_loop(
                sip_sock,
                Some(sip_tcp_listener),
                rtp_sock,
                sip_tx,
                sip_send_rx,
                session_lookup_for_packet,
                rtp_port_map_for_packet,
                None,
                rtp_cfg_for_packet,
                timeouts.sip_tcp_idle,
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

    let notification_port: Arc<dyn AppNotificationPort> = {
        let cfg = config::line_notify_config();
        if cfg.enabled {
            let token = cfg.channel_access_token.clone().unwrap_or_default();
            let user_id = cfg.user_id.clone().unwrap_or_default();
            match LineAdapter::new(token, user_id) {
                Ok(adapter) => Arc::new(adapter),
                Err(err) => {
                    log::warn!("[main] line adapter init failed: {}", err);
                    Arc::new(NoopNotification::new())
                }
            }
        } else {
            Arc::new(NoopNotification::new())
        }
    };
    let ingest_port = Arc::new(http::ingest::HttpIngestPort::new(timeouts.ingest_http)?);
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

                            let rtp_handle = RtpTxHandle::new(rtp_cfg.clone());
                            let ingest_url = ingest_call_url.clone();
                            let recording_base_url = recording_base_url.clone();

                            let app_tx = app::spawn_app_worker(
                                call_id.clone(),
                                session_out_tx.clone(),
                                ai_port.clone(),
                                phone_lookup.clone(),
                                notification_port.clone(),
                                app_cfg.clone(),
                            );
                            let sess_handle = spawn_session(
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
                                session_cfg.clone(),
                            )
                            .await;

                            if let Ok(peer_addr) =
                                format!("{}:{}", offer.ip, offer.port).parse()
                            {
                                rtp_port_map
                                    .lock()
                                    .await
                                    .insert(peer_addr, call_id.clone());
                                rtp_peers.insert(call_id.clone(), peer_addr);
                            } else {
                                log::warn!(
                                    "[main] invalid RTP peer {}:{} for call_id={}",
                                    offer.ip,
                                    offer.port,
                                    call_id
                                );
                            }

                            rtp_handles.insert(call_id.clone(), rtp_handle);
                            let _ = sess_handle
                                .control_tx
                                .send(SessionControlIn::SipInvite {
                                    call_id,
                                    from,
                                    to,
                                    offer,
                                    session_timer,
                                })
                                .await;
                        }
                        SipEvent::ReInvite {
                            call_id,
                            offer,
                            session_timer,
                        } => {
                            if let Ok(peer_addr) =
                                format!("{}:{}", offer.ip, offer.port).parse()
                            {
                                if let Some(old) = rtp_peers.insert(call_id.clone(), peer_addr) {
                                    let mut map = rtp_port_map.lock().await;
                                    map.remove(&old);
                                    map.insert(peer_addr, call_id.clone());
                                } else {
                                    rtp_port_map
                                        .lock()
                                        .await
                                        .insert(peer_addr, call_id.clone());
                                }
                            } else {
                                log::warn!(
                                    "[main] invalid RTP peer {}:{} for call_id={}",
                                    offer.ip,
                                    offer.port,
                                    call_id
                                );
                            }
                            if let Some(sess_tx) = session_registry.get(&call_id).await {
                                let _ = sess_tx
                                    .control_tx
                                    .send(SessionControlIn::SipReInvite {
                                        offer,
                                        session_timer,
                                    })
                                    .await;
                            }
                        }
                        SipEvent::Ack { call_id } => {
                            log::info!("[main] ACK for call_id={}", call_id);
                            if let Some(sess_tx) = session_registry.get(&call_id).await {
                                let _ = sess_tx.control_tx.send(SessionControlIn::SipAck).await;
                            }
                        }
                        SipEvent::Cancel { call_id } => {
                            log::info!("[main] CANCEL for call_id={}", call_id);
                            if let Some(sess_tx) = session_registry.get(&call_id).await {
                                let _ = sess_tx
                                    .control_tx
                                    .send(SessionControlIn::SipCancel)
                                    .await;
                            }
                        }
                        SipEvent::Bye { call_id } => {
                            log::info!("[main] BYE for call_id={}", call_id);
                            if let Some(sess_tx) = session_registry.get(&call_id).await {
                                let _ = sess_tx.control_tx.send(SessionControlIn::SipBye).await;
                            }
                        }
                        SipEvent::SessionRefresh { call_id, timer } => {
                            if let Some(sess_tx) = session_registry.get(&call_id).await {
                                let _ = sess_tx
                                    .control_tx
                                    .send(SessionControlIn::SipSessionExpires { timer })
                                    .await;
                            }
                        }
                        SipEvent::TransactionTimeout { call_id } => {
                            log::warn!("[main] TransactionTimeout for call_id={}", call_id);
                            if let Some(sess_tx) = session_registry.get(&call_id).await {
                                let _ = sess_tx
                                    .control_tx
                                    .send(SessionControlIn::SipTransactionTimeout { call_id })
                                    .await;
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
                            handle.stop(call_id.as_str());
                        }
                        if let Some(peer) = rtp_peers.remove(&call_id) {
                            rtp_port_map.lock().await.remove(&peer);
                        }
                    }
                    SessionOut::AppSessionTimeout => {
                        log::warn!("[main] session timer fired for call_id={}", call_id);
                        if let Some(handle) = rtp_handles.remove(&call_id) {
                            handle.stop(call_id.as_str());
                        }
                        if let Some(peer) = rtp_peers.remove(&call_id) {
                            rtp_port_map.lock().await.remove(&peer);
                        }
                    }
                    SessionOut::AppSendBotAudioFile { path } => {
                        if let Some(sess_tx) = session_registry.get(&call_id).await {
                            let _ = sess_tx
                                .control_tx
                                .send(SessionControlIn::AppBotAudioFile { path })
                                .await;
                        }
                    }
                    SessionOut::AppRequestHangup => {
                        if let Some(sess_tx) = session_registry.get(&call_id).await {
                            let _ = sess_tx
                                .control_tx
                                .send(SessionControlIn::AppHangup)
                                .await;
                        }
                    }
                    SessionOut::AppRequestTransfer { person } => {
                        if let Some(sess_tx) = session_registry.get(&call_id).await {
                            let _ = sess_tx
                                .control_tx
                                .send(SessionControlIn::AppTransferRequest { person })
                                .await;
                        }
                    }
                    SessionOut::AppRequestTts { text } => {
                        log::debug!(
                            "[main] AppRequestTts received (stub): call_id={} text_len={}",
                            call_id,
                            text.len()
                        );
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
                    SessionOut::SipSend100 => {
                        sip_core.handle_sip_command(&call_id, SipCommand::Send100);
                    }
                    SessionOut::SipSend180 => {
                        sip_core.handle_sip_command(&call_id, SipCommand::Send180);
                    }
                    SessionOut::SipSend183 { answer } => {
                        sip_core.handle_sip_command(&call_id, SipCommand::Send183 { answer });
                    }
                    SessionOut::SipSend200 { answer } => {
                        sip_core.handle_sip_command(&call_id, SipCommand::Send200 { answer });
                    }
                    SessionOut::SipSendUpdate { expires } => {
                        sip_core.handle_sip_command(&call_id, SipCommand::SendUpdate { expires });
                    }
                    SessionOut::SipSendError { code, reason } => {
                        sip_core.handle_sip_command(
                            &call_id,
                            SipCommand::SendError { code, reason },
                        );
                    }
                    SessionOut::SipSendBye => {
                        sip_core.handle_sip_command(&call_id, SipCommand::SendBye);
                    }
                    SessionOut::SipSendBye200 => {
                        sip_core.handle_sip_command(&call_id, SipCommand::SendBye200);
                    }
                }
            }
            else => break,
        }
    }

    Ok(())
}
