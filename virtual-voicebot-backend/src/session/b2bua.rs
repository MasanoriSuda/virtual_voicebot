use std::future::Future;
use std::net::{SocketAddr, ToSocketAddrs};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use log::{info, warn};
use rand::Rng;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::sync::Notify;
use tokio::time::sleep;

use crate::config::{RegistrarConfig, RegistrarTransport, SessionRuntimeConfig};
use crate::rtp::codec::{codec_from_pt, decode_to_mulaw};
use crate::rtp::parser::parse_rtp_packet;
use crate::session::types::{Sdp, SessionIn};
use crate::sip::auth::{build_authorization_header, parse_digest_challenge};
use crate::sip::auth_cache::{self, DigestAuthChallenge, DigestAuthHeader};
use crate::sip::b2bua_bridge::{self, B2buaRegistration, B2buaSipMessage};
use crate::sip::builder::response_simple_from_request;
use crate::sip::message::{SipHeader, SipMessage, SipMethod, SipRequest, SipResponse};
use crate::sip::{parse_cseq_header, parse_offer_sdp, parse_uri, SipRequestBuilder};
use crate::transport::TransportPeer;
use crate::utils::mask_pii;

const RTP_BUFFER_SIZE: usize = 2048;
const DEFAULT_SIP_PORT: u16 = 5060;

#[derive(Debug)]
pub struct BLeg {
    pub call_id: String,
    pub rtp_key: String,
    pub remote_rtp_addr: SocketAddr,
    sip_peer: SocketAddr,
    from_header: String,
    to_header: String,
    remote_uri: String,
    cseq: u32,
    via_host: String,
    via_port: u16,
    _b2bua_reg: B2buaRegistration,
    shutdown: Arc<AtomicBool>,
    shutdown_notify: Arc<Notify>,
}

impl BLeg {
    pub async fn send_bye(&mut self) -> Result<()> {
        self.cseq = self.cseq.saturating_add(1).max(2);
        let via = build_via(self.via_host.as_str(), self.via_port);
        let req = SipRequestBuilder::new(SipMethod::Bye, self.remote_uri.clone())
            .header("Via", via)
            .header("Max-Forwards", "70")
            .header("From", self.from_header.clone())
            .header("To", self.to_header.clone())
            .header("Call-ID", self.call_id.clone())
            .header("CSeq", format!("{} BYE", self.cseq))
            .build();
        send_b2bua_payload(TransportPeer::Udp(self.sip_peer), req.to_bytes())?;
        Ok(())
    }

    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
        self.shutdown_notify.notify_waiters();
    }
}

pub fn spawn_transfer(
    a_call_id: String,
    tx_in: mpsc::Sender<SessionIn>,
    runtime_cfg: Arc<SessionRuntimeConfig>,
) -> tokio::sync::oneshot::Sender<()> {
    let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        match run_transfer(a_call_id.clone(), tx_in.clone(), cancel_rx, runtime_cfg).await {
            Ok(Some(b_leg)) => {
                let _ = tx_in.try_send(SessionIn::B2buaEstablished { b_leg });
            }
            Ok(None) => {
                info!("[b2bua {}] transfer cancelled", a_call_id);
            }
            Err(err) => {
                let _ = tx_in.try_send(SessionIn::B2buaFailed {
                    reason: err.to_string(),
                    status: None,
                });
            }
        }
    });
    cancel_tx
}

pub fn spawn_outbound(
    a_call_id: String,
    number: String,
    tx_in: mpsc::Sender<SessionIn>,
    runtime_cfg: Arc<SessionRuntimeConfig>,
) -> tokio::sync::oneshot::Sender<()> {
    let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        match run_outbound(a_call_id.clone(), number, tx_in.clone(), cancel_rx, runtime_cfg).await {
            Ok(Some(b_leg)) => {
                let _ = tx_in.try_send(SessionIn::B2buaEstablished { b_leg });
            }
            Ok(None) => {
                info!("[b2bua {}] outbound cancelled", a_call_id);
            }
            Err(err) => {
                let _ = tx_in.try_send(SessionIn::B2buaFailed {
                    reason: err.to_string(),
                    status: err.downcast_ref::<OutboundError>().map(|e| e.status),
                });
            }
        }
    });
    cancel_tx
}

