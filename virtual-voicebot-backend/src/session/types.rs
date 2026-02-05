#![allow(dead_code)]
// types.rs
use std::time::Duration;

use crate::session::b2bua::BLeg;
use thiserror::Error;

#[derive(Clone, Debug)]
pub struct Sdp {
    pub ip: String,
    pub port: u16,
    pub payload_type: u8,
    pub codec: String, // e.g. "PCMU/8000"
}

/// Call-ID を表す（設計ドキュメント上はセッション識別子と一致させる）
pub use crate::entities::CallId;

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

/// sip/session 間で受け取る制御イベント（SIP/タイマー/app など）
#[derive(Debug)]
pub enum SessionControlIn {
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
    /// BレグからのBYE
    BLegBye,
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
    /// 180 Ringing 後の遅延満了
    RingDurationElapsed,
    /// Session-Expires による更新（INVITE/UPDATE）
    SipSessionExpires {
        timer: SessionTimerInfo,
    },
    Abort(SessionError),
}

/// RTP/DTMF など高頻度メディアイベント
#[derive(Debug)]
pub enum SessionMediaIn {
    /// RTP入力（メディア/PCM経路）
    MediaRtpIn {
        call_id: CallId,
        stream_id: String,
        ts: u32,
        payload: Vec<u8>,
    },
    /// DTMF tone detected (in-band)
    Dtmf {
        call_id: CallId,
        stream_id: String,
        digit: char,
    },
    /// BレグからのRTP
    BLegRtp {
        call_id: CallId,
        stream_id: String,
        payload: Vec<u8>,
    },
}

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("internal session error: {0}")]
    Internal(String),
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

/// Compute the next session state given the current state and an incoming event.
///
/// The next state is determined by the event: terminal events transition to `SessState::Terminated`; a
/// `SipInvite` moves `SessState::Idle` to `SessState::Early`; a `SipAck` moves `SessState::Early` to
/// `SessState::Established`; `RingDurationElapsed` preserves the current state; all other events leave
/// the state unchanged.
///
/// # Returns
///
/// `SessState` representing the session's next lifecycle state.
///
/// # Examples
///
/// ```
/// use crate::types::{next_session_state, CallId, SessState, SessionControlIn, Sdp};
///
/// let s = SessState::Idle;
/// let next = next_session_state(s, &SessionControlIn::SipInvite { call_id: CallId::new("call-1").unwrap(), from: "".into(), to: "".into(), offer: Sdp::pcmu("127.0.0.1", 10000), session_timer: None });
/// assert_eq!(next, SessState::Early);
/// ```
pub(crate) fn next_session_state(current: SessState, event: &SessionControlIn) -> SessState {
    match event {
        SessionControlIn::SipBye
        | SessionControlIn::SipCancel
        | SessionControlIn::BLegBye
        | SessionControlIn::AppHangup
        | SessionControlIn::SessionTimerFired
        | SessionControlIn::Abort(_) => SessState::Terminated,
        SessionControlIn::RingDurationElapsed => current,
        SessionControlIn::SipInvite { .. } => match current {
            SessState::Idle => SessState::Early,
            _ => current,
        },
        SessionControlIn::SipAck => match current {
            SessState::Early => SessState::Established,
            _ => current,
        },
        _ => current,
    }
}

use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};

#[derive(Clone)]
pub struct SessionHandle {
    pub control_tx: mpsc::Sender<SessionControlIn>,
    pub media_tx: mpsc::Sender<SessionMediaIn>,
}

#[derive(Clone)]
/// SessionRegistry keeps the active session channels keyed by CallId.
///
/// Register immediately after spawning a session and unregister on termination
/// (e.g., after BYE/CANCEL handling or when the session task exits) to avoid
/// stale entries or duplicate registrations.
pub struct SessionRegistry {
    tx: mpsc::Sender<RegistryCommand>,
}

enum RegistryCommand {
    Register {
        call_id: CallId,
        handle: SessionHandle,
        reply: oneshot::Sender<()>,
    },
    Unregister {
        call_id: CallId,
        reply: oneshot::Sender<Option<SessionHandle>>,
    },
    Get {
        call_id: CallId,
        reply: oneshot::Sender<Option<SessionHandle>>,
    },
    List {
        reply: oneshot::Sender<Vec<CallId>>,
    },
}

impl SessionRegistry {
    pub fn new() -> Self {
        let (tx, mut rx) = mpsc::channel(128);
        tokio::spawn(async move {
            let mut map: HashMap<CallId, SessionHandle> = HashMap::new();
            while let Some(cmd) = rx.recv().await {
                match cmd {
                    RegistryCommand::Register {
                        call_id,
                        handle,
                        reply,
                    } => {
                        map.insert(call_id, handle);
                        let _ = reply.send(());
                    }
                    RegistryCommand::Unregister { call_id, reply } => {
                        let removed = map.remove(&call_id);
                        let _ = reply.send(removed);
                    }
                    RegistryCommand::Get { call_id, reply } => {
                        let found = map.get(&call_id).cloned();
                        let _ = reply.send(found);
                    }
                    RegistryCommand::List { reply } => {
                        let list = map.keys().cloned().collect();
                        let _ = reply.send(list);
                    }
                }
            }
        });
        Self { tx }
    }

    pub async fn insert(&self, call_id: CallId, handle: SessionHandle) {
        let (reply_tx, reply_rx) = oneshot::channel();
        if self
            .tx
            .send(RegistryCommand::Register {
                call_id,
                handle,
                reply: reply_tx,
            })
            .await
            .is_ok()
        {
            let _ = reply_rx.await;
        }
    }

    pub async fn get(&self, call_id: &CallId) -> Option<SessionHandle> {
        let (reply_tx, reply_rx) = oneshot::channel();
        if self
            .tx
            .send(RegistryCommand::Get {
                call_id: call_id.clone(),
                reply: reply_tx,
            })
            .await
            .is_err()
        {
            return None;
        }
        reply_rx.await.ok().flatten()
    }

    pub async fn remove(&self, call_id: &CallId) -> Option<SessionHandle> {
        let (reply_tx, reply_rx) = oneshot::channel();
        if self
            .tx
            .send(RegistryCommand::Unregister {
                call_id: call_id.clone(),
                reply: reply_tx,
            })
            .await
            .is_err()
        {
            return None;
        }
        reply_rx.await.ok().flatten()
    }

    pub async fn list(&self) -> Vec<CallId> {
        let (reply_tx, reply_rx) = oneshot::channel();
        if self
            .tx
            .send(RegistryCommand::List { reply: reply_tx })
            .await
            .is_err()
        {
            return Vec::new();
        }
        reply_rx.await.unwrap_or_default()
    }
}
