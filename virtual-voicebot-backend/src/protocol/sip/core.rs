use crate::protocol::sip::b2bua_bridge;
use crate::protocol::sip::builder::{
    response_final_with_sdp, response_options_from_request, response_provisional_from_request,
    response_simple_from_request,
};
use crate::protocol::sip::codec::{parse_cseq_header, parse_sip_message, SipRequestBuilder};
use crate::protocol::sip::message::{SipHeader, SipMessage, SipMethod, SipRequest, SipResponse};
use crate::protocol::sip::register::RegisterClient;
use crate::protocol::sip::transaction::{
    InviteServerTransaction, InviteTxAction, InviteTxState, NonInviteServerTransaction,
    NonInviteTxState,
};
use crate::protocol::sip::transport::{SipTransportRequest, SipTransportTx};
use crate::protocol::sip::types::{SipConfig, SipEvent};
use crate::protocol::transport::{SipInput, TransportPeer};
use crate::shared::config;
use crate::shared::entities::CallId;
use crate::shared::ports::sip::{Sdp, SessionRefresher, SessionTimerInfo, SipCommand};
use rand::Rng;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::Notify;
use tokio::time::sleep;

/// SIP 処理のエントリポイント。トランザクション状態と送信経路を保持する。
pub struct SipCore {
    cfg: SipConfig,
    transport_tx: SipTransportTx,
    invites: HashMap<CallId, InviteContext>,
    non_invites: HashMap<CallId, NonInviteServerTransaction>,
    register: Option<Arc<Mutex<RegisterClient>>>,
    register_notify: Option<Arc<Notify>>,
    active_call_id: Option<CallId>,
}

struct InviteContext {
    tx: InviteServerTransaction,
    req: SipRequest,
    reliable: Option<ReliableProvisional>,
    expected_rack: Option<RAckHeader>,
    last_rseq: Option<u32>,
    session_timer: Option<SessionTimerConfig>,
    final_ok: Option<FinalOkRetransmit>,
    final_ok_payload: Option<Vec<u8>>,
    local_cseq: u32,
    expires_at: Option<Instant>,
}

struct ReliableProvisional {
    stop: Arc<AtomicBool>,
}

struct FinalOkRetransmit {
    stop: Arc<AtomicBool>,
}

#[derive(Debug, Clone)]
struct SessionTimerConfig {
    expires: Duration,
    refresher: SessionRefresher,
    min_se: u64,
}

#[derive(Debug, Clone)]
struct RAckHeader {
    rseq: u32,
    cseq: u32,
    method: String,
}

const STATIC_PT_MAP: &[(u8, &str)] = &[
    (0, "PCMU/8000"),
    (3, "GSM/8000"),
    (4, "G723/8000"),
    (8, "PCMA/8000"),
    (9, "G722/8000"),
    (18, "G729/8000"),
];

fn static_pt_to_codec(pt: u8) -> Option<&'static str> {
    STATIC_PT_MAP
        .iter()
        .find(|(entry_pt, _)| *entry_pt == pt)
        .map(|(_, codec)| *codec)
}

fn parse_rtpmap_line(line: &str) -> Option<(u8, String)> {
    let value = line.trim().strip_prefix("a=rtpmap:")?;
    let mut parts = value.split_whitespace();
    let pt = parts.next()?.parse::<u8>().ok()?;
    let encoding = parts.next()?;
    let mut enc_parts = encoding.split('/');
    let name = enc_parts.next()?.trim();
    let rate = enc_parts.next()?.trim();
    if name.is_empty() || rate.is_empty() {
        return None;
    }
    Some((pt, format!("{}/{}", name, rate)))
}

pub fn parse_offer_sdp(body: &[u8]) -> Option<Sdp> {
    let s = std::str::from_utf8(body).ok()?;
    let mut ip = None;
    let mut port = None;
    let mut pt = None;
    let mut rtpmap: HashMap<u8, String> = HashMap::new();
    for line in s.lines() {
        let line = line.trim();
        if line.starts_with("c=IN IP4 ") {
            let v = line.trim_start_matches("c=IN IP4 ").trim();
            ip = Some(v.to_string());
        } else if line.starts_with("m=audio ") {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() >= 4 {
                port = cols[1].parse::<u16>().ok();
                pt = cols[3].parse::<u8>().ok();
            }
        } else if line.starts_with("a=rtpmap:") {
            if let Some((pt, codec)) = parse_rtpmap_line(line) {
                rtpmap.insert(pt, codec);
            }
        }
    }
    let ip = ip?;
    let port = port?;
    let pt = pt.unwrap_or(0);
    let codec = rtpmap
        .get(&pt)
        .cloned()
        .or_else(|| static_pt_to_codec(pt).map(|codec| codec.to_string()))
        .unwrap_or_else(|| "unknown".to_string());
    Some(Sdp {
        ip,
        port,
        payload_type: pt,
        codec,
    })
}

fn decode_sip_text(data: &[u8]) -> Result<String, ()> {
    String::from_utf8(data.to_vec()).map_err(|_| ())
}

fn parse_rack(value: &str) -> Option<RAckHeader> {
    let mut parts = value.split_whitespace();
    let rseq_str = parts.next()?;
    let cseq_str = parts.next()?;
    let method = parts.next()?.to_string();
    let rseq = rseq_str.parse::<u32>().ok()?;
    let cseq = cseq_str.parse::<u32>().ok()?;
    Some(RAckHeader { rseq, cseq, method })
}

fn rack_matches(expected: &RAckHeader, actual: &RAckHeader) -> bool {
    expected.rseq == actual.rseq
        && expected.cseq == actual.cseq
        && expected.method.eq_ignore_ascii_case(&actual.method)
}

fn invite_has_to_tag(req: &SipRequest) -> bool {
    let Some(to_header) = req.header_value("To") else {
        return false;
    };
    if let Some(params) = to_header.split('>').nth(1) {
        return params.split(';').any(|param| {
            param
                .split('=')
                .next()
                .map(|key| key.trim().eq_ignore_ascii_case("tag"))
                .unwrap_or(false)
        });
    }
    to_header.split(';').skip(1).any(|param| {
        param
            .split('=')
            .next()
            .map(|key| key.trim().eq_ignore_ascii_case("tag"))
            .unwrap_or(false)
    })
}

fn header_has_token(value: Option<&str>, token: &str) -> bool {
    let Some(value) = value else {
        return false;
    };
    value
        .split([',', ' ', '\t'])
        .filter(|part| !part.is_empty())
        .any(|part| part.eq_ignore_ascii_case(token))
}

fn supports_100rel(req: &SipRequest) -> bool {
    header_has_token(req.header_value("Supported"), "100rel")
        || header_has_token(req.header_value("Require"), "100rel")
}

impl SessionRefresher {
    fn as_str(self) -> &'static str {
        match self {
            SessionRefresher::Uac => "uac",
            SessionRefresher::Uas => "uas",
        }
    }
}

fn parse_session_expires(value: &str) -> Option<(u64, Option<SessionRefresher>)> {
    let mut parts = value.split(';');
    let expires = parts.next()?.trim().parse::<u64>().ok()?;
    let mut refresher = None;
    for part in parts {
        let part = part.trim();
        if let Some(val) = part.strip_prefix("refresher=") {
            refresher = match val.trim().to_ascii_lowercase().as_str() {
                "uac" => Some(SessionRefresher::Uac),
                "uas" => Some(SessionRefresher::Uas),
                _ => None,
            };
        }
    }
    Some((expires, refresher))
}

fn parse_min_se(value: &str) -> Option<u64> {
    value.trim().parse::<u64>().ok()
}

const RSEQ_MIN: u32 = 1;
const RSEQ_MAX: u32 = 0x7FFF_FFFF;

fn next_rseq(ctx: &mut InviteContext) -> Option<u32> {
    let mut rng = rand::thread_rng();
    next_rseq_with_rng(ctx, &mut rng)
}

fn next_rseq_with_rng<R: Rng + ?Sized>(ctx: &mut InviteContext, rng: &mut R) -> Option<u32> {
    let rseq = match ctx.last_rseq {
        Some(prev) => {
            if prev >= RSEQ_MAX {
                return None;
            }
            prev + 1
        }
        None => rng.gen_range(RSEQ_MIN..=RSEQ_MAX),
    };
    ctx.last_rseq = Some(rseq);
    Some(rseq)
}

#[derive(Debug)]
enum SessionTimerError {
    Invalid,
    TooSmall { min_se: u64 },
}

