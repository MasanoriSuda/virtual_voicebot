use std::time::Duration;

use crate::entities::CallId;

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

/// sip 層から session 層へ渡すイベント（設計ドキュメントの「sip→session 通知」と対応）
#[derive(Debug)]
pub enum SipEvent {
    /// INVITE を受けたときの session への通知（call_id/from/to/offer を引き渡す）
    IncomingInvite {
        call_id: CallId,
        from: String,
        to: String,
        offer: Sdp,
        session_timer: Option<SessionTimerInfo>,
    },
    /// 既存ダイアログ内の re-INVITE
    ReInvite {
        call_id: CallId,
        offer: Sdp,
        session_timer: Option<SessionTimerInfo>,
    },
    /// 既存ダイアログに対する ACK
    Ack {
        call_id: CallId,
    },
    /// INVITE 取り消し（CANCEL）
    Cancel {
        call_id: CallId,
    },
    /// 既存ダイアログに対する BYE
    Bye {
        call_id: CallId,
    },
    /// トランザクションのタイムアウト通知（Timer J など）
    TransactionTimeout {
        call_id: CallId,
    },
    /// Session-Expires を受けたときのセッション更新通知
    SessionRefresh {
        call_id: CallId,
        timer: SessionTimerInfo,
    },
    Unknown,
}

/// session 層から sip 層へ渡す送信指示
#[derive(Debug)]
pub enum SipCommand {
    /// SIP provisional (100)
    Send100,
    /// SIP provisional (180)
    Send180,
    /// SIP provisional (183 + SDP)
    Send183 { answer: Sdp },
    /// SIP final (200 + SDP)
    Send200 { answer: Sdp },
    /// SIP UPDATE によるセッションリフレッシュ
    SendUpdate { expires: Duration },
    /// SIP エラー応答（INVITE の最終応答）
    SendError { code: u16, reason: String },
    /// SIP BYE送信（UAC側として終話）
    SendBye,
    /// SIP BYEに対する200
    SendBye200,
}
