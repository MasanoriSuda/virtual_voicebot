use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::OnceLock;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct Config {
    pub sip_bind_ip: String,
    pub sip_port: u16,
    pub rtp_port: u16,
    pub local_ip: String,
    pub advertised_ip: String,
    pub advertised_rtp_port: u16,
    pub recording_http_addr: String,
    pub ingest_call_url: Option<String>,
    pub recording_base_url: Option<String>,
}

#[derive(Clone, Debug)]
pub struct AppRuntimeConfig {
    pub phone_lookup_enabled: bool,
}

impl AppRuntimeConfig {
    pub fn from_env() -> Self {
        Self {
            phone_lookup_enabled: env_bool("PHONE_LOOKUP_ENABLED", false),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SessionRuntimeConfig {
    pub vad: VadConfig,
    pub ring_duration: Duration,
    pub ivr_timeout: Duration,
    pub transfer_target_uri: String,
    pub transfer_timeout: Duration,
    pub registrar: Option<RegistrarConfig>,
    pub outbound: OutboundConfig,
    pub advertised_ip: String,
    pub sip_port: u16,
}

impl SessionRuntimeConfig {
    pub fn from_env(base: &Config) -> Self {
        let registrar = RegistrarConfig::from_env();
        let outbound = OutboundConfig::from_env_with(registrar.as_ref());
        Self {
            vad: VadConfig::from_env(),
            ring_duration: ring_duration_from_env(),
            ivr_timeout: Duration::from_secs(env_u64("IVR_TIMEOUT_SEC", 10)),
            transfer_target_uri: transfer_target_uri_from_env(),
            transfer_timeout: Duration::from_secs(env_u64("TRANSFER_TIMEOUT_SEC", 30)),
            registrar,
            outbound,
            advertised_ip: base.advertised_ip.clone(),
            sip_port: base.sip_port,
        }
    }
}

impl Config {
    /// Create a Config populated from environment variables, falling back to sensible defaults when keys are absent.
    ///
    /// Reads (and defaults) the following environment variables:
    /// - SIP_BIND_IP (default "0.0.0.0")
    /// - SIP_PORT (default 5060)
    /// - RTP_PORT (default 10000)
    /// - LOCAL_IP (default "0.0.0.0")
    /// - ADVERTISED_IP (defaults to LOCAL_IP)
    /// - ADVERTISED_RTP_PORT (defaults to RTP_PORT)
    /// - RECORDING_HTTP_ADDR (default "0.0.0.0:18080")
    /// - INGEST_CALL_URL (optional)
    /// - RECORDING_BASE_URL (optional; if absent, derived from RECORDING_HTTP_ADDR and ADVERTISED_IP)
    ///
    /// # Examples
    ///
    /// ```
    /// use virtual_voicebot_backend::config::Config;
    ///
    /// let cfg = Config::from_env().unwrap();
    /// // Access common fields
    /// let _sip_port = cfg.sip_port;
    /// ```
    pub fn from_env() -> Result<Self> {
        let sip_bind_ip = std::env::var("SIP_BIND_IP").unwrap_or_else(|_| "0.0.0.0".to_string());
        let sip_port = std::env::var("SIP_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5060);
        let rtp_port = std::env::var("RTP_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10000);
        let local_ip = std::env::var("LOCAL_IP").unwrap_or_else(|_| "0.0.0.0".to_string());
        let advertised_ip = std::env::var("ADVERTISED_IP").unwrap_or_else(|_| local_ip.clone());
        let advertised_rtp_port = std::env::var("ADVERTISED_RTP_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(rtp_port);
        let recording_http_addr =
            std::env::var("RECORDING_HTTP_ADDR").unwrap_or_else(|_| "0.0.0.0:18080".to_string());
        let ingest_call_url = if std::env::var("INGEST_CALL_URL").is_ok() {
            log::warn!("[config] INGEST_CALL_URL is deprecated and ignored (serversync only mode)");
            None
        } else {
            None
        };
        let recording_base_url = std::env::var("RECORDING_BASE_URL").ok().or_else(|| {
            if let Some(port) = recording_http_addr.strip_prefix("0.0.0.0:") {
                Some(format!("http://{}:{}", advertised_ip, port))
            } else {
                Some(format!("http://{}", recording_http_addr))
            }
        });

        Ok(Self {
            sip_bind_ip,
            sip_port,
            rtp_port,
            local_ip,
            advertised_ip,
            advertised_rtp_port,
            recording_http_addr,
            ingest_call_url,
            recording_base_url,
        })
    }
}

#[derive(Clone, Debug)]
pub struct TlsSettings {
    pub bind_ip: String,
    pub port: u16,
    pub cert_path: String,
    pub key_path: String,
    pub ca_path: Option<String>,
}

impl TlsSettings {
    pub fn from_env() -> Option<Self> {
        let cert_path = env_non_empty("TLS_CERT_PATH")?;
        let key_path = env_non_empty("TLS_KEY_PATH")?;
        let bind_ip = std::env::var("SIP_BIND_IP").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = env_u16("SIP_TLS_PORT", 5061);
        let ca_path = env_non_empty("TLS_CA_PATH");
        Some(Self {
            bind_ip,
            port,
            cert_path,
            key_path,
            ca_path,
        })
    }
}

static TLS_SETTINGS: OnceLock<Option<TlsSettings>> = OnceLock::new();

pub fn tls_settings() -> Option<&'static TlsSettings> {
    TLS_SETTINGS.get_or_init(TlsSettings::from_env).as_ref()
}

#[derive(Clone, Debug)]
pub struct VadConfig {
    pub rms_threshold: u32,
    pub start_silence_ms: u64,
    pub end_silence_ms: u64,
    pub min_speech_ms: u64,
    pub max_speech_ms: u64,
}

impl VadConfig {
    fn from_env() -> Self {
        Self {
            rms_threshold: env_u32("VAD_ENERGY_THRESHOLD", 500),
            start_silence_ms: env_u64("VAD_START_SILENCE_MS", 800),
            end_silence_ms: env_u64("VAD_END_SILENCE_MS", 800),
            min_speech_ms: env_u64("VAD_MIN_SPEECH_MS", 300),
            max_speech_ms: env_u64("VAD_MAX_SPEECH_MS", 30_000),
        }
    }
}

static VAD_CONFIG: OnceLock<VadConfig> = OnceLock::new();

pub fn vad_config() -> &'static VadConfig {
    VAD_CONFIG.get_or_init(VadConfig::from_env)
}

#[derive(Clone, Debug)]
pub struct SessionConfig {
    pub default_expires: Option<Duration>,
    pub min_se: u64,
}

impl SessionConfig {
    fn from_env() -> Self {
        let timeout = env_u64("SESSION_TIMEOUT_SEC", 1800);
        let default_expires = if timeout == 0 {
            None
        } else {
            Some(Duration::from_secs(timeout))
        };
        let min_se = env_u64("SESSION_MIN_SE", 90);
        Self {
            default_expires,
            min_se,
        }
    }
}

static SESSION_CONFIG: OnceLock<SessionConfig> = OnceLock::new();

pub fn session_config() -> &'static SessionConfig {
    SESSION_CONFIG.get_or_init(SessionConfig::from_env)
}

static RING_DURATION: OnceLock<Duration> = OnceLock::new();

pub fn ring_duration() -> Duration {
    *RING_DURATION.get_or_init(ring_duration_from_env)
}

fn ring_duration_from_env() -> Duration {
    const DEFAULT_MS: u64 = 3000;
    const MAX_MS: u64 = 10_000;
    let raw = std::env::var("RING_DURATION_MS").ok();
    let mut ms = match raw.as_deref() {
        Some(value) => match value.trim().parse::<u64>() {
            Ok(v) => v,
            Err(_) => {
                log::warn!(
                    "[config] invalid RING_DURATION_MS={}, fallback to {}",
                    value,
                    DEFAULT_MS
                );
                DEFAULT_MS
            }
        },
        None => DEFAULT_MS,
    };
    if ms > MAX_MS {
        log::warn!(
            "[config] RING_DURATION_MS={} exceeds max {}, clamped",
            ms,
            MAX_MS
        );
        ms = MAX_MS;
    }
    Duration::from_millis(ms)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistrarTransport {
    Udp,
    Tcp,
    Tls,
}

impl RegistrarTransport {
    fn from_env(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "udp" => Some(Self::Udp),
            "tcp" => Some(Self::Tcp),
            "tls" => Some(Self::Tls),
            _ => None,
        }
    }

    pub fn via_protocol(self) -> &'static str {
        match self {
            Self::Udp => "UDP",
            Self::Tcp => "TCP",
            Self::Tls => "TLS",
        }
    }

    pub fn scheme(self) -> &'static str {
        match self {
            Self::Tls => "sips",
            _ => "sip",
        }
    }

    fn default_port(self) -> u16 {
        match self {
            Self::Tls => 5061,
            _ => 5060,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RegistrarConfig {
    pub addr: SocketAddr,
    pub domain: String,
    pub user: String,
    pub contact_host: String,
    pub contact_port: u16,
    pub expires: u32,
    pub transport: RegistrarTransport,
    pub auth_username: String,
    pub auth_password: Option<String>,
}

impl RegistrarConfig {
    /// Builds a `RegistrarConfig` from environment variables, returning `None` if required values are missing or the registrar address cannot be resolved.
    ///
    /// Required environment variables:
    /// - `REGISTRAR_HOST` (host or IP of the registrar)
    /// - `REGISTER_USER` (username to register as)
    ///
    /// Optional environment variables influence transport, ports, contact host/port, authentication, and expiration; sensible defaults and fallbacks are applied when they are omitted.
    ///
    /// # Returns
    ///
    /// `Some(RegistrarConfig)` when the required environment variables are present and the registrar address resolves, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// std::env::set_var("REGISTRAR_HOST", "127.0.0.1");
    /// std::env::set_var("REGISTER_USER", "alice");
    /// // Optional: set transport/port/auth vars as needed
    ///
    /// if let Some(cfg) = RegistrarConfig::from_env() {
    ///     assert_eq!(cfg.user, "alice");
    ///     // cfg.addr is a resolved SocketAddr for 127.0.0.1 with the chosen port
    /// } else {
    ///     panic!("expected RegistrarConfig to be constructed from environment");
    /// }
    /// ```
    fn from_env() -> Option<Self> {
        let registrar_host = env_non_empty("REGISTRAR_HOST")?;
        let transport = env_non_empty("REGISTRAR_TRANSPORT")
            .and_then(|value| RegistrarTransport::from_env(&value))
            .unwrap_or(RegistrarTransport::Udp);
        let registrar_port = env_u16("REGISTRAR_PORT", transport.default_port());
        let addr = resolve_socket_addr(&registrar_host, registrar_port)?;
        let user = env_non_empty("REGISTER_USER")?;
        let domain = env_non_empty("REGISTER_DOMAIN").unwrap_or_else(|| registrar_host.clone());
        let expires = env_u32("REGISTER_EXPIRES", 3600);
        let contact_host = env_non_empty("REGISTER_CONTACT_HOST")
            .or_else(|| env_non_empty("ADVERTISED_IP"))
            .or_else(|| env_non_empty("LOCAL_IP"))
            .unwrap_or_else(|| "0.0.0.0".to_string());
        let contact_port = std::env::var("REGISTER_CONTACT_PORT")
            .ok()
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or_else(|| match transport {
                RegistrarTransport::Tls => env_u16("SIP_TLS_PORT", 5061),
                _ => env_u16("SIP_PORT", 5060),
            });
        let auth_username = env_non_empty("REGISTER_AUTH_USER").unwrap_or_else(|| user.clone());
        let auth_password = env_non_empty("REGISTER_AUTH_PASSWORD");

        Some(Self {
            addr,
            domain,
            user,
            contact_host,
            contact_port,
            expires,
            transport,
            auth_username,
            auth_password,
        })
    }
}

static REGISTRAR_CONFIG: OnceLock<Option<RegistrarConfig>> = OnceLock::new();

/// Accesses the global registrar configuration initialized from environment variables.
///
/// This returns a cached, static reference to the registrar configuration constructed by
/// `RegistrarConfig::from_env`. The configuration is initialized on first use and reused
/// thereafter.
///
/// # Returns
///
/// `Some(&RegistrarConfig)` when a valid registrar configuration can be constructed from
/// environment variables (for example, `REGISTRAR_HOST` and `REGISTER_USER` are present and
/// the host resolves); `None` when required environment values are missing or resolution fails.
///
/// # Examples
///
/// ```
/// use virtual_voicebot_backend::config::registrar_config;
///
/// if let Some(cfg) = registrar_config() {
///     // Use cfg.addr, cfg.domain, cfg.user, etc.
///     println!("Registering {} at {}", cfg.user, cfg.addr);
/// } else {
///     eprintln!("No registrar configured");
/// }
/// ```
pub fn registrar_config() -> Option<&'static RegistrarConfig> {
    REGISTRAR_CONFIG
        .get_or_init(RegistrarConfig::from_env)
        .as_ref()
}

#[derive(Clone, Debug)]
pub struct OutboundConfig {
    pub enabled: bool,
    pub domain: String,
    pub default_number: Option<String>,
    pub dial_plan: HashMap<String, String>,
}

impl OutboundConfig {
    fn from_env_with(registrar: Option<&RegistrarConfig>) -> Self {
        let enabled = env_bool("OUTBOUND_ENABLED", false);
        let default_number = env_non_empty("OUTBOUND_DEFAULT_NUMBER");
        let dial_plan = load_dial_plan();
        let domain = env_non_empty("OUTBOUND_DOMAIN")
            .or_else(|| registrar.map(|cfg| cfg.domain.clone()))
            .unwrap_or_default();
        if enabled && domain.is_empty() {
            log::warn!("[config] OUTBOUND_ENABLED is true but no outbound domain configured");
        }
        Self {
            enabled,
            domain,
            default_number,
            dial_plan,
        }
    }

    fn from_env() -> Self {
        let registrar = registrar_config();
        Self::from_env_with(registrar)
    }

    pub fn resolve_number(&self, user: &str) -> Option<String> {
        if let Some(number) = self.dial_plan.get(user) {
            return Some(number.clone());
        }
        if is_phone_number(user) {
            return Some(user.to_string());
        }
        self.default_number.clone()
    }
}

static OUTBOUND_CONFIG: OnceLock<OutboundConfig> = OnceLock::new();

pub fn outbound_config() -> &'static OutboundConfig {
    OUTBOUND_CONFIG.get_or_init(OutboundConfig::from_env)
}

#[derive(Clone, Debug)]
pub struct PhoneLookupConfig {
    pub enabled: bool,
    pub database_url: Option<String>,
}

impl PhoneLookupConfig {
    fn from_env() -> Self {
        let enabled = env_bool("PHONE_LOOKUP_ENABLED", false);
        let database_url = env_non_empty("DATABASE_URL");
        if enabled && database_url.is_none() {
            log::warn!("[config] PHONE_LOOKUP_ENABLED is true but DATABASE_URL is missing");
        }
        Self {
            enabled,
            database_url,
        }
    }
}

static PHONE_LOOKUP_CONFIG: OnceLock<PhoneLookupConfig> = OnceLock::new();

pub fn phone_lookup_config() -> &'static PhoneLookupConfig {
    PHONE_LOOKUP_CONFIG.get_or_init(PhoneLookupConfig::from_env)
}

pub fn phone_lookup_enabled() -> bool {
    phone_lookup_config().enabled
}

/// Returns the configured PostgreSQL DSN for phone lookup, if any.
///
/// # Examples
///
/// ```
/// use virtual_voicebot_backend::config::database_url;
///
/// let _dsn = database_url();
/// ```
pub fn database_url() -> Option<String> {
    phone_lookup_config().database_url.clone()
}

#[derive(Clone, Debug)]
pub struct SyncConfig {
    pub poll_interval_sec: u64,
    pub frontend_poll_interval_sec: u64,
    pub batch_size: i64,
    pub frontend_base_url: String,
    pub timeout_sec: u64,
}

impl SyncConfig {
    pub fn from_env() -> Result<Self> {
        let frontend_base_url = env_non_empty("FRONTEND_BASE_URL")
            .ok_or_else(|| anyhow!("FRONTEND_BASE_URL must be set"))?;
        let poll_interval_sec = env_u64("SYNC_POLL_INTERVAL_SEC", 300);
        let frontend_poll_interval_sec = env_u64("FRONTEND_SYNC_INTERVAL_SEC", 30);
        let batch_size = env_i64("SYNC_BATCH_SIZE", 100);
        if batch_size <= 0 {
            return Err(anyhow!("SYNC_BATCH_SIZE must be greater than 0"));
        }
        let timeout_sec = env_u64("SYNC_TIMEOUT_SEC", 30);
        Ok(Self {
            poll_interval_sec,
            frontend_poll_interval_sec,
            batch_size,
            frontend_base_url,
            timeout_sec,
        })
    }
}

#[derive(Clone, Debug)]
pub struct LineNotifyConfig {
    pub enabled: bool,
    pub channel_access_token: Option<String>,
    pub user_id: Option<String>,
}

impl LineNotifyConfig {
    /// Creates a LineNotifyConfig by reading the following environment variables:
    /// - `LINE_NOTIFY_ENABLED` (default: `true`)
    /// - `LINE_CHANNEL_ACCESS_TOKEN` (optional)
    /// - `LINE_USER_ID` (optional)
    ///
    /// If `LINE_NOTIFY_ENABLED` is true but either `LINE_CHANNEL_ACCESS_TOKEN` or
    /// `LINE_USER_ID` is missing, a runtime warning is emitted. The returned
    /// configuration's `enabled` field is true only when `LINE_NOTIFY_ENABLED` is
    /// true and both `channel_access_token` and `user_id` are present; otherwise it
    /// is false.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// std::env::set_var("LINE_NOTIFY_ENABLED", "true");
    /// std::env::set_var("LINE_CHANNEL_ACCESS_TOKEN", "tok");
    /// std::env::set_var("LINE_USER_ID", "uid");
    /// let cfg = LineNotifyConfig::from_env();
    /// assert!(cfg.enabled);
    /// assert_eq!(cfg.channel_access_token.as_deref(), Some("tok"));
    /// assert_eq!(cfg.user_id.as_deref(), Some("uid"));
    /// ```
    fn from_env() -> Self {
        let enabled = env_bool("LINE_NOTIFY_ENABLED", true);
        let channel_access_token = env_non_empty("LINE_CHANNEL_ACCESS_TOKEN");
        let user_id = env_non_empty("LINE_USER_ID");
        if enabled && (channel_access_token.is_none() || user_id.is_none()) {
            log::warn!(
                "[config] LINE_NOTIFY_ENABLED is true but LINE_CHANNEL_ACCESS_TOKEN/LINE_USER_ID is missing"
            );
        }
        let effective_enabled = enabled && channel_access_token.is_some() && user_id.is_some();
        Self {
            enabled: effective_enabled,
            channel_access_token,
            user_id,
        }
    }
}

static LINE_NOTIFY_CONFIG: OnceLock<LineNotifyConfig> = OnceLock::new();

/// Accesses the global LineNotify configuration, initializing it from environment variables on first use.
///
/// The configuration is created once and then cached for the lifetime of the process.
///
/// # Returns
///
/// A reference to the global `LineNotifyConfig`.
///
/// # Examples
///
/// ```
/// use virtual_voicebot_backend::config::line_notify_config;
///
/// let cfg = line_notify_config();
/// // Access fields, e.g. `enabled`.
/// let _ = cfg.enabled;
/// ```
pub fn line_notify_config() -> &'static LineNotifyConfig {
    LINE_NOTIFY_CONFIG.get_or_init(LineNotifyConfig::from_env)
}

