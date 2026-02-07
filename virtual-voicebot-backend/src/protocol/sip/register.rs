use rand::Rng;
use std::time::{Duration, Instant};

use crate::protocol::sip::auth::{build_authorization_header, parse_digest_challenge};
use crate::protocol::sip::auth_cache::{self, DigestAuthChallenge, DigestAuthHeader};
use crate::protocol::sip::builder::build_register_request;
use crate::protocol::sip::{SipHeader, SipRequest, SipResponse};
use crate::protocol::transport::TransportPeer;
use crate::shared::config::{RegistrarConfig, RegistrarTransport};

pub struct RegisterClient {
    cfg: RegistrarConfig,
    call_id: String,
    from_tag: String,
    cseq: u32,
    registered: bool,
    auth_nc: u32,
    auth_attempts: u32,
    last_nonce: Option<String>,
    pending_request: Option<SipRequest>,
    current_expires: u32,
    expires_at: Option<Instant>,
    next_refresh_at: Option<Instant>,
    next_retry_at: Option<Instant>,
    retry_delay: Duration,
    expired_notified: bool,
}

impl RegisterClient {
    pub fn new(cfg: RegistrarConfig) -> Self {
        let expires = cfg.expires;
        Self {
            cfg,
            call_id: generate_call_id(),
            from_tag: generate_tag(),
            cseq: 1,
            registered: false,
            auth_nc: 0,
            auth_attempts: 0,
            last_nonce: None,
            pending_request: None,
            current_expires: expires,
            expires_at: None,
            next_refresh_at: None,
            next_retry_at: None,
            retry_delay: Duration::from_secs(RETRY_BASE_SECS),
            expired_notified: false,
        }
    }

    pub fn transport_peer(&self) -> Option<TransportPeer> {
        match self.cfg.transport {
            RegistrarTransport::Udp => Some(TransportPeer::Udp(self.cfg.addr)),
            _ => None,
        }
    }

    pub fn build_request(&self) -> SipRequest {
        self.build_request_with_expires(self.current_expires, self.cseq)
    }

    pub fn build_unregister_request(&mut self) -> SipRequest {
        self.cseq = self.cseq.saturating_add(1);
        self.build_request_with_expires(0, self.cseq)
    }

    fn build_next_request(&mut self) -> SipRequest {
        self.cseq = self.cseq.saturating_add(1);
        self.build_request_with_expires(self.current_expires, self.cseq)
    }

    /// Constructs a SIP REGISTER request for this client using the provided `expires` and `cseq`.
    ///
    /// The returned `SipRequest` contains standard REGISTER headers (Via, From, To, Contact, Call-ID, CSeq)
    /// derived from the client's configuration and the supplied values.
    ///
    /// # Examples
    ///
    /// ```
    /// // Given a configured RegisterClient `client`:
    /// let req = client.build_request_with_expires(3600, client.cseq);
    /// // request URI should target the registrar domain configured on the client
    /// assert!(req.request_uri.contains(&client.cfg.domain));
    /// ```
    fn build_request_with_expires(&self, expires: u32, cseq: u32) -> SipRequest {
        let scheme = self.cfg.transport.scheme();
        let via = format!(
            "SIP/2.0/{} {}:{};branch={}",
            self.cfg.transport.via_protocol(),
            self.cfg.contact_host,
            self.cfg.contact_port,
            generate_branch()
        );
        let from = format!(
            "<{}:{}@{}>;tag={}",
            scheme, self.cfg.user, self.cfg.domain, self.from_tag
        );
        let to = format!("<{}:{}@{}>", scheme, self.cfg.user, self.cfg.domain);
        let contact = format!(
            "<{}:{}@{}:{}>",
            scheme, self.cfg.user, self.cfg.contact_host, self.cfg.contact_port
        );
        let request_uri = format!("{}:{}", scheme, self.cfg.domain);
        build_register_request(
            &request_uri,
            via,
            from,
            to,
            self.call_id.clone(),
            cseq,
            contact,
            expires,
        )
    }

    pub fn take_pending_request(&mut self) -> Option<SipRequest> {
        self.pending_request.take()
    }

