use chrono::{DateTime, FixedOffset};
use std::fmt;

use crate::entities::identifiers::CallId;

pub enum AppEvent {
    CallRinging {
        call_id: CallId,
        from: String,
        timestamp: DateTime<FixedOffset>,
    },
    CallStarted {
        call_id: CallId,
        caller: Option<String>,
    },
    AudioBuffered {
        call_id: CallId,
        pcm_mulaw: Vec<u8>,
        pcm_linear16: Vec<i16>,
    },
    CallEnded {
        call_id: CallId,
        from: String,
        reason: EndReason,
        duration_sec: Option<u64>,
        timestamp: DateTime<FixedOffset>,
    },
}

impl fmt::Debug for AppEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CallRinging {
                call_id,
                from,
                timestamp,
            } => f
                .debug_struct("CallRinging")
                .field("call_id", call_id)
                .field("from", from)
                .field("timestamp", timestamp)
                .finish(),
            Self::CallStarted { call_id, caller } => f
                .debug_struct("CallStarted")
                .field("call_id", call_id)
                .field("caller", caller)
                .finish(),
            Self::AudioBuffered {
                call_id,
                pcm_mulaw,
                pcm_linear16,
            } => f
                .debug_struct("AudioBuffered")
                .field("call_id", call_id)
                .field("pcm_mulaw_len", &pcm_mulaw.len())
                .field("pcm_linear16_len", &pcm_linear16.len())
                .finish(),
            Self::CallEnded {
                call_id,
                from,
                reason,
                duration_sec,
                timestamp,
            } => f
                .debug_struct("CallEnded")
                .field("call_id", call_id)
                .field("from", from)
                .field("reason", reason)
                .field("duration_sec", duration_sec)
                .field("timestamp", timestamp)
                .finish(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndReason {
    Bye,
    Cancel,
    Timeout,
    Error,
    AppHangup,
}