fn session_timer_from_request(
    req: &SipRequest,
    cfg: &config::SessionConfig,
    allow_default: bool,
) -> Result<Option<SessionTimerConfig>, SessionTimerError> {
    let raw = req.header_value("Session-Expires");
    let min_se_header = match req.header_value("Min-SE") {
        Some(value) => parse_min_se(value).ok_or(SessionTimerError::Invalid)?,
        None => 0,
    };
    let min_se = std::cmp::max(cfg.min_se, min_se_header);
    let (expires, refresher, from_request) = match raw {
        Some(value) => {
            let (expires, refresher_opt) =
                parse_session_expires(value).ok_or(SessionTimerError::Invalid)?;
            let refresher = refresher_opt.unwrap_or(SessionRefresher::Uac);
            (expires, refresher, true)
        }
        None => {
            if !allow_default {
                return Ok(None);
            }
            let Some(default_expires) = cfg.default_expires else {
                return Ok(None);
            };
            let expires = default_expires.as_secs();
            (expires, SessionRefresher::Uas, false)
        }
    };

    if from_request && expires < min_se {
        return Err(SessionTimerError::TooSmall { min_se });
    }
    let expires = if from_request {
        expires
    } else {
        std::cmp::max(expires, min_se)
    };
    Ok(Some(SessionTimerConfig {
        expires: Duration::from_secs(expires),
        refresher,
        min_se,
    }))
}

fn apply_session_timer_headers(resp: &mut SipResponse, cfg: &SessionTimerConfig) {
    resp.headers.push(SipHeader::new(
        "Session-Expires",
        format!(
            "{};refresher={}",
            cfg.expires.as_secs(),
            cfg.refresher.as_str()
        ),
    ));
    resp.headers
        .push(SipHeader::new("Min-SE", cfg.min_se.to_string()));
    let supported = resp
        .headers
        .iter()
        .find(|h| h.name.eq_ignore_ascii_case("Supported"))
        .map(|h| h.value.as_str());
    if !header_has_token(supported, "timer") {
        resp.headers.push(SipHeader::new("Supported", "timer"));
    }
}

/// Builds a SIP UPDATE request for the dialog represented by `ctx` using `cfg` and the given session expiration.
///
/// If the request contained in `ctx` lacks a `To`, `From`, or `Call-ID` header (or other required header values),
/// the function returns `None`. The function increments `ctx.local_cseq` (minimum value 1) and inserts a `Session-Expires`
/// header set from `expires`. If the original `From` header has no `tag` parameter, a `;tag=rustbot` is appended.
///
/// # Parameters
///
/// - `expires`: used to populate the `Session-Expires` header (seconds).
///
/// # Returns
///
/// `Some` containing the serialized bytes of the constructed UPDATE request on success, `None` if required header values are missing.
///
/// # Examples
///
/// ```ignore
/// // Example usage (types omitted for brevity):
/// let mut ctx = /* InviteContext built from an incoming INVITE */;
/// let cfg = /* SipConfig with advertised_ip and sip_port */;
/// let bytes = build_update_request(&mut ctx, &cfg, std::time::Duration::from_secs(90));
/// if let Some(pkt) = bytes {
///     // send pkt over transport
/// }
/// ```
fn build_update_request(
    ctx: &mut InviteContext,
    cfg: &SipConfig,
    expires: Duration,
) -> Option<Vec<u8>> {
    let req = &ctx.req;
    let to = req.header_value("From")?.to_string();
    let mut from = req.header_value("To")?.to_string();
    if !from.to_ascii_lowercase().contains("tag=") {
        from = format!("{from};tag=rustbot");
    }
    let call_id = req.header_value("Call-ID")?.to_string();
    let uri = req
        .header_value("Contact")
        .map(extract_contact_uri)
        .unwrap_or_else(|| req.uri.as_str())
        .to_string();
    let transport = match ctx.tx.peer {
        TransportPeer::Udp(_) => "UDP",
        TransportPeer::Tcp(_) => "TCP",
    };
    let via = format!(
        "SIP/2.0/{} {}:{};branch={}",
        transport,
        cfg.advertised_ip,
        cfg.sip_port,
        generate_branch()
    );
    let cseq = ctx.local_cseq.saturating_add(1).max(1);
    ctx.local_cseq = cseq;

    let contact_scheme = contact_scheme_from_uri(&req.uri);
    let builder = SipRequestBuilder::new(SipMethod::Update, uri)
        .header("Via", via)
        .header("Max-Forwards", "70")
        .header("From", from)
        .header("To", to)
        .header("Call-ID", call_id)
        .header("CSeq", format!("{cseq} UPDATE"))
        .header(
            "Contact",
            format!(
                "{contact_scheme}:rustbot@{}:{}",
                cfg.advertised_ip, cfg.sip_port
            ),
        )
        .header("Session-Expires", expires.as_secs().to_string())
        .header("Supported", "timer");
    Some(builder.build().to_bytes())
}

fn build_bye_request(ctx: &mut InviteContext, cfg: &SipConfig) -> Option<Vec<u8>> {
    let req = &ctx.req;
    let to = req.header_value("From")?.to_string();
    let mut from = req.header_value("To")?.to_string();
    if !from.to_ascii_lowercase().contains("tag=") {
        from = format!("{from};tag=rustbot");
    }
    let call_id = req.header_value("Call-ID")?.to_string();
    let uri = req
        .header_value("Contact")
        .map(extract_contact_uri)
        .unwrap_or_else(|| req.uri.as_str())
        .to_string();
    let transport = match ctx.tx.peer {
        TransportPeer::Udp(_) => "UDP",
        TransportPeer::Tcp(_) => "TCP",
    };
    let via = format!(
        "SIP/2.0/{} {}:{};branch={}",
        transport,
        cfg.advertised_ip,
        cfg.sip_port,
        generate_branch()
    );
    let cseq = ctx.local_cseq.saturating_add(1).max(1);
    ctx.local_cseq = cseq;

    let builder = SipRequestBuilder::new(SipMethod::Bye, uri)
        .header("Via", via)
        .header("Max-Forwards", "70")
        .header("From", from)
        .header("To", to)
        .header("Call-ID", call_id)
        .header("CSeq", format!("{cseq} BYE"));
    Some(builder.build().to_bytes())
}

/// Extracts the URI portion from a Contact header value.
///
/// If the header contains an angle-bracketed address, returns the substring inside the first `<...>`.
/// Otherwise returns the header value up to the first semicolon (`;`), trimmed of surrounding whitespace.
///
/// # Examples
///
/// ```
/// let uri = extract_contact_uri("Alice <sip:alice@example.com>;expires=3600");
/// assert_eq!(uri, "sip:alice@example.com");
///
/// let uri = extract_contact_uri("sip:bob@example.com;tag=123");
/// assert_eq!(uri, "sip:bob@example.com");
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

/// Selects the SIP contact scheme from a URI.
///
/// The check is case-insensitive and ignores leading ASCII whitespace.
///
/// # Returns
///
/// `'sips'` if the URI begins with `sips:` (case-insensitive, ignoring leading whitespace), `'sip'` otherwise.
///
/// # Examples
///
/// ```
/// assert_eq!(contact_scheme_from_uri("sips:alice@example.com"), "sips");
/// assert_eq!(contact_scheme_from_uri(" SIP:alice@example.com"), "sip");
/// assert_eq!(contact_scheme_from_uri("   sIps:bob@ex"), "sips");
/// assert_eq!(contact_scheme_from_uri("alice@example.com"), "sip");
/// ```
fn contact_scheme_from_uri(uri: &str) -> &'static str {
    if uri.trim_start().to_ascii_lowercase().starts_with("sips:") {
        "sips"
    } else {
        "sip"
    }
}

fn generate_branch() -> String {
    let mut rng = rand::thread_rng();
    format!("z9hG4bK-{}", rng.gen::<u64>())
}

#[derive(Debug)]
#[allow(dead_code)]
struct CoreHeaderSnapshot {
    // トランザクション導入時に再利用するためのコアヘッダ（現状は挙動維持のまま取り出す）
    via: String,
    from: String,
    to: String,
    call_id: CallId,
    cseq: String,
}

impl CoreHeaderSnapshot {
    fn from_request(
        req: &SipRequest,
    ) -> Result<Self, crate::shared::entities::identifiers::CallIdError> {
        Ok(Self {
            via: req.header_value("Via").unwrap_or("").to_string(),
            from: req.header_value("From").unwrap_or("").to_string(),
            to: req.header_value("To").unwrap_or("").to_string(),
            call_id: CallId::new(req.header_value("Call-ID").unwrap_or("").to_string())?,
            cseq: req.header_value("CSeq").unwrap_or("").to_string(),
        })
    }
}

fn spawn_register_task(
    register: Arc<Mutex<RegisterClient>>,
    notify: Arc<Notify>,
    transport_tx: SipTransportTx,
    src_port: u16,
) {
    tokio::spawn(async move {
        loop {
            let next_deadline = {
                let mut reg = register.lock().unwrap();
                let now = Instant::now();
                reg.check_expired(now);
                reg.next_timer_at()
            };
            let Some(deadline) = next_deadline else {
                notify.notified().await;
                continue;
            };
            tokio::select! {
                _ = tokio::time::sleep_until(tokio::time::Instant::from_std(deadline)) => {
                    let (peer, payload) = {
                        let mut reg = register.lock().unwrap();
                        let req = reg.pop_due_request(Instant::now());
                        let peer = req.as_ref().and_then(|_| reg.transport_peer());
                        let payload = req.map(|request| request.to_bytes());
                        (peer, payload)
                    };
                    match (peer, payload) {
                        (Some(peer), Some(payload)) => {
                            if let Err(err) = transport_tx.try_send(SipTransportRequest {
                                peer,
                                src_port,
                                payload,
                            }) {
                                log::error!(
                                    "[sip register] failed to enqueue refresh request peer={:?} err={:?}",
                                    peer,
                                    err
                                );
                            }
                        }
                        (None, Some(_)) => {
                            log::warn!("[sip register] no transport for refresh");
                        }
                        _ => {}
                    }
                }
                _ = notify.notified() => {}
            }
        }
    });
}