    /// Process a SIP response intended for this registrar client and update internal registration state.
    ///
    /// Validates that the response comes from the expected transport peer (if applicable), matches the
    /// client's Call-ID and CSeq for a REGISTER request, and then handles the response status:
    /// - 200: parse the Expires header (falling back to the configured expiry) and record a successful registration.
    /// - 401 / 407: attempt to prepare an authenticated REGISTER; if a challenge header is missing or auth preparation fails, schedule a retry.
    /// - other status codes: mark as unregistered and schedule a retry.
    ///
    /// Returns `true` if the response matched this client (peer, Call-ID, CSeq and method) and was processed, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Assume `client` is a RegisterClient, `resp` a SipResponse and `peer` the TransportPeer source.
    /// // The call returns `true` when `resp` was accepted and handled by `client`.
    /// // let handled = client.handle_response(&resp, peer);
    /// ```
    pub fn handle_response(&mut self, resp: &SipResponse, peer: TransportPeer) -> bool {
        if let Some(expected_peer) = self.transport_peer() {
            if peer != expected_peer {
                return false;
            }
        }
        let Some(call_id) = header_value(resp, "Call-ID") else {
            return false;
        };
        if call_id != self.call_id {
            return false;
        }
        let Some(cseq) = header_value(resp, "CSeq") else {
            return false;
        };
        let Some((cseq_num, method)) = parse_cseq(cseq) else {
            return false;
        };
        if cseq_num != self.cseq || !method.eq_ignore_ascii_case("REGISTER") {
            return false;
        }
        match resp.status_code {
            200 => {
                let expires = header_value(resp, "Expires")
                    .and_then(|value| value.parse::<u32>().ok())
                    .unwrap_or(self.cfg.expires);
                self.on_register_success(expires);
            }
            401 | 407 => {
                let (challenge_header, auth_header) = if resp.status_code == 401 {
                    ("WWW-Authenticate", "Authorization")
                } else {
                    ("Proxy-Authenticate", "Proxy-Authorization")
                };
                let Some(challenge_value) = header_value(resp, challenge_header) else {
                    log::warn!(
                        "[sip register] missing {} call_id={}",
                        challenge_header,
                        self.call_id
                    );
                    self.schedule_retry();
                    return true;
                };
                if let Some(req) = self.prepare_authenticated_request(challenge_value, auth_header)
                {
                    self.pending_request = Some(req);
                } else {
                    self.schedule_retry();
                }
            }
            _ => {
                log::warn!(
                    "[sip register] register failed status={} call_id={}",
                    resp.status_code,
                    self.call_id
                );
                self.registered = false;
                self.schedule_retry();
            }
        }
        true
    }

    pub fn call_id(&self) -> &str {
        &self.call_id
    }

    pub fn cseq(&self) -> u32 {
        self.cseq
    }

    pub fn registered(&self) -> bool {
        self.registered
    }

    pub fn transport(&self) -> RegistrarTransport {
        self.cfg.transport
    }

    pub fn target_addr(&self) -> std::net::SocketAddr {
        self.cfg.addr
    }

    pub fn next_timer_at(&self) -> Option<Instant> {
        let mut next = None;
        for candidate in [self.next_refresh_at, self.next_retry_at] {
            if let Some(value) = candidate {
                next = match next {
                    Some(existing) => Some(std::cmp::min(existing, value)),
                    None => Some(value),
                };
            }
        }
        if let Some(expires_at) = self.expires_at {
            if !self.expired_notified {
                next = match next {
                    Some(existing) => Some(std::cmp::min(existing, expires_at)),
                    None => Some(expires_at),
                };
            }
        }
        next
    }

    pub fn pop_due_request(&mut self, now: Instant) -> Option<SipRequest> {
        if self.next_retry_at.map_or(false, |t| now >= t) {
            self.next_retry_at = None;
            return Some(self.build_next_request());
        }
        if self.next_refresh_at.map_or(false, |t| now >= t) {
            self.next_refresh_at = None;
            return Some(self.build_next_request());
        }
        None
    }

