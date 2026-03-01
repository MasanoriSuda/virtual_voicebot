use crate::protocol::sip::{parse_name_addr, parse_uri};

/// Extracts a candidate user identifier or telephone number from a SIP `To`/`From`-style header string.
///
/// Attempts to parse a name-addr or raw URI and returns the URI user component when present;
/// for `tel:` URIs it returns the telephone host (digits) if non-empty. Returns `None` when no
/// suitable user/telephone could be parsed.
pub(crate) fn extract_user_from_to(value: &str) -> Option<String> {
    if let Ok(name_addr) = parse_name_addr(value) {
        if name_addr.uri.scheme.eq_ignore_ascii_case("tel") && !name_addr.uri.host.trim().is_empty()
        {
            return Some(name_addr.uri.host);
        }
        if let Some(user) = name_addr.uri.user {
            return Some(user);
        }
    }
    let trimmed = value.trim();
    let addr = if let Some(start) = trimmed.find('<') {
        if let Some(end) = trimmed[start + 1..].find('>') {
            &trimmed[start + 1..start + 1 + end]
        } else {
            trimmed
        }
    } else {
        trimmed
    };
    let addr = addr.split(';').next().unwrap_or(addr).trim();
    let uri = parse_uri(addr).ok()?;
    if uri.scheme.eq_ignore_ascii_case("tel") && !uri.host.trim().is_empty() {
        return Some(uri.host);
    }
    uri.user
}
