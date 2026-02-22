//! ai モジュール: ASR/LLM/TTS の外部サービスクライアント。
//! - 外部 I/O（HTTP/AWS SDK、ローカル WAV 一時ファイル）をここに閉じ込める。
//! - app/session にはテキスト/PCM 抽象のみ渡す想定だが、現状は直接関数を呼ぶ。
//! - ポリシー（タイムアウト/リトライ/フォールバック）は既存のまま。

use anyhow::{anyhow, Result};
use log::info;
use reqwest::{multipart, Client};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::future::Future;
use std::io::Cursor;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, timeout};

use aws_config::meta::region::RegionProviderChain;
use aws_config::{BehaviorVersion, SdkConfig};
use aws_sdk_s3 as s3;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_transcribe as transcribe;

use crate::shared::config;
use crate::shared::error::ai::{AsrError, IntentError, LlmError, TtsError, WeatherError};
use crate::shared::ports::ai::{
    AiFuture, AsrChunk, AsrPort, ChatMessage, IntentPort, LlmPort, Role, SerInputPcm, SerOutcome,
    SerPort, TtsPort, WeatherPort, WeatherQuery,
};
use crate::shared::utils::mask_pii;

#[derive(Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
}

#[derive(Serialize, Deserialize, Clone)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Serialize, Deserialize)]
struct GeminiPart {
    text: String,
}

#[derive(Serialize, Deserialize)]
struct GeminiContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    parts: Vec<GeminiPart>,
}

#[derive(Serialize, Deserialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: GeminiContentOut,
}

#[derive(Deserialize)]
struct GeminiContentOut {
    parts: Vec<GeminiPart>,
}

#[derive(Deserialize)]
struct WhisperResponse {
    text: String,
}

pub mod asr;
pub mod intent;
pub mod llm;
pub mod ser;
pub mod tts;
pub mod weather;

fn http_client(timeout: Duration) -> Result<Client> {
    Ok(Client::builder().timeout(timeout).build()?)
}

#[derive(Clone, Copy)]
enum AsrStage {
    Cloud,
    Local,
    Raspi,
}

impl AsrStage {
    fn as_str(self) -> &'static str {
        match self {
            Self::Cloud => "cloud",
            Self::Local => "local",
            Self::Raspi => "raspi",
        }
    }
}

#[derive(Clone, Copy)]
enum LlmStage {
    Cloud,
    Local,
    Raspi,
}

impl LlmStage {
    fn as_str(self) -> &'static str {
        match self {
            Self::Cloud => "cloud",
            Self::Local => "local",
            Self::Raspi => "raspi",
        }
    }
}

#[derive(Clone, Copy)]
enum TtsStage {
    Local,
    Raspi,
}

impl TtsStage {
    fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Raspi => "raspi",
        }
    }
}

#[derive(Clone, Copy)]
enum IntentStage {
    Local,
    Raspi,
}

impl IntentStage {
    fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Raspi => "raspi",
        }
    }
}

fn asr_stage_count(ai_cfg: &config::AiConfig) -> usize {
    usize::from(ai_cfg.use_aws_transcribe)
        + usize::from(ai_cfg.asr_local_server_enabled)
        + usize::from(ai_cfg.asr_raspi_enabled)
}

fn llm_cloud_enabled(ai_cfg: &config::AiConfig) -> bool {
    ai_cfg
        .gemini_api_key
        .as_deref()
        .map(str::trim)
        .is_some_and(|key| !key.is_empty())
}

fn llm_stage_count(ai_cfg: &config::AiConfig) -> usize {
    usize::from(llm_cloud_enabled(ai_cfg))
        + usize::from(ai_cfg.llm_local_server_enabled)
        + usize::from(ai_cfg.llm_raspi_enabled)
}

fn tts_stage_count(ai_cfg: &config::AiConfig) -> usize {
    usize::from(ai_cfg.tts_local_server_enabled) + usize::from(ai_cfg.tts_raspi_enabled)
}

fn intent_stage_count(ai_cfg: &config::AiConfig) -> usize {
    usize::from(ai_cfg.intent_local_server_enabled) + usize::from(ai_cfg.intent_raspi_enabled)
}

fn log_asr_startup_warnings() {
    let ai_cfg = config::ai_config();
    if asr_stage_count(ai_cfg) == 0 {
        log::warn!(
            "[asr] no ASR stages enabled (USE_AWS_TRANSCRIBE=0, ASR_LOCAL_SERVER_ENABLED=0, ASR_RASPI_ENABLED=0)"
        );
    }
    if ai_cfg.asr_raspi_enabled && ai_cfg.asr_raspi_url.is_none() {
        log::warn!(
            "[asr] ASR_RASPI_ENABLED=1 but ASR_RASPI_URL is not set; raspi stage will be skipped"
        );
    }
}