#[derive(Clone, Debug)]
pub struct Timeouts {
    pub ai_http: Duration,
    pub ingest_http: Duration,
    pub recording_io: Duration,
    pub sip_tcp_idle: Duration,
}

impl Timeouts {
    fn from_env() -> Self {
        // Defaults (MVP): AI 20s, ingest 5s, recording I/O 5s, SIP TCP idle 30s.
        // Env: AI_HTTP_TIMEOUT_MS / INGEST_HTTP_TIMEOUT_MS / RECORDING_IO_TIMEOUT_MS / SIP_TCP_IDLE_TIMEOUT_MS.
        // Timeout behavior: HTTP clients return an error; recording delivery returns 504.
        Self {
            ai_http: env_duration_ms("AI_HTTP_TIMEOUT_MS", 20_000),
            ingest_http: env_duration_ms("INGEST_HTTP_TIMEOUT_MS", 5_000),
            recording_io: env_duration_ms("RECORDING_IO_TIMEOUT_MS", 5_000),
            sip_tcp_idle: env_duration_ms("SIP_TCP_IDLE_TIMEOUT_MS", 30_000),
        }
    }
}

static TIMEOUTS: OnceLock<Timeouts> = OnceLock::new();

pub fn timeouts() -> &'static Timeouts {
    TIMEOUTS.get_or_init(Timeouts::from_env)
}

