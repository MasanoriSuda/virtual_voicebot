#![allow(dead_code)]

use anyhow::{anyhow, Result};

use crate::protocol::sip::message::{SipHeader, SipUri};
use crate::protocol::sip::protocols::HeaderCodec;

#[derive(Debug, Clone)]
pub struct NameAddrHeader {
    pub display: Option<String>,
    pub uri: SipUri,
    pub params: Vec<(String, String)>,
}

pub type FromHeader = NameAddrHeader;
pub type ToHeader = NameAddrHeader;
pub type ContactHeader = NameAddrHeader;

impl HeaderCodec for NameAddrHeader {
    const NAME: &'static str = "Name-Addr"; // 実際のヘッダ名は利用側で指定

    fn parse(value: &str) -> Result<Self> {
        parse_name_addr(value)
    }

    fn to_header(&self) -> SipHeader {
        let mut value = String::new();
        if let Some(disp) = &self.display {
            value.push_str(disp);
            value.push(' ');
        }
        value.push('<');
        value.push_str(&format_uri(&self.uri));
        value.push('>');
        for (k, v) in &self.params {
            if v.is_empty() {
                value.push_str(&format!(";{}", k));
            } else {
                value.push_str(&format!(";{}={}", k, v));
            }
        }
        SipHeader::new(Self::NAME, value)
    }
}

pub fn parse_name_addr(value: &str) -> Result<NameAddrHeader> {
    let value = value.trim();
    let (display, uri_and_params) = if let Some(start) = value.find('<') {
        let end = value
            .find('>')
            .ok_or_else(|| anyhow!("invalid name-addr"))?;
        let display = value[..start].trim().trim_matches('"');
        let uri = &value[start + 1..end];
        let after = value[end + 1..].trim();
        (
            if display.is_empty() {
                None
            } else {
                Some(display.to_string())
            },
            (uri, after),
        )
    } else {
        (None, (value, ""))
    };

    let uri = parse_uri(uri_and_params.0)?;
    let params = parse_params(uri_and_params.1);

    Ok(NameAddrHeader {
        display,
        uri,
        params,
    })
}

pub fn parse_uri(input: &str) -> Result<SipUri> {
    // 超簡易: scheme:user@host:port;param=val
    let mut rest = input.trim();

    let (scheme, after_scheme) = rest
        .split_once(':')
        .ok_or_else(|| anyhow!("uri missing scheme"))?;
    rest = after_scheme;

    let (user_part, host_part) = if let Some(idx) = rest.find('@') {
        let (u, h) = rest.split_at(idx);
        (Some(u.to_string()), &h[1..])
    } else {
        (None, rest)
    };

    let mut host = host_part.to_string();
    let mut port = None;
    let mut params = Vec::new();

    if let Some(idx) = host_part.find(';') {
        host = host_part[..idx].to_string();
        let param_str = &host_part[idx + 1..];
        params = parse_params(param_str);
    }

    if let Some(idx) = host.find(':') {
        let p = host[idx + 1..].parse::<u16>().ok();
        port = p;
        host = host[..idx].to_string();
    }

    Ok(SipUri {
        scheme: scheme.to_string(),
        user: user_part,
        host,
        port,
        params,
    })
}

pub fn parse_params(input: &str) -> Vec<(String, String)> {
    input
        .split(';')
        .filter(|s| !s.trim().is_empty())
        .filter_map(|p| {
            let mut iter = p.splitn(2, '=');
            let k = iter.next()?.trim();
            let v = iter.next().unwrap_or("").trim();
            Some((k.to_string(), v.to_string()))
        })
        .collect()
}

fn format_uri(uri: &SipUri) -> String {
    let mut s = String::new();
    s.push_str(&uri.scheme);
    s.push(':');
    if let Some(u) = &uri.user {
        s.push_str(u);
        s.push('@');
    }
    s.push_str(&uri.host);
    if let Some(p) = uri.port {
        s.push(':');
        s.push_str(&p.to_string());
    }
    for (k, v) in &uri.params {
        if v.is_empty() {
            s.push_str(&format!(";{}", k));
        } else {
            s.push_str(&format!(";{}={}", k, v));
        }
    }
    s
}
