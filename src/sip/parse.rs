use anyhow::{anyhow, Result};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_till1, take_until, take_while1},
    character::complete::{digit1, not_line_ending, space1},
    combinator::{map, map_res},
    sequence::{terminated, tuple},
    IResult,
};

use crate::sip::message::{SipHeader, SipMessage, SipMethod, SipRequest, SipResponse};

enum StartLine {
    Request {
        method: SipMethod,
        uri: String,
        version: String,
    },
    Response {
        version: String,
        status: u16,
        reason: String,
    },
}

pub fn parse_sip_message(input: &str) -> Result<SipMessage> {
    let (head, body) = split_head_and_body(input);
    let (start_line, headers) = parse_head(head)?;

    match start_line {
        StartLine::Request {
            method,
            uri,
            version,
        } => Ok(SipMessage::Request(SipRequest {
            method,
            uri,
            version,
            headers,
            body: body.as_bytes().to_vec(),
        })),
        StartLine::Response {
            version,
            status,
            reason,
        } => Ok(SipMessage::Response(SipResponse {
            version,
            status_code: status,
            reason_phrase: reason,
            headers,
            body: body.as_bytes().to_vec(),
        })),
    }
}

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

fn parse_head(input: &str) -> Result<(StartLine, Vec<SipHeader>)> {
    let (rest, start) = parse_start_line(input)
        .map_err(|e| anyhow!("failed to parse start line: {:?}", e))?;

    let headers = parse_headers_block(rest)
        .map_err(|e| anyhow!("failed to parse headers: {:?}", e))?;

    Ok((start, headers))
}

fn parse_start_line(input: &str) -> IResult<&str, StartLine> {
    alt((
        map(terminated(parse_request_line, parse_crlf), |v| {
            StartLine::Request {
                method: v.0,
                uri: v.1,
                version: v.2,
            }
        }),
        map(terminated(parse_status_line, parse_crlf), |v| {
            StartLine::Response {
                version: v.0,
                status: v.1,
                reason: v.2,
            }
        }),
    ))(input)
}

fn parse_request_line(input: &str) -> IResult<&str, (SipMethod, String, String)> {
    let (rest, (method_raw, _, uri, _, version)) = tuple((
        take_while1(|c: char| c != ' '),
        space1,
        take_till1(|c| c == ' ' || c == '\r' || c == '\n'),
        space1,
        take_while1(|c: char| c != '\r' && c != '\n'),
    ))(input)?;

    let method = parse_method(method_raw);
    Ok((rest, (method, uri.to_string(), version.to_string())))
}

fn parse_status_line(input: &str) -> IResult<&str, (String, u16, String)> {
    let (rest, (_, _, code, _, reason)) = tuple((
        tag("SIP/2.0"),
        space1,
        map_res(digit1, |d: &str| d.parse::<u16>()),
        space1,
        not_line_ending,
    ))(input)?;
    Ok((
        rest,
        ("SIP/2.0".to_string(), code, reason.trim().to_string()),
    ))
}

fn parse_headers_block(input: &str) -> Result<Vec<SipHeader>> {
    let mut headers = Vec::new();
    let mut current = String::new();

    for raw_line in input.lines() {
        let line = raw_line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        if line.starts_with(' ') || line.starts_with('\t') {
            // header folding: append to previous header value
            if current.is_empty() {
                continue;
            }
            current.push(' ');
            current.push_str(line.trim_start());
            continue;
        }

        if !current.is_empty() {
            headers.push(parse_header_line_nom(&current)?);
        }
        current.clear();
        current.push_str(line);
    }

    if !current.is_empty() {
        headers.push(parse_header_line_nom(&current)?);
    }

    Ok(headers)
}

fn parse_header_line_nom(input: &str) -> Result<SipHeader> {
    type NomErr<'a> = nom::Err<nom::error::Error<&'a str>>;

    let res: IResult<&str, (&str, &str, &str, &str), nom::error::Error<&str>> = tuple((
        take_until(":"),
        tag(":"),
        nom::character::complete::space0,
        not_line_ending,
    ))(input);

    let (_, (name, _, _, value)) = res
        .map_err(|e: NomErr| anyhow!("invalid SIP header line {:?}: {:?}", input, e))?;

    Ok(SipHeader {
        name: name.trim().to_string(),
        value: value.trim().to_string(),
    })
}

fn parse_crlf(input: &str) -> IResult<&str, &str> {
    alt((tag("\r\n"), tag("\n")))(input)
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