#[derive(Clone, Debug)]
pub struct RtpConfig {
    pub jitter_max_reorder: u16,
    pub rtcp_interval: Duration,
}

impl RtpConfig {
    fn from_env() -> Self {
        // Defaults (MVP/NEXT): jitter reorder 5, RTCP interval 5s.
        // Env: RTP_JITTER_MAX_REORDER / RTCP_INTERVAL_MS.
        Self {
            jitter_max_reorder: env_u16("RTP_JITTER_MAX_REORDER", 30),
            rtcp_interval: env_duration_ms("RTCP_INTERVAL_MS", 5_000),
        }
    }
}

static RTP_CONFIG: OnceLock<RtpConfig> = OnceLock::new();

pub fn rtp_config() -> &'static RtpConfig {
    RTP_CONFIG.get_or_init(RtpConfig::from_env)
}

static IVR_TIMEOUT: OnceLock<Duration> = OnceLock::new();

pub fn ivr_timeout() -> Duration {
    *IVR_TIMEOUT.get_or_init(|| Duration::from_secs(env_u64("IVR_TIMEOUT_SEC", 10)))
}

static TRANSFER_TARGET_URI: OnceLock<String> = OnceLock::new();

pub fn transfer_target_uri() -> String {
    TRANSFER_TARGET_URI
        .get_or_init(transfer_target_uri_from_env)
        .clone()
}

