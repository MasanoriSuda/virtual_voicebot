use std::sync::Arc;

use tokio::sync::{mpsc, oneshot, Mutex};

use crate::shared::error::ai::AsrError;

use super::{AiFuture, AsrChunk};

pub trait AsrPort: Send + Sync {
    fn transcribe_chunks(
        &self,
        call_id: String,
        chunks: Vec<AsrChunk>,
    ) -> AiFuture<Result<String, AsrError>>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AsrAudioMsg {
    Chunk(Vec<u8>),
    End,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AsrStreamEvent {
    Partial(String),
    Final(String),
}

#[derive(Clone)]
pub struct AsrAudioTx {
    tx: mpsc::Sender<AsrAudioMsg>,
    rx: Arc<Mutex<mpsc::Receiver<AsrAudioMsg>>>,
}

pub struct AsrAudioRx {
    rx: Arc<Mutex<mpsc::Receiver<AsrAudioMsg>>>,
}

pub fn asr_audio_channel(capacity: usize) -> (AsrAudioTx, AsrAudioRx) {
    let (tx, rx) = mpsc::channel(capacity);
    let shared = Arc::new(Mutex::new(rx));
    (
        AsrAudioTx {
            tx,
            rx: Arc::clone(&shared),
        },
        AsrAudioRx { rx: shared },
    )
}

impl AsrAudioTx {
    /// Try to send a chunk and, if full, drop one oldest queued audio message before retrying.
    /// This keeps the newest PCM (latest-first) for RTP->ASR backpressure.
    pub fn try_send_chunk_latest(
        &self,
        pcm_mulaw: Vec<u8>,
    ) -> Result<(), mpsc::error::TrySendError<AsrAudioMsg>> {
        let msg = AsrAudioMsg::Chunk(pcm_mulaw);
        match self.tx.try_send(msg) {
            Ok(()) => Ok(()),
            Err(mpsc::error::TrySendError::Full(msg)) => {
                // NOTE: best-effort oldest-drop.
                // If `try_lock()` fails because the consumer holds the mutex during `recv().await`,
                // the retry below may still return `Full(msg)`, which drops the newest chunk.
                // A dedicated overwrite buffer/watch-style channel is needed for strict latest-first.
                if let Ok(mut rx) = self.rx.try_lock() {
                    let _ = rx.try_recv();
                }
                self.tx.try_send(msg)
            }
            Err(e) => Err(e),
        }
    }

    pub fn try_send_end(&self) -> Result<(), mpsc::error::TrySendError<AsrAudioMsg>> {
        match self.tx.try_send(AsrAudioMsg::End) {
            Ok(()) => Ok(()),
            Err(mpsc::error::TrySendError::Full(msg)) => {
                // NOTE: best-effort oldest-drop with the same try_lock race as chunk sends.
                if let Ok(mut rx) = self.rx.try_lock() {
                    let _ = rx.try_recv();
                }
                self.tx.try_send(msg)
            }
            Err(e) => Err(e),
        }
    }

    pub async fn send_end(&self) -> Result<(), mpsc::error::SendError<AsrAudioMsg>> {
        self.tx.send(AsrAudioMsg::End).await
    }
}

impl AsrAudioRx {
    pub async fn recv(&self) -> Option<AsrAudioMsg> {
        let mut rx = self.rx.lock().await;
        rx.recv().await
    }
}

pub struct AsrStreamHandle {
    pub audio_tx: AsrAudioTx,
    pub final_rx: oneshot::Receiver<Result<String, AsrError>>,
}

pub trait AsrStreamPort: Send + Sync {
    fn transcribe_stream(
        &self,
        call_id: String,
        endpoint_url: String,
    ) -> AiFuture<Result<AsrStreamHandle, AsrError>>;
}
