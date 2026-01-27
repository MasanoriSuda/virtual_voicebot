#![allow(dead_code)]
// types.rs
use std::time::Duration;

use crate::session::b2bua::BLeg;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionRefresher {
    Uac,
    Uas,
}

#[derive(Clone, Copy, Debug)]
pub struct SessionTimerInfo {
    pub expires: Duration,
    pub refresher: SessionRefresher,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IvrState {
    #[default]
    IvrMenuWaiting,
    VoicebotIntroPlaying,
    VoicebotMode,
    Transferring,
    B2buaMode,
}

/// sip/session 間で受け取るイベント（上位: sip・rtp・app からの入力）
#[derive(Debug)]
pub enum SessionIn {
    /// SIP側からのINVITE入力
    SipInvite {
        call_id: CallId,
        from: String,
        to: String,
        offer: Sdp,
        session_timer: Option<SessionTimerInfo>,
    },
    /// 既存ダイアログ内の re-INVITE
    SipReInvite {
        offer: Sdp,
        session_timer: Option<SessionTimerInfo>,
    },
    /// SIP側からのACK
    SipAck,
    /// SIP側からのBYE
    SipBye,
    /// SIP側からのCANCEL
    SipCancel,
    /// SIPトランザクションタイムアウト通知
    SipTransactionTimeout {
        call_id: CallId,
    },
    /// RTP入力（メディア/PCM経路）
    MediaRtpIn {
        ts: u32,
        payload: Vec<u8>,
    },
    /// Bレグ確立（B2BUA）
    B2buaEstablished {
        b_leg: BLeg,
    },
    /// Bレグの呼び出し中（180 Ringing）
    B2buaRinging,
    /// Bレグの早期メディア（183 Session Progress）
    B2buaEarlyMedia,
    /// Bレグ転送失敗
    B2buaFailed {
        reason: String,
        status: Option<u16>,
    },
    /// BレグからのRTP
    BLegRtp {
        payload: Vec<u8>,
    },
    /// BレグからのBYE
    BLegBye,
    /// DTMF tone detected (in-band)
    Dtmf {
        digit: char,
    },
    /// IVR menu timeout
    IvrTimeout,
    /// 転送中アナウンスの繰り返し
    TransferAnnounce,
    /// app から返ってきたボット応答音声（WAVファイルパス）
    AppBotAudioFile {
        path: String,
    },
    /// app からの終了指示
    AppHangup,
    /// app からの転送指示
    AppTransferRequest {
        person: String,
    },
    /// Session Timer (keepalive 含む) の失効
    SessionTimerFired,
    /// Session-Expires の更新時刻（refresher=uas 用）
    SessionRefreshDue,
    /// keepalive tick
    MediaTimerTick,
    /// Session-Expires による更新（INVITE/UPDATE）
    SipSessionExpires {
        timer: SessionTimerInfo,
    },
    Abort(anyhow::Error),
}

/// session → 上位（sip/rtp/app/metrics）への通知/指示
#[derive(Debug)]
pub enum SessionOut {
    /// SIP provisional (100)
    SipSend100,
    /// SIP provisional (180)
    SipSend180,
    /// SIP provisional (183 + SDP)
    SipSend183 {
        answer: Sdp,
    },
    /// SIP final (200 + SDP)
    SipSend200 {
        answer: Sdp,
    },
    /// SIP UPDATE によるセッションリフレッシュ
    SipSendUpdate {
        expires: Duration,
    },
    /// SIP エラー応答（INVITE の最終応答）
    SipSendError {
        code: u16,
        reason: String,
    },
    /// SIP BYE送信（UAC側として終話）
    SipSendBye,
    /// SIP BYEに対する200
    SipSendBye200,
    /// RTP送信開始指示
    RtpStartTx {
        dst_ip: String,
        dst_port: u16,
        pt: u8,
    },
    /// RTP送信停止指示
    RtpStopTx,
    /// app/tts への合成依頼（将来用のスタブ）
    AppRequestTts {
        text: String,
    }, // → VOICEVOXへ
    /// Session Timer 失効を app 等へ通知
    AppSessionTimeout,
    /// app が生成したボット音声（WAVパス）を session へ戻す
    AppSendBotAudioFile {
        path: String,
    },
    /// app からの切断指示
    AppRequestHangup,
    /// app からの転送指示
    AppRequestTransfer {
        person: String,
    },
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

pub(crate) fn next_session_state(current: SessState, event: &SessionIn) -> SessState {
    match event {
        SessionIn::SipBye
        | SessionIn::SipCancel
        | SessionIn::BLegBye
        | SessionIn::AppHangup
        | SessionIn::SessionTimerFired
        | SessionIn::Abort(_) => SessState::Terminated,
        SessionIn::SipInvite { .. } => match current {
            SessState::Idle => SessState::Early,
            _ => current,
        },
        SessionIn::SipAck => match current {
            SessState::Early => SessState::Established,
            _ => current,
        },
        _ => current,
    }
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

    pub fn insert(&self, call_id: CallId, tx: UnboundedSender<SessionIn>) {
        self.inner.lock().unwrap().insert(call_id, tx);
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
