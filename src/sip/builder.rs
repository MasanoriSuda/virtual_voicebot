use std::fmt::Write;

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
