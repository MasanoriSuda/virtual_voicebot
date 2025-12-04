pub mod message;
pub mod parse;
pub mod builder;

#[allow(unused_imports)]
pub use message::{SipHeader, SipMessage, SipMethod, SipRequest, SipResponse};

pub use parse::parse_sip_message;

#[allow(unused_imports)]
pub use crate::sip::builder::{SipRequestBuilder, SipResponseBuilder};

use crate::packet::SipInput;
use crate::session::types::Sdp;

#[derive(Debug)]
pub enum SipEvent {
    IncomingInvite {
        call_id: String,
        from: String,
        to: String,
        offer: Sdp,
    },
    Ack { call_id: String },
    Bye { call_id: String },
    Unknown,
}

pub fn process_sip_datagram(input: &SipInput) -> Vec<SipEvent> {
    let text = match String::from_utf8(input.data.clone()) {
        Ok(t) => t,
        Err(_) => return vec![SipEvent::Unknown],
    };

    let msg = match parse_sip_message(&text) {
        Ok(m) => m,
        Err(_) => return vec![SipEvent::Unknown],
    };

    match msg {
        SipMessage::Request(req) => match req.method {
            SipMethod::Invite => {
                let call_id = req
                    .header_value("Call-ID")
                    .unwrap_or("")
                    .to_string();
                let from = req.header_value("From").unwrap_or("").to_string();
                let to = req.header_value("To").unwrap_or("").to_string();
                let offer = parse_offer_sdp(&req.body).unwrap_or_else(|| Sdp::pcmu("0.0.0.0", 0));
                vec![SipEvent::IncomingInvite {
                    call_id,
                    from,
                    to,
                    offer,
                }]
            }
            SipMethod::Ack => {
                let call_id = req
                    .header_value("Call-ID")
                    .unwrap_or("")
                    .to_string();
                vec![SipEvent::Ack { call_id }]
            }
            SipMethod::Bye => {
                let call_id = req
                    .header_value("Call-ID")
                    .unwrap_or("")
                    .to_string();
                vec![SipEvent::Bye { call_id }]
            }
            _ => vec![SipEvent::Unknown],
        },
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