impl SipCore {
    pub fn new(cfg: SipConfig, transport_tx: SipTransportTx) -> Self {
        let register = config::registrar_config()
            .cloned()
            .map(RegisterClient::new)
            .map(|client| Arc::new(Mutex::new(client)));
        let register_notify = register.as_ref().map(|_| Arc::new(Notify::new()));
        if let (Some(register), Some(notify)) = (register.as_ref(), register_notify.as_ref()) {
            spawn_register_task(
                Arc::clone(register),
                Arc::clone(notify),
                transport_tx.clone(),
                cfg.sip_port,
            );
        }
        let mut core = Self {
            cfg,
            transport_tx,
            invites: std::collections::HashMap::new(),
            non_invites: std::collections::HashMap::new(),
            register,
            register_notify,
            active_call_id: None,
        };
        core.maybe_send_register();
        core
    }

    /// SIP ソケットで受けた datagram を処理し、必要ならレスポンス送信と session へのイベントを返す。
    /// トランザクション状態機械（INVITEサーバトランザクション）を内部に持つ。
    pub fn handle_input(&mut self, input: &SipInput) -> Vec<SipEvent> {
        let mut events = self.prune_expired();

        let text = match decode_sip_text(&input.data) {
            Ok(t) => t,
            Err(err) => {
                log::warn!(
                    "[sip] dropped non-utf8 packet peer={:?} len={} err={:?}",
                    input.peer,
                    input.data.len(),
                    err
                );
                return vec![SipEvent::Unknown];
            }
        };

        let msg = match parse_sip_message(&text) {
            Ok(m) => m,
            Err(err) => {
                let first_line = text.lines().next().unwrap_or("<empty>");
                log::warn!(
                    "[sip] parse failed peer={:?} len={} first_line={} err={:?}",
                    input.peer,
                    input.data.len(),
                    first_line,
                    err
                );
                return vec![SipEvent::Unknown];
            }
        };

        if b2bua_bridge::dispatch_message(input.peer, &msg) {
            return vec![];
        }

        let mut ev = match msg {
            SipMessage::Request(req) => self.handle_request(req, input.peer),
            SipMessage::Response(resp) => self.handle_response(resp, input.peer),
        };

        events.append(&mut ev);
        events
    }

    /// Shuts down the registrar by sending an explicit SIP REGISTER with expires=0 if a register client is configured.
    ///
    /// If no register client is configured this is a no-op. If a register client exists but has no transport, a warning is logged instead of sending.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // assuming `core` is an initialized `SipCore`
    /// // core.shutdown();
    /// ```
    pub fn shutdown(&mut self) {
        let Some(register) = self.register.as_ref() else {
            return;
        };
        let (peer, payload) = {
            let mut reg = register.lock().unwrap();
            let peer = reg.transport_peer();
            let payload = peer.map(|_| reg.build_unregister_request().to_bytes());
            (peer, payload)
        };
        match (peer, payload) {
            (Some(peer), Some(payload)) => {
                log::info!("[sip register] sending unregister");
                self.send_payload(peer, payload);
            }
            (None, _) => {
                let transport = register.lock().unwrap().transport();
                log::warn!(
                    "[sip register] no transport for unregister: {:?}",
                    transport
                );
            }
            _ => {}
        }
    }

    fn maybe_send_register(&mut self) {
        let Some(register) = self.register.as_ref() else {
            return;
        };
        let (peer, payload) = {
            let reg = register.lock().unwrap();
            let peer = reg.transport_peer();
            let payload = peer.map(|_| reg.build_request().to_bytes());
            (peer, payload)
        };
        match (peer, payload) {
            (Some(peer), Some(payload)) => self.send_payload(peer, payload),
            (None, _) => {
                let transport = register.lock().unwrap().transport();
                log::warn!("[sip register] transport {:?} not supported", transport);
            }
            _ => {}
        }
    }

    fn handle_request(&mut self, req: SipRequest, peer: TransportPeer) -> Vec<SipEvent> {
        let headers = match CoreHeaderSnapshot::from_request(&req) {
            Ok(headers) => headers,
            Err(err) => {
                log::warn!("[sip] invalid Call-ID: {}", err);
                return vec![SipEvent::Unknown];
            }
        };
        match req.method {
            SipMethod::Invite => self.handle_invite(req, headers, peer),
            SipMethod::Ack => self.handle_ack(headers.call_id),
            SipMethod::Cancel => self.handle_cancel(req, headers, peer),
            SipMethod::Bye => self.handle_non_invite(req, headers, peer, 200, "OK", true),
            SipMethod::Options => self.handle_options(req, headers, peer),
            SipMethod::Register => self.handle_non_invite(req, headers, peer, 200, "OK", false),
            SipMethod::Update => self.handle_update(req, headers, peer),
            SipMethod::Prack => self.handle_prack(req, headers, peer),
            _ => {
                log::warn!(
                    "[sip] unsupported request method={:?} call_id={} peer={:?}",
                    req.method,
                    headers.call_id,
                    peer
                );
                if let Some(resp) = response_simple_from_request(&req, 501, "Not Implemented") {
                    self.send_payload(peer, resp.to_bytes());
                }
                vec![]
            }
        }
    }

    /// Process an incoming SIP response and delegate it to the registrar when configured.
    ///
    /// If a registrar is present and it handles the response, this method will optionally
    /// retransmit a pending REGISTER via the transport, notify any waiter, and return an
    /// empty event list. If the response is not handled by the registrar (or no registrar
    /// exists), the method returns a single `SipEvent::Unknown`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Assume `core` is a mutable SipCore, `resp` is a SipResponse and `peer` is TransportPeer.
    /// // When registrar handles the response the returned vector will be empty:
    /// let events = core.handle_response(resp, peer);
    /// // Otherwise:
    /// // assert_eq!(events, vec![SipEvent::Unknown]);
    /// ```
    fn handle_response(&mut self, resp: SipResponse, peer: TransportPeer) -> Vec<SipEvent> {
        if let Some(register) = self.register.as_ref() {
            let (handled, pending_req, pending_peer) = {
                let mut reg = register.lock().unwrap();
                let handled = reg.handle_response(&resp, peer);
                let pending_req = if handled {
                    reg.take_pending_request()
                } else {
                    None
                };
                let pending_peer = pending_req.as_ref().and_then(|_| reg.transport_peer());
                (handled, pending_req, pending_peer)
            };
            if handled {
                if let Some(req) = pending_req {
                    if let Some(peer) = pending_peer {
                        self.send_payload(peer, req.to_bytes());
                    } else {
                        log::warn!("[sip register] no transport for retry");
                    }
                }
                if let Some(notify) = self.register_notify.as_ref() {
                    notify.notify_one();
                }
                return vec![];
            }
        }
        let call_id = resp
            .headers
            .iter()
            .find(|header| header.name.eq_ignore_ascii_case("Call-ID"))
            .map(|header| header.value.as_str())
            .unwrap_or("-");
        let cseq = resp
            .headers
            .iter()
            .find(|header| header.name.eq_ignore_ascii_case("CSeq"))
            .map(|header| header.value.as_str())
            .unwrap_or("-");
        log::warn!(
            "[sip] unhandled response status={} call_id={} cseq={} peer={:?}",
            resp.status_code,
            call_id,
            cseq,
            peer
        );
        vec![SipEvent::Unknown]
    }

