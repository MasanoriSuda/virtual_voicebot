use std::collections::HashMap;
use std::sync::Arc;

use tokio::net::{TcpListener, UdpSocket};
use tokio::sync::{mpsc, Mutex};

use virtual_voicebot_backend::interface::db::{PostgresAdapter, RoutingRepoImpl};
use virtual_voicebot_backend::interface::http;
use virtual_voicebot_backend::interface::notification::{LineAdapter, NoopNotification};
use virtual_voicebot_backend::protocol::rtp::tx::RtpTxHandle;
use virtual_voicebot_backend::protocol::session::types::CallId;
use virtual_voicebot_backend::protocol::session::{
    spawn_session, MediaConfig, SessionControlIn, SessionOut, SessionRegistry,
};
use virtual_voicebot_backend::protocol::sip::{
    b2bua_bridge, SipCommand, SipConfig, SipCore, SipEvent,
};
use virtual_voicebot_backend::protocol::transport::{
    run_packet_loop, RtpPortMap, SipInput, TransportSendRequest,
};
use virtual_voicebot_backend::service::ai;
use virtual_voicebot_backend::service::call_control as app;
use virtual_voicebot_backend::service::call_control::AppNotificationPort;
use virtual_voicebot_backend::service::recording;
use virtual_voicebot_backend::shared::ports::call_log_port::{CallLogPort, NoopCallLogPort};
use virtual_voicebot_backend::shared::ports::phone_lookup::{NoopPhoneLookup, PhoneLookupPort};
use virtual_voicebot_backend::shared::ports::routing_port::{NoopRoutingPort, RoutingPort};
use virtual_voicebot_backend::shared::ports::session_lookup::SessionLookup;
use virtual_voicebot_backend::shared::{config, logging};

const SIP_INPUT_CHANNEL_CAPACITY: usize = 256;
const SIP_SEND_CHANNEL_CAPACITY: usize = 256;
const SESSION_OUT_CHANNEL_CAPACITY: usize = 128;

fn sanitized_display_url_for_log(url: &str) -> String {
    let trimmed = url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return "none".to_string();
    }

    if let Ok(mut parsed) = reqwest::Url::parse(trimmed) {
        let _ = parsed.set_username("");
        let _ = parsed.set_password(None);
        parsed.set_query(None);
        parsed.set_fragment(None);
        return parsed.to_string().trim_end_matches('/').to_string();
    }

    if let Some((scheme, rest)) = trimmed.split_once("://") {
        let host_part = rest.rsplit_once('@').map_or(rest, |(_, host)| host);
        let host_part = host_part
            .split(['?', '#'])
            .next()
            .unwrap_or(host_part)
            .trim_end_matches('/');
        return format!("{scheme}://{host_part}");
    }

    trimmed
        .rsplit_once('@')
        .map_or(trimmed, |(_, host)| host)
        .split(['?', '#'])
        .next()
        .unwrap_or(trimmed)
        .trim_end_matches('/')
        .to_string()
}

fn sanitize_optional_url(url: Option<&str>) -> String {
    url.map(sanitized_display_url_for_log)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "none".to_string())
}