    pub fn check_expired(&mut self, now: Instant) {
        if let Some(expires_at) = self.expires_at {
            if now >= expires_at && !self.expired_notified {
                self.expired_notified = true;
                self.registered = false;
                log::warn!("[sip register] expired call_id={}", self.call_id);
                self.retry_delay = Duration::from_secs(RETRY_BASE_SECS);
                self.schedule_retry();
            }
        }
    }

    fn build_request_with_auth(&self, header_name: &str, header_value: String) -> SipRequest {
        let mut req = self.build_request_with_expires(self.current_expires, self.cseq);
        req.headers.push(SipHeader::new(header_name, header_value));
        req
    }

    fn prepare_authenticated_request(
        &mut self,
        challenge_value: &str,
        auth_header: &str,
    ) -> Option<SipRequest> {
        let Some(password) = self.cfg.auth_password.as_deref() else {
            log::warn!(
                "[sip register] auth password missing call_id={}",
                self.call_id
            );
            return None;
        };
        let Some(challenge) = parse_digest_challenge(challenge_value) else {
            log::warn!(
                "[sip register] unsupported auth challenge call_id={}",
                self.call_id
            );
            return None;
        };
        if let Some(header_kind) = DigestAuthHeader::from_name(auth_header) {
            auth_cache::store(DigestAuthChallenge {
                header: header_kind,
                challenge: challenge.clone(),
            });
        }

        if self.last_nonce.as_deref() != Some(challenge.nonce.as_str()) {
            self.last_nonce = Some(challenge.nonce.clone());
            self.auth_nc = 0;
            self.auth_attempts = 0;
        }
        if self.auth_attempts >= 1 {
            log::warn!(
                "[sip register] auth retry limit reached call_id={}",
                self.call_id
            );
            return None;
        }

        self.cseq = self.cseq.saturating_add(1);
        self.auth_nc = self.auth_nc.saturating_add(1);
        let request_uri = format!("{}:{}", self.cfg.transport.scheme(), self.cfg.domain);
        let Some(header_value) = build_authorization_header(
            &self.cfg.auth_username,
            password,
            "REGISTER",
            &request_uri,
            &challenge,
            self.auth_nc,
        ) else {
            log::warn!(
                "[sip register] unsupported auth parameters call_id={}",
                self.call_id
            );
            return None;
        };
        self.auth_attempts = self.auth_attempts.saturating_add(1);
        Some(self.build_request_with_auth(auth_header, header_value))
    }

    fn on_register_success(&mut self, expires: u32) {
        self.registered = true;
        self.auth_attempts = 0;
        self.auth_nc = 0;
        self.schedule_refresh(expires);
        log::info!(
            "[sip register] registered call_id={} expires={}",
            self.call_id,
            expires
        );
    }

    fn schedule_refresh(&mut self, expires: u32) {
        let now = Instant::now();
        let expires = std::cmp::max(1, expires);
        let refresh_after = ((expires as f32) * REFRESH_RATIO).max(1.0) as u64;
        self.current_expires = expires;
        self.expires_at = Some(now + Duration::from_secs(expires as u64));
        self.next_refresh_at = Some(now + Duration::from_secs(refresh_after));
        self.next_retry_at = None;
        self.retry_delay = Duration::from_secs(RETRY_BASE_SECS);
        self.expired_notified = false;
    }

    fn schedule_retry(&mut self) {
        let now = Instant::now();
        let delay = self.retry_delay;
        self.next_retry_at = Some(now + delay);
        self.retry_delay = std::cmp::min(
            self.retry_delay.saturating_mul(2),
            Duration::from_secs(RETRY_MAX_SECS),
        );
        log::warn!(
            "[sip register] retry scheduled in {:?} call_id={}",
            delay,
            self.call_id
        );
    }
}

fn header_value<'a>(resp: &'a SipResponse, name: &str) -> Option<&'a str> {
    resp.headers
        .iter()
        .find(|h| h.name.eq_ignore_ascii_case(name))
        .map(|h| h.value.as_str())
}

fn parse_cseq(raw: &str) -> Option<(u32, &str)> {
    let mut parts = raw.split_whitespace();
    let num = parts.next()?.parse().ok()?;
    let method = parts.next()?;
    Some((num, method))
}

