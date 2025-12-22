#![allow(dead_code)]
use std::fmt::{self, Write};

use crate::session::types::Sdp;
use crate::sip::message::{SipHeader, SipMethod, SipRequest, SipResponse};

/// 追加で使いやすい Builder スタイル
pub struct SipResponseBuilder {
    status_code: u16,
    reason_phrase: String,
    headers: Vec<SipHeader>,
    body: Vec<u8>,
}

pub struct SipRequestBuilder {
    method: SipMethod,
    uri: String,
    headers: Vec<SipHeader>,
    body: Vec<u8>,
}

impl SipResponseBuilder {
    pub fn new(code: u16, reason: impl Into<String>) -> Self {
        Self {
            status_code: code,
            reason_phrase: reason.into(),
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push(SipHeader::new(name, value));
        self
    }

    pub fn body(mut self, body: impl Into<Vec<u8>>, content_type: Option<&str>) -> Self {
        self.body = body.into();
        if let Some(ct) = content_type {
            let has_ct = self
                .headers
                .iter()
                .any(|h| h.name.eq_ignore_ascii_case("Content-Type"));
            if !has_ct {
                self.headers.push(SipHeader::new("Content-Type", ct));
            }
        }
        self
    }

    pub fn build(mut self) -> SipResponse {
        ensure_content_length(&mut self.headers, self.body.len());
        SipResponse {
            version: "SIP/2.0".to_string(),
            status_code: self.status_code,
            reason_phrase: self.reason_phrase,
            headers: self.headers,
            body: self.body,
        }
    }
}

impl SipRequestBuilder {
    pub fn new(method: SipMethod, uri: impl Into<String>) -> Self {
        Self {
            method,
            uri: uri.into(),
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push(SipHeader::new(name, value));
        self
    }

    pub fn body(mut self, body: impl Into<Vec<u8>>, content_type: Option<&str>) -> Self {
        self.body = body.into();
        if let Some(ct) = content_type {
            let has_ct = self
                .headers
                .iter()
                .any(|h| h.name.eq_ignore_ascii_case("Content-Type"));
            if !has_ct {
                self.headers.push(SipHeader::new("Content-Type", ct));
            }
        }
        self
    }

    pub fn build(mut self) -> SipRequest {
        ensure_content_length(&mut self.headers, self.body.len());
        SipRequest {
            method: self.method,
            uri: self.uri,
            version: "SIP/2.0".to_string(),
            headers: self.headers,
            body: self.body,
        }
    }
}

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
fn method_to_str(method: &SipMethod) -> &str {
    match method {
        SipMethod::Invite => "INVITE",
        SipMethod::Ack => "ACK",
        SipMethod::Bye => "BYE",
        SipMethod::Cancel => "CANCEL",
        SipMethod::Options => "OPTIONS",
        SipMethod::Register => "REGISTER",
        SipMethod::Update => "UPDATE",
        SipMethod::Prack => "PRACK",
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

impl fmt::Display for SipRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

        f.write_str(&out)
    }
}

impl SipRequest {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = self.to_string().into_bytes();
        buf.extend_from_slice(&self.body);
        buf
    }
}

/// リクエストヘッダから 1xx/空ボディレスポンスを組み立てる（To-tag を付与）。
pub fn response_provisional_from_request(
    req: &SipRequest,
    code: u16,
    reason: &str,
) -> Option<SipResponse> {
    let via = req.header_value("Via")?;
    let from = req.header_value("From")?;
    let mut to = req.header_value("To")?.to_string();
    let call_id = req.header_value("Call-ID")?;
    let cseq = req.header_value("CSeq")?;

    if !to.to_ascii_lowercase().contains("tag=") {
        to = format!("{to};tag=rustbot");
    }

    Some(
        SipResponseBuilder::new(code, reason)
            .header("Via", via)
            .header("From", from)
            .header("To", to)
            .header("Call-ID", call_id)
            .header("CSeq", cseq)
            .build(),
    )
}

/// リクエストヘッダ＋SDPから 2xx 応答を組み立てる。
pub fn response_final_with_sdp(
    req: &SipRequest,
    code: u16,
    reason: &str,
    contact_ip: &str,
    sip_port: u16,
    answer: &Sdp,
) -> Option<SipResponse> {
    let via = req.header_value("Via")?;
    let from = req.header_value("From")?;
    let mut to = req.header_value("To")?.to_string();
    let call_id = req.header_value("Call-ID")?;
    let cseq = req.header_value("CSeq")?;

    if !to.to_ascii_lowercase().contains("tag=") {
        to = format!("{to};tag=rustbot");
    }

    let sdp = format!(
        concat!(
            "v=0\r\n",
            "o=rustbot 1 1 IN IP4 {ip}\r\n",
            "s=Rust PCMU Bot\r\n",
            "c=IN IP4 {ip}\r\n",
            "t=0 0\r\n",
            "m=audio {rtp} RTP/AVP {pt}\r\n",
            "a=rtpmap:{pt} {codec}\r\n",
            "a=sendrecv\r\n",
        ),
        ip = answer.ip,
        rtp = answer.port,
        pt = answer.payload_type,
        codec = answer.codec
    );

    Some(
        SipResponseBuilder::new(code, reason)
            .header("Via", via)
            .header("From", from)
            .header("To", to)
            .header("Call-ID", call_id)
            .header("CSeq", cseq)
            .header("Contact", format!("sip:rustbot@{contact_ip}:{sip_port}"))
            .body(sdp.as_bytes(), Some("application/sdp"))
            .build(),
    )
}

/// BYE/REGISTER など 2xx 空ボディ応答。
pub fn response_simple_from_request(
    req: &SipRequest,
    code: u16,
    reason: &str,
) -> Option<SipResponse> {
    let via = req.header_value("Via")?;
    let from = req.header_value("From")?;
    let mut to = req.header_value("To")?.to_string();
    let call_id = req.header_value("Call-ID")?;
    let cseq = req.header_value("CSeq")?;

    if !to.to_ascii_lowercase().contains("tag=") {
        to = format!("{to};tag=rustbot");
    }

    Some(
        SipResponseBuilder::new(code, reason)
            .header("Via", via)
            .header("From", from)
            .header("To", to)
            .header("Call-ID", call_id)
            .header("CSeq", cseq)
            .build(),
    )
}

impl fmt::Display for SipResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = String::new();
        let mut headers = self.headers.clone();
        ensure_content_length(&mut headers, self.body.len());

        let _ = writeln!(
            out,
            "{} {} {}\r",
            self.version, self.status_code, self.reason_phrase
        );
        render_headers(&headers, &mut out);
        out.push_str("\r\n");

        f.write_str(&out)
    }
}

impl SipResponse {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = self.to_string().into_bytes();
        buf.extend_from_slice(&self.body);
        buf
    }
}