    /// Process an incoming INVITE request, handling retransmits, re-INVITEs, busy call rejection,
    /// and session-timer validation, and start a new invite server transaction when appropriate.
    ///
    /// If the request is a retransmit the function will retransmit or resend the final 2xx as needed
    /// and return an empty vector. If it is a re-INVITE for an existing dialog it delegates to the
    /// re-INVITE handler. If the core is busy with another active call the INVITE is rejected with
    /// 486 Busy Here. If the Session-Expires header is invalid or too small the function responds with
    /// 400 or 422 (including Min-SE) respectively. On successful acceptance a new InviteContext is
    /// created and a single `SipEvent::IncomingInvite` is returned containing the parsed SDP offer and
    /// optional session-timer info.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// // Illustrative example (omits construction of realistic SipCore/SipRequest values).
    /// // let mut core = SipCore::new(cfg, transport_tx);
    /// // let events = core.handle_invite(req, headers, peer);
    /// // assert!(matches!(events.as_slice(), [SipEvent::IncomingInvite{ .. }]) || events.is_empty());
    /// ```
    fn handle_invite(
        &mut self,
        req: SipRequest,
        headers: CoreHeaderSnapshot,
        peer: TransportPeer,
    ) -> Vec<SipEvent> {
        // 再送判定 or re-INVITE 判定
        if let Some(ctx) = self.invites.get_mut(&headers.call_id) {
            let req_cseq = req
                .header_value("CSeq")
                .and_then(|value| parse_cseq_header(value).ok())
                .map(|cseq| cseq.num);
            let prev_cseq = ctx
                .req
                .header_value("CSeq")
                .and_then(|value| parse_cseq_header(value).ok())
                .map(|cseq| cseq.num);
            if req_cseq.is_some() && prev_cseq.is_some() && req_cseq == prev_cseq {
                let (action, final_ok, tx_peer) = (
                    ctx.tx.on_retransmit(),
                    ctx.final_ok_payload.clone(),
                    ctx.tx.peer,
                );
                if let Some(action) = action {
                    self.send_tx_action(action, peer);
                } else if let Some(payload) = final_ok {
                    self.send_payload(tx_peer, payload);
                }
                return vec![];
            }
            return self.handle_reinvite(req, headers, peer);
        }

        // In-dialog INVITE without dialog context must be rejected.
        if invite_has_to_tag(&req) {
            log::info!(
                "[sip invite] in-dialog INVITE without dialog context call_id={}, rejecting with 481",
                headers.call_id
            );
            if let Some(resp) =
                response_simple_from_request(&req, 481, "Call/Transaction Does Not Exist")
            {
                self.send_payload(peer, resp.to_bytes());
            }
            return vec![];
        }

        if let Some(active) = &self.active_call_id {
            if active != &headers.call_id {
                log::info!(
                    "[sip] busy (active call_id={}), rejecting {}",
                    active,
                    headers.call_id
                );
                if let Some(resp) = response_simple_from_request(&req, 486, "Busy Here") {
                    self.send_payload(peer, resp.to_bytes());
                }
                return vec![];
            }
        }

        let session_timer = match session_timer_from_request(&req, config::session_config(), true) {
            Ok(timer) => timer,
            Err(SessionTimerError::Invalid) => {
                if let Some(resp) = response_simple_from_request(&req, 400, "Bad Request") {
                    self.send_payload(peer, resp.to_bytes());
                }
                return vec![];
            }
            Err(SessionTimerError::TooSmall { min_se }) => {
                if let Some(mut resp) =
                    response_simple_from_request(&req, 422, "Session Interval Too Small")
                {
                    resp.headers
                        .push(SipHeader::new("Min-SE", min_se.to_string()));
                    self.send_payload(peer, resp.to_bytes());
                }
                return vec![];
            }
        };

        // 新規 INVITE: トランザクション生成（レスポンスは SipCommand 経由で送るためここでは送信しない）
        let mut tx = InviteServerTransaction::new(peer);
        tx.invite_req = Some(req.clone());
        let ctx = InviteContext {
            tx,
            req: req.clone(),
            reliable: None,
            expected_rack: None,
            last_rseq: None,
            session_timer: session_timer.clone(),
            final_ok: None,
            final_ok_payload: None,
            local_cseq: 0,
            expires_at: None,
        };
        self.active_call_id = Some(headers.call_id.clone());
        self.invites.insert(headers.call_id.clone(), ctx);

        let offer = parse_offer_sdp(&req.body).unwrap_or_else(|| Sdp::pcmu("0.0.0.0", 0));
        if let Ok(sdp) = std::str::from_utf8(&req.body) {
            let sdp_inline = sdp.replace('\r', "").replace('\n', "\\n");
            log::info!(
                "[sip invite sdp] call_id={} sdp={}",
                headers.call_id,
                sdp_inline
            );
        } else {
            log::info!(
                "[sip invite sdp] call_id={} sdp_len={}",
                headers.call_id,
                req.body.len()
            );
        }
        vec![SipEvent::IncomingInvite {
            call_id: headers.call_id,
            from: headers.from,
            to: headers.to,
            offer,
            session_timer: session_timer.map(|cfg| SessionTimerInfo {
                expires: cfg.expires,
                refresher: cfg.refresher,
            }),
        }]
    }

    /// Handle an in-dialog re-INVITE request and produce a ReInvite event.
    ///
    /// Validates Session-Expires and Min-SE headers; on validation failure this method
    /// sends the appropriate error response (400 or 422) and returns an empty vector.
    /// When accepted, it updates the stored invite context (resets reliable/retransmit
    /// state and applies any session-timer configuration) and returns a single
    /// `SipEvent::ReInvite` containing the call ID, parsed SDP offer (falls back to
    /// a silent SDP when parsing fails), and optional session timer info.
    ///
    /// # Returns
    ///
    /// A `Vec<SipEvent>` containing one `SipEvent::ReInvite` when the re-INVITE is
    /// accepted, or an empty vector if the request was rejected or handled locally.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Assume `core` is a mutable SipCore and `req`, `headers`, `peer` are prepared.
    /// // The call below will validate session timers, update internal invite state,
    /// // and return a ReInvite event when accepted.
    /// let events = core.handle_reinvite(req, headers, peer);
    /// match events.first() {
    ///     Some(SipEvent::ReInvite { call_id, offer, session_timer }) => {
    ///         // handle re-INVITE
    ///     }
    ///     _ => { /* rejected or no event */ }
    /// }
    /// ```
    fn handle_reinvite(
        &mut self,
        req: SipRequest,
        headers: CoreHeaderSnapshot,
        peer: TransportPeer,
    ) -> Vec<SipEvent> {
        let session_timer = match session_timer_from_request(&req, config::session_config(), false)
        {
            Ok(timer) => timer,
            Err(SessionTimerError::Invalid) => {
                if let Some(resp) = response_simple_from_request(&req, 400, "Bad Request") {
                    self.send_payload(peer, resp.to_bytes());
                }
                return vec![];
            }
            Err(SessionTimerError::TooSmall { min_se }) => {
                if let Some(mut resp) =
                    response_simple_from_request(&req, 422, "Session Interval Too Small")
                {
                    resp.headers
                        .push(SipHeader::new("Min-SE", min_se.to_string()));
                    self.send_payload(peer, resp.to_bytes());
                }
                return vec![];
            }
        };

        if let Some(ctx) = self.invites.get_mut(&headers.call_id) {
            ctx.req = req.clone();
            ctx.tx = InviteServerTransaction::new(peer);
            ctx.tx.invite_req = Some(req.clone());
            ctx.reliable = None;
            ctx.expected_rack = None;
            ctx.last_rseq = None;
            ctx.final_ok = None;
            ctx.final_ok_payload = None;
            if let Some(cfg) = &session_timer {
                ctx.session_timer = Some(cfg.clone());
            }
        }

        let offer = parse_offer_sdp(&req.body).unwrap_or_else(|| Sdp::pcmu("0.0.0.0", 0));
        vec![SipEvent::ReInvite {
            call_id: headers.call_id,
            offer,
            session_timer: session_timer.map(|cfg| SessionTimerInfo {
                expires: cfg.expires,
                refresher: cfg.refresher,
            }),
        }]
    }

    fn handle_ack(&mut self, call_id: CallId) -> Vec<SipEvent> {
        let mut stop_final_ok = false;
        let (action, terminate, peer_opt) = if let Some(ctx) = self.invites.get_mut(&call_id) {
            let peer = ctx.tx.peer;
            let action = ctx.tx.on_ack();
            let terminate = ctx.tx.state == InviteTxState::Terminated;
            stop_final_ok = ctx.final_ok.is_some();
            (action, terminate, Some(peer))
        } else {
            (None, false, None)
        };

        if stop_final_ok {
            self.stop_final_ok_retransmit(&call_id);
        }
        if let Some(action) = action {
            if let Some(peer) = peer_opt {
                self.send_tx_action(action, peer);
            }
        }
        if terminate && !stop_final_ok {
            self.invites.remove(&call_id);
        }
        vec![SipEvent::Ack { call_id }]
    }

    /// Handle an incoming CANCEL request for an existing INVITE transaction.
    ///
    /// Sends a 200 OK and returns a `SipEvent::Cancel` when the CANCEL is accepted
    /// for an INVITE that is currently in the Proceeding state. If no matching
    /// invite exists or the invite is not in Proceeding, sends a 481 "Call/Transaction
    /// Does Not Exist" and returns an empty vector. If the CANCEL request is
    /// missing required headers, no response is sent and an empty vector is returned.
    ///
    /// # Returns
    ///
    /// `Vec<SipEvent>` containing `SipEvent::Cancel { call_id }` if the CANCEL was
    /// accepted, or an empty vector otherwise.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Assuming `core` is a mutable SipCore, `req` is a parsed CANCEL request,
    /// // `headers` is a CoreHeaderSnapshot extracted from the request, and
    /// // `peer` is the transport peer the request arrived from:
    /// let events = core.handle_cancel(req, headers, peer);
    /// if let Some(event) = events.into_iter().next() {
    ///     // handle the Cancel event
    /// }
    /// ```
    fn handle_cancel(
        &mut self,
        req: SipRequest,
        headers: CoreHeaderSnapshot,
        peer: TransportPeer,
    ) -> Vec<SipEvent> {
        let call_id = headers.call_id.clone();
        let mut notify_session = false;
        let (code, reason) = match self.invites.get(&call_id) {
            Some(ctx) => {
                if ctx.tx.state == InviteTxState::Proceeding {
                    notify_session = true;
                    (200, "OK")
                } else {
                    (481, "Call/Transaction Does Not Exist")
                }
            }
            None => (481, "Call/Transaction Does Not Exist"),
        };

        if let Some(resp) = response_simple_from_request(&req, code, reason) {
            self.send_payload(peer, resp.to_bytes());
        } else {
            log::warn!(
                "[sip cancel] invalid CANCEL (missing headers) call_id={}",
                call_id
            );
            return vec![];
        }

        if notify_session {
            log::info!("[sip cancel] accepted call_id={}", call_id);
            vec![SipEvent::Cancel { call_id }]
        } else {
            vec![]
        }
    }

