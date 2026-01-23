use anyhow::Result;
use std::future::Future;
use std::pin::Pin;

/// チャンク入力（μ-law）をまとめて ASR するための簡易I/F（MVPでは一括まとめて既存ASRを呼ぶ）。
#[derive(Debug, Clone)]
pub struct AsrChunk {
    pub pcm_mulaw: Vec<u8>,
    pub end: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

pub type AiFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// app 層が依存する AI ポート（外部I/Oは実装側に閉じ込める）。
pub trait AiPort: Send + Sync {
    fn transcribe_chunks(&self, call_id: String, chunks: Vec<AsrChunk>) -> AiFuture<Result<String>>;
    fn generate_answer(&self, messages: Vec<ChatMessage>) -> AiFuture<Result<String>>;
    fn synth_to_wav(&self, text: String, path: Option<String>) -> AiFuture<Result<String>>;
}

#[derive(Debug, Clone)]
pub struct SerInputPcm {
    pub session_id: String,
    pub stream_id: String,
    pub pcm: Vec<i16>,
    pub sample_rate: u32,
    pub channels: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Emotion {
    Neutral,
    Happy,
    Sad,
    Angry,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct SerResult {
    pub session_id: String,
    pub stream_id: String,
    pub emotion: Emotion,
    pub confidence: f32,
    pub arousal: Option<f32>,
    pub valence: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct SerError {
    pub session_id: String,
    pub reason: String,
}

pub type SerOutcome = std::result::Result<SerResult, SerError>;

pub trait SerPort: Send + Sync {
    fn analyze(&self, input: SerInputPcm) -> AiFuture<SerOutcome>;
}

pub trait AiSerPort: AiPort + SerPort {}

impl<T: AiPort + SerPort> AiSerPort for T {}