fn transfer_target_uri_from_env() -> String {
    std::env::var("TRANSFER_TARGET_SIP_URI")
        .unwrap_or_else(|_| "sip:zoiper@192.168.1.4:8000".to_string())
}

static TRANSFER_TIMEOUT: OnceLock<Duration> = OnceLock::new();

pub fn transfer_timeout() -> Duration {
    *TRANSFER_TIMEOUT.get_or_init(|| Duration::from_secs(env_u64("TRANSFER_TIMEOUT_SEC", 30)))
}

fn env_duration_ms(key: &str, default_ms: u64) -> Duration {
    let ms = std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(default_ms);
    Duration::from_millis(ms)
}

fn env_duration_sec(key: &str, default_sec: u64) -> Duration {
    let sec = std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(default_sec);
    Duration::from_secs(sec)
}

fn env_bool(key: &str, default_value: bool) -> bool {
    std::env::var(key)
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(default_value)
}

fn env_u16(key: &str, default_value: u16) -> u16 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(default_value)
}

fn env_u32(key: &str, default_value: u32) -> u32 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(default_value)
}

fn env_u64(key: &str, default_value: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(default_value)
}

fn env_i64(key: &str, default_value: i64) -> i64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(default_value)
}

fn env_non_empty(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn load_dial_plan() -> HashMap<String, String> {
    let mut map = HashMap::new();
    for (key, value) in std::env::vars() {
        let Some(suffix) = key.strip_prefix("DIAL_") else {
            continue;
        };
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        map.insert(suffix.to_string(), trimmed.to_string());
    }
    map
}

fn is_phone_number(s: &str) -> bool {
    s.starts_with('0') && s.chars().all(|c| c.is_ascii_digit())
}

fn resolve_socket_addr(host: &str, port: u16) -> Option<SocketAddr> {
    (host, port).to_socket_addrs().ok()?.next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outbound_resolve_number_prefers_dial_plan() {
        let mut dial_plan = HashMap::new();
        dial_plan.insert("100".to_string(), "09012345678".to_string());
        let cfg = OutboundConfig {
            enabled: true,
            domain: "example.com".to_string(),
            default_number: Some("09000000000".to_string()),
            dial_plan,
        };
        assert_eq!(cfg.resolve_number("100"), Some("09012345678".to_string()));
        assert_eq!(
            cfg.resolve_number("09011112222"),
            Some("09011112222".to_string())
        );
        assert_eq!(
            cfg.resolve_number("unknown"),
            Some("09000000000".to_string())
        );
    }

    #[test]
    fn outbound_phone_number_check() {
        assert!(is_phone_number("09012345678"));
        assert!(!is_phone_number("9012345678"));
        assert!(!is_phone_number("abc"));
    }
}
#[derive(Clone, Debug)]
pub enum LogMode {
    Stdout,
    File,
}

#[derive(Clone, Debug)]
pub enum LogFormat {
    Text,
    Json,
}

#[derive(Clone, Debug)]
pub struct LoggingConfig {
    pub mode: LogMode,
    pub format: LogFormat,
    pub dir: Option<String>,
    pub file_name: String,
}

impl LoggingConfig {
    fn from_env() -> Self {
        let dir_env = std::env::var("LOG_DIR").ok();
        let mode_env = std::env::var("LOG_MODE").ok();
        let format_env = std::env::var("LOG_FORMAT").ok();

        let format = match format_env.as_deref() {
            Some("json") => LogFormat::Json,
            _ => LogFormat::Text,
        };

        let mode = match mode_env.as_deref() {
            Some("file") => LogMode::File,
            Some("stdout") => LogMode::Stdout,
            _ => {
                if dir_env.is_some() {
                    LogMode::File
                } else {
                    LogMode::Stdout
                }
            }
        };

        let dir = match mode {
            LogMode::File => Some(dir_env.unwrap_or_else(|| "logs".to_string())),
            LogMode::Stdout => None,
        };

        let file_name = std::env::var("LOG_FILE_NAME").unwrap_or_else(|_| "app.log".to_string());

        Self {
            mode,
            format,
            dir,
            file_name,
        }
    }
}

static LOGGING: OnceLock<LoggingConfig> = OnceLock::new();

pub fn logging_config() -> &'static LoggingConfig {
    LOGGING.get_or_init(LoggingConfig::from_env)
}

