#![allow(dead_code)]
// types.rs
#[derive(Clone, Debug)]
pub struct Sdp {
    pub ip: String,
    pub port: u16,
    pub payload_type: u8,
    pub codec: String, // e.g. "PCMU/8000"
}

impl Sdp {
    pub fn pcmu(ip: impl Into<String>, port: u16) -> Self {
        Self {
            ip: ip.into(),
            port,
            payload_type: 0,
            codec: "PCMU/8000".to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct MediaConfig {
    pub local_ip: String,
    pub local_port: u16,
    pub payload_type: u8,
}

impl MediaConfig {
    pub fn pcmu(local_ip: impl Into<String>, local_port: u16) -> Self {
        Self {
            local_ip: local_ip.into(),
            local_port,
            payload_type: 0,
        }
    }
}

#[derive(Debug)]
pub enum SessionIn {
    Invite { call_id: String, from: String, to: String, offer: Sdp },
    Ack,
    Bye,
    RtpIn { ts: u32, payload: Vec<u8> },
    BotAudio { pcm48k: Vec<i16> },
    TimerTick,
    Abort(anyhow::Error),
}

#[derive(Debug)]
pub enum SessionOut {
    SendSip180,
    SendSip200 { answer: Sdp },
    SendSipBye200,
    StartRtpTx { dst_ip: String, dst_port: u16, pt: u8 },
    StopRtpTx,
    BotSynthesize { text: String }, // → VOICEVOXへ
    Metrics { name: &'static str, value: i64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessState { Idle, Early, Established, Terminating, Terminated }