fn log_llm_startup_warnings() {
    let ai_cfg = config::ai_config();
    if llm_stage_count(ai_cfg) == 0 {
        log::warn!(
            "[llm] no LLM stages enabled (GEMINI_API_KEY missing, LLM_LOCAL_SERVER_ENABLED=0, LLM_RASPI_ENABLED=0)"
        );
    }
    if ai_cfg.llm_raspi_enabled && ai_cfg.llm_raspi_url.is_none() {
        log::warn!(
            "[llm] LLM_RASPI_ENABLED=1 but LLM_RASPI_URL is not set; raspi stage will be skipped"
        );
    }
}

fn log_tts_startup_warnings() {
    let ai_cfg = config::ai_config();
    if tts_stage_count(ai_cfg) == 0 {
        log::warn!("[tts] no TTS stages enabled (TTS_LOCAL_SERVER_ENABLED=0, TTS_RASPI_ENABLED=0)");
    }
    if ai_cfg.tts_raspi_enabled && ai_cfg.tts_raspi_base_url.is_none() {
        log::warn!(
            "[tts] TTS_RASPI_ENABLED=1 but TTS_RASPI_BASE_URL is not set; raspi stage will be skipped"
        );
    }
}

fn log_intent_startup_warnings() {
    let ai_cfg = config::ai_config();
    if intent_stage_count(ai_cfg) == 0 {
        log::warn!(
            "[intent] no intent stages enabled (INTENT_LOCAL_SERVER_ENABLED=0, INTENT_RASPI_ENABLED=0)"
        );
    }
    if ai_cfg.intent_raspi_enabled && ai_cfg.intent_raspi_url.is_none() {
        log::warn!(
            "[intent] INTENT_RASPI_ENABLED=1 but INTENT_RASPI_URL is not set; raspi stage will be skipped"
        );
    }
}

async fn transcribe_with_http_asr(
    url: &str,
    wav_path: &str,
    http_timeout: Duration,
) -> Result<String> {
    let client = http_client(http_timeout)?;
    let bytes = tokio::fs::read(wav_path).await?;

    let part = multipart::Part::bytes(bytes)
        .file_name("question.wav")
        .mime_str("audio/wav")?;

    let form = multipart::Form::new().part("file", part);

    let resp = client.post(url).multipart(form).send().await?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("whisper error: {} - {}", status, body);
    }

    let result: WhisperResponse = resp.json().await?;
    Ok(result.text)
}

async fn try_asr_stage<F, Fut>(
    call_id: &str,
    stage: AsrStage,
    stage_timeout: Duration,
    run: F,
) -> Option<String>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<String>>,
{
    let stage_name = stage.as_str();
    log::debug!(
        "[asr {call_id}] ASR stage start: asr_stage={} timeout_ms={}",
        stage_name,
        stage_timeout.as_millis()
    );

    let text = match timeout(stage_timeout, run()).await {
        Ok(Ok(text)) => text,
        Ok(Err(err)) => {
            log::warn!(
                "[asr {call_id}] ASR stage failed: asr_stage={} reason={}",
                stage_name,
                err
            );
            return None;
        }
        Err(_) => {
            log::warn!(
                "[asr {call_id}] ASR stage failed: asr_stage={} reason=timeout timeout_ms={}",
                stage_name,
                stage_timeout.as_millis()
            );
            return None;
        }
    };

    let trimmed = text.trim();
    if trimmed.is_empty() {
        log::warn!(
            "[asr {call_id}] ASR stage failed: asr_stage={} reason=empty_text",
            stage_name
        );
        return None;
    }

    if asr::is_hallucination(trimmed) {
        log::warn!(
            "[asr {call_id}] ASR stage failed: asr_stage={} reason=hallucination_filtered",
            stage_name
        );
        log::debug!(
            "[asr {call_id}] ASR hallucination filtered: asr_stage={} text={}",
            stage_name,
            mask_pii(trimmed)
        );
        return None;
    }

    info!(
        "[asr {call_id}] ASR stage success: asr_stage={} text_len={}",
        stage_name,
        trimmed.chars().count()
    );
    Some(text)
}

