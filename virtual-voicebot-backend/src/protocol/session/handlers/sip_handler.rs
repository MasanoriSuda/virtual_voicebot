use chrono::{DateTime, FixedOffset, Utc};

use super::super::SessionCoordinator;
use crate::protocol::session::types::{Sdp, SessionOut};
use crate::protocol::sip::{parse_name_addr, parse_uri};
use crate::shared::ports::app::{AppEvent, EndReason};

/// Extracts a candidate user identifier or telephone number from a SIP `To`/`From`-style header string.
///
/// Attempts to parse a name-addr or raw URI and returns the URI user component when present;
/// for `tel:` URIs it returns the telephone host (digits) if non-empty. Returns `None` when no
/// suitable user/telephone could be parsed.
pub(crate) fn extract_user_from_to(value: &str) -> Option<String> {
    if let Ok(name_addr) = parse_name_addr(value) {
        if name_addr.uri.scheme.eq_ignore_ascii_case("tel") {
            if !name_addr.uri.host.trim().is_empty() {
                return Some(name_addr.uri.host);
            }
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
    if uri.scheme.eq_ignore_ascii_case("tel") {
        if !uri.host.trim().is_empty() {
            return Some(uri.host);
        }
    }
    uri.user
}

/// Extracts the notification identifier from a SIP name-addr string.
pub(crate) fn extract_notify_from(value: &str) -> String {
    extract_user_from_to(value).unwrap_or_default()
}

/// Get the current time in Japan Standard Time (UTC+9).
pub(crate) fn now_jst() -> DateTime<FixedOffset> {
    let offset = FixedOffset::east_opt(9 * 3600).unwrap();
    Utc::now().with_timezone(&offset)
}

impl SessionCoordinator {
    pub(crate) fn build_answer_pcmu8k(&self) -> Sdp {
        Sdp::pcmu(self.media_cfg.local_ip.clone(), self.media_cfg.local_port)
    }

    pub(crate) fn send_call_ended(&self, reason: EndReason) {
        let from = extract_notify_from(self.from_uri.as_str());
        let timestamp = now_jst();
        let duration_sec = self.started_at.map(|started| started.elapsed().as_secs());
        if let Err(err) = self.app_tx.try_send(AppEvent::CallEnded {
            call_id: self.call_id.clone(),
            from,
            reason,
            duration_sec,
            timestamp,
        }) {
            log::warn!(
                "[session {}] dropped CallEnded event (channel full): {:?}",
                self.call_id,
                err
            );
        }
    }

    pub(crate) fn send_bye_to_a_leg(&self) {
        if let Err(err) = self
            .session_out_tx
            .try_send((self.call_id.clone(), SessionOut::SipSendBye))
        {
            log::warn!(
                "[session {}] dropped SipSendBye (channel full): {:?}",
                self.call_id,
                err
            );
        }
    }
}
