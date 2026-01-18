use anyhow::Result;
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

impl Config {
    pub fn from_env() -> Result<Self> {
        let sip_bind_ip =
            std::env::var("SIP_BIND_IP").unwrap_or_else(|_| "0.0.0.0".to_string());
        let sip_port = std::env::var("SIP_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5060);
        let rtp_port = std::env::var("RTP_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10000);
        let local_ip = std::env::var("LOCAL_IP").unwrap_or_else(|_| "0.0.0.0".to_string());
        let advertised_ip =
            std::env::var("ADVERTISED_IP").unwrap_or_else(|_| local_ip.clone());
        let advertised_rtp_port = std::env::var("ADVERTISED_RTP_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(rtp_port);
        let recording_http_addr =
            std::env::var("RECORDING_HTTP_ADDR").unwrap_or_else(|_| "0.0.0.0:18080".to_string());
        let ingest_call_url = std::env::var("INGEST_CALL_URL").ok();
        let recording_base_url = std::env::var("RECORDING_BASE_URL")
            .ok()
            .or_else(|| {
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
    fn from_env() -> Option<Self> {
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
        let auth_username =
            env_non_empty("REGISTER_AUTH_USER").unwrap_or_else(|| user.clone());
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

pub fn registrar_config() -> Option<&'static RegistrarConfig> {
    REGISTRAR_CONFIG.get_or_init(RegistrarConfig::from_env).as_ref()
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
            jitter_max_reorder: env_u16("RTP_JITTER_MAX_REORDER", 5),
            rtcp_interval: env_duration_ms("RTCP_INTERVAL_MS", 5_000),
        }
    }
}

static RTP_CONFIG: OnceLock<RtpConfig> = OnceLock::new();

pub fn rtp_config() -> &'static RtpConfig {
    RTP_CONFIG.get_or_init(RtpConfig::from_env)
}

fn env_duration_ms(key: &str, default_ms: u64) -> Duration {
    let ms = std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(default_ms);
    Duration::from_millis(ms)
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

fn env_non_empty(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn resolve_socket_addr(host: &str, port: u16) -> Option<SocketAddr> {
    (host, port).to_socket_addrs().ok()?.next()
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
    pub use_aws_transcribe: bool,
    pub aws_transcribe_bucket: Option<String>,
    pub aws_transcribe_prefix: String,
}

impl AiConfig {
    fn from_env() -> Self {
        Self {
            gemini_api_key: std::env::var("GEMINI_API_KEY").ok(),
            gemini_model: std::env::var("GEMINI_MODEL")
                .unwrap_or_else(|_| "gemini-2.5-flash-lite".to_string()),
            use_aws_transcribe: env_bool("USE_AWS_TRANSCRIBE", false),
            aws_transcribe_bucket: std::env::var("AWS_TRANSCRIBE_BUCKET").ok(),
            aws_transcribe_prefix: std::env::var("AWS_TRANSCRIBE_PREFIX")
                .unwrap_or_else(|_| "voicebot".to_string()),
        }
    }
}

static AI_CONFIG: OnceLock<AiConfig> = OnceLock::new();

pub fn ai_config() -> &'static AiConfig {
    AI_CONFIG.get_or_init(AiConfig::from_env)
}

fn env_bool(key: &str, default_value: bool) -> bool {
    match std::env::var(key) {
        Ok(value) => {
            let lower = value.to_ascii_lowercase();
            lower == "1" || lower == "true" || lower == "yes"
        }
        Err(_) => default_value,
    }
}