    fn handle_non_invite(
        &mut self,
        req: SipRequest,
        headers: CoreHeaderSnapshot,
        peer: TransportPeer,
        _status: u16,
        _reason: &str,
        emit_bye_event: bool,
    ) -> Vec<SipEvent> {
        let tx = self
            .non_invites
            .entry(headers.call_id.clone())
            .or_insert_with(|| NonInviteServerTransaction::new(peer, req.clone()));

        if let Some(resp) = tx.on_retransmit() {
            self.send_payload(peer, resp);
            return if emit_bye_event {
                vec![SipEvent::Bye {
                    call_id: headers.call_id,
                }]
            } else {
                vec![]
            };
        }

        // 最終応答は SipCommand::SendBye200 等から送るため、ここでは送信しない
        tx.last_request = Some(req);

        if emit_bye_event {
            vec![SipEvent::Bye {
                call_id: headers.call_id,
            }]
        } else {
            vec![]
        }
    }

    fn handle_options(
        &mut self,
        req: SipRequest,
        headers: CoreHeaderSnapshot,
        peer: TransportPeer,
    ) -> Vec<SipEvent> {
        let tx = self
            .non_invites
            .entry(headers.call_id.clone())
            .or_insert_with(|| NonInviteServerTransaction::new(peer, req.clone()));

        if let Some(resp) = tx.on_retransmit() {
            self.send_payload(peer, resp);
            return vec![];
        }

        if let Some(resp) = response_options_from_request(&req) {
            let bytes = resp.to_bytes();
            tx.on_final_sent(bytes.clone());
            tx.last_request = Some(req);
            self.send_payload(peer, bytes);
        }

        vec![]
    }

    /// Handle an incoming PRACK request for a pending reliable provisional response.
    ///
    /// Validates the `RAck` header and ensures it matches the expected RAck stored for the
    /// invite identified by `headers.call_id`. On malformed or unexpected RAck values this
    /// sends the appropriate SIP error response (400 or 481) and takes no further action.
    /// When the RAck matches, the function updates/creates the non-invite transaction state,
    /// replies with `200 OK`, stops retransmission of the reliable provisional, and does not
    /// emit any session-level events.
    ///
    /// # Returns
    ///
    /// A `Vec<SipEvent>`; currently always empty (no session events are emitted by PRACK handling).
    fn handle_prack(
        &mut self,
        req: SipRequest,
        headers: CoreHeaderSnapshot,
        peer: TransportPeer,
    ) -> Vec<SipEvent> {
        let rack = match req.header_value("RAck").and_then(parse_rack) {
            Some(rack) => rack,
            None => {
                log::warn!(
                    "[sip 100rel] invalid/missing RAck call_id={}",
                    headers.call_id
                );
                if let Some(resp) = response_simple_from_request(&req, 400, "Bad Request") {
                    self.send_payload(peer, resp.to_bytes());
                }
                return vec![];
            }
        };

        let expected = self
            .invites
            .get(&headers.call_id)
            .and_then(|ctx| ctx.expected_rack.clone());
        let Some(expected) = expected else {
            log::warn!(
                "[sip 100rel] unexpected PRACK (no pending reliable provisional) call_id={}",
                headers.call_id
            );
            if let Some(resp) =
                response_simple_from_request(&req, 481, "Call/Transaction Does Not Exist")
            {
                self.send_payload(peer, resp.to_bytes());
            }
            return vec![];
        };

        if !rack_matches(&expected, &rack) {
            log::warn!(
                "[sip 100rel] RAck mismatch call_id={} expected={:?} got={:?}",
                headers.call_id,
                expected,
                rack
            );
            if let Some(resp) =
                response_simple_from_request(&req, 481, "Call/Transaction Does Not Exist")
            {
                self.send_payload(peer, resp.to_bytes());
            }
            return vec![];
        }

        let tx = self
            .non_invites
            .entry(headers.call_id.clone())
            .or_insert_with(|| NonInviteServerTransaction::new(peer, req.clone()));

        if let Some(resp) = tx.on_retransmit() {
            self.send_payload(peer, resp);
            return vec![];
        }

        if let Some(resp) = response_simple_from_request(&req, 200, "OK") {
            let bytes = resp.to_bytes();
            tx.on_final_sent(bytes.clone());
            tx.last_request = Some(req);
            self.send_payload(peer, bytes);
        }

        self.stop_reliable_provisional(&headers.call_id);
        vec![]
    }

    /// Handle an incoming UPDATE request: validate session-timer headers, respond, update transaction state,
    /// and emit a SessionRefresh event when a valid session-timer is present.
    ///
    /// This updates or creates a non-invite server transaction for the request, handles retransmits,
    /// sends a 200 OK (including Session-Expires/Min-SE headers when appropriate), records the last
    /// request and final response for retransmission, and updates the associated invite's session timer
    /// if one is negotiated.
    ///
    /// # Parameters
    ///
    /// - `req`: the incoming UPDATE request message.
    /// - `headers`: core header snapshot extracted from the request (call-id, via, from, to, cseq).
    /// - `peer`: transport peer information used to send responses.
    ///
    /// # Returns
    ///
    /// A vector containing `SipEvent::SessionRefresh` with the negotiated timer when a Session-Expires
    /// value was accepted; otherwise an empty vector.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// // `core` is a mutable SipCore instance, `req` is a parsed UPDATE request,
    /// // `headers` is a CoreHeaderSnapshot for that request, and `peer` is the transport peer.
    /// let events = core.handle_update(req, headers, peer);
    /// if let Some(evt) = events.into_iter().next() {
    ///     // handle SessionRefresh
    /// }
    /// ```
    fn handle_update(
        &mut self,
        req: SipRequest,
        headers: CoreHeaderSnapshot,
        peer: TransportPeer,
    ) -> Vec<SipEvent> {
        let session_timer = match session_timer_from_request(&req, config::session_config(), false)
        {
            Ok(timer) => timer,
            Err(SessionTimerError::Invalid) => {
                if let Some(resp) = response_simple_from_request(&req, 400, "Bad Request") {
                    self.send_payload(peer, resp.to_bytes());
                }
                return vec![];
            }
            Err(SessionTimerError::TooSmall { min_se }) => {
                if let Some(mut resp) =
                    response_simple_from_request(&req, 422, "Session Interval Too Small")
                {
                    resp.headers
                        .push(SipHeader::new("Min-SE", min_se.to_string()));
                    self.send_payload(peer, resp.to_bytes());
                }
                return vec![];
            }
        };

        let tx = self
            .non_invites
            .entry(headers.call_id.clone())
            .or_insert_with(|| NonInviteServerTransaction::new(peer, req.clone()));

        if let Some(resp) = tx.on_retransmit() {
            self.send_payload(peer, resp);
            return vec![];
        }

        if let Some(mut resp) = response_simple_from_request(&req, 200, "OK") {
            if let Some(cfg) = &session_timer {
                apply_session_timer_headers(&mut resp, cfg);
            }
            let bytes = resp.to_bytes();
            tx.on_final_sent(bytes.clone());
            tx.last_request = Some(req);
            self.send_payload(peer, bytes);
        }

        if let Some(cfg) = session_timer {
            if let Some(ctx) = self.invites.get_mut(&headers.call_id) {
                ctx.session_timer = Some(cfg.clone());
            }
            vec![SipEvent::SessionRefresh {
                call_id: headers.call_id,
                timer: SessionTimerInfo {
                    expires: cfg.expires,
                    refresher: cfg.refresher,
                },
            }]
        } else {
            vec![]
        }
    }