fn log_ai_config() {
    let ai = config::ai_config();

    let asr_streaming = config::voicebot_asr_streaming_enabled();
    let asr_local_url = if asr_streaming {
        sanitized_display_url_for_log(&config::asr_streaming_server_url())
    } else {
        sanitized_display_url_for_log(&ai.asr_local_server_url)
    };
    let asr_raspi_url = sanitize_optional_url(ai.asr_raspi_url.as_deref());
    log::info!(
        "[main] startup ai-config asr_streaming={} asr_local_enabled={} asr_local_url={} asr_raspi_enabled={} asr_raspi_url={}",
        asr_streaming,
        ai.asr_local_server_enabled,
        asr_local_url,
        ai.asr_raspi_enabled,
        asr_raspi_url,
    );

    let llm_raspi_url = sanitize_optional_url(ai.llm_raspi_url.as_deref());
    log::info!(
        "[main] startup ai-config llm_streaming={} llm_local_enabled={} llm_local_url={} llm_model={} llm_raspi_enabled={} llm_raspi_url={}",
        config::voicebot_streaming_enabled(),
        ai.llm_local_server_enabled,
        sanitized_display_url_for_log(&ai.llm_local_server_url),
        ai.llm_local_model,
        ai.llm_raspi_enabled,
        llm_raspi_url,
    );

    let tts_raspi_url = sanitize_optional_url(ai.tts_raspi_base_url.as_deref());
    log::info!(
        "[main] startup ai-config tts_streaming={} tts_local_enabled={} tts_local_url={} tts_raspi_enabled={} tts_raspi_url={}",
        config::voicebot_tts_streaming_enabled(),
        ai.tts_local_server_enabled,
        sanitized_display_url_for_log(&ai.tts_local_server_base_url),
        ai.tts_raspi_enabled,
        tts_raspi_url,
    );
}

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
    log_ai_config();
    let sip_bind_ip = cfg.sip_bind_ip;
    let sip_port = cfg.sip_port;
    let rtp_port_cfg = cfg.rtp_port;
    let advertised_ip = cfg.advertised_ip;
    let advertised_rtp_port = cfg.advertised_rtp_port;
    let recording_http_addr = cfg.recording_http_addr;
    let ingest_call_url = cfg.ingest_call_url;
    let recording_base_url = cfg.recording_base_url;
    let postgres_adapter = if let Some(database_url) = config::database_url() {
        match tokio::time::timeout(
            std::time::Duration::from_secs(5),
            PostgresAdapter::new(database_url),
        )
        .await
        {
            Ok(Ok(adapter)) => Some(Arc::new(adapter)),
            Ok(Err(err)) => {
                log::warn!("[main] postgres adapter init failed: {}", err);
                None
            }
            Err(_) => {
                log::warn!("[main] postgres adapter init timed out");
                None
            }
        }
    } else {
        None
    };

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
        let recording_http_pool = postgres_adapter
            .as_ref()
            .map(|adapter| adapter.pool().clone());
        http::spawn_recording_server(&recording_http_addr, base_dir, recording_http_pool).await;
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
        match postgres_adapter.clone() {
            Some(adapter) => adapter,
            None => {
                log::warn!("[main] phone lookup enabled but DATABASE_URL is missing/unavailable");
                Arc::new(NoopPhoneLookup::new())
            }
        }
    } else {
        Arc::new(NoopPhoneLookup::new())
    };
    let call_log_port: Arc<dyn CallLogPort> = match postgres_adapter.clone() {
        Some(adapter) => adapter,
        None => {
            log::warn!("[main] call_log/outbox persist disabled (DATABASE_URL unavailable)");
            Arc::new(NoopCallLogPort::new())
        }
    };
    let routing_port: Arc<dyn RoutingPort> = match postgres_adapter.clone() {
        Some(adapter) => Arc::new(RoutingRepoImpl::new(adapter.pool().clone())),
        None => {
            log::warn!("[main] routing evaluation disabled (DATABASE_URL unavailable)");
            Arc::new(NoopRoutingPort::new())
        }
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

                            let asr_streaming_enabled = config::voicebot_asr_streaming_enabled();
                            let (audio_chunk_tx, audio_chunk_rx) = if asr_streaming_enabled {
                                let (tx, rx) = app::audio_chunk_channel(16);
                                (Some(tx), Some(rx))
                            } else {
                                (None, None)
                            };

                            let app_tx = app::spawn_app_worker(
                                call_id.clone(),
                                session_out_tx.clone(),
                                ai_port.clone(),
                                if config::voicebot_streaming_enabled() {
                                    Some(ai_port.clone())
                                } else {
                                    None
                                },
                                if asr_streaming_enabled {
                                    Some(ai_port.clone())
                                } else {
                                    None
                                },
                                if config::voicebot_tts_streaming_enabled() {
                                    Some(ai_port.clone())
                                } else {
                                    None
                                },
                                audio_chunk_rx,
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
                                audio_chunk_tx,
                                rtp_handle.clone(),
                                ingest_url,
                                recording_base_url,
                                ingest_port.clone(),
                                storage_port.clone(),
                                call_log_port.clone(),
                                routing_port.clone(),
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
                            log::info!(
                                "[main] re-INVITE received call_id={} offer={}:{}",
                                call_id,
                                offer.ip,
                                offer.port
                            );
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
                                if let Err(err) = sess_tx
                                    .control_tx
                                    .send(SessionControlIn::SipReInvite {
                                        offer,
                                        session_timer,
                                    })
                                    .await
                                {
                                    log::warn!(
                                        "[main] failed to forward re-INVITE to session call_id={}: {:?}",
                                        call_id,
                                        err
                                    );
                                }
                            } else {
                                log::warn!(
                                    "[main] re-INVITE for unknown session call_id={}, sending 481",
                                    call_id
                                );
                                sip_core.handle_sip_command(
                                    &call_id,
                                    SipCommand::SendError {
                                        code: 481,
                                        reason: "Call/Transaction Does Not Exist".to_string(),
                                    },
                                );
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
                            } else {
                                log::warn!("[main] received BYE for unknown call_id: {}", call_id);
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
                        SipEvent::SessionRefreshFailed { call_id } => {
                            if let Some(sess_tx) = session_registry.get(&call_id).await {
                                let _ = sess_tx
                                    .control_tx
                                    .send(SessionControlIn::SipSessionRefreshFailed {
                                        call_id: call_id.clone(),
                                    })
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
                    SessionOut::AppEnqueueBotAudioFile { path, generation_id } => {
                        if let Some(sess_tx) = session_registry.get(&call_id).await {
                            let _ = sess_tx
                                .control_tx
                                .send(SessionControlIn::AppBotAudioFileEnqueue { path, generation_id })
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
                    SessionOut::SipSendSessionRefresh { expires, local_sdp } => {
                        for event in sip_core.handle_sip_command(
                            &call_id,
                            SipCommand::SendSessionRefresh { expires, local_sdp },
                        ) {
                            if let SipEvent::SessionRefreshFailed { call_id } = event {
                                if let Some(sess_tx) = session_registry.get(&call_id).await {
                                    let _ = sess_tx
                                        .control_tx
                                        .send(SessionControlIn::SipSessionRefreshFailed {
                                            call_id: call_id.clone(),
                                        })
                                        .await;
                                }
                            }
                        }
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

#[cfg(test)]
mod tests {
    use super::{sanitize_optional_url, sanitized_display_url_for_log};

    #[test]
    fn sanitized_display_url_for_log_returns_none_for_empty_or_whitespace() {
        assert_eq!(sanitized_display_url_for_log(""), "none");
        assert_eq!(sanitized_display_url_for_log("   \t  "), "none");
    }

    #[test]
    fn sanitized_display_url_for_log_trims_trailing_slash_and_strips_userinfo() {
        assert_eq!(
            sanitized_display_url_for_log("http://user:pass@example.com:8080/api/"),
            "http://example.com:8080/api"
        );
        assert_eq!(
            sanitized_display_url_for_log("https://example.com/base/"),
            "https://example.com/base"
        );
        assert_eq!(
            sanitized_display_url_for_log("https://user:pass@example.com/base/?token=secret#frag"),
            "https://example.com/base"
        );
    }

    #[test]
    fn sanitized_display_url_for_log_handles_invalid_strings_with_and_without_scheme() {
        assert_eq!(
            sanitized_display_url_for_log("custom scheme://user:pass@host.local/path"),
            "custom scheme://host.local/path"
        );
        assert_eq!(
            sanitized_display_url_for_log(
                "custom scheme://user:pass@host.local/path?token=secret#frag"
            ),
            "custom scheme://host.local/path"
        );
        assert_eq!(
            sanitized_display_url_for_log("not a url@host.local/path?token=secret#frag"),
            "host.local/path"
        );
        assert_eq!(
            sanitized_display_url_for_log("just-hostname"),
            "just-hostname"
        );
    }

    #[test]
    fn sanitize_optional_url_handles_some_and_none() {
        assert_eq!(
            sanitize_optional_url(Some("http://user:pass@example.com/")),
            "http://example.com"
        );
        assert_eq!(sanitize_optional_url(Some("   ")), "none");
        assert_eq!(sanitize_optional_url(None), "none");
    }
}