async fn try_llm_stage<F, Fut>(
    call_id: &str,
    stage: LlmStage,
    stage_timeout: Duration,
    run: F,
) -> Option<String>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<String>>,
{
    let stage_name = stage.as_str();
    log::debug!(
        "[llm {call_id}] LLM stage start: llm_stage={} timeout_ms={}",
        stage_name,
        stage_timeout.as_millis()
    );

    let text = match timeout(stage_timeout, run()).await {
        Ok(Ok(text)) => text,
        Ok(Err(err)) => {
            log::warn!(
                "[llm {call_id}] LLM stage failed: llm_stage={} reason={}",
                stage_name,
                err
            );
            return None;
        }
        Err(_) => {
            log::warn!(
                "[llm {call_id}] LLM stage failed: llm_stage={} reason=timeout timeout_ms={}",
                stage_name,
                stage_timeout.as_millis()
            );
            return None;
        }
    };

    info!(
        "[llm {call_id}] LLM stage success: llm_stage={} text_len={}",
        stage_name,
        text.chars().count()
    );
    Some(text)
}

async fn try_tts_stage<F, Fut>(
    call_id: &str,
    stage: TtsStage,
    stage_timeout: Duration,
    run: F,
) -> Option<Vec<u8>>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<Vec<u8>>>,
{
    let stage_name = stage.as_str();
    log::debug!(
        "[tts {call_id}] TTS stage start: tts_stage={} timeout_ms={}",
        stage_name,
        stage_timeout.as_millis()
    );

    let wav_bytes = match timeout(stage_timeout, run()).await {
        Ok(Ok(wav_bytes)) => wav_bytes,
        Ok(Err(err)) => {
            log::warn!(
                "[tts {call_id}] TTS stage failed: tts_stage={} reason={}",
                stage_name,
                err
            );
            return None;
        }
        Err(_) => {
            log::warn!(
                "[tts {call_id}] TTS stage failed: tts_stage={} reason=timeout timeout_ms={}",
                stage_name,
                stage_timeout.as_millis()
            );
            return None;
        }
    };

    info!(
        "[tts {call_id}] TTS stage success: tts_stage={} wav_size={}",
        stage_name,
        wav_bytes.len()
    );
    Some(wav_bytes)
}

/// ASR 実行（現行実装を拡張）: cloud(AWS) -> local server -> raspi server の3段フォールバック。
pub async fn transcribe_and_log(call_id: &str, wav_path: &str) -> Result<String> {
    let ai_cfg = config::ai_config();

    if asr_stage_count(ai_cfg) == 0 {
        log::error!("[asr {call_id}] ASR failed: reason=all ASR stages disabled");
        anyhow::bail!("all ASR stages failed");
    }

    if ai_cfg.use_aws_transcribe {
        if let Some(text) =
            try_asr_stage(call_id, AsrStage::Cloud, ai_cfg.asr_cloud_timeout, || {
                transcribe_with_aws(wav_path)
            })
            .await
        {
            return Ok(text);
        }
    }

    if ai_cfg.asr_local_server_enabled {
        let local_url = ai_cfg.asr_local_server_url.clone();
        if let Some(text) =
            try_asr_stage(call_id, AsrStage::Local, ai_cfg.asr_local_timeout, || {
                transcribe_with_http_asr(&local_url, wav_path, ai_cfg.asr_local_timeout)
            })
            .await
        {
            return Ok(text);
        }
    }

    if ai_cfg.asr_raspi_enabled {
        if let Some(raspi_url) = ai_cfg.asr_raspi_url.clone() {
            if let Some(text) =
                try_asr_stage(call_id, AsrStage::Raspi, ai_cfg.asr_raspi_timeout, || {
                    transcribe_with_http_asr(&raspi_url, wav_path, ai_cfg.asr_raspi_timeout)
                })
                .await
            {
                return Ok(text);
            }
        } else {
            log::warn!(
                "[asr {call_id}] ASR stage failed: asr_stage=raspi reason=ASR_RASPI_URL missing"
            );
        }
    }

    log::error!("[asr {call_id}] ASR failed: reason=all ASR stages failed");
    anyhow::bail!("all ASR stages failed")
}

/// LLM + TTS 実行（現行実装: Gemini→Ollama fallback→ずんだもんTTS）。挙動は変更なし。
/// I/F はテキスト入力→WAVパス出力（将来はチャネル/PCM化予定、現状は一時ファイルのまま）。
pub async fn handle_user_question_from_whisper(messages: Vec<ChatMessage>) -> Result<String> {
    let answer = match handle_user_question_from_whisper_llm_only("standalone", messages).await {
        Ok(answer) => answer,
        Err(err) => {
            log::error!(
                "[llm standalone] all LLM stages failed in handle_user_question_from_whisper: {err:?}"
            );
            "すみません、うまく答えを用意できませんでした。".to_string()
        }
    };

    // 一時WAVファイル経由のまま（責務は ai モジュール内に閉じ込める）
    let answer_wav = "/tmp/ollama_answer.wav";
    synth_zundamon_wav("standalone", &answer, answer_wav).await?;

    Ok(answer_wav.to_string())
}

