use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use log::{info, warn};
use rand::Rng;
use tokio::net::UdpSocket;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::Notify;
use tokio::time::sleep;

use crate::config;
use crate::config::RegistrarConfig;
use crate::rtp::codec::{codec_from_pt, decode_to_mulaw};
use crate::rtp::parser::parse_rtp_packet;
use crate::session::types::{SessionIn, Sdp};
use crate::sip::auth::{build_authorization_header, parse_digest_challenge};
use crate::sip::auth_cache::{self, DigestAuthChallenge, DigestAuthHeader};
use crate::sip::b2bua_bridge::{self, B2buaRegistration, B2buaSipMessage};
use crate::sip::builder::response_simple_from_request;
use crate::sip::message::{SipHeader, SipMessage, SipMethod, SipRequest, SipResponse};
use crate::sip::{parse_uri, SipRequestBuilder};
use crate::transport::TransportPeer;

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
    tx_in: UnboundedSender<SessionIn>,
) -> tokio::sync::oneshot::Sender<()> {
    let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        match run_transfer(a_call_id.clone(), tx_in.clone(), cancel_rx).await {
            Ok(Some(b_leg)) => {
                let _ = tx_in.send(SessionIn::B2buaEstablished { b_leg });
            }
            Ok(None) => {
                info!("[b2bua {}] transfer cancelled", a_call_id);
            }
            Err(err) => {
                let _ = tx_in.send(SessionIn::B2buaFailed {
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
    tx_in: UnboundedSender<SessionIn>,
) -> tokio::sync::oneshot::Sender<()> {
    let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        match run_outbound(a_call_id.clone(), number, tx_in.clone(), cancel_rx).await {
            Ok(Some(b_leg)) => {
                let _ = tx_in.send(SessionIn::B2buaEstablished { b_leg });
            }
            Ok(None) => {
                info!("[b2bua {}] outbound cancelled", a_call_id);
            }
            Err(err) => {
                let _ = tx_in.send(SessionIn::B2buaFailed {
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
    tx_in: UnboundedSender<SessionIn>,
    mut cancel_rx: tokio::sync::oneshot::Receiver<()>,
) -> Result<Option<BLeg>> {
    let cfg = config::Config::from_env()?;
    let target_uri = config::transfer_target_uri();
    let target_addr = resolve_target_addr(&target_uri)?;

    let sip_port = cfg.sip_port;
    let via_host = cfg.advertised_ip.clone();
    let via = build_via(via_host.as_str(), sip_port);
    let local_tag = generate_tag();
    let from_header = format!(
        "<sip:rustbot@{}:{}>;tag={}",
        cfg.advertised_ip, sip_port, local_tag
    );
    let to_header = format!("<{}>", target_uri);
    let b_call_id = format!("b2bua-{}-{}", a_call_id, rand::thread_rng().gen::<u32>());
    let (b2bua_reg, mut sip_rx) = b2bua_bridge::register(b_call_id.clone());

    let rtp_socket = Arc::new(UdpSocket::bind("0.0.0.0:0").await?);
    let rtp_port = rtp_socket.local_addr()?.port();
    let sdp = build_sdp(cfg.advertised_ip.as_str(), rtp_port);

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
            format!("<sip:rustbot@{}:{}>", cfg.advertised_ip, sip_port),
        )
        .body(sdp.as_bytes(), Some("application/sdp"))
        .build();

    log_invite("transfer", target_addr, &invite);
    send_b2bua_payload(TransportPeer::Udp(target_addr), invite.to_bytes())?;

    let timeout = config::transfer_timeout();
    let timeout_sleep = sleep(timeout);
    tokio::pin!(timeout_sleep);
    let mut provisional_received = false;

    loop {
        tokio::select! {
            _ = &mut cancel_rx => {
                if provisional_received {
                    let cancel = SipRequestBuilder::new(SipMethod::Cancel, target_uri.clone())
                        .header("Via", via.clone())
                        .header("Max-Forwards", "70")
                        .header("From", from_header.clone())
                        .header("To", to_header.clone())
                        .header("Call-ID", b_call_id.clone())
                        .header("CSeq", format!("{cseq} CANCEL"))
                        .build();
                    let _ = send_b2bua_payload(TransportPeer::Udp(target_addr), cancel.to_bytes());
                }
                return Ok(None);
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
                if resp.status_code < 200 {
                    provisional_received = true;
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
                    parse_sdp(&resp.body).ok_or_else(|| anyhow!("missing SDP in 200 OK"))?;
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

async fn run_outbound(
    a_call_id: String,
    number: String,
    tx_in: UnboundedSender<SessionIn>,
    mut cancel_rx: tokio::sync::oneshot::Receiver<()>,
) -> Result<Option<BLeg>> {
    let registrar =
        config::registrar_config().ok_or_else(|| anyhow!("missing registrar config"))?;
    if registrar.auth_password.is_none() {
        return Err(anyhow!("missing registrar auth password"));
    }
    if registrar.transport != crate::config::RegistrarTransport::Udp {
        return Err(anyhow!("outbound transport must be UDP"));
    }

    let outbound_cfg = config::outbound_config();
    let outbound_domain = outbound_cfg.domain.clone();
    if outbound_domain.is_empty() {
        return Err(anyhow!("missing outbound domain"));
    }

    let cfg = config::Config::from_env()?;
    let request_uri = format!("sip:{}@{}", number, outbound_domain);
    let sip_peer = registrar.addr;

    let sip_port = cfg.sip_port;
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
    let sdp = build_sdp(cfg.advertised_ip.as_str(), rtp_port);

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

    let timeout = config::transfer_timeout();
    let timeout_sleep = sleep(timeout);
    tokio::pin!(timeout_sleep);
    let mut provisional_received = false;
    let mut early_media_sent = false;
    let mut rtp_listener_started = false;
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_notify = Arc::new(Notify::new());

    loop {
        tokio::select! {
            _ = &mut cancel_rx => {
                if provisional_received {
                    let cancel = SipRequestBuilder::new(SipMethod::Cancel, request_uri.clone())
                        .header("Via", invite_via.clone())
                        .header("Max-Forwards", "70")
                        .header("From", from_header.clone())
                        .header("To", to_header.clone())
                        .header("Call-ID", call_id.clone())
                        .header("CSeq", format!("{cseq} CANCEL"))
                        .build();
                    let _ = send_b2bua_payload(TransportPeer::Udp(sip_peer), cancel.to_bytes());
                }
                return Ok(None);
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
                if resp.status_code < 200 {
                    provisional_received = true;
                    if resp.status_code == 180 {
                        let _ = tx_in.send(SessionIn::B2buaRinging);
                    } else if resp.status_code == 183
                        && !early_media_sent
                        && !resp.body.is_empty()
                    {
                        let _ = tx_in.send(SessionIn::B2buaEarlyMedia);
                        if !rtp_listener_started {
                            spawn_rtp_listener(
                                a_call_id.clone(),
                                rtp_socket.clone(),
                                tx_in.clone(),
                                shutdown.clone(),
                                shutdown_notify.clone(),
                            );
                            rtp_listener_started = true;
                        }
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
                    parse_sdp(&resp.body).ok_or_else(|| anyhow!("missing SDP in 200 OK"))?;
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

                spawn_sip_listener(
                    a_call_id.clone(),
                    call_id.clone(),
                    sip_rx,
                    tx_in.clone(),
                    shutdown.clone(),
                    shutdown_notify.clone(),
                );
                if !rtp_listener_started {
                    spawn_rtp_listener(
                        a_call_id.clone(),
                        rtp_socket.clone(),
                        tx_in.clone(),
                        shutdown.clone(),
                        shutdown_notify.clone(),
                    );
                }

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
                return Ok(Some(b_leg));
            }
        }
    }
}

fn spawn_sip_listener(
    a_call_id: String,
    b_call_id: String,
    mut sip_rx: UnboundedReceiver<B2buaSipMessage>,
    tx_in: UnboundedSender<SessionIn>,
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
                                let _ = tx_in.send(SessionIn::BLegBye);
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
    tx_in: UnboundedSender<SessionIn>,
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
                    let _ = tx_in.send(SessionIn::BLegRtp { payload });
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

fn resolve_target_addr(uri: &str) -> Result<SocketAddr> {
    let parsed = parse_uri(uri)?;
    let port = parsed.port.unwrap_or(DEFAULT_SIP_PORT);
    let host = parsed.host;
    let mut addrs = (host.as_str(), port).to_socket_addrs()?;
    addrs.next().ok_or_else(|| anyhow!("unable to resolve {}", host))
}

fn resolve_rtp_addr(sdp: &Sdp) -> Result<SocketAddr> {
    let mut addrs = (sdp.ip.as_str(), sdp.port).to_socket_addrs()?;
    addrs
        .next()
        .ok_or_else(|| anyhow!("unable to resolve {}", sdp.ip))
}

fn parse_sdp(body: &[u8]) -> Option<Sdp> {
    let s = std::str::from_utf8(body).ok()?;
    let mut ip = None;
    let mut port = None;
    let mut pt = None;
    for line in s.lines() {
        let line = line.trim();
        if line.starts_with("c=IN IP4 ") {
            ip = Some(line.trim_start_matches("c=IN IP4 ").trim().to_string());
        } else if line.starts_with("m=audio ") {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() >= 4 {
                port = cols[1].parse::<u16>().ok();
                pt = cols[3].parse::<u8>().ok();
            }
        }
    }
    Some(Sdp {
        ip: ip?,
        port: port?,
        payload_type: pt.unwrap_or(0),
        codec: "PCMU/8000".to_string(),
    })
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

fn build_via(host: &str, port: u16) -> String {
    format!(
        "SIP/2.0/UDP {}:{};branch={}",
        host,
        port,
        generate_branch()
    )
}

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

fn extract_contact_uri(value: &str) -> &str {
    let trimmed = value.trim();
    if let Some(start) = trimmed.find('<') {
        if let Some(end) = trimmed[start + 1..].find('>') {
            return &trimmed[start + 1..start + 1 + end];
        }
    }
    trimmed
        .split(';')
        .next()
        .unwrap_or(trimmed)
        .trim()
}

fn log_invite(label: &str, peer: SocketAddr, request: &SipRequest) {
    if !log::log_enabled!(log::Level::Info) {
        return;
    }
    let from = request.header_value("From").unwrap_or("-");
    let to = request.header_value("To").unwrap_or("-");
    let contact = request.header_value("Contact").unwrap_or("-");
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

fn format_response_dump(resp: &SipResponse) -> String {
    let mut out = String::new();
    let _ = std::fmt::Write::write_fmt(
        &mut out,
        format_args!("{} {} {}\r\n", resp.version, resp.status_code, resp.reason_phrase),
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