async fn run_transfer(
    a_call_id: String,
    tx_in: mpsc::Sender<SessionIn>,
    mut cancel_rx: tokio::sync::oneshot::Receiver<()>,
    runtime_cfg: Arc<SessionRuntimeConfig>,
) -> Result<Option<BLeg>> {
    let target_uri = runtime_cfg.transfer_target_uri.clone();
    let target_addr = resolve_target_addr(&target_uri)?;

    let sip_port = runtime_cfg.sip_port;
    let via_host = runtime_cfg.advertised_ip.clone();
    let via = build_via(via_host.as_str(), sip_port);
    let local_tag = generate_tag();
    let from_header = format!(
        "<sip:rustbot@{}:{}>;tag={}",
        runtime_cfg.advertised_ip, sip_port, local_tag
    );
    let to_header = format!("<{}>", target_uri);
    let b_call_id = format!("b2bua-{}-{}", a_call_id, rand::thread_rng().gen::<u32>());
    let (b2bua_reg, mut sip_rx) = b2bua_bridge::register(b_call_id.clone());

    let rtp_socket = Arc::new(UdpSocket::bind("0.0.0.0:0").await?);
    let rtp_port = rtp_socket.local_addr()?.port();
    let sdp = build_sdp(runtime_cfg.advertised_ip.as_str(), rtp_port);

    let cseq: u32 = 1;
    let invite = SipRequestBuilder::new(SipMethod::Invite, target_uri.clone())
        .header("Via", via.clone())
        .header("Max-Forwards", "70")
        .header("From", from_header.clone())
        .header("To", to_header.clone())
        .header("Call-ID", b_call_id.clone())
        .header("CSeq", format!("{cseq} INVITE"))
        .header(
            "Contact",
            format!("<sip:rustbot@{}:{}>", runtime_cfg.advertised_ip, sip_port),
        )
        .body(sdp.as_bytes(), Some("application/sdp"))
        .build();

    log_invite("transfer", target_addr, &invite);
    send_b2bua_payload(TransportPeer::Udp(target_addr), invite.to_bytes())?;

    let timeout = runtime_cfg.transfer_timeout;
    let timeout_sleep = sleep(timeout);
    tokio::pin!(timeout_sleep);
    let mut provisional_received = false;
    let mut cancel_requested = false;
    let mut cancel_sent = false;
    let mut cancel_fut: Pin<Box<dyn Future<Output = ()> + Send>> = Box::pin(async move {
        let _ = cancel_rx.await;
    });

    loop {
        tokio::select! {
            _ = &mut cancel_fut => {
                cancel_requested = true;
                if provisional_received && !cancel_sent {
                    let cancel = SipRequestBuilder::new(SipMethod::Cancel, target_uri.clone())
                        .header("Via", via.clone())
                        .header("Max-Forwards", "70")
                        .header("From", from_header.clone())
                        .header("To", to_header.clone())
                        .header("Call-ID", b_call_id.clone())
                        .header("CSeq", format!("{cseq} CANCEL"))
                        .build();
                    log_cancel("transfer", target_addr, &cancel);
                    let _ = send_b2bua_payload(TransportPeer::Udp(target_addr), cancel.to_bytes());
                    cancel_sent = true;
                } else if !provisional_received {
                    info!("[b2bua {}] cancel pending (no provisional)", a_call_id);
                }
                cancel_fut = Box::pin(std::future::pending());
            }
            _ = &mut timeout_sleep => {
                return Err(anyhow!("transfer timeout after {}s", timeout.as_secs()));
            }
            maybe_msg = sip_rx.recv() => {
                let Some(msg) = maybe_msg else {
                    return Err(anyhow!("transfer sip channel closed"));
                };
                let B2buaSipMessage { peer, message } = msg;
                let SipMessage::Response(resp) = message else {
                    continue;
                };
                if !response_matches_call_id(&resp, &b_call_id) {
                    continue;
                }
                if is_cancel_response(&resp) {
                    continue;
                }
                if resp.status_code < 200 {
                    provisional_received = true;
                    if cancel_requested && !cancel_sent {
                        let cancel = SipRequestBuilder::new(SipMethod::Cancel, target_uri.clone())
                            .header("Via", via.clone())
                            .header("Max-Forwards", "70")
                            .header("From", from_header.clone())
                            .header("To", to_header.clone())
                            .header("Call-ID", b_call_id.clone())
                            .header("CSeq", format!("{cseq} CANCEL"))
                            .build();
                        log_cancel("transfer", target_addr, &cancel);
                        let _ =
                            send_b2bua_payload(TransportPeer::Udp(target_addr), cancel.to_bytes());
                        cancel_sent = true;
                    }
                    info!(
                        "[b2bua {}] provisional response {} from {:?}",
                        a_call_id, resp.status_code, peer
                    );
                    continue;
                }
                if resp.status_code >= 300 {
                    let ack_to = header_value(&resp.headers, "To").unwrap_or(to_header.as_str());
                    send_non2xx_ack(
                        peer,
                        target_uri.as_str(),
                        via.as_str(),
                        from_header.as_str(),
                        ack_to,
                        b_call_id.as_str(),
                        cseq,
                    );
                    if cancel_requested {
                        return Ok(None);
                    }
                    return Err(anyhow!("transfer failed status {}", resp.status_code));
                }

                let to_header = header_value(&resp.headers, "To")
                    .ok_or_else(|| anyhow!("missing To header"))?
                    .to_string();
                if extract_tag(&to_header).is_none() {
                    return Err(anyhow!("missing To tag in 200 OK"));
                }

                let remote_uri = header_value(&resp.headers, "Contact")
                    .map(extract_contact_uri)
                    .unwrap_or(target_uri.as_str())
                    .to_string();
                let sip_peer = resolve_target_addr(&remote_uri).unwrap_or(target_addr);

                let remote_sdp =
                    parse_offer_sdp(&resp.body).ok_or_else(|| anyhow!("missing SDP in 200 OK"))?;
                let remote_rtp_addr = resolve_rtp_addr(&remote_sdp)?;

                let ack = SipRequestBuilder::new(SipMethod::Ack, remote_uri.clone())
                    .header("Via", build_via(via_host.as_str(), sip_port))
                    .header("Max-Forwards", "70")
                    .header("From", from_header.clone())
                    .header("To", to_header.clone())
                    .header("Call-ID", b_call_id.clone())
                    .header("CSeq", format!("{cseq} ACK"))
                    .build();
                send_b2bua_payload(TransportPeer::Udp(sip_peer), ack.to_bytes())?;

                if cancel_requested {
                    let bye_cseq = cseq.saturating_add(1).max(2);
                    let bye = SipRequestBuilder::new(SipMethod::Bye, remote_uri.clone())
                        .header("Via", build_via(via_host.as_str(), sip_port))
                        .header("Max-Forwards", "70")
                        .header("From", from_header.clone())
                        .header("To", to_header.clone())
                        .header("Call-ID", b_call_id.clone())
                        .header("CSeq", format!("{bye_cseq} BYE"))
                        .build();
                    let _ = send_b2bua_payload(TransportPeer::Udp(sip_peer), bye.to_bytes());
                    return Ok(None);
                }

                let shutdown = Arc::new(AtomicBool::new(false));
                let shutdown_notify = Arc::new(Notify::new());
                spawn_sip_listener(
                    a_call_id.clone(),
                    b_call_id.clone(),
                    sip_rx,
                    tx_in.clone(),
                    shutdown.clone(),
                    shutdown_notify.clone(),
                );
                spawn_rtp_listener(
                    a_call_id.clone(),
                    rtp_socket.clone(),
                    tx_in.clone(),
                    shutdown.clone(),
                    shutdown_notify.clone(),
                );

                let b_leg = BLeg {
                    call_id: b_call_id,
                    rtp_key: format!("{}-b", a_call_id),
                    remote_rtp_addr,
                    sip_peer,
                    from_header,
                    to_header,
                    remote_uri,
                    cseq: 1,
                    via_host,
                    via_port: sip_port,
                    _b2bua_reg: b2bua_reg,
                    shutdown,
                    shutdown_notify,
                };
                return Ok(Some(b_leg));
            }
        }
    }
}