    fn start_reliable_provisional(
        &mut self,
        call_id: &CallId,
        peer: TransportPeer,
        payload: Vec<u8>,
        rseq: u32,
    ) {
        let Some(ctx) = self.invites.get_mut(call_id) else {
            return;
        };
        if let Some(prev) = ctx.reliable.take() {
            prev.stop.store(true, Ordering::SeqCst);
        }

        let stop = Arc::new(AtomicBool::new(false));
        let expected_rack = ctx
            .req
            .header_value("CSeq")
            .and_then(|value| parse_cseq_header(value).ok())
            .map(|cseq| RAckHeader {
                rseq,
                cseq: cseq.num,
                method: cseq.method,
            });
        if expected_rack.is_none() {
            log::warn!(
                "[sip 100rel] failed to build expected RAck call_id={}",
                call_id
            );
        }
        ctx.expected_rack = expected_rack;
        ctx.reliable = Some(ReliableProvisional { stop: stop.clone() });

        let transport_tx = self.transport_tx.clone();
        let src_port = self.cfg.sip_port;
        let timeout_resp = response_simple_from_request(&ctx.req, 504, "Server Time-out")
            .map(|resp| resp.to_bytes());
        let call_id = call_id.clone();
        tokio::spawn(async move {
            let mut interval = Duration::from_millis(500);
            let max_duration = Duration::from_secs(32);
            let start = Instant::now();
            loop {
                sleep(interval).await;
                if stop.load(Ordering::SeqCst) {
                    break;
                }
                if start.elapsed() >= max_duration {
                    log::warn!("[sip 100rel] PRACK timeout call_id={}", call_id);
                    if let Some(resp) = timeout_resp {
                        if let Err(err) = transport_tx.try_send(SipTransportRequest {
                            peer,
                            src_port,
                            payload: resp,
                        }) {
                            log::error!(
                                "[sip 100rel] failed to enqueue timeout response call_id={} peer={:?} err={:?}",
                                call_id,
                                peer,
                                err
                            );
                        }
                    }
                    break;
                }
                if let Err(err) = transport_tx.try_send(SipTransportRequest {
                    peer,
                    src_port,
                    payload: payload.clone(),
                }) {
                    log::error!(
                        "[sip 100rel] failed to enqueue provisional retransmit call_id={} peer={:?} err={:?}",
                        call_id,
                        peer,
                        err
                    );
                }
                interval = std::cmp::min(interval * 2, Duration::from_secs(4));
            }
            stop.store(true, Ordering::SeqCst);
        });
    }

    fn stop_reliable_provisional(&mut self, call_id: &CallId) {
        if let Some(ctx) = self.invites.get_mut(call_id) {
            if let Some(rel) = ctx.reliable.take() {
                rel.stop.store(true, Ordering::SeqCst);
            }
        }
    }

    fn start_final_ok_retransmit(
        &mut self,
        call_id: &CallId,
        peer: TransportPeer,
        payload: Vec<u8>,
    ) {
        let Some(ctx) = self.invites.get_mut(call_id) else {
            return;
        };
        if let Some(prev) = ctx.final_ok.take() {
            prev.stop.store(true, Ordering::SeqCst);
        }

        let stop = Arc::new(AtomicBool::new(false));
        ctx.final_ok = Some(FinalOkRetransmit { stop: stop.clone() });
        ctx.final_ok_payload = Some(payload.clone());

        let transport_tx = self.transport_tx.clone();
        let src_port = self.cfg.sip_port;
        let call_id = call_id.clone();
        tokio::spawn(async move {
            let mut interval = Duration::from_millis(500);
            let max_duration = Duration::from_secs(32);
            let start = Instant::now();
            loop {
                sleep(interval).await;
                if stop.load(Ordering::SeqCst) {
                    break;
                }
                if start.elapsed() >= max_duration {
                    log::warn!("[sip] 2xx retransmit timeout (no ACK) call_id={}", call_id);
                    break;
                }
                if let Err(err) = transport_tx.try_send(SipTransportRequest {
                    peer,
                    src_port,
                    payload: payload.clone(),
                }) {
                    log::error!(
                        "[sip] failed to enqueue 2xx retransmit call_id={} peer={:?} err={:?}",
                        call_id,
                        peer,
                        err
                    );
                }
                interval = std::cmp::min(interval * 2, Duration::from_secs(4));
            }
            stop.store(true, Ordering::SeqCst);
        });
    }

    fn stop_final_ok_retransmit(&mut self, call_id: &CallId) {
        if let Some(ctx) = self.invites.get_mut(call_id) {
            if let Some(final_ok) = ctx.final_ok.take() {
                final_ok.stop.store(true, Ordering::SeqCst);
            }
            ctx.final_ok_payload = None;
        }
    }

    fn send_tx_action(&self, action: InviteTxAction, peer: TransportPeer) {
        match action {
            InviteTxAction::Retransmit(resp) => self.send_payload(peer, resp),
            InviteTxAction::Timeout => { /* Timer H/I 経由の通知などは未使用 */ }
        }
    }