/// LLM 部分のみを切り出した I/F（app→ai で分離できるようにする）
pub async fn handle_user_question_from_whisper_llm_only(
    call_id: &str,
    messages: Vec<ChatMessage>,
) -> Result<String> {
    if let Some(last_user) = messages.iter().rev().find(|m| m.role == Role::User) {
        log::debug!(
            "[llm {call_id}] user question: {}",
            mask_pii(&last_user.content)
        );
    }

    let ai_cfg = config::ai_config();
    if llm_stage_count(ai_cfg) == 0 {
        log::error!("[llm {call_id}] LLM failed: reason=all LLM stages disabled");
        anyhow::bail!("all LLM stages failed");
    }

    if llm_cloud_enabled(ai_cfg) {
        if let Some(text) =
            try_llm_stage(call_id, LlmStage::Cloud, ai_cfg.llm_cloud_timeout, || {
                call_gemini_with_http_timeout(&messages, ai_cfg.llm_cloud_timeout)
            })
            .await
        {
            return Ok(text);
        }
    }

    let system_prompt = llm::system_prompt();

    if ai_cfg.llm_local_server_enabled {
        let local_url = ai_cfg.llm_local_server_url.clone();
        let local_model = ai_cfg.llm_local_model.clone();
        if let Some(text) =
            try_llm_stage(call_id, LlmStage::Local, ai_cfg.llm_local_timeout, || {
                call_ollama_for_stage(
                    &messages,
                    &system_prompt,
                    &local_model,
                    &local_url,
                    ai_cfg.llm_local_timeout,
                )
            })
            .await
        {
            return Ok(text);
        }
    }

    if ai_cfg.llm_raspi_enabled {
        if let Some(raspi_url) = ai_cfg.llm_raspi_url.clone() {
            let raspi_model = ai_cfg.llm_raspi_model.clone();
            if let Some(text) =
                try_llm_stage(call_id, LlmStage::Raspi, ai_cfg.llm_raspi_timeout, || {
                    call_ollama_for_stage(
                        &messages,
                        &system_prompt,
                        &raspi_model,
                        &raspi_url,
                        ai_cfg.llm_raspi_timeout,
                    )
                })
                .await
            {
                return Ok(text);
            }
        } else {
            log::warn!(
                "[llm {call_id}] LLM stage failed: llm_stage=raspi reason=LLM_RASPI_URL missing"
            );
        }
    }

    log::error!("[llm {call_id}] LLM failed: reason=all LLM stages failed");
    anyhow::bail!("all LLM stages failed")
}

#[allow(dead_code)]
async fn call_ollama(messages: &[ChatMessage]) -> Result<String> {
    let model = config::ai_config().ollama_model.clone();
    let system_prompt = llm::system_prompt();
    call_ollama_with_prompt(messages, &system_prompt, &model).await
}

async fn call_ollama_for_stage(
    messages: &[ChatMessage],
    system_prompt: &str,
    model: &str,
    endpoint_url: &str,
    http_timeout: Duration,
) -> Result<String> {
    call_ollama_with_prompt_internal(messages, system_prompt, model, endpoint_url, http_timeout)
        .await
}

pub(crate) async fn call_ollama_for_intent_stage(
    messages: &[ChatMessage],
    system_prompt: &str,
    model: &str,
    endpoint_url: &str,
    http_timeout: Duration,
) -> Result<String> {
    let client = http_client(http_timeout)?;

    let mut ollama_messages = Vec::with_capacity(messages.len() + 1);
    ollama_messages.push(OllamaMessage {
        role: "system".to_string(),
        content: system_prompt.to_string(),
    });
    for msg in messages {
        let role = match msg.role {
            Role::User => "user",
            Role::Assistant => "assistant",
        };
        ollama_messages.push(OllamaMessage {
            role: role.to_string(),
            content: msg.content.clone(),
        });
    }

    let req = OllamaChatRequest {
        model: model.to_string(),
        messages: ollama_messages,
        stream: false,
    };

    let resp = client.post(endpoint_url).json(&req).send().await?;
    let status = resp.status();
    let body_text = resp.text().await?;
    if !status.is_success() {
        anyhow::bail!(
            "Ollama HTTP error {} (body_len={})",
            status,
            body_text.len()
        );
    }

    #[derive(Deserialize)]
    struct ChatResponse {
        message: Option<OllamaMessage>,
    }

    let body: ChatResponse = serde_json::from_str(&body_text)?;
    let answer = body
        .message
        .map(|m| m.content)
        .unwrap_or_else(|| "<no response>".to_string());

    Ok(answer)
}

