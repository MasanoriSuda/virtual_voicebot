use chrono::{DateTime, FixedOffset};
use std::fmt;
use std::sync::Arc;

use tokio::sync::{mpsc, Mutex};

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

/// Bounded channel pair for AppEvent with "latest-wins" semantics for high-volume audio events.
///
/// Backpressure policy:
/// - Control events should use `send` (awaitable).
/// - Audio events should use `try_send_latest` (drops oldest queued item if full).
#[derive(Clone)]
pub struct AppEventTx {
    tx: mpsc::Sender<AppEvent>,
    rx: Arc<Mutex<mpsc::Receiver<AppEvent>>>,
}

pub struct AppEventRx {
    rx: Arc<Mutex<mpsc::Receiver<AppEvent>>>,
}

pub fn app_event_channel(capacity: usize) -> (AppEventTx, AppEventRx) {
    let (tx, rx) = mpsc::channel(capacity);
    let shared = Arc::new(Mutex::new(rx));
    (
        AppEventTx {
            tx,
            rx: Arc::clone(&shared),
        },
        AppEventRx { rx: shared },
    )
}

impl AppEventTx {
    pub async fn send(&self, event: AppEvent) -> Result<(), mpsc::error::SendError<AppEvent>> {
        self.tx.send(event).await
    }

    pub fn try_send(
        &self,
        event: AppEvent,
    ) -> Result<(), mpsc::error::TrySendError<AppEvent>> {
        self.tx.try_send(event)
    }

    /// Try to send and, if full, drop one oldest queued event before retrying.
    pub fn try_send_latest(
        &self,
        event: AppEvent,
    ) -> Result<(), mpsc::error::TrySendError<AppEvent>> {
        match self.tx.try_send(event) {
            Ok(()) => Ok(()),
            Err(mpsc::error::TrySendError::Full(event)) => {
                if let Ok(mut rx) = self.rx.try_lock() {
                    let _ = rx.try_recv();
                }
                self.tx.try_send(event)
            }
            Err(e) => Err(e),
        }
    }
}

impl AppEventRx {
    pub async fn recv(&self) -> Option<AppEvent> {
        let mut rx = self.rx.lock().await;
        rx.recv().await
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
