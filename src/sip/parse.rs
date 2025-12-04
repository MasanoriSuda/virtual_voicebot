use anyhow::{anyhow, Result};

use crate::sip::message::{
    SipHeader, SipMessage, SipMethod, SipRequest, SipResponse,
};

fn split_head_and_body(input: &str) -> (&str, &str) {
    if let Some(pos) = input.find("\r\n\r\n") {
        let (head, rest) = input.split_at(pos);
        return (head, &rest[4..]);
    }
    if let Some(pos) = input.find("\n\n") {
        let (head, rest) = input.split_at(pos);
        return (head, &rest[2..]);
    }
    (input, "")
}

fn parse_headers<'a, I>(lines: I) -> Result<Vec<SipHeader>>
where
    I: Iterator<Item = &'a str>,
{
    let mut headers = Vec::new();
    for line in lines {
        let line = line.trim_end_matches('\r').trim();
        if line.is_empty() {
            continue;
        }
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| anyhow!("invalid SIP header line: {}", line))?;
        headers.push(SipHeader::new(name.trim(), value.trim()));
    }
    Ok(headers)
}

fn parse_method(token: &str) -> SipMethod {
    match token.to_ascii_uppercase().as_str() {
        "INVITE" => SipMethod::Invite,
        "ACK" => SipMethod::Ack,
        "BYE" => SipMethod::Bye,
        "CANCEL" => SipMethod::Cancel,
        "OPTIONS" => SipMethod::Options,
        "REGISTER" => SipMethod::Register,
        other => SipMethod::Unknown(other.to_string()),
    }
}

pub fn parse_sip_message(input: &str) -> Result<SipMessage> {
    let (head, body) = split_head_and_body(input);

    let mut lines = head.lines();
    let start_line = lines
        .next()
        .ok_or_else(|| anyhow!("empty SIP message"))?
        .trim_end_matches('\r')
        .trim();

    if start_line.starts_with("SIP/2.0") {
        // Response
        let mut parts = start_line.splitn(3, ' ');
        let version = parts.next().unwrap_or("SIP/2.0").to_string();
        let status_code = parts
            .next()
            .ok_or_else(|| anyhow!("missing status code"))?
            .parse::<u16>()?;
        let reason_phrase = parts.next().unwrap_or("").trim().to_string();
        let headers = parse_headers(lines)?;

        Ok(SipMessage::Response(SipResponse {
            version,
            status_code,
            reason_phrase,
            headers,
            body: body.as_bytes().to_vec(),
        }))
    } else {
        // Request
        let mut parts = start_line.split_whitespace();
        let method_token = parts
            .next()
            .ok_or_else(|| anyhow!("missing SIP method"))?;
        let uri = parts
            .next()
            .ok_or_else(|| anyhow!("missing request URI"))?
            .to_string();
        let version = parts.next().unwrap_or("SIP/2.0").to_string();

        let headers = parse_headers(lines)?;

        Ok(SipMessage::Request(SipRequest {
            method: parse_method(method_token),
            uri,
            version,
            headers,
            body: body.as_bytes().to_vec(),
        }))
    }
}