pub(crate) async fn call_ollama_with_prompt(
    messages: &[ChatMessage],
    system_prompt: &str,
    model: &str,
) -> Result<String> {
    call_ollama_with_prompt_internal(
        messages,
        system_prompt,
        model,
        "http://localhost:11434/api/chat",
        config::timeouts().ai_http,
    )
    .await
}

async fn call_ollama_with_prompt_internal(
    messages: &[ChatMessage],
    system_prompt: &str,
    model: &str,
    endpoint_url: &str,
    http_timeout: Duration,
) -> Result<String> {
    let client = http_client(http_timeout)?;

    let mut ollama_messages = Vec::with_capacity(messages.len() + 1);
    ollama_messages.push(OllamaMessage {
        role: "system".to_string(),
        content: system_prompt.to_string(),
    });
    for msg in messages {
        let role = match msg.role {
            Role::User => "user",
            Role::Assistant => "assistant",
        };
        ollama_messages.push(OllamaMessage {
            role: role.to_string(),
            content: msg.content.clone(),
        });
    }

    let req = OllamaChatRequest {
        model: model.to_string(),
        messages: ollama_messages,
        stream: false,
    };

    let resp = client.post(endpoint_url).json(&req).send().await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    info!("Ollama status: {}", status);
    info!("Ollama raw body: {}", body_text);

    if !status.is_success() {
        anyhow::bail!("Ollama HTTP error {}: {}", status, body_text);
    }

    #[derive(Deserialize)]
    struct ChatResponse {
        message: Option<OllamaMessage>,
    }

    let body: ChatResponse = serde_json::from_str(&body_text)?;

    let answer = body
        .message
        .map(|m| m.content)
        .unwrap_or_else(|| "<no response>".to_string());

    Ok(answer)
}

pub struct DefaultAiPort;

impl DefaultAiPort {
    pub fn new() -> Self {
        log_asr_startup_warnings();
        log_llm_startup_warnings();
        log_tts_startup_warnings();
        log_intent_startup_warnings();
        Self
    }
}

impl Default for DefaultAiPort {
    fn default() -> Self {
        Self::new()
    }
}

impl AsrPort for DefaultAiPort {
    /// Transcribes a sequence of ASR audio chunks for a given call and returns the resulting transcript.
    ///
    /// # Parameters
    ///
    /// - `call_id`: identifier for the call/session associated with the chunks (used for tracking/logging).
    /// - `chunks`: list of `AsrChunk` items representing the audio segments to transcribe.
    ///
    /// # Returns
    ///
    /// `Ok(String)` with the full transcript on success, or an `Err` describing the failure.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use virtual_voicebot_backend::service::ai::DefaultAiPort;
    /// use virtual_voicebot_backend::ports::ai::AsrPort;
    ///
    /// # async fn example() {
    /// let port = DefaultAiPort::new();
    /// let transcript = port.transcribe_chunks("call-1".to_string(), vec![]).await;
    /// let _ = transcript;
    /// # }
    /// ```
    fn transcribe_chunks(
        &self,
        call_id: String,
        chunks: Vec<AsrChunk>,
    ) -> AiFuture<Result<String, AsrError>> {
        Box::pin(async move {
            asr::transcribe_chunks(&call_id, &chunks)
                .await
                .map_err(|e| AsrError::TranscriptionFailed(e.to_string()))
        })
    }
}

impl IntentPort for DefaultAiPort {
    fn classify_intent(
        &self,
        call_id: String,
        text: String,
    ) -> AiFuture<Result<String, IntentError>> {
        Box::pin(async move {
            intent::classify_intent(&call_id, text)
                .await
                .map_err(|e| IntentError::ClassificationFailed(e.to_string()))
        })
    }
}

impl LlmPort for DefaultAiPort {
    fn generate_answer(
        &self,
        call_id: String,
        messages: Vec<ChatMessage>,
    ) -> AiFuture<Result<String, LlmError>> {
        Box::pin(async move {
            llm::generate_answer(&call_id, messages)
                .await
                .map_err(|e| LlmError::GenerationFailed(e.to_string()))
        })
    }
}

