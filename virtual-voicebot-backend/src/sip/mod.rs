pub mod builder;
pub mod register;
pub mod message;
pub mod parse;
pub mod protocols;
pub mod transaction;
pub mod tx;

#[allow(unused_imports)]
pub use message::{SipHeader, SipMessage, SipMethod, SipRequest, SipResponse};

pub use parse::parse_sip_message;

#[allow(unused_imports)]
pub use crate::sip::builder::{SipRequestBuilder, SipResponseBuilder};

#[allow(unused_imports)]
pub use crate::sip::parse::{
    collect_common_headers, parse_cseq as parse_cseq_header, parse_name_addr, parse_uri,
    parse_via_header,
};

#[allow(unused_imports)]
pub use protocols::*;

use crate::session::types::{CallId, Sdp, SessionOut, SessionRefresher, SessionTimerInfo};
use crate::sip::builder::{
    response_final_with_sdp, response_options_from_request, response_provisional_from_request,
    response_simple_from_request,
};
use crate::sip::register::RegisterClient;
use crate::sip::transaction::{
    InviteServerTransaction, InviteTxAction, InviteTxState, NonInviteServerTransaction,
    NonInviteTxState,
};
use crate::sip::tx::{SipTransportRequest, SipTransportTx};
use crate::transport::{SipInput, TransportPeer};
use crate::config;
use rand::Rng;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tokio::sync::Notify;

/// sip 層から session 層へ渡すイベント（設計ドキュメントの「sip→session 通知」と対応）
#[derive(Debug)]
pub enum SipEvent {
    /// INVITE を受けたときの session への通知（call_id/from/to/offer を引き渡す）
    IncomingInvite {
        call_id: CallId,
        from: String,
        to: String,
        offer: Sdp,
        session_timer: Option<SessionTimerInfo>,
    },
    /// 既存ダイアログ内の re-INVITE
    ReInvite {
        call_id: CallId,
        offer: Sdp,
        session_timer: Option<SessionTimerInfo>,
    },
    /// 既存ダイアログに対する ACK
    Ack {
        call_id: CallId,
    },
    /// 既存ダイアログに対する BYE
    Bye {
        call_id: CallId,
    },
    /// トランザクションのタイムアウト通知（Timer J など）
    TransactionTimeout {
        call_id: CallId,
    },
    /// Session-Expires を受けたときのセッション更新通知
    SessionRefresh {
        call_id: CallId,
        timer: SessionTimerInfo,
    },
    Unknown,
}

#[derive(Clone)]
pub struct SipConfig {
    pub advertised_ip: String,
    pub sip_port: u16,
    #[allow(dead_code)]
    pub advertised_rtp_port: u16,
}

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