#[derive(Debug)]
struct OutboundError {
    status: u16,
}

impl std::fmt::Display for OutboundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "outbound failed status {}", self.status)
    }
}

impl std::error::Error for OutboundError {}

struct RtpListenerGuard {
    shutdown: Arc<AtomicBool>,
    shutdown_notify: Arc<Notify>,
    started: bool,
}

impl RtpListenerGuard {
    fn new(shutdown: Arc<AtomicBool>, shutdown_notify: Arc<Notify>) -> Self {
        Self {
            shutdown,
            shutdown_notify,
            started: false,
        }
    }

    fn start(
        &mut self,
        a_call_id: &str,
        rtp_socket: Arc<UdpSocket>,
        tx_in: mpsc::Sender<SessionIn>,
    ) {
        if self.started {
            return;
        }
        spawn_rtp_listener(
            a_call_id.to_string(),
            rtp_socket,
            tx_in,
            self.shutdown.clone(),
            self.shutdown_notify.clone(),
        );
        self.started = true;
    }

    fn disarm(&mut self) {
        self.started = false;
    }
}

impl Drop for RtpListenerGuard {
    fn drop(&mut self) {
        if self.started {
            self.shutdown.store(true, Ordering::SeqCst);
            self.shutdown_notify.notify_waiters();
        }
    }
}

