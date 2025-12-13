pub mod builder;
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

use crate::session::types::{CallId, Sdp, SessionOut};
use crate::sip::builder::{
    response_final_with_sdp, response_provisional_from_request, response_simple_from_request,
};
use crate::sip::transaction::{
    InviteServerTransaction, InviteTxAction, InviteTxState, NonInviteServerTransaction,
    NonInviteTxState,
};
use crate::sip::tx::{SipTransportRequest, SipTransportTx};
use crate::transport::SipInput;
use std::collections::HashMap;
use std::time::Instant;
use tokio::time::sleep;

/// sip 層から session 層へ渡すイベント（設計ドキュメントの「sip→session 通知」と対応）
#[derive(Debug)]
pub enum SipEvent {
    /// INVITE を受けたときの session への通知（call_id/from/to/offer を引き渡す）
    IncomingInvite {
        call_id: CallId,
        from: String,
        to: String,
        offer: Sdp,
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
}

struct InviteContext {
    tx: InviteServerTransaction,
    req: SipRequest,
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

impl SipCore {
    pub fn new(cfg: SipConfig, transport_tx: SipTransportTx) -> Self {
        Self {
            cfg,
            transport_tx,
            invites: std::collections::HashMap::new(),
            non_invites: std::collections::HashMap::new(),
        }
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
            SipMessage::Request(req) => self.handle_request(req, input.src),
            SipMessage::Response(_) => vec![SipEvent::Unknown],
        };

        events.append(&mut ev);
        events
    }

    fn handle_request(&mut self, req: SipRequest, peer: std::net::SocketAddr) -> Vec<SipEvent> {
        let headers = CoreHeaderSnapshot::from_request(&req);
        match req.method {
            SipMethod::Invite => self.handle_invite(req, headers, peer),
            SipMethod::Ack => self.handle_ack(headers.call_id),
            SipMethod::Bye => self.handle_non_invite(req, headers, peer, 200, "OK", true),
            SipMethod::Register => self.handle_non_invite(req, headers, peer, 200, "OK", false),
            _ => vec![SipEvent::Unknown],
        }
    }

    fn handle_invite(
        &mut self,
        req: SipRequest,
        headers: CoreHeaderSnapshot,
        peer: std::net::SocketAddr,
    ) -> Vec<SipEvent> {
        // 再送判定: 既存トランザクションがあれば最新レスポンスを再送し、イベントは出さない
        if let Some(ctx) = self.invites.get_mut(&headers.call_id) {
            if let Some(action) = ctx.tx.on_retransmit() {
                self.send_tx_action(action, peer);
            }
            return vec![];
        }

        // 新規 INVITE: トランザクション生成（レスポンスは SessionOut 経由で送るためここでは送信しない）
        let mut tx = InviteServerTransaction::new(peer);
        tx.invite_req = Some(req.clone());
        let ctx = InviteContext {
            tx,
            req: req.clone(),
        };
        self.invites.insert(headers.call_id.clone(), ctx);

        let offer = parse_offer_sdp(&req.body).unwrap_or_else(|| Sdp::pcmu("0.0.0.0", 0));
        vec![SipEvent::IncomingInvite {
            call_id: headers.call_id,
            from: headers.from,
            to: headers.to,
            offer,
        }]
    }

    fn handle_ack(&mut self, call_id: CallId) -> Vec<SipEvent> {
        let (action, terminate, peer_opt) = if let Some(ctx) = self.invites.get_mut(&call_id) {
            let peer = ctx.tx.peer;
            let action = ctx.tx.on_ack();
            let terminate = ctx.tx.state == InviteTxState::Terminated;
            (action, terminate, Some(peer))
        } else {
            (None, false, None)
        };

        if let Some(action) = action {
            if let Some(peer) = peer_opt {
                self.send_tx_action(action, peer);
            }
        }
        if terminate {
            self.invites.remove(&call_id);
        }
        vec![SipEvent::Ack { call_id }]
    }

    fn handle_non_invite(
        &mut self,
        req: SipRequest,
        headers: CoreHeaderSnapshot,
        peer: std::net::SocketAddr,
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

    fn send_tx_action(&self, action: InviteTxAction, peer: std::net::SocketAddr) {
        match action {
            InviteTxAction::Retransmit(resp) => self.send_payload(peer, resp),
            InviteTxAction::Timeout => { /* Timer H/I 経由の通知などは未使用 */ }
        }
    }

    pub fn handle_session_out(&mut self, call_id: &CallId, out: SessionOut) {
        match out {
            SessionOut::SipSend180 => {
                if let Some(ctx) = self.invites.get_mut(call_id) {
                    if let Some(resp) = response_provisional_from_request(&ctx.req, 180, "Ringing")
                    {
                        let bytes = resp.to_bytes();
                        ctx.tx.remember_provisional(bytes.clone());
                        let peer = ctx.tx.peer;
                        self.send_payload(peer, bytes);
                    }
                }
            }
            SessionOut::SipSend200 { answer } => {
                if let Some(ctx) = self.invites.get_mut(call_id) {
                    if let Some(resp) = response_final_with_sdp(
                        &ctx.req,
                        200,
                        "OK",
                        &self.cfg.advertised_ip,
                        self.cfg.sip_port,
                        &answer,
                    ) {
                        let bytes = resp.to_bytes();
                        ctx.tx.on_final_sent(bytes.clone(), 200);
                        let peer = ctx.tx.peer;
                        self.send_payload(peer, bytes);
                    }
                }
            }
            SessionOut::SipSendBye200 => {
                if let Some(tx) = self.non_invites.get_mut(call_id) {
                    if let Some(req) = tx.last_request.clone() {
                        if let Some(resp) = response_simple_from_request(&req, 200, "OK") {
                            let bytes = resp.to_bytes();
                            tx.on_final_sent(bytes.clone());
                            let peer = tx.peer;
                            let final_resp = tx.last_final.clone();
                            let expires = tx.expires_at;
                            self.send_payload(peer, bytes.clone());
                            // Timer J 相当の再送（送信キュー経由）
                            if let Some(final_resp) = final_resp {
                                let transport_tx = self.transport_tx.clone();
                                let src_port = self.cfg.sip_port;
                                tokio::spawn(async move {
                                    let mut interval = std::time::Duration::from_millis(500);
                                    while Instant::now() < expires {
                                        let _ = transport_tx.send(SipTransportRequest {
                                            dst: peer,
                                            src_port,
                                            payload: final_resp.clone(),
                                        });
                                        sleep(interval).await;
                                        interval = std::cmp::min(
                                            interval * 2,
                                            std::time::Duration::from_secs(4),
                                        );
                                    }
                                });
                            }
                        }
                    }
                }
            }
            _ => { /* 他の SessionOut は現状未配線 */ }
        }
    }

    fn send_payload(&self, dst: std::net::SocketAddr, payload: Vec<u8>) {
        if let Some(first_line) = payload
            .split(|b| *b == b'\n')
            .next()
            .and_then(|line| std::str::from_utf8(line).ok())
        {
            log::info!("[sip ->] to {} {}", dst, first_line.trim());
        } else {
            log::info!("[sip ->] to {} len={}", dst, payload.len());
        }

        let _ = self.transport_tx.send(SipTransportRequest {
            dst,
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
