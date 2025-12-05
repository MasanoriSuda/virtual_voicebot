use anyhow::Result;

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
        let local_ip = std::env::var("LOCAL_IP").unwrap_or_else(|_| "127.0.0.1".to_string());

        Ok(Self {
            sip_bind_ip,
            sip_port,
            rtp_port,
            local_ip,
        })
    }
}