async fn run_outbound(
    a_call_id: String,
    number: String,
    tx_in: mpsc::Sender<SessionIn>,
    mut cancel_rx: tokio::sync::oneshot::Receiver<()>,
    runtime_cfg: Arc<SessionRuntimeConfig>,
) -> Result<Option<BLeg>> {
    let registrar = runtime_cfg
        .registrar
        .as_ref()
        .ok_or_else(|| anyhow!("missing registrar config"))?;
    if registrar.auth_password.is_none() {
        return Err(anyhow!("missing registrar auth password"));
    }
    if registrar.transport != RegistrarTransport::Udp {
        return Err(anyhow!("outbound transport must be UDP"));
    }

    let outbound_cfg = &runtime_cfg.outbound;
    let outbound_domain = outbound_cfg.domain.clone();
    if outbound_domain.is_empty() {
        return Err(anyhow!("missing outbound domain"));
    }

    let request_uri = format!("sip:{}@{}", number, outbound_domain);
    let sip_peer = registrar.addr;

    let sip_port = runtime_cfg.sip_port;
    let from_header = format!(
        "<sip:{}@{}>;tag={}",
        registrar.user,
        registrar.domain,
        generate_tag()
    );
    let to_header = format!("<{}>", request_uri);
    let call_id = format!("outbound-{}-{}", a_call_id, rand::thread_rng().gen::<u32>());
    let (b2bua_reg, mut sip_rx) = b2bua_bridge::register(call_id.clone());
    let via_host = registrar.contact_host.clone();

    let rtp_socket = Arc::new(UdpSocket::bind("0.0.0.0:0").await?);
    let rtp_port = rtp_socket.local_addr()?.port();
    let sdp = build_sdp(runtime_cfg.advertised_ip.as_str(), rtp_port);

    let mut cseq: u32 = 1;
    let mut auth_attempts: u32 = 0;
    let mut max_auth_attempts: u32 = 1;
    let mut auth_nc: u32 = 0;
    let mut auth_nonce: Option<String> = None;

    let mut invite_via = build_via(via_host.as_str(), sip_port);
    let mut initial_auth: Option<(&'static str, String)> = None;
    if let Some(cached) = auth_cache::load() {
        if let Some(auth_value) = build_outbound_auth_value(
            registrar,
            request_uri.as_str(),
            &cached.challenge,
            &mut auth_nc,
            &mut auth_nonce,
        ) {
            initial_auth = Some((cached.header.as_str(), auth_value));
            auth_attempts = 1;
            max_auth_attempts = 2;
        }
    }
    send_outbound_invite(
        sip_peer,
        &request_uri,
        &from_header,
        &to_header,
        &call_id,
        cseq,
        invite_via.as_str(),
        sip_port,
        registrar,
        &sdp,
        initial_auth,
    )
    .await?;

    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_notify = Arc::new(Notify::new());
    let mut rtp_guard = RtpListenerGuard::new(shutdown.clone(), shutdown_notify.clone());
    rtp_guard.start(a_call_id.as_str(), rtp_socket.clone(), tx_in.clone());

    let timeout = runtime_cfg.transfer_timeout;
    let timeout_sleep = sleep(timeout);
    tokio::pin!(timeout_sleep);
    let mut provisional_received = false;
    let mut early_media_sent = false;
    let mut cancel_requested = false;
    let mut cancel_sent = false;
    let mut cancel_fut: Pin<Box<dyn Future<Output = ()> + Send>> = Box::pin(async move {
        let _ = cancel_rx.await;
    });

    loop {
        tokio::select! {
            _ = &mut cancel_fut => {
                cancel_requested = true;
                if provisional_received && !cancel_sent {
                    let cancel = SipRequestBuilder::new(SipMethod::Cancel, request_uri.clone())
                        .header("Via", invite_via.clone())
                        .header("Max-Forwards", "70")
                        .header("From", from_header.clone())
                        .header("To", to_header.clone())
                        .header("Call-ID", call_id.clone())
                        .header("CSeq", format!("{cseq} CANCEL"))
                        .build();
                    log_cancel("outbound", sip_peer, &cancel);
                    let _ = send_b2bua_payload(TransportPeer::Udp(sip_peer), cancel.to_bytes());
                    cancel_sent = true;
                } else if !provisional_received {
                    info!("[b2bua {}] cancel pending (no provisional)", a_call_id);
                }
                cancel_fut = Box::pin(std::future::pending());
            }
            _ = &mut timeout_sleep => {
                return Err(anyhow!("outbound timeout after {}s", timeout.as_secs()));
            }
            maybe_msg = sip_rx.recv() => {
                let Some(msg) = maybe_msg else {
                    return Err(anyhow!("outbound sip channel closed"));
                };
                let B2buaSipMessage { peer, message } = msg;
                let SipMessage::Response(resp) = message else { continue; };
                if !response_matches_call_id(&resp, &call_id) {
                    continue;
                }
                if is_cancel_response(&resp) {
                    continue;
                }
                if resp.status_code < 200 {
                    provisional_received = true;
                    if cancel_requested && !cancel_sent {
                        let cancel = SipRequestBuilder::new(SipMethod::Cancel, request_uri.clone())
                            .header("Via", invite_via.clone())
                            .header("Max-Forwards", "70")
                            .header("From", from_header.clone())
                            .header("To", to_header.clone())
                            .header("Call-ID", call_id.clone())
                            .header("CSeq", format!("{cseq} CANCEL"))
                            .build();
                        log_cancel("outbound", sip_peer, &cancel);
                        let _ = send_b2bua_payload(TransportPeer::Udp(sip_peer), cancel.to_bytes());
                        cancel_sent = true;
                    }
                    if resp.status_code == 180 {
                        let _ = tx_in.try_send(SessionIn::B2buaRinging);
                    } else if resp.status_code == 183
                        && !early_media_sent
                        && !resp.body.is_empty()
                    {
                        let _ = tx_in.try_send(SessionIn::B2buaEarlyMedia);
                        rtp_guard.start(a_call_id.as_str(), rtp_socket.clone(), tx_in.clone());
                        early_media_sent = true;
                    }
                    continue;
                }
                if resp.status_code == 401 || resp.status_code == 407 {
                    let ack_to = header_value(&resp.headers, "To").unwrap_or(to_header.as_str());
                    send_non2xx_ack(
                        peer,
                        request_uri.as_str(),
                        invite_via.as_str(),
                        from_header.as_str(),
                        ack_to,
                        call_id.as_str(),
                        cseq,
                    );
                    if auth_attempts >= max_auth_attempts {
                        return Err(anyhow!(OutboundError { status: resp.status_code }));
                    }
                    let (challenge_header, auth_header) = if resp.status_code == 401 {
                        ("WWW-Authenticate", "Authorization")
                    } else {
                        ("Proxy-Authenticate", "Proxy-Authorization")
                    };
                    let Some(challenge_value) = header_value(&resp.headers, challenge_header) else {
                        return Err(anyhow!(OutboundError { status: resp.status_code }));
                    };
                    let Some(challenge) = parse_digest_challenge(challenge_value) else {
                        return Err(anyhow!(OutboundError { status: resp.status_code }));
                    };
                    if let Some(header_kind) = DigestAuthHeader::from_name(auth_header) {
                        auth_cache::store(DigestAuthChallenge {
                            header: header_kind,
                            challenge: challenge.clone(),
                        });
                    }
                    let Some(auth_value) = build_outbound_auth_value(
                        registrar,
                        request_uri.as_str(),
                        &challenge,
                        &mut auth_nc,
                        &mut auth_nonce,
                    ) else {
                        return Err(anyhow!(OutboundError { status: resp.status_code }));
                    };
                    auth_attempts = auth_attempts.saturating_add(1);
                    cseq = cseq.saturating_add(1);
                    provisional_received = false;
                    invite_via = build_via(via_host.as_str(), sip_port);
                    send_outbound_invite(
                        sip_peer,
                        &request_uri,
                        &from_header,
                        &to_header,
                        &call_id,
                        cseq,
                        invite_via.as_str(),
                        sip_port,
                        registrar,
                        &sdp,
                        Some((auth_header, auth_value)),
                    )
                    .await?;
                    continue;
                }
                if resp.status_code >= 300 {
                    let ack_to = header_value(&resp.headers, "To").unwrap_or(to_header.as_str());
                    send_non2xx_ack(
                        peer,
                        request_uri.as_str(),
                        invite_via.as_str(),
                        from_header.as_str(),
                        ack_to,
                        call_id.as_str(),
                        cseq,
                    );
                    if resp.status_code == 403 {
                        info!(
                            "[b2bua {}] outbound response dump:\n{}",
                            a_call_id,
                            format_response_dump(&resp)
                        );
                    }
                    if cancel_requested {
                        return Ok(None);
                    }
                    return Err(anyhow!(OutboundError { status: resp.status_code }));
                }

                let to_header = header_value(&resp.headers, "To")
                    .ok_or_else(|| anyhow!("missing To header"))?
                    .to_string();
                if extract_tag(&to_header).is_none() {
                    return Err(anyhow!("missing To tag in 200 OK"));
                }

                let remote_uri = header_value(&resp.headers, "Contact")
                    .map(extract_contact_uri)
                    .unwrap_or(request_uri.as_str())
                    .to_string();
                let remote_sdp =
                    parse_offer_sdp(&resp.body).ok_or_else(|| anyhow!("missing SDP in 200 OK"))?;
                let remote_rtp_addr = resolve_rtp_addr(&remote_sdp)?;

                let ack = SipRequestBuilder::new(SipMethod::Ack, remote_uri.clone())
                    .header("Via", build_via(via_host.as_str(), sip_port))
                    .header("Max-Forwards", "70")
                    .header("From", from_header.clone())
                    .header("To", to_header.clone())
                    .header("Call-ID", call_id.clone())
                    .header("CSeq", format!("{} ACK", cseq))
                    .build();
                send_b2bua_payload(TransportPeer::Udp(sip_peer), ack.to_bytes())?;

                if cancel_requested {
                    let bye_cseq = cseq.saturating_add(1).max(2);
                    let bye = SipRequestBuilder::new(SipMethod::Bye, remote_uri.clone())
                        .header("Via", build_via(via_host.as_str(), sip_port))
                        .header("Max-Forwards", "70")
                        .header("From", from_header.clone())
                        .header("To", to_header.clone())
                        .header("Call-ID", call_id.clone())
                        .header("CSeq", format!("{bye_cseq} BYE"))
                        .build();
                    let _ = send_b2bua_payload(TransportPeer::Udp(sip_peer), bye.to_bytes());
                    return Ok(None);
                }

                spawn_sip_listener(
                    a_call_id.clone(),
                    call_id.clone(),
                    sip_rx,
                    tx_in.clone(),
                    shutdown.clone(),
                    shutdown_notify.clone(),
                );
                rtp_guard.start(a_call_id.as_str(), rtp_socket.clone(), tx_in.clone());

                let b_leg = BLeg {
                    call_id,
                    rtp_key: format!("{}-b", a_call_id),
                    remote_rtp_addr,
                    sip_peer,
                    from_header,
                    to_header,
                    remote_uri,
                    cseq,
                    via_host,
                    via_port: sip_port,
                    _b2bua_reg: b2bua_reg,
                    shutdown,
                    shutdown_notify,
                };
                rtp_guard.disarm();
                return Ok(Some(b_leg));
            }
        }
    }
}

fn spawn_sip_listener(
    a_call_id: String,
    b_call_id: String,
    mut sip_rx: mpsc::Receiver<B2buaSipMessage>,
    tx_in: mpsc::Sender<SessionIn>,
    shutdown: Arc<AtomicBool>,
    shutdown_notify: Arc<Notify>,
) {
    tokio::spawn(async move {
        loop {
            if shutdown.load(Ordering::SeqCst) {
                break;
            }
            tokio::select! {
                _ = shutdown_notify.notified() => {
                    if shutdown.load(Ordering::SeqCst) {
                        break;
                    }
                }
                maybe_msg = sip_rx.recv() => {
                    let Some(msg) = maybe_msg else { break; };
                    let B2buaSipMessage { peer, message } = msg;
                    match message {
                        SipMessage::Request(req) => {
                            if !request_matches_call_id(&req, &b_call_id) {
                                continue;
                            }
                            if matches!(req.method, SipMethod::Bye) {
                                if let Some(resp) = response_simple_from_request(&req, 200, "OK") {
                                    let _ = send_b2bua_payload(peer, resp.to_bytes());
                                }
                                let _ = tx_in.try_send(SessionIn::BLegBye);
                                break;
                            }
                        }
                        SipMessage::Response(_) => {
                            // ignore
                        }
                    }
                }
            }
        }
        info!("[b2bua {}] sip listener ended", a_call_id);
    });
}

fn spawn_rtp_listener(
    a_call_id: String,
    rtp_socket: Arc<UdpSocket>,
    tx_in: mpsc::Sender<SessionIn>,
    shutdown: Arc<AtomicBool>,
    shutdown_notify: Arc<Notify>,
) {
    tokio::spawn(async move {
        let mut buf = vec![0u8; RTP_BUFFER_SIZE];
        loop {
            if shutdown.load(Ordering::SeqCst) {
                break;
            }
            tokio::select! {
                _ = shutdown_notify.notified() => {
                    if shutdown.load(Ordering::SeqCst) {
                        break;
                    }
                }
                recv = rtp_socket.recv_from(&mut buf) => {
                    let Ok((len, _src)) = recv else { continue; };
                    let Ok(pkt) = parse_rtp_packet(&buf[..len]) else { continue; };
                    let codec = match codec_from_pt(pkt.payload_type) {
                        Ok(codec) => codec,
                        Err(err) => {
                            warn!(
                                "[b2bua {}] unsupported payload type {}",
                                a_call_id, err.0
                            );
                            continue;
                        }
                    };
                    let payload = decode_to_mulaw(codec, &pkt.payload);
                    let _ = tx_in.try_send(SessionIn::BLegRtp { payload });
                }
            }
        }
        info!("[b2bua {}] rtp listener ended", a_call_id);
    });
}

fn send_b2bua_payload(peer: TransportPeer, payload: Vec<u8>) -> Result<()> {
    if b2bua_bridge::send(peer, payload) {
        Ok(())
    } else {
        Err(anyhow!("b2bua transport not initialized"))
    }
}

fn send_non2xx_ack(
    peer: TransportPeer,
    request_uri: &str,
    via: &str,
    from_header: &str,
    to_header: &str,
    call_id: &str,
    cseq: u32,
) {
    let ack = SipRequestBuilder::new(SipMethod::Ack, request_uri.to_string())
        .header("Via", via.to_string())
        .header("Max-Forwards", "70")
        .header("From", from_header.to_string())
        .header("To", to_header.to_string())
        .header("Call-ID", call_id.to_string())
        .header("CSeq", format!("{cseq} ACK"))
        .build();
    if let Err(err) = send_b2bua_payload(peer, ack.to_bytes()) {
        warn!("[b2bua] failed to send non-2xx ACK: {}", err);
    }
}

/// Resolve a SIP URI's host and port to a socket address.
///
/// If the URI has no port, the default SIP port (5060) is used. The function parses
/// the provided SIP URI, performs DNS resolution for the host:port pair, and returns
/// the first SocketAddr found.
///
/// # Errors
///
/// Returns an error if the URI cannot be parsed or if name resolution yields no addresses.
///
/// # Examples
///
/// ```no_run
/// let addr = resolve_target_addr("sip:alice@example.com").unwrap();
/// println!("{}", addr);
/// ```
fn resolve_target_addr(uri: &str) -> Result<SocketAddr> {
    let parsed = parse_uri(uri)?;
    let port = parsed.port.unwrap_or(DEFAULT_SIP_PORT);
    let host = parsed.host;
    let mut addrs = (host.as_str(), port).to_socket_addrs()?;
    addrs
        .next()
        .ok_or_else(|| anyhow!("unable to resolve {}", host))
}

/// Resolve the first socket address for the SDP's IP and port.
///
/// # Parameters
///
/// * `sdp` - SDP containing the `ip` and `port` to resolve.
///
/// # Returns
///
/// A `SocketAddr` for the SDP's IP and port, or an error if the address cannot be resolved.
///
/// # Examples
///
/// ```
/// use std::net::SocketAddr;
/// let sdp = crate::Sdp { ip: "127.0.0.1".into(), port: 1234 };
/// let addr = crate::resolve_rtp_addr(&sdp).unwrap();
/// assert_eq!(addr, "127.0.0.1:1234".parse::<SocketAddr>().unwrap());
/// ```
fn resolve_rtp_addr(sdp: &Sdp) -> Result<SocketAddr> {
    let mut addrs = (sdp.ip.as_str(), sdp.port).to_socket_addrs()?;
    addrs
        .next()
        .ok_or_else(|| anyhow!("unable to resolve {}", sdp.ip))
}

fn build_sdp(ip: &str, port: u16) -> String {
    format!(
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
        ip = ip,
        rtp = port
    )
}

fn build_outbound_auth_value(
    registrar: &RegistrarConfig,
    request_uri: &str,
    challenge: &crate::sip::auth::DigestChallenge,
    auth_nc: &mut u32,
    last_nonce: &mut Option<String>,
) -> Option<String> {
    let password = registrar.auth_password.as_deref()?;
    if last_nonce.as_deref() != Some(challenge.nonce.as_str()) {
        *last_nonce = Some(challenge.nonce.clone());
        *auth_nc = 0;
    }
    let next_nc = auth_nc.saturating_add(1);
    let auth_value = build_authorization_header(
        registrar.auth_username.as_str(),
        password,
        "INVITE",
        request_uri,
        challenge,
        next_nc,
    )?;
    *auth_nc = next_nc;
    Some(auth_value)
}

/// Sends an outbound SIP INVITE to the specified peer using the B2BUA transport.
///
/// Constructs an INVITE request with the provided request URI, headers, Contact built
/// from the registrar and `via_port`, an optional authentication header, and the given SDP
/// body, then sends it via the B2BUA UDP transport to `peer`.
///
/// # Parameters
///
/// - `auth`: when `Some((name, value))`, the pair is added as an extra header (typically
///   an Authorization or Proxy-Authorization header) where `name` is the header name and
///   `value` is the header value.
///
/// # Errors
///
/// Returns an `Err` if sending the constructed request payload over the B2BUA transport fails.
///
/// # Examples
///
/// ```no_run
/// use std::net::SocketAddr;
///
/// // `RegistrarConfig` is the registrar configuration containing `user` and `contact_host`.
/// // Assume `registrar` and other values are available in the calling context.
/// # struct RegistrarConfig { user: String, contact_host: String }
/// # async fn example(registrar: RegistrarConfig) -> Result<(), Box<dyn std::error::Error>> {
/// let peer: SocketAddr = "192.0.2.10:5060".parse()?;
/// let request_uri = "sip:1234@example.com";
/// let from_header = "<sip:alice@example.com>";
/// let to_header = "<sip:1234@example.com>";
/// let call_id = "callid123";
/// let cseq = 1;
/// let via = "SIP/2.0/UDP 198.51.100.1:5060;branch=z9hG4bK...";
/// let via_port = 5060;
/// let sdp = "v=0\r\n...";
/// let auth = Some(("Authorization", "Digest ...".to_string()));
///
/// // send_outbound_invite(peer, request_uri, from_header, to_header, call_id, cseq,
/// //     via, via_port, &registrar, sdp, auth).await?;
/// # Ok(()) }
/// ```
async fn send_outbound_invite(
    peer: SocketAddr,
    request_uri: &str,
    from_header: &str,
    to_header: &str,
    call_id: &str,
    cseq: u32,
    via: &str,
    via_port: u16,
    registrar: &RegistrarConfig,
    sdp: &str,
    auth: Option<(&str, String)>,
) -> Result<()> {
    let mut builder = SipRequestBuilder::new(SipMethod::Invite, request_uri.to_string())
        .header("Via", via.to_string())
        .header("Max-Forwards", "70")
        .header("From", from_header.to_string())
        .header("To", to_header.to_string())
        .header("Call-ID", call_id.to_string())
        .header("CSeq", format!("{cseq} INVITE"))
        .header(
            "Contact",
            format!(
                "<sip:{}@{}:{}>",
                registrar.user, registrar.contact_host, via_port
            ),
        )
        .body(sdp.as_bytes(), Some("application/sdp"));
    if let Some((name, value)) = auth {
        builder = builder.header(name, value);
    }
    let request = builder.build();
    log_invite("outbound", peer, &request);
    send_b2bua_payload(TransportPeer::Udp(peer), request.to_bytes())?;
    Ok(())
}

/// Builds a SIP Via header value for a UDP transport with a generated branch parameter.
///
/// # Examples
///
/// ```
/// let via = build_via("198.51.100.1", 5060);
/// assert!(via.starts_with("SIP/2.0/UDP 198.51.100.1:5060"));
/// assert!(via.contains(";branch="));
/// ```
fn build_via(host: &str, port: u16) -> String {
    format!("SIP/2.0/UDP {}:{};branch={}", host, port, generate_branch())
}

/// Generates a unique SIP "branch" parameter suitable for Via headers.

///

/// The returned string is a branch token prefixed with `z9hG4bK-` followed by a

/// randomized numeric component to avoid collisions.

///

/// # Examples

///

/// ```

/// let branch = generate_branch();

/// assert!(branch.starts_with("z9hG4bK-"));

/// ```
fn generate_branch() -> String {
    let mut rng = rand::thread_rng();
    format!("z9hG4bK-{}", rng.gen::<u64>())
}

fn generate_tag() -> String {
    let mut rng = rand::thread_rng();
    format!("b2bua{}", rng.gen::<u32>())
}

fn header_value<'a>(headers: &'a [SipHeader], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|h| h.name.eq_ignore_ascii_case(name))
        .map(|h| h.value.as_str())
}

