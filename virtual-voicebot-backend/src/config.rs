use anyhow::Result;
use std::sync::OnceLock;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct Config {
    pub sip_bind_ip: String,
    pub sip_port: u16,
    pub rtp_port: u16,
    pub local_ip: String,
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

        Ok(Self {
            sip_bind_ip,
            sip_port,
            rtp_port,
            local_ip,
        })
    }
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

fn env_duration_ms(key: &str, default_ms: u64) -> Duration {
    let ms = std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(default_ms);
    Duration::from_millis(ms)
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
