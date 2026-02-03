use chrono::{DateTime, FixedOffset};

#[derive(Debug)]
pub enum AppEvent {
    CallRinging {
        call_id: String,
        from: String,
        timestamp: DateTime<FixedOffset>,
    },
    CallStarted {
        call_id: String,
        caller: Option<String>,
    },
    AudioBuffered {
        call_id: String,
        pcm_mulaw: Vec<u8>,
        pcm_linear16: Vec<i16>,
    },
    CallEnded {
        call_id: String,
        from: String,
        reason: EndReason,
        duration_sec: Option<u64>,
        timestamp: DateTime<FixedOffset>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndReason {
    Bye,
    Cancel,
    Timeout,
    Error,
    AppHangup,
}
