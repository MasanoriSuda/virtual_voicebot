#![allow(dead_code)]
// types.rs
#[derive(Clone, Debug)]
pub struct Sdp {
    pub ip: String,
    pub port: u16,
    pub payload_type: u8,
    pub codec: String, // e.g. "PCMU/8000"
}

/// Call-ID を表す（設計ドキュメント上はセッション識別子と一致させる）
pub type CallId = String;

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

/// sip/session 間で受け取るイベント（上位: sip・rtp・app からの入力）
#[derive(Debug)]
pub enum SessionIn {
    Invite {
        call_id: CallId,
        from: String,
        to: String,
        offer: Sdp,
    },
    Ack,
    Bye,
    TransactionTimeout {
        call_id: CallId,
    },
    RtpIn {
        ts: u32,
        payload: Vec<u8>,
    },
    BotAudio {
        pcm48k: Vec<i16>,
    },
    TimerTick,
    Abort(anyhow::Error),
}

/// session → 上位（sip/rtp/app/metrics）への通知/指示
#[derive(Debug)]
pub enum SessionOut {
    SendSip180,
    SendSip200 {
        answer: Sdp,
    },
    SendSipBye200,
    StartRtpTx {
        dst_ip: String,
        dst_port: u16,
        pt: u8,
    },
    StopRtpTx,
    BotSynthesize {
        text: String,
    }, // → VOICEVOXへ
    Metrics {
        name: &'static str,
        value: i64,
    },
}

/// セッション状態（設計 doc の Idle/Early/Confirmed/Terminating/Terminated に対応）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessState {
    Idle,
    Early,
    /// Confirmed 相当
    Established,
    Terminating,
    Terminated,
}

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::UnboundedSender;

pub type SessionMap = Arc<Mutex<HashMap<CallId, UnboundedSender<SessionIn>>>>;

/// session manager の薄いラッパ（挙動は従来のマップ操作と同じ）
#[derive(Clone)]
pub struct SessionRegistry {
    inner: SessionMap,
}

impl SessionRegistry {
    pub fn new(inner: SessionMap) -> Self {
        Self { inner }
    }

    pub fn insert(
        &self,
        call_id: CallId,
        tx: UnboundedSender<SessionIn>,
    ) -> Option<UnboundedSender<SessionIn>> {
        self.inner.lock().unwrap().insert(call_id, tx)
    }

    pub fn get(&self, call_id: &CallId) -> Option<UnboundedSender<SessionIn>> {
        self.inner.lock().unwrap().get(call_id).cloned()
    }

    pub fn remove(&self, call_id: &CallId) -> Option<UnboundedSender<SessionIn>> {
        self.inner.lock().unwrap().remove(call_id)
    }

    pub fn list(&self) -> Vec<CallId> {
        self.inner.lock().unwrap().keys().cloned().collect()
    }
}