impl WeatherPort for DefaultAiPort {
    fn handle_weather(&self, query: WeatherQuery) -> AiFuture<Result<String, WeatherError>> {
        Box::pin(async move {
            weather::handle_weather(query)
                .await
                .map_err(|e| WeatherError::QueryFailed(e.to_string()))
        })
    }
}

impl TtsPort for DefaultAiPort {
    fn synth_to_wav(
        &self,
        call_id: String,
        text: String,
        path: Option<String>,
    ) -> AiFuture<Result<std::path::PathBuf, TtsError>> {
        Box::pin(async move {
            tts::synth_to_wav(&call_id, &text, path.as_deref())
                .await
                .map(std::path::PathBuf::from)
                .map_err(|e| TtsError::SynthesisFailed(e.to_string()))
        })
    }
}

impl SerPort for DefaultAiPort {
    fn analyze(
        &self,
        input: SerInputPcm,
    ) -> AiFuture<Result<SerOutcome, crate::shared::error::ai::SerError>> {
        Box::pin(async move { ser::analyze(input).await })
    }
}

/// ずんだもん TTS の呼び出し。I/F はテキストと出力 WAV パス（従来どおり）。
pub async fn synth_zundamon_wav(call_id: &str, text: &str, out_path: &str) -> Result<()> {
    let ai_cfg = config::ai_config();

    if tts_stage_count(ai_cfg) == 0 {
        log::error!("[tts {call_id}] TTS failed: reason=all TTS stages disabled");
        anyhow::bail!("all TTS stages failed");
    }

    if ai_cfg.tts_local_server_enabled {
        let local_base_url = ai_cfg.tts_local_server_base_url.clone();
        if let Some(wav_bytes) =
            try_tts_stage(call_id, TtsStage::Local, ai_cfg.tts_local_timeout, || {
                synth_zundamon_for_stage(&local_base_url, call_id, text, ai_cfg.tts_local_timeout)
            })
            .await
        {
            tokio::fs::write(out_path, &wav_bytes).await?;
            info!("[tts {call_id}] Zundamon TTS written to {}", out_path);
            return Ok(());
        }
    }

    if ai_cfg.tts_raspi_enabled {
        if let Some(raspi_base_url) = ai_cfg.tts_raspi_base_url.clone() {
            if let Some(wav_bytes) =
                try_tts_stage(call_id, TtsStage::Raspi, ai_cfg.tts_raspi_timeout, || {
                    synth_zundamon_for_stage(
                        &raspi_base_url,
                        call_id,
                        text,
                        ai_cfg.tts_raspi_timeout,
                    )
                })
                .await
            {
                tokio::fs::write(out_path, &wav_bytes).await?;
                info!("[tts {call_id}] Zundamon TTS written to {}", out_path);
                return Ok(());
            }
        } else {
            log::warn!(
                "[tts {call_id}] TTS stage failed: tts_stage=raspi reason=TTS_RASPI_BASE_URL missing"
            );
        }
    }

    log::error!("[tts {call_id}] TTS failed: reason=all TTS stages failed");
    anyhow::bail!("all TTS stages failed")
}