fn response_matches_call_id(resp: &crate::sip::message::SipResponse, call_id: &str) -> bool {
    header_value(&resp.headers, "Call-ID")
        .map(|value| value == call_id)
        .unwrap_or(false)
}

fn is_cancel_response(resp: &crate::sip::message::SipResponse) -> bool {
    let Some(value) = header_value(&resp.headers, "CSeq") else {
        return false;
    };
    let Ok(cseq) = parse_cseq_header(value) else {
        return false;
    };
    cseq.method.eq_ignore_ascii_case("CANCEL")
}

fn request_matches_call_id(req: &SipRequest, call_id: &str) -> bool {
    req.header_value("Call-ID")
        .map(|value| value == call_id)
        .unwrap_or(false)
}

fn extract_tag(value: &str) -> Option<String> {
    let lower = value.to_ascii_lowercase();
    let idx = lower.find("tag=")?;
    let rest = &value[idx + 4..];
    let end = rest
        .find(|c: char| c == ';' || c == '>' || c == ' ')
        .unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

/// Extracts the URI portion from a SIP Contact header value.
///
/// The function returns the URI without surrounding `<` and `>` when present,
/// otherwise returns the left-most token before any `;` parameter, trimmed of whitespace.
///
/// # Parameters
///
/// - `value`: SIP Contact header value, which may contain an angle-bracketed URI and optional parameters.
///
/// # Returns
///
/// A string slice containing the extracted URI (without angle brackets or parameters).
///
/// # Examples
///
/// ```
/// assert_eq!(extract_contact_uri("<sip:alice@example.com>;expires=3600"), "sip:alice@example.com");
/// assert_eq!(extract_contact_uri("sip:bob@example.org"), "sip:bob@example.org");
/// assert_eq!(extract_contact_uri("  <sip:carol@host>  "), "sip:carol@host");
/// ```
fn extract_contact_uri(value: &str) -> &str {
    let trimmed = value.trim();
    if let Some(start) = trimmed.find('<') {
        if let Some(end) = trimmed[start + 1..].find('>') {
            return &trimmed[start + 1..start + 1 + end];
        }
    }
    trimmed.split(';').next().unwrap_or(trimmed).trim()
}

/// Logs key fields of an INVITE SIP request at INFO level.
///
/// The log includes the request URI, `From`, `To`, `Contact`, `Call-ID`, and
/// which authorization header is present (`Authorization`, `Proxy-Authorization`,
/// or `none`). If the INFO log level is disabled, the function returns without
/// performing any work.
///
/// # Examples
///
/// ```ignore
/// // Assuming `request` is a `SipRequest` and `peer` is a `SocketAddr`:
/// log_invite("outbound", peer, &request);
/// ```
fn log_invite(label: &str, peer: SocketAddr, request: &SipRequest) {
    if !log::log_enabled!(log::Level::Info) {
        return;
    }
    let from = mask_pii(request.header_value("From").unwrap_or("-"));
    let to = mask_pii(request.header_value("To").unwrap_or("-"));
    let contact = mask_pii(request.header_value("Contact").unwrap_or("-"));
    let call_id = request.header_value("Call-ID").unwrap_or("-");
    let auth_header = if request.header_value("Authorization").is_some() {
        "Authorization"
    } else if request.header_value("Proxy-Authorization").is_some() {
        "Proxy-Authorization"
    } else {
        "none"
    };
    info!(
        "[b2bua {}] INVITE -> {} uri={} from={} to={} contact={} call_id={} auth={}",
        label, peer, request.uri, from, to, contact, call_id, auth_header
    );
}

fn log_cancel(label: &str, peer: SocketAddr, request: &SipRequest) {
    if !log::log_enabled!(log::Level::Info) {
        return;
    }
    let from = mask_pii(request.header_value("From").unwrap_or("-"));
    let to = mask_pii(request.header_value("To").unwrap_or("-"));
    let call_id = request.header_value("Call-ID").unwrap_or("-");
    let cseq = request.header_value("CSeq").unwrap_or("-");
    info!(
        "[b2bua {}] CANCEL -> {} uri={} from={} to={} call_id={} cseq={}",
        label, peer, request.uri, from, to, call_id, cseq
    );
}

/// Formats a SIP response for logging, omitting Authorization headers and summarizing large or non-UTF-8 bodies.
///
/// The returned string contains the status line, all headers except `Authorization` and
/// `Proxy-Authorization`, and then either the UTF-8 body if its length is 1024 bytes or less,
/// or a `<body_len=N>` placeholder when the body is larger or not valid UTF-8.
///
/// # Examples
///
/// ```
/// let resp = SipResponse {
///     version: "SIP/2.0".to_string(),
///     status_code: 200,
///     reason_phrase: "OK".to_string(),
///     headers: vec![],
///     body: vec![],
/// };
/// let dump = format_response_dump(&resp);
/// assert!(dump.starts_with("SIP/2.0 200 OK"));
/// ```
fn format_response_dump(resp: &SipResponse) -> String {
    let mut out = String::new();
    let _ = std::fmt::Write::write_fmt(
        &mut out,
        format_args!(
            "{} {} {}\r\n",
            resp.version, resp.status_code, resp.reason_phrase
        ),
    );
    for header in &resp.headers {
        if header.name.eq_ignore_ascii_case("Authorization")
            || header.name.eq_ignore_ascii_case("Proxy-Authorization")
        {
            continue;
        }
        let _ = std::fmt::Write::write_fmt(
            &mut out,
            format_args!("{}: {}\r\n", header.name, header.value),
        );
    }
    if !resp.body.is_empty() {
        if let Ok(body) = std::str::from_utf8(&resp.body) {
            if body.len() <= 1024 {
                let _ = std::fmt::Write::write_fmt(&mut out, format_args!("\r\n{}", body));
            } else {
                let _ = std::fmt::Write::write_fmt(
                    &mut out,
                    format_args!("\r\n<body_len={}>", resp.body.len()),
                );
            }
        } else {
            let _ = std::fmt::Write::write_fmt(
                &mut out,
                format_args!("\r\n<body_len={}>", resp.body.len()),
            );
        }
    }
    out
}
