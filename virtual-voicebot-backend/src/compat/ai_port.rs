use std::sync::Arc;

use anyhow::{anyhow, Result as AnyResult};

use crate::error::ai::{AsrError, IntentError, LlmError, TtsError, WeatherError};
use crate::ports::ai::{
    AiFuture, AsrChunk, AsrPort, ChatMessage, IntentPort, LlmPort, SerInputPcm, SerOutcome,
    SerPort, TtsPort, WeatherPort, WeatherQuery,
};

#[deprecated(
    since = "0.3.0",
    note = "Use individual ports (AsrPort, IntentPort, LlmPort, WeatherPort, TtsPort, SerPort) instead."
)]
pub trait AiPort: Send + Sync {
    fn transcribe_chunks(
        &self,
        call_id: String,
        chunks: Vec<AsrChunk>,
    ) -> AiFuture<AnyResult<String>>;
    fn classify_intent(&self, text: String) -> AiFuture<AnyResult<String>>;
    fn generate_answer(&self, messages: Vec<ChatMessage>) -> AiFuture<AnyResult<String>>;
    fn handle_weather(&self, query: WeatherQuery) -> AiFuture<AnyResult<String>>;
    fn synth_to_wav(&self, text: String, path: Option<String>) -> AiFuture<AnyResult<String>>;
}

pub struct LegacyAiPortAdapter<T> {
    inner: Arc<T>,
}

impl<T> LegacyAiPortAdapter<T> {
    pub fn new(inner: Arc<T>) -> Self {
        Self { inner }
    }
}

impl<T> AiPort for LegacyAiPortAdapter<T>
where
    T: AsrPort + IntentPort + LlmPort + WeatherPort + TtsPort + Send + Sync + 'static,
{
    fn transcribe_chunks(
        &self,
        call_id: String,
        chunks: Vec<AsrChunk>,
    ) -> AiFuture<AnyResult<String>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            inner
                .transcribe_chunks(call_id, chunks)
                .await
                .map_err(|e| anyhow!(e.to_string()))
        })
    }

    fn classify_intent(&self, text: String) -> AiFuture<AnyResult<String>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            inner
                .classify_intent(text)
                .await
                .map_err(|e| anyhow!(e.to_string()))
        })
    }

    fn generate_answer(&self, messages: Vec<ChatMessage>) -> AiFuture<AnyResult<String>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            inner
                .generate_answer(messages)
                .await
                .map_err(|e| anyhow!(e.to_string()))
        })
    }

    fn handle_weather(&self, query: WeatherQuery) -> AiFuture<AnyResult<String>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            inner
                .handle_weather(query)
                .await
                .map_err(|e| anyhow!(e.to_string()))
        })
    }

    fn synth_to_wav(&self, text: String, path: Option<String>) -> AiFuture<AnyResult<String>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            inner
                .synth_to_wav(text, path)
                .await
                .map(|p| p.to_string_lossy().to_string())
                .map_err(|e| anyhow!(e.to_string()))
        })
    }
}

pub struct CompatAiServices<T> {
    inner: Arc<T>,
}

impl<T> CompatAiServices<T> {
    pub fn new(inner: Arc<T>) -> Self {
        Self { inner }
    }
}

impl<T> AsrPort for CompatAiServices<T>
where
    T: AiPort + Send + Sync + 'static,
{
    fn transcribe_chunks(
        &self,
        call_id: String,
        chunks: Vec<AsrChunk>,
    ) -> AiFuture<Result<String, AsrError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            inner
                .transcribe_chunks(call_id, chunks)
                .await
                .map_err(|e| AsrError::TranscriptionFailed(e.to_string()))
        })
    }
}

impl<T> IntentPort for CompatAiServices<T>
where
    T: AiPort + Send + Sync + 'static,
{
    fn classify_intent(&self, text: String) -> AiFuture<Result<String, IntentError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            inner
                .classify_intent(text)
                .await
                .map_err(|e| IntentError::ClassificationFailed(e.to_string()))
        })
    }
}

impl<T> LlmPort for CompatAiServices<T>
where
    T: AiPort + Send + Sync + 'static,
{
    fn generate_answer(&self, messages: Vec<ChatMessage>) -> AiFuture<Result<String, LlmError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            inner
                .generate_answer(messages)
                .await
                .map_err(|e| LlmError::GenerationFailed(e.to_string()))
        })
    }
}

impl<T> WeatherPort for CompatAiServices<T>
where
    T: AiPort + Send + Sync + 'static,
{
    fn handle_weather(&self, query: WeatherQuery) -> AiFuture<Result<String, WeatherError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            inner
                .handle_weather(query)
                .await
                .map_err(|e| WeatherError::QueryFailed(e.to_string()))
        })
    }
}

impl<T> TtsPort for CompatAiServices<T>
where
    T: AiPort + Send + Sync + 'static,
{
    fn synth_to_wav(
        &self,
        text: String,
        path: Option<String>,
    ) -> AiFuture<Result<std::path::PathBuf, TtsError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            inner
                .synth_to_wav(text, path)
                .await
                .map(std::path::PathBuf::from)
                .map_err(|e| TtsError::SynthesisFailed(e.to_string()))
        })
    }
}

impl<T> SerPort for CompatAiServices<T>
where
    T: SerPort + AiPort + Send + Sync + 'static,
{
    fn analyze(
        &self,
        input: SerInputPcm,
    ) -> AiFuture<Result<SerOutcome, crate::error::ai::SerError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move { inner.analyze(input).await })
    }
}