fn tts_endpoint_url(base_url: &str, path: &str) -> String {
    format!(
        "{}/{}",
        base_url.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

async fn synth_zundamon_for_stage(
    base_url: &str,
    _call_id: &str,
    text: &str,
    http_timeout: Duration,
) -> Result<Vec<u8>> {
    let client = http_client(http_timeout)?;
    let speaker_id = 3; // ずんだもん ノーマル

    let query_url = tts_endpoint_url(base_url, "/audio_query");
    let query_resp = client
        .post(query_url)
        .query(&[("text", text), ("speaker", &speaker_id.to_string())])
        .send()
        .await?;

    let status = query_resp.status();
    let query_body = query_resp.text().await?;
    if !status.is_success() {
        anyhow::bail!("audio_query error {} ({} bytes)", status, query_body.len());
    }

    let synth_url = tts_endpoint_url(base_url, "/synthesis");
    let synth_resp = client
        .post(synth_url)
        .query(&[("speaker", &speaker_id.to_string())])
        .header("Content-Type", "application/json")
        .body(query_body)
        .send()
        .await?;

    let status = synth_resp.status();
    let wav_bytes = synth_resp.bytes().await?;
    if !status.is_success() {
        anyhow::bail!("synthesis error {} ({} bytes)", status, wav_bytes.len());
    }

    Ok(wav_bytes.to_vec())
}

/// Call Google's Gemini (Generative Language) API with a sequence of chat messages and return the model's reply.
///
/// The messages are converted into Gemini `contents` with the module's system prompt prepended; the function POSTs the assembled `GeminiRequest` to the Generative Language API and returns the text of the first candidate's first part. If Gemini returns no candidates or parts, the function returns `"<no response>"`.
///
/// # Errors
///
/// Returns an error if required configuration (API key) is missing, the HTTP request fails, or the response cannot be parsed.
///
/// # Returns
///
/// The first candidate's first part text from Gemini, or `"<no response>"` if the response contains no usable content.
///
/// # Examples
///
/// ```ignore
/// # use crate::service::ai::llm::{call_gemini, ChatMessage, Role};
/// # tokio_test::block_on(async {
/// let msgs = vec![ChatMessage { role: Role::User, content: "Hello".into() }];
/// let reply = call_gemini(&msgs).await.unwrap();
/// println!("{}", reply);
/// # });
/// ```
#[allow(dead_code)]
async fn call_gemini(messages: &[ChatMessage]) -> Result<String> {
    call_gemini_with_http_timeout(messages, config::timeouts().ai_http).await
}

async fn call_gemini_with_http_timeout(
    messages: &[ChatMessage],
    http_timeout: Duration,
) -> Result<String> {
    let client = http_client(http_timeout)?;

    let ai_cfg = config::ai_config();
    let api_key = ai_cfg
        .gemini_api_key
        .as_deref()
        .ok_or_else(|| anyhow!("GEMINI_API_KEY must be set"))?;
    let model = ai_cfg.gemini_model.as_str();

    let url = format!(
        "https://generativelanguage.googleapis.com/v1/models/{}:generateContent?key={}",
        model, api_key
    );

    let mut contents = Vec::with_capacity(messages.len() + 1);
    let system_prompt = llm::system_prompt();
    contents.push(GeminiContent {
        role: Some("user".to_string()),
        parts: vec![GeminiPart {
            text: system_prompt,
        }],
    });
    for msg in messages {
        let role = match msg.role {
            Role::User => "user",
            Role::Assistant => "model",
        };
        contents.push(GeminiContent {
            role: Some(role.to_string()),
            parts: vec![GeminiPart {
                text: msg.content.clone(),
            }],
        });
    }

    let req_body = GeminiRequest { contents };

    let resp = client.post(&url).json(&req_body).send().await?;
    let status = resp.status();
    let body_text = resp.text().await?;

    info!("Gemini status: {}", status);
    info!("Gemini raw body: {}", body_text);

    if !status.is_success() {
        anyhow::bail!("Gemini HTTP error {}: {}", status, body_text);
    }

    let body: GeminiResponse = serde_json::from_str(&body_text)?;

    let answer = body
        .candidates
        .as_ref()
        .and_then(|cands| cands.first())
        .and_then(|cand| cand.content.parts.first())
        .map(|p| p.text.clone())
        .unwrap_or_else(|| "<no response>".to_string());

    Ok(answer)
}

/// Uploads a WAV file to S3, starts an AWS Transcribe job, and returns the resulting transcript.
///
/// The function prepares the WAV for AWS Transcribe (mono 16-bit 16 kHz when required), uploads it to the configured
/// S3 bucket and prefix, starts a transcription job with a timestamped job name, polls until completion, and returns
/// the final transcript text.
///
/// # Parameters
///
/// - `wav_path`: Path to the local WAV file to transcribe.
///
/// # Returns
///
/// `Ok(String)` containing the transcribed text on success, or an `Err` if any step (preparation, upload, job start,
/// polling, or transcript retrieval/parsing) fails.
///
/// # Examples
///
/// ```ignore
/// # tokio_test::block_on(async {
/// let transcript = crate::service::ai::transcribe_with_aws("/tmp/example.wav").await;
/// match transcript {
///     Ok(text) => println!("Transcript: {}", text),
///     Err(e) => eprintln!("Transcription failed: {}", e),
/// }
/// # });
/// ```
async fn transcribe_with_aws(wav_path: &str) -> Result<String> {
    let ai_cfg = config::ai_config();
    let bucket = ai_cfg
        .aws_transcribe_bucket
        .as_deref()
        .ok_or_else(|| anyhow!("AWS_TRANSCRIBE_BUCKET must be set when USE_AWS_TRANSCRIBE=1"))?;
    let prefix = ai_cfg.aws_transcribe_prefix.as_str();

    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let job_name = format!("voicebot-{}", timestamp);

    let normalized_prefix = if prefix.is_empty() {
        String::new()
    } else if prefix.ends_with('/') {
        prefix.to_string()
    } else {
        format!("{}/", prefix)
    };
    let object_key = format!("{}{}.wav", normalized_prefix, job_name);

    let region_provider = RegionProviderChain::default_provider().or_default_provider();
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;

    let wav_bytes = prepare_wav_for_transcribe(wav_path)?;
    let body_stream = ByteStream::from(wav_bytes);
    let s3_client = s3::Client::new(&config);
    info!("Uploading audio to s3://{}/{}", bucket, object_key);
    s3_client
        .put_object()
        .bucket(bucket)
        .key(&object_key)
        .body(body_stream)
        .content_type("audio/wav")
        .send()
        .await?;

    let s3_uri = format!("s3://{}/{}", bucket, object_key);
    transcribe_with_aws_job(&config, &s3_uri, &job_name).await
}

async fn transcribe_with_aws_job(
    config: &SdkConfig,
    s3_uri: &str,
    job_name: &str,
) -> Result<String> {
    let http = http_client(crate::shared::config::timeouts().ai_http)?;
    let client = transcribe::Client::new(config);

    let media = transcribe::types::Media::builder()
        .media_file_uri(s3_uri)
        .build();

    client
        .start_transcription_job()
        .transcription_job_name(job_name)
        .language_code(transcribe::types::LanguageCode::JaJp)
        .media(media)
        .media_format(transcribe::types::MediaFormat::Wav)
        .send()
        .await?;

    loop {
        let resp = client
            .get_transcription_job()
            .transcription_job_name(job_name)
            .send()
            .await?;

        if let Some(job) = resp.transcription_job() {
            use transcribe::types::TranscriptionJobStatus as Status;
            match job.transcription_job_status() {
                Some(Status::Completed) => {
                    if let Some(uri) = job.transcript().and_then(|t| t.transcript_file_uri()) {
                        let resp = http.get(uri).send().await?;
                        let body_text = resp.text().await?;

                        log::info!("AWS transcript raw JSON: {}", mask_pii(&body_text));

                        let transcript = parse_aws_transcript(&body_text)?;
                        return Ok(transcript);
                    } else {
                        anyhow::bail!("Transcribe job completed but transcript URI missing");
                    }
                }
                Some(Status::Failed) => {
                    anyhow::bail!("Transcribe job failed: {:?}", job.failure_reason());
                }
                _ => {
                    sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }
}

fn parse_aws_transcript(body_text: &str) -> Result<String> {
    let value: Value = serde_json::from_str(body_text)?;
    let transcript = value["results"]["transcripts"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|first| first.get("transcript"))
        .and_then(|node| node.as_str())
        .ok_or_else(|| anyhow!("Transcript JSON missing text"))?;
    Ok(transcript.to_string())
}

fn prepare_wav_for_transcribe(wav_path: &str) -> Result<Vec<u8>> {
    const TARGET_RATE: u32 = 16_000;

    let mut reader = hound::WavReader::open(wav_path)?;
    let spec = reader.spec();
    if spec.channels != 1 || spec.bits_per_sample != 16 {
        anyhow::bail!(
            "Expected mono 16-bit WAV for AWS Transcribe, got {} ch / {} bits",
            spec.channels,
            spec.bits_per_sample
        );
    }

    if spec.sample_rate == TARGET_RATE {
        return Ok(fs::read(wav_path)?);
    }

    let mut samples: Vec<i16> = Vec::new();
    for s in reader.samples::<i16>() {
        samples.push(s?);
    }

    let mut new_spec = spec;
    new_spec.sample_rate = TARGET_RATE;

    let mut output: Vec<i16> = Vec::new();
    if spec.sample_rate == 8_000 {
        output.reserve(samples.len() * 2);
        for sample in samples {
            output.push(sample);
            output.push(sample);
        }
    } else {
        log::warn!(
            "Unexpected WAV sample rate {} Hz, sending original file to AWS Transcribe",
            spec.sample_rate
        );
        drop(reader);
        return Ok(fs::read(wav_path)?);
    }

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor, new_spec)?;
        for sample in output {
            writer.write_sample(sample)?;
        }
        writer.finalize()?;
    }
    Ok(cursor.into_inner())
}

#[cfg(test)]
mod tests {
    use super::tts_endpoint_url;

    #[test]
    fn tts_endpoint_url_handles_slashes() {
        assert_eq!(
            tts_endpoint_url("http://localhost:50021/", "/audio_query"),
            "http://localhost:50021/audio_query"
        );
        assert_eq!(
            tts_endpoint_url("http://localhost:50021", "synthesis"),
            "http://localhost:50021/synthesis"
        );
    }
}
