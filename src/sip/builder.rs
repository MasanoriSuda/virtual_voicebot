use std::fmt::Write;

use crate::sip::message::{SipHeader, SipMethod, SipRequest, SipResponse};

fn ensure_content_length(headers: &mut Vec<SipHeader>, body_len: usize) {
    let has_len = headers
        .iter()
        .any(|h| h.name.eq_ignore_ascii_case("Content-Length"));
    if !has_len {
        headers.push(SipHeader::new("Content-Length", body_len.to_string()));
    }
}

fn render_headers(headers: &[SipHeader], out: &mut String) {
    for h in headers {
        // SIP は CRLF 区切り
        let _ = writeln!(out, "{}: {}\r", h.name, h.value);
    }
}

#[allow(dead_code)]
fn method_to_str<'a>(method: &'a SipMethod) -> &'a str {
    match method {
        SipMethod::Invite => "INVITE",
        SipMethod::Ack => "ACK",
        SipMethod::Bye => "BYE",
        SipMethod::Cancel => "CANCEL",
        SipMethod::Options => "OPTIONS",
        SipMethod::Register => "REGISTER",
        SipMethod::Unknown(token) => token.as_str(),
    }
}

#[allow(dead_code)]
pub fn build_request(
    method: SipMethod,
    uri: impl Into<String>,
    mut headers: Vec<SipHeader>,
    body: Vec<u8>,
) -> SipRequest {
    ensure_content_length(&mut headers, body.len());
    SipRequest {
        method,
        uri: uri.into(),
        version: "SIP/2.0".to_string(),
        headers,
        body,
    }
}

pub fn build_response(
    status_code: u16,
    reason_phrase: impl Into<String>,
    mut headers: Vec<SipHeader>,
    body: Vec<u8>,
) -> SipResponse {
    ensure_content_length(&mut headers, body.len());
    SipResponse {
        version: "SIP/2.0".to_string(),
        status_code,
        reason_phrase: reason_phrase.into(),
        headers,
        body,
    }
}

impl SipRequest {
    #[allow(dead_code)]
    pub fn to_string(&self) -> String {
        let mut out = String::new();
        let mut headers = self.headers.clone();
        ensure_content_length(&mut headers, self.body.len());

        let _ = writeln!(
            out,
            "{} {} {}\r",
            method_to_str(&self.method),
            self.uri,
            self.version
        );
        render_headers(&headers, &mut out);
        out.push_str("\r\n");
        out
    }

    #[allow(dead_code)]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = self.to_string().into_bytes();
        buf.extend_from_slice(&self.body);
        buf
    }
}

impl SipResponse {
    pub fn to_string(&self) -> String {
        let mut out = String::new();
        let mut headers = self.headers.clone();
        ensure_content_length(&mut headers, self.body.len());

        let _ = writeln!(
            out,
            "{} {} {}\r",
            self.version,
            self.status_code,
            self.reason_phrase
        );
        render_headers(&headers, &mut out);
        out.push_str("\r\n");
        out
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = self.to_string().into_bytes();
        buf.extend_from_slice(&self.body);
        buf
    }
}
