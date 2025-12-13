pub mod builder;
pub mod message;
pub mod parse;
pub mod protocols;
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

use crate::session::types::{CallId, Sdp};
use crate::transport::SipInput;

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
    Unknown,
}

/// SIP ソケットで受けた datagram（transport 層から渡される生バイト列）を
/// 「構造化メッセージ → session へのイベント」に変換する。
/// 責務: 入力バイトのデコード・簡易パース・イベント化のみ（送信やトランザクションは扱わない）。
pub fn process_sip_datagram(input: &SipInput) -> Vec<SipEvent> {
    let text = match decode_sip_text(&input.data) {
        Ok(t) => t,
        Err(_) => return vec![SipEvent::Unknown],
    };

    let msg = match parse_sip_message(&text) {
        Ok(m) => m,
        Err(_) => return vec![SipEvent::Unknown],
    };

    match msg {
        SipMessage::Request(req) => vec![sip_request_to_event(req)],
        SipMessage::Response(_) => vec![SipEvent::Unknown],
    }
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

fn sip_request_to_event(req: SipRequest) -> SipEvent {
    let headers = CoreHeaderSnapshot::from_request(&req);
    match req.method {
        SipMethod::Invite => {
            let offer = parse_offer_sdp(&req.body).unwrap_or_else(|| Sdp::pcmu("0.0.0.0", 0));
            SipEvent::IncomingInvite {
                call_id: headers.call_id,
                from: headers.from,
                to: headers.to,
                offer,
            }
        }
        SipMethod::Ack => SipEvent::Ack {
            call_id: headers.call_id,
        },
        SipMethod::Bye => SipEvent::Bye {
            call_id: headers.call_id,
        },
        _ => SipEvent::Unknown,
    }
}