fn generate_call_id() -> String {
    let mut rng = rand::thread_rng();
    format!("reg-{}", rng.gen::<u64>())
}

fn generate_tag() -> String {
    let mut rng = rand::thread_rng();
    format!("t{}", rng.gen::<u64>())
}

fn generate_branch() -> String {
    let mut rng = rand::thread_rng();
    format!("z9hG4bK-{}", rng.gen::<u64>())
}

const REFRESH_RATIO: f32 = 0.8;
const RETRY_BASE_SECS: u64 = 5;
const RETRY_MAX_SECS: u64 = 60;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::sip::builder::SipResponseBuilder;

    fn sample_config() -> RegistrarConfig {
        RegistrarConfig {
            addr: "127.0.0.1:5060".parse().unwrap(),
            domain: "example.com".to_string(),
            user: "alice".to_string(),
            contact_host: "127.0.0.1".to_string(),
            contact_port: 5060,
            expires: 3600,
            transport: RegistrarTransport::Udp,
            auth_username: "alice".to_string(),
            auth_password: Some("secret".to_string()),
        }
    }

    #[test]
    fn build_request_sets_expires_and_contact() {
        let client = RegisterClient::new(sample_config());
        let req = client.build_request();
        let expires = req
            .headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case("Expires"))
            .map(|h| h.value.as_str())
            .unwrap();
        assert_eq!(expires, "3600");
        let contact = req
            .headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case("Contact"))
            .map(|h| h.value.as_str())
            .unwrap();
        assert!(contact.starts_with("<sip:"));
        assert!(contact.contains("@127.0.0.1:5060"));
    }

    #[test]
    fn handle_response_marks_registered() {
        let mut client = RegisterClient::new(sample_config());
        let resp = SipResponseBuilder::new(200, "OK")
            .header("Via", "SIP/2.0/UDP 127.0.0.1:5060")
            .header("From", "<sip:alice@example.com>;tag=alice")
            .header("To", "<sip:alice@example.com>")
            .header("Call-ID", client.call_id().to_string())
            .header("CSeq", format!("{} REGISTER", client.cseq()))
            .build();
        let handled = client.handle_response(&resp, TransportPeer::Udp(client.target_addr()));
        assert!(handled);
        assert!(client.registered());
    }

    #[test]
    fn handle_response_with_auth_challenge_schedules_retry() {
        let mut client = RegisterClient::new(sample_config());
        let resp = SipResponseBuilder::new(401, "Unauthorized")
            .header("Via", "SIP/2.0/UDP 127.0.0.1:5060")
            .header("From", "<sip:alice@example.com>;tag=alice")
            .header("To", "<sip:alice@example.com>")
            .header("Call-ID", client.call_id().to_string())
            .header("CSeq", format!("{} REGISTER", client.cseq()))
            .header(
                "WWW-Authenticate",
                r#"Digest realm="example.com", nonce="abc", qop="auth""#,
            )
            .build();
        let handled = client.handle_response(&resp, TransportPeer::Udp(client.target_addr()));
        assert!(handled);
        let req = client.take_pending_request().expect("pending request");
        let auth = req
            .headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case("Authorization"))
            .expect("authorization header");
        assert!(auth.value.starts_with("Digest "));
        assert!(auth.value.contains("username=\"alice\""));
        assert!(auth.value.contains("realm=\"example.com\""));
    }

    #[test]
    fn refresh_due_builds_request_with_new_cseq() {
        let mut client = RegisterClient::new(sample_config());
        client.next_refresh_at = Some(Instant::now() - Duration::from_secs(1));
        let req = client
            .pop_due_request(Instant::now())
            .expect("refresh request");
        let cseq = req
            .headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case("CSeq"))
            .map(|h| h.value.as_str())
            .unwrap();
        assert!(cseq.starts_with("2 "));
    }

    #[test]
    fn check_expired_marks_unregistered_and_schedules_retry() {
        let mut client = RegisterClient::new(sample_config());
        client.registered = true;
        client.expires_at = Some(Instant::now() - Duration::from_secs(1));
        client.check_expired(Instant::now());
        assert!(!client.registered);
        assert!(client.next_retry_at.is_some());
    }
}