    /// Process a SipCommand by sending the corresponding SIP message(s)
    /// and updating transaction state.
    ///
    /// This applies the requested outgoing session action (provisional responses,
    /// final answers, UPDATE, BYE, error responses, and related retransmit control),
    /// including starting/stopping reliable-provisional or final-OK retransmit loops,
    /// applying session-timer headers when applicable, clearing or removing invite
    /// state, and emitting warnings when message construction fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crate::shared::entities::CallId;
    /// # use crate::protocol::sip::{SipCore, SipCommand};
    /// # fn example(core: &mut SipCore, call_id: &CallId, cmd: SipCommand) {
    /// core.handle_sip_command(call_id, cmd);
    /// # }
    /// ```
    pub fn handle_sip_command(&mut self, call_id: &CallId, cmd: SipCommand) {
        match cmd {
            SipCommand::Send100 => {
                if let Some(ctx) = self.invites.get_mut(call_id) {
                    if let Some(resp) = response_provisional_from_request(&ctx.req, 100, "Trying") {
                        let bytes = resp.to_bytes();
                        ctx.tx.remember_provisional(bytes.clone());
                        let peer = ctx.tx.peer;
                        self.send_payload(peer, bytes);
                    }
                }
            }
            SipCommand::Send180 => {
                let provision = if let Some(ctx) = self.invites.get_mut(call_id) {
                    if let Some(mut resp) =
                        response_provisional_from_request(&ctx.req, 180, "Ringing")
                    {
                        let reliable = supports_100rel(&ctx.req);
                        let mut rseq = None;
                        let mut reliable_send = reliable;
                        if reliable {
                            rseq = next_rseq(ctx);
                            if let Some(value) = rseq {
                                resp.headers.push(SipHeader::new("Require", "100rel"));
                                resp.headers.push(SipHeader::new("RSeq", value.to_string()));
                            } else {
                                log::warn!(
                                    "[sip 100rel] RSeq max reached; sending non-100rel provisional call_id={}",
                                    call_id
                                );
                                reliable_send = false;
                            }
                        }
                        let bytes = resp.to_bytes();
                        let peer = ctx.tx.peer;
                        ctx.tx.remember_provisional(bytes.clone());
                        Some((peer, bytes, reliable_send, rseq))
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some((peer, bytes, reliable, rseq)) = provision {
                    if reliable {
                        if let Some(rseq) = rseq {
                            self.start_reliable_provisional(call_id, peer, bytes.clone(), rseq);
                        }
                    }
                    self.send_payload(peer, bytes);
                }
            }
            SipCommand::Send183 { answer } => {
                let provision = if let Some(ctx) = self.invites.get_mut(call_id) {
                    if let Some(mut resp) = response_final_with_sdp(
                        &ctx.req,
                        183,
                        "Session Progress",
                        &self.cfg.advertised_ip,
                        self.cfg.sip_port,
                        &answer,
                    ) {
                        let reliable = supports_100rel(&ctx.req);
                        let mut rseq = None;
                        let mut reliable_send = reliable;
                        if reliable {
                            rseq = next_rseq(ctx);
                            if let Some(value) = rseq {
                                resp.headers.push(SipHeader::new("Require", "100rel"));
                                resp.headers.push(SipHeader::new("RSeq", value.to_string()));
                            } else {
                                log::warn!(
                                    "[sip 100rel] RSeq max reached; sending non-100rel provisional call_id={}",
                                    call_id
                                );
                                reliable_send = false;
                            }
                        }
                        if let Ok(sdp) = std::str::from_utf8(&resp.body) {
                            let sdp_inline = sdp.replace('\r', "").replace('\n', "\\n");
                            log::info!("[sip 183 sdp] call_id={} sdp={}", call_id, sdp_inline);
                        } else {
                            log::info!(
                                "[sip 183 sdp] call_id={} sdp_len={}",
                                call_id,
                                resp.body.len()
                            );
                        }
                        let bytes = resp.to_bytes();
                        let peer = ctx.tx.peer;
                        ctx.tx.remember_provisional(bytes.clone());
                        Some((peer, bytes, reliable_send, rseq))
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some((peer, bytes, reliable, rseq)) = provision {
                    if reliable {
                        if let Some(rseq) = rseq {
                            self.start_reliable_provisional(call_id, peer, bytes.clone(), rseq);
                        }
                    }
                    self.send_payload(peer, bytes);
                }
            }
            SipCommand::Send200 { answer } => {
                if let Some(ctx) = self.invites.get_mut(call_id) {
                    if let Some(mut resp) = response_final_with_sdp(
                        &ctx.req,
                        200,
                        "OK",
                        &self.cfg.advertised_ip,
                        self.cfg.sip_port,
                        &answer,
                    ) {
                        if let Some(cfg) = &ctx.session_timer {
                            apply_session_timer_headers(&mut resp, cfg);
                        }
                        if let Ok(sdp) = std::str::from_utf8(&resp.body) {
                            let sdp_inline = sdp.replace('\r', "").replace('\n', "\\n");
                            log::info!("[sip 200 sdp] call_id={} sdp={}", call_id, sdp_inline);
                        } else {
                            log::info!(
                                "[sip 200 sdp] call_id={} sdp_len={}",
                                call_id,
                                resp.body.len()
                            );
                        }
                        let bytes = resp.to_bytes();
                        ctx.tx.on_final_sent(bytes.clone(), 200);
                        let peer = ctx.tx.peer;
                        self.start_final_ok_retransmit(call_id, peer, bytes.clone());
                        self.send_payload(peer, bytes);
                        self.stop_reliable_provisional(call_id);
                    }
                } else {
                    log::warn!(
                        "[sip 200] missing invite context, cannot respond call_id={}",
                        call_id
                    );
                }
            }
            SipCommand::SendUpdate { expires } => {
                let (peer, payload) = if let Some(ctx) = self.invites.get_mut(call_id) {
                    let peer = ctx.tx.peer;
                    let payload = build_update_request(ctx, &self.cfg, expires);
                    (Some(peer), payload)
                } else {
                    (None, None)
                };
                if let (Some(peer), Some(payload)) = (peer, payload) {
                    self.send_payload(peer, payload);
                } else {
                    log::warn!("[sip update] failed to build UPDATE call_id={}", call_id);
                }
            }
            SipCommand::SendError { code, reason } => {
                if self.active_call_id.as_ref() == Some(call_id) {
                    self.active_call_id = None;
                }
                let mut stop_reliable = false;
                let mut send_payload = None;
                if let Some(ctx) = self.invites.get_mut(call_id) {
                    if let Some(resp) = response_simple_from_request(&ctx.req, code, &reason) {
                        let bytes = resp.to_bytes();
                        ctx.tx.on_final_sent(bytes.clone(), code);
                        send_payload = Some((ctx.tx.peer, bytes));
                        stop_reliable = true;
                    }
                    ctx.expires_at = Some(Instant::now() + Duration::from_secs(32));
                }
                if let Some((peer, payload)) = send_payload {
                    self.send_payload(peer, payload);
                }
                if stop_reliable {
                    self.stop_reliable_provisional(call_id);
                }
            }
            SipCommand::SendBye => {
                if self.active_call_id.as_ref() == Some(call_id) {
                    self.active_call_id = None;
                }
                let (peer, payload) = if let Some(ctx) = self.invites.get_mut(call_id) {
                    let peer = ctx.tx.peer;
                    let payload = build_bye_request(ctx, &self.cfg);
                    (Some(peer), payload)
                } else {
                    (None, None)
                };
                if let (Some(peer), Some(payload)) = (peer, payload) {
                    self.send_payload(peer, payload);
                } else {
                    log::warn!("[sip bye] failed to build BYE call_id={}", call_id);
                }
                self.invites.remove(call_id);
            }
            SipCommand::SendBye200 => {
                if self.active_call_id.as_ref() == Some(call_id) {
                    self.active_call_id = None;
                }
                if let Some(tx) = self.non_invites.get_mut(call_id) {
                    if let Some(req) = tx.last_request.clone() {
                        if let Some(resp) = response_simple_from_request(&req, 200, "OK") {
                            let bytes = resp.to_bytes();
                            tx.on_final_sent(bytes.clone());
                            let peer = tx.peer;
                            self.send_payload(peer, bytes.clone());
                        }
                    }
                }
                self.invites.remove(call_id);
            }
        }
    }

    fn extract_call_id_from_payload(payload: &[u8]) -> Option<&str> {
        for raw_line in payload.split(|b| *b == b'\n') {
            let Ok(line) = std::str::from_utf8(raw_line) else {
                continue;
            };
            let line = line.trim_end_matches('\r');
            if line.is_empty() {
                break;
            }
            let Some((name, value)) = line.split_once(':') else {
                continue;
            };
            if name.eq_ignore_ascii_case("Call-ID") {
                return Some(value.trim());
            }
        }
        None
    }

    fn send_payload(&self, peer: TransportPeer, payload: Vec<u8>) {
        let call_id = Self::extract_call_id_from_payload(&payload)
            .unwrap_or("-")
            .to_string();
        if let Some(first_line) = payload
            .split(|b| *b == b'\n')
            .next()
            .and_then(|line| std::str::from_utf8(line).ok())
        {
            log::info!(
                "[sip ->] {}:{} -> {:?} call_id={} {}",
                self.cfg.advertised_ip,
                self.cfg.sip_port,
                peer,
                call_id.as_str(),
                first_line.trim()
            );
        } else {
            log::info!(
                "[sip ->] {}:{} -> {:?} call_id={} len={}",
                self.cfg.advertised_ip,
                self.cfg.sip_port,
                peer,
                call_id.as_str(),
                payload.len()
            );
        }

        if let Err(err) = self.transport_tx.try_send(SipTransportRequest {
            peer,
            src_port: self.cfg.sip_port,
            payload,
        }) {
            log::error!(
                "[sip ->] failed to enqueue transport payload call_id={} peer={:?} err={:?}",
                call_id.as_str(),
                peer,
                err
            );
        }
    }

    fn prune_expired(&mut self) -> Vec<SipEvent> {
        let now = Instant::now();
        let mut events = Vec::new();
        self.non_invites.retain(|call_id, tx| {
            let alive = tx.state != NonInviteTxState::Terminated && tx.expires_at > now;
            if !alive {
                events.push(SipEvent::TransactionTimeout {
                    call_id: call_id.clone(),
                });
            }
            alive
        });
        self.invites.retain(|call_id, ctx| {
            let Some(expires_at) = ctx.expires_at else {
                return true;
            };
            let alive = expires_at > now;
            if !alive {
                events.push(SipEvent::TransactionTimeout {
                    call_id: call_id.clone(),
                });
            }
            alive
        });
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::sip::codec::SipResponseBuilder;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use tokio::sync::mpsc;

    fn dummy_peer() -> TransportPeer {
        TransportPeer::Udp("127.0.0.1:5060".parse().unwrap())
    }

    fn dummy_invite_context() -> InviteContext {
        let req = SipRequestBuilder::new(SipMethod::Invite, "sip:test@example.com").build();
        InviteContext {
            tx: InviteServerTransaction::new(dummy_peer()),
            req,
            reliable: None,
            expected_rack: None,
            last_rseq: None,
            session_timer: None,
            final_ok: None,
            final_ok_payload: None,
            local_cseq: 0,
            expires_at: None,
        }
    }

    #[test]
    fn rseq_random_and_increment() {
        let mut ctx = dummy_invite_context();
        let mut rng = StdRng::seed_from_u64(7);
        let first = next_rseq_with_rng(&mut ctx, &mut rng).expect("rseq");
        assert!((RSEQ_MIN..=RSEQ_MAX).contains(&first));
        let second = next_rseq_with_rng(&mut ctx, &mut rng).expect("rseq");
        assert_eq!(second, first + 1);
    }

    #[test]
    fn rseq_overflow_returns_none() {
        let mut ctx = dummy_invite_context();
        ctx.last_rseq = Some(RSEQ_MAX);
        let mut rng = StdRng::seed_from_u64(1);
        assert!(next_rseq_with_rng(&mut ctx, &mut rng).is_none());
        assert_eq!(ctx.last_rseq, Some(RSEQ_MAX));
    }

    #[test]
    fn retransmit_preserves_rseq_payload() {
        let mut tx = InviteServerTransaction::new(dummy_peer());
        let resp = SipResponseBuilder::new(180, "Ringing")
            .header("Via", "SIP/2.0/UDP example.com")
            .header("From", "<sip:alice@example.com>")
            .header("To", "<sip:bob@example.com>")
            .header("Call-ID", "call-1")
            .header("CSeq", "1 INVITE")
            .header("Require", "100rel")
            .header("RSeq", "777")
            .build();
        let bytes = resp.to_bytes();
        tx.remember_provisional(bytes.clone());
        match tx.on_retransmit() {
            Some(InviteTxAction::Retransmit(retransmit)) => assert_eq!(retransmit, bytes),
            _ => panic!("expected retransmit"),
        }
    }

    #[test]
    fn extract_call_id_from_payload_finds_header_value() {
        let payload = b"INVITE sip:bob@example.com SIP/2.0\r\nVia: SIP/2.0/UDP 127.0.0.1:5060\r\nCall-ID: call-123\r\nCSeq: 1 INVITE\r\n\r\n";
        assert_eq!(
            SipCore::extract_call_id_from_payload(payload),
            Some("call-123")
        );
    }

    #[test]
    fn extract_call_id_from_payload_returns_none_without_header() {
        let payload =
            b"SIP/2.0 200 OK\r\nVia: SIP/2.0/UDP 127.0.0.1:5060\r\nCSeq: 1 INVITE\r\n\r\n";
        assert_eq!(SipCore::extract_call_id_from_payload(payload), None);
    }

    #[test]
    fn options_response_includes_allow_and_supported() {
        let (tx, mut rx) = mpsc::channel(16);
        let mut core = SipCore::new(
            SipConfig {
                advertised_ip: "127.0.0.1".to_string(),
                sip_port: 5060,
                advertised_rtp_port: 4000,
            },
            tx,
        );

        let req = SipRequestBuilder::new(SipMethod::Options, "sip:test@example.com")
            .header("Via", "SIP/2.0/UDP 127.0.0.1:5060")
            .header("From", "<sip:alice@example.com>;tag=alice")
            .header("To", "<sip:bob@example.com>")
            .header("Call-ID", "call-1")
            .header("CSeq", "1 OPTIONS")
            .build();
        let input = SipInput {
            peer: dummy_peer(),
            data: req.to_bytes(),
        };

        let events = core.handle_input(&input);
        assert!(events.is_empty());

        let sent = rx.try_recv().expect("options response");
        let resp_text = String::from_utf8(sent.payload).expect("utf8 response");
        let resp = match parse_sip_message(&resp_text).expect("parse response") {
            SipMessage::Response(resp) => resp,
            _ => panic!("expected response"),
        };
        assert_eq!(resp.status_code, 200);

        let allow = resp
            .headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case("Allow"))
            .map(|h| h.value.as_str());
        for method in ["INVITE", "ACK", "BYE", "OPTIONS", "UPDATE", "PRACK"] {
            assert!(header_has_token(allow, method));
        }

        let supported = resp
            .headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case("Supported"))
            .map(|h| h.value.as_str());
        assert!(header_has_token(supported, "100rel"));
        assert!(header_has_token(supported, "timer"));
    }

    #[test]
    fn invite_when_busy_returns_486() {
        let (tx, mut rx) = mpsc::channel(16);
        let mut core = SipCore::new(
            SipConfig {
                advertised_ip: "127.0.0.1".to_string(),
                sip_port: 5060,
                advertised_rtp_port: 4000,
            },
            tx,
        );

        let req1 = SipRequestBuilder::new(SipMethod::Invite, "sip:test@example.com")
            .header("Via", "SIP/2.0/UDP 127.0.0.1:5060")
            .header("From", "<sip:alice@example.com>;tag=alice")
            .header("To", "<sip:bob@example.com>")
            .header("Call-ID", "call-1")
            .header("CSeq", "1 INVITE")
            .build();
        let input1 = SipInput {
            peer: dummy_peer(),
            data: req1.to_bytes(),
        };

        let events = core.handle_input(&input1);
        assert_eq!(events.len(), 1);

        let req2 = SipRequestBuilder::new(SipMethod::Invite, "sip:test@example.com")
            .header("Via", "SIP/2.0/UDP 127.0.0.1:5060")
            .header("From", "<sip:carol@example.com>;tag=carol")
            .header("To", "<sip:dave@example.com>")
            .header("Call-ID", "call-2")
            .header("CSeq", "1 INVITE")
            .build();
        let input2 = SipInput {
            peer: dummy_peer(),
            data: req2.to_bytes(),
        };

        let events = core.handle_input(&input2);
        assert!(events.is_empty());

        let sent = rx.try_recv().expect("busy response");
        let resp_text = String::from_utf8(sent.payload).expect("utf8 response");
        let resp = match parse_sip_message(&resp_text).expect("parse response") {
            SipMessage::Response(resp) => resp,
            _ => panic!("expected response"),
        };
        assert_eq!(resp.status_code, 486);
    }

    #[test]
    fn invite_with_to_tag_without_dialog_returns_481() {
        let (tx, mut rx) = mpsc::channel(16);
        let mut core = SipCore::new(
            SipConfig {
                advertised_ip: "127.0.0.1".to_string(),
                sip_port: 5060,
                advertised_rtp_port: 4000,
            },
            tx,
        );

        let req = SipRequestBuilder::new(SipMethod::Invite, "sip:test@example.com")
            .header("Via", "SIP/2.0/UDP 127.0.0.1:5060")
            .header("From", "<sip:alice@example.com>;tag=alice")
            .header("To", "<sip:bob@example.com>;tag=remote")
            .header("Call-ID", "call-1")
            .header("CSeq", "2 INVITE")
            .build();
        let input = SipInput {
            peer: dummy_peer(),
            data: req.to_bytes(),
        };

        let events = core.handle_input(&input);
        assert!(events.is_empty());

        let sent = rx.try_recv().expect("481 response");
        let resp_text = String::from_utf8(sent.payload).expect("utf8 response");
        let resp = match parse_sip_message(&resp_text).expect("parse response") {
            SipMessage::Response(resp) => resp,
            _ => panic!("expected response"),
        };
        assert_eq!(resp.status_code, 481);
    }

    #[test]
    fn cancel_sends_200_and_event() {
        let (tx, mut rx) = mpsc::channel(16);
        let mut core = SipCore::new(
            SipConfig {
                advertised_ip: "127.0.0.1".to_string(),
                sip_port: 5060,
                advertised_rtp_port: 4000,
            },
            tx,
        );

        let invite = SipRequestBuilder::new(SipMethod::Invite, "sip:test@example.com")
            .header("Via", "SIP/2.0/UDP 127.0.0.1:5060")
            .header("From", "<sip:alice@example.com>;tag=alice")
            .header("To", "<sip:bob@example.com>")
            .header("Call-ID", "call-1")
            .header("CSeq", "1 INVITE")
            .build();
        let invite_input = SipInput {
            peer: dummy_peer(),
            data: invite.to_bytes(),
        };
        let events = core.handle_input(&invite_input);
        assert_eq!(events.len(), 1);

        let cancel = SipRequestBuilder::new(SipMethod::Cancel, "sip:test@example.com")
            .header("Via", "SIP/2.0/UDP 127.0.0.1:5060")
            .header("From", "<sip:alice@example.com>;tag=alice")
            .header("To", "<sip:bob@example.com>")
            .header("Call-ID", "call-1")
            .header("CSeq", "1 CANCEL")
            .build();
        let cancel_input = SipInput {
            peer: dummy_peer(),
            data: cancel.to_bytes(),
        };
        let events = core.handle_input(&cancel_input);
        assert_eq!(events.len(), 1);
        match &events[0] {
            SipEvent::Cancel { call_id } => assert_eq!(call_id.as_str(), "call-1"),
            other => panic!("expected Cancel event, got {:?}", other),
        }

        let sent = rx.try_recv().expect("cancel response");
        let resp_text = String::from_utf8(sent.payload).expect("utf8 response");
        let resp = match parse_sip_message(&resp_text).expect("parse response") {
            SipMessage::Response(resp) => resp,
            _ => panic!("expected response"),
        };
        assert_eq!(resp.status_code, 200);
    }

    #[test]
    fn session_timer_defaults_when_missing() {
        let req = SipRequestBuilder::new(SipMethod::Invite, "sip:test@example.com")
            .header("Via", "SIP/2.0/UDP 127.0.0.1:5060")
            .header("From", "<sip:alice@example.com>;tag=alice")
            .header("To", "<sip:bob@example.com>")
            .header("Call-ID", "call-1")
            .header("CSeq", "1 INVITE")
            .build();
        let cfg = config::SessionConfig {
            default_expires: Some(Duration::from_secs(1800)),
            min_se: 90,
        };
        let timer = session_timer_from_request(&req, &cfg, true)
            .expect("ok")
            .expect("timer");
        assert_eq!(timer.expires, Duration::from_secs(1800));
        assert_eq!(timer.refresher, SessionRefresher::Uas);
    }

    #[test]
    fn session_timer_rejects_too_small() {
        let req = SipRequestBuilder::new(SipMethod::Invite, "sip:test@example.com")
            .header("Via", "SIP/2.0/UDP 127.0.0.1:5060")
            .header("From", "<sip:alice@example.com>;tag=alice")
            .header("To", "<sip:bob@example.com>")
            .header("Call-ID", "call-1")
            .header("CSeq", "1 INVITE")
            .header("Session-Expires", "60")
            .build();
        let cfg = config::SessionConfig {
            default_expires: Some(Duration::from_secs(1800)),
            min_se: 90,
        };
        match session_timer_from_request(&req, &cfg, true) {
            Err(SessionTimerError::TooSmall { min_se }) => assert_eq!(min_se, 90),
            other => panic!("expected TooSmall, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_rtpmap_prefers_rtpmap_entry() {
        let sdp = "v=0\r\n\
c=IN IP4 192.0.2.10\r\n\
m=audio 4000 RTP/AVP 96 0\r\n\
a=rtpmap:96 PCMU/8000/1\r\n";
        let parsed = parse_offer_sdp(sdp.as_bytes()).expect("parse sdp");
        assert_eq!(parsed.ip, "192.0.2.10");
        assert_eq!(parsed.port, 4000);
        assert_eq!(parsed.payload_type, 96);
        assert_eq!(parsed.codec, "PCMU/8000");
    }

    #[test]
    fn test_parse_rtpmap_falls_back_to_static_pt() {
        let sdp = "v=0\r\n\
c=IN IP4 192.0.2.11\r\n\
m=audio 5000 RTP/AVP 8\r\n";
        let parsed = parse_offer_sdp(sdp.as_bytes()).expect("parse sdp");
        assert_eq!(parsed.codec, "PCMA/8000");
    }

    #[test]
    fn test_parse_rtpmap_dynamic_pt_without_mapping_is_unknown() {
        let sdp = "v=0\r\n\
c=IN IP4 192.0.2.12\r\n\
m=audio 6000 RTP/AVP 97\r\n";
        let parsed = parse_offer_sdp(sdp.as_bytes()).expect("parse sdp");
        assert_eq!(parsed.codec, "unknown");
    }
}
