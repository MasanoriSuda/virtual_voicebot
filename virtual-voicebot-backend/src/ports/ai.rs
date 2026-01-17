use anyhow::Result;
use std::future::Future;
use std::pin::Pin;

/// チャンク入力（μ-law）をまとめて ASR するための簡易I/F（MVPでは一括まとめて既存ASRを呼ぶ）。
#[derive(Debug, Clone)]
pub struct AsrChunk {
    pub pcm_mulaw: Vec<u8>,
    pub end: bool,
}

pub type AiFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// app 層が依存する AI ポート（外部I/Oは実装側に閉じ込める）。
pub trait AiPort: Send + Sync {
    fn transcribe_chunks(&self, call_id: String, chunks: Vec<AsrChunk>) -> AiFuture<Result<String>>;
    fn generate_answer(&self, text: String) -> AiFuture<Result<String>>;
    fn synth_to_wav(&self, text: String, path: Option<String>) -> AiFuture<Result<String>>;
}
