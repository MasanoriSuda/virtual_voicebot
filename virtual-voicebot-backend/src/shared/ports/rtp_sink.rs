use crate::shared::entities::CallId;
use tokio::sync::mpsc;

/// RTP/DTMF など高頻度メディアイベント
#[derive(Debug)]
pub enum RtpEvent {
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

#[derive(Debug)]
pub enum RtpEventSendError {
    Full,
    Closed,
}

impl From<mpsc::error::TrySendError<RtpEvent>> for RtpEventSendError {
    fn from(err: mpsc::error::TrySendError<RtpEvent>) -> Self {
        match err {
            mpsc::error::TrySendError::Full(_) => Self::Full,
            mpsc::error::TrySendError::Closed(_) => Self::Closed,
        }
    }
}

/// Sink trait for delivering RTP events across the L3→L4 boundary.
pub trait RtpEventSink: Send + Sync {
    fn try_send(&self, event: RtpEvent) -> Result<(), RtpEventSendError>;
}

impl RtpEventSink for mpsc::Sender<RtpEvent> {
    fn try_send(&self, event: RtpEvent) -> Result<(), RtpEventSendError> {
        mpsc::Sender::try_send(self, event).map_err(RtpEventSendError::from)
    }
}
