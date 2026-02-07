pub use crate::shared::ports::sip::{
    Sdp, SessionRefresher, SessionTimerInfo, SipCommand, SipEvent,
};

#[derive(Clone)]
pub struct SipConfig {
    pub advertised_ip: String,
    pub sip_port: u16,
    #[allow(dead_code)]
    pub advertised_rtp_port: u16,
}