#[derive(Clone, Debug)]
pub struct AiConfig {
    pub gemini_api_key: Option<String>,
    pub gemini_model: String,
    pub ollama_model: String,
    pub ollama_intent_model: String,
    pub use_aws_transcribe: bool,
    pub aws_transcribe_bucket: Option<String>,
    pub aws_transcribe_prefix: String,
    pub ser_url: Option<String>,
}

impl AiConfig {
    /// Constructs an AI-related configuration from environment variables, using sensible defaults when variables are absent.
    ///
    /// The following environment variables are read:
    /// - `GEMINI_API_KEY`: optional API key for Gemini (kept as `None` if unset).
    /// - `GEMINI_MODEL`: model name for Gemini; defaults to `"gemini-2.5-flash-lite"`.
    /// - `OLLAMA_MODEL`: model name for Ollama; defaults to `"gemma3:4b"`.
    /// - `OLLAMA_INTENT_MODEL`: intent model for Ollama; defaults to the value of `OLLAMA_MODEL`.
    /// - `USE_AWS_TRANSCRIBE`: treated as a boolean; defaults to `false`.
    /// - `AWS_TRANSCRIBE_BUCKET`: optional S3 bucket name for AWS Transcribe.
    /// - `AWS_TRANSCRIBE_PREFIX`: prefix for transcribe objects; defaults to `"voicebot"`.
    /// - `SER_URL`: optional SER service URL.
    ///
    /// The returned value is an instance populated from these environment variables with the described defaults.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use std::env;
    /// // Ensure relevant vars are not set to exercise defaults in this example.
    /// env::remove_var("GEMINI_API_KEY");
    /// env::remove_var("GEMINI_MODEL");
    /// env::remove_var("OLLAMA_MODEL");
    /// env::remove_var("OLLAMA_INTENT_MODEL");
    /// env::remove_var("USE_AWS_TRANSCRIBE");
    /// env::remove_var("AWS_TRANSCRIBE_BUCKET");
    /// env::remove_var("AWS_TRANSCRIBE_PREFIX");
    /// env::remove_var("SER_URL");
    ///
    /// let cfg = AiConfig::from_env();
    /// assert_eq!(cfg.ollama_model, "gemma3:4b");
    /// assert_eq!(cfg.gemini_model, "gemini-2.5-flash-lite");
    /// ```
    fn from_env() -> Self {
        let ollama_model =
            std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "gemma3:4b".to_string());
        let ollama_intent_model =
            std::env::var("OLLAMA_INTENT_MODEL").unwrap_or_else(|_| ollama_model.clone());
        Self {
            gemini_api_key: std::env::var("GEMINI_API_KEY").ok(),
            gemini_model: std::env::var("GEMINI_MODEL")
                .unwrap_or_else(|_| "gemini-2.5-flash-lite".to_string()),
            ollama_model,
            ollama_intent_model,
            use_aws_transcribe: env_bool("USE_AWS_TRANSCRIBE", false),
            aws_transcribe_bucket: std::env::var("AWS_TRANSCRIBE_BUCKET").ok(),
            aws_transcribe_prefix: std::env::var("AWS_TRANSCRIBE_PREFIX")
                .unwrap_or_else(|_| "voicebot".to_string()),
            ser_url: std::env::var("SER_URL").ok(),
        }
    }
}

static AI_CONFIG: OnceLock<AiConfig> = OnceLock::new();

pub fn ai_config() -> &'static AiConfig {
    AI_CONFIG.get_or_init(AiConfig::from_env)
}

#[derive(Clone, Debug)]
pub struct WeatherConfig {
    pub api_base: String,
    pub default_area_code: String,
    pub cache_ttl: Duration,
}

impl WeatherConfig {
    fn from_env() -> Self {
        let api_base = std::env::var("WEATHER_API_BASE")
            .unwrap_or_else(|_| "https://www.jma.go.jp/bosai/forecast/data/forecast".to_string());
        let default_area_code =
            std::env::var("WEATHER_DEFAULT_AREA_CODE").unwrap_or_else(|_| "130000".to_string());
        let cache_ttl = env_duration_sec("WEATHER_CACHE_TTL_SEC", 600);
        Self {
            api_base,
            default_area_code,
            cache_ttl,
        }
    }
}

static WEATHER_CONFIG: OnceLock<WeatherConfig> = OnceLock::new();

pub fn weather_config() -> &'static WeatherConfig {
    WEATHER_CONFIG.get_or_init(WeatherConfig::from_env)
}
