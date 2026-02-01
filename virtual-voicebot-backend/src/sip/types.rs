use crate::session::types::{CallId, Sdp, SessionTimerInfo};

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

#[derive(Clone)]
pub struct SipConfig {
    pub advertised_ip: String,
    pub sip_port: u16,
    #[allow(dead_code)]
    pub advertised_rtp_port: u16,
}