fn parse_offer_sdp(body: &[u8]) -> Option<Sdp> {
    let s = std::str::from_utf8(body).ok()?;
    let mut ip = None;
    let mut port = None;
    let mut pt = None;
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
        }
    }
    Some(Sdp {
        ip: ip?,
        port: port?,
        payload_type: pt.unwrap_or(0),
        codec: "PCMU/8000".to_string(),
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

fn header_has_token(value: Option<&str>, token: &str) -> bool {
    let Some(value) = value else {
        return false;
    };
    value
        .split(|c| c == ',' || c == ' ' || c == '\t')
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
            format!("{contact_scheme}:rustbot@{}:{}", cfg.advertised_ip, cfg.sip_port),
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
    call_id: String,
    cseq: String,
}

impl CoreHeaderSnapshot {
    fn from_request(req: &SipRequest) -> Self {
        Self {
            via: req.header_value("Via").unwrap_or("").to_string(),
            from: req.header_value("From").unwrap_or("").to_string(),
            to: req.header_value("To").unwrap_or("").to_string(),
            call_id: req.header_value("Call-ID").unwrap_or("").to_string(),
            cseq: req.header_value("CSeq").unwrap_or("").to_string(),
        }
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
                            let _ = transport_tx.send(SipTransportRequest {
                                peer,
                                src_port,
                                payload,
                            });
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
            Err(_) => return vec![SipEvent::Unknown],
        };

        let msg = match parse_sip_message(&text) {
            Ok(m) => m,
            Err(_) => return vec![SipEvent::Unknown],
        };

        let mut ev = match msg {
            SipMessage::Request(req) => self.handle_request(req, input.peer),
            SipMessage::Response(resp) => self.handle_response(resp, input.peer),
        };

        events.append(&mut ev);
        events
    }

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
                log::warn!("[sip register] no transport for unregister: {:?}", transport);
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
        let headers = CoreHeaderSnapshot::from_request(&req);
        match req.method {
            SipMethod::Invite => self.handle_invite(req, headers, peer),
            SipMethod::Ack => self.handle_ack(headers.call_id),
            SipMethod::Bye => self.handle_non_invite(req, headers, peer, 200, "OK", true),
            SipMethod::Options => self.handle_options(req, headers, peer),
            SipMethod::Register => self.handle_non_invite(req, headers, peer, 200, "OK", false),
            SipMethod::Update => self.handle_update(req, headers, peer),
            SipMethod::Prack => self.handle_prack(req, headers, peer),
            _ => vec![SipEvent::Unknown],
        }
    }

    fn handle_response(&mut self, resp: SipResponse, peer: TransportPeer) -> Vec<SipEvent> {
        if let Some(register) = self.register.as_ref() {
            let (handled, pending_req, pending_peer) = {
                let mut reg = register.lock().unwrap();
                let handled = reg.handle_response(&resp, peer);
                let pending_req = if handled { reg.take_pending_request() } else { None };
                let pending_peer = pending_req
                    .as_ref()
                    .and_then(|_| reg.transport_peer());
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
        vec![SipEvent::Unknown]
    }

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

        let session_timer =
            match session_timer_from_request(&req, config::session_config(), true) {
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
                    resp.headers.push(SipHeader::new("Min-SE", min_se.to_string()));
                    self.send_payload(peer, resp.to_bytes());
                }
                return vec![];
            }
        };

        // 新規 INVITE: トランザクション生成（レスポンスは SessionOut 経由で送るためここでは送信しない）
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

    fn handle_reinvite(
        &mut self,
        req: SipRequest,
        headers: CoreHeaderSnapshot,
        peer: TransportPeer,
    ) -> Vec<SipEvent> {
        let session_timer =
            match session_timer_from_request(&req, config::session_config(), false) {
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
                        resp.headers.push(SipHeader::new("Min-SE", min_se.to_string()));
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

        // 最終応答は SessionOut::SipSendBye200 等から送るため、ここでは送信しない
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
            if let Some(resp) = response_simple_from_request(&req, 481, "Call/Transaction Does Not Exist") {
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
            if let Some(resp) = response_simple_from_request(&req, 481, "Call/Transaction Does Not Exist") {
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

    fn handle_update(
        &mut self,
        req: SipRequest,
        headers: CoreHeaderSnapshot,
        peer: TransportPeer,
    ) -> Vec<SipEvent> {
        let session_timer =
            match session_timer_from_request(&req, config::session_config(), false) {
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
                    resp.headers.push(SipHeader::new("Min-SE", min_se.to_string()));
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
                        let _ = transport_tx.send(SipTransportRequest {
                            peer,
                            src_port,
                            payload: resp,
                        });
                    }
                    break;
                }
                let _ = transport_tx.send(SipTransportRequest {
                    peer,
                    src_port,
                    payload: payload.clone(),
                });
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
                let _ = transport_tx.send(SipTransportRequest {
                    peer,
                    src_port,
                    payload: payload.clone(),
                });
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

    pub fn handle_session_out(&mut self, call_id: &CallId, out: SessionOut) {
        match out {
            SessionOut::SipSend100 => {
                if let Some(ctx) = self.invites.get_mut(call_id) {
                    if let Some(resp) = response_provisional_from_request(&ctx.req, 100, "Trying")
                    {
                        let bytes = resp.to_bytes();
                        ctx.tx.remember_provisional(bytes.clone());
                        let peer = ctx.tx.peer;
                        self.send_payload(peer, bytes);
                    }
                }
            }
            SessionOut::SipSend180 => {
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
            SessionOut::SipSend200 { answer } => {
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
                            log::info!(
                                "[sip 200 sdp] call_id={} sdp={}",
                                call_id,
                                sdp_inline
                            );
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
                }
            }
            SessionOut::SipSendUpdate { expires } => {
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
            SessionOut::SipSendBye => {
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
            SessionOut::SipSendBye200 => {
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
            _ => { /* 他の SessionOut は現状未配線 */ }
        }
    }

    fn send_payload(&self, peer: TransportPeer, payload: Vec<u8>) {
        if let Some(first_line) = payload
            .split(|b| *b == b'\n')
            .next()
            .and_then(|line| std::str::from_utf8(line).ok())
        {
            log::info!(
                "[sip ->] {}:{} -> {:?} {}",
                self.cfg.advertised_ip,
                self.cfg.sip_port,
                peer,
                first_line.trim()
            );
        } else {
            log::info!(
                "[sip ->] {}:{} -> {:?} len={}",
                self.cfg.advertised_ip,
                self.cfg.sip_port,
                peer,
                payload.len()
            );
        }

        let _ = self.transport_tx.send(SipTransportRequest {
            peer,
            src_port: self.cfg.sip_port,
            payload,
        });
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
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use tokio::sync::mpsc::unbounded_channel;

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
    fn options_response_includes_allow_and_supported() {
        let (tx, mut rx) = unbounded_channel();
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
        let (tx, mut rx) = unbounded_channel();
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
}
