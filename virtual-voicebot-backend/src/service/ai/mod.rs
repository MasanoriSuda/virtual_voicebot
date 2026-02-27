//! ai モジュール: ASR/LLM/TTS の外部サービスクライアント。
//! - 外部 I/O（HTTP/AWS SDK、ローカル WAV 一時ファイル）をここに閉じ込める。
//! - app/session にはテキスト/PCM 抽象のみ渡す想定だが、現状は直接関数を呼ぶ。
//! - ポリシー（タイムアウト/リトライ/フォールバック）は既存のまま。

use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt as FuturesStreamExt};
use log::info;
use reqwest::{multipart, Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::future::Future;
use std::io::Cursor;
use std::pin::Pin;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::time::{sleep, timeout};
use tokio_stream::wrappers::ReceiverStream;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message as WsMessage;

use aws_config::meta::region::RegionProviderChain;
use aws_config::{BehaviorVersion, SdkConfig};
use aws_sdk_s3 as s3;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_transcribe as transcribe;

use crate::shared::config;
use crate::shared::error::ai::{AsrError, IntentError, LlmError, TtsError, WeatherError};
use crate::shared::ports::ai::{
    asr_audio_channel, AiFuture, AsrAudioMsg, AsrChunk, AsrPort, AsrStreamEvent, AsrStreamHandle,
    AsrStreamPort, ChatMessage, IntentPort, LlmPort, LlmStream, LlmStreamEvent, LlmStreamPort,
    Role, SerInputPcm, SerOutcome, SerPort, TtsPort, TtsStream, TtsStreamPort, WeatherPort,
    WeatherQuery,
};
use crate::shared::utils::mask_pii;

#[derive(Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaChatStreamResponse {
    message: Option<OllamaMessage>,
    done: Option<bool>,
    error: Option<String>,
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

#[derive(Serialize)]
struct OpenAiChatCompletionsRequest {
    model: String,
    messages: Vec<OpenAiChatMessage>,
}

#[derive(Serialize)]
struct OpenAiChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAiChatCompletionsResponse {
    choices: Vec<OpenAiChatChoice>,
}

#[derive(Deserialize)]
struct OpenAiChatChoice {
    message: OpenAiChatMessageOut,
}

#[derive(Deserialize)]
struct OpenAiChatMessageOut {
    content: Option<String>,
}

#[derive(Serialize)]
struct OpenAiSpeechRequest {
    model: String,
    voice: String,
    input: String,
    response_format: String,
}

#[derive(Deserialize)]
struct WhisperResponse {
    text: String,
}

#[derive(Serialize)]
struct WhisperStreamEndRequest {
    #[serde(rename = "type")]
    kind: &'static str,
}

#[derive(Deserialize)]
struct WhisperStreamEventResponse {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    error: Option<String>,
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

fn http_client_for_stream(connect_timeout: Duration) -> Result<Client> {
    Ok(Client::builder().connect_timeout(connect_timeout).build()?)
}

const LOCAL_SERVICE_PROBE_TIMEOUT_MS: u64 = 2_000;

#[derive(Serialize)]
pub struct LocalServicesStatusResponse {
    ok: bool,
    #[serde(rename = "localServices")]
    local_services: LocalServicesMap,
}

#[derive(Serialize)]
struct LocalServicesMap {
    asr: LocalServiceEntry,
    llm: LocalServiceEntry,
    tts: LocalServiceEntry,
}

#[derive(Serialize)]
struct LocalServiceEntry {
    status: &'static str,
    #[serde(rename = "displayUrl")]
    display_url: Option<String>,
}

pub async fn probe_local_services_status(
    ai_cfg: &config::AiConfig,
) -> Result<LocalServicesStatusResponse> {
    let client = Client::builder()
        .timeout(Duration::from_millis(LOCAL_SERVICE_PROBE_TIMEOUT_MS))
        .build()?;

    let asr_probe_base = extract_base_url(&ai_cfg.asr_local_server_url);
    let asr_display_base = sanitized_display_url(&asr_probe_base);
    let llm_probe_base = extract_base_url(&ai_cfg.llm_local_server_url);
    let llm_display_base = sanitized_display_url(&llm_probe_base);
    let tts_probe_base = ai_cfg
        .tts_local_server_base_url
        .trim_end_matches('/')
        .to_string();
    let tts_display_base = sanitized_display_url(&tts_probe_base);

    let (asr, llm, tts) = tokio::join!(
        probe_service_entry(
            client.clone(),
            ai_cfg.asr_local_server_enabled,
            asr_probe_base,
            asr_display_base,
            "/healthz",
        ),
        probe_service_entry(
            client.clone(),
            ai_cfg.llm_local_server_enabled,
            llm_probe_base,
            llm_display_base,
            "/api/tags",
        ),
        probe_service_entry(
            client,
            ai_cfg.tts_local_server_enabled,
            tts_probe_base,
            tts_display_base,
            "/speakers",
        ),
    );

    Ok(LocalServicesStatusResponse {
        ok: true,
        local_services: LocalServicesMap { asr, llm, tts },
    })
}

fn disabled_local_service_entry() -> LocalServiceEntry {
    LocalServiceEntry {
        status: "disabled",
        display_url: None,
    }
}

async fn probe_service_entry(
    client: Client,
    enabled: bool,
    probe_base_url: String,
    display_url: String,
    probe_path: &str,
) -> LocalServiceEntry {
    if !enabled {
        return disabled_local_service_entry();
    }

    let probe_url = join_url_path(&probe_base_url, probe_path);
    let status = if probe_once(&client, &probe_url).await {
        "ok"
    } else {
        "error"
    };

    LocalServiceEntry {
        status,
        display_url: Some(display_url),
    }
}

async fn probe_once(client: &Client, url: &str) -> bool {
    match client.get(url).send().await {
        Ok(response) => response.status() == StatusCode::OK,
        Err(_) => false,
    }
}

fn extract_base_url(url: &str) -> String {
    let url = url.trim_end_matches('/');
    if let Some(after_scheme) = url.strip_prefix("https://") {
        let host_part = after_scheme.split('/').next().unwrap_or(after_scheme);
        return format!("https://{host_part}");
    }
    if let Some(after_scheme) = url.strip_prefix("http://") {
        let host_part = after_scheme.split('/').next().unwrap_or(after_scheme);
        return format!("http://{host_part}");
    }

    // AiConfig の URL は通常スキーム付き想定だが、防御的に path を除去して返す。
    url.split('/').next().unwrap_or(url).to_string()
}

fn sanitized_display_url(base_url: &str) -> String {
    let base_url = base_url.trim_end_matches('/');
    if let Ok(mut parsed) = reqwest::Url::parse(base_url) {
        let _ = parsed.set_username("");
        let _ = parsed.set_password(None);
        return parsed.to_string().trim_end_matches('/').to_string();
    }

    if let Some((scheme, rest)) = base_url.split_once("://") {
        let host_part = rest.rsplit_once('@').map_or(rest, |(_, host)| host);
        return format!("{scheme}://{host_part}");
    }

    base_url
        .rsplit_once('@')
        .map_or(base_url, |(_, host)| host)
        .to_string()
}

fn join_url_path(base_url: &str, path: &str) -> String {
    format!(
        "{}/{}",
        base_url.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

const OPENAI_ASR_MODEL: &str = "gpt-4o-mini-transcribe";
const OPENAI_LLM_MODEL: &str = "gpt-4o-mini";
const OPENAI_TTS_MODEL: &str = "gpt-4o-mini-tts";
const OPENAI_TTS_VOICE: &str = "alloy";
const OPENAI_TTS_PCM_SAMPLE_RATE: u32 = 24_000;

fn openai_endpoint_url(base_url: &str, path: &str) -> String {
    format!(
        "{}/{}",
        base_url.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

fn openai_api_key(ai_cfg: &config::AiConfig) -> Option<&str> {
    ai_cfg
        .openai_api_key
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty())
}

fn openai_asr_stage_enabled(ai_cfg: &config::AiConfig) -> bool {
    ai_cfg.openai_asr_enabled && openai_api_key(ai_cfg).is_some()
}

fn openai_llm_stage_enabled(ai_cfg: &config::AiConfig) -> bool {
    ai_cfg.openai_llm_enabled && openai_api_key(ai_cfg).is_some()
}

fn openai_tts_stage_enabled(ai_cfg: &config::AiConfig) -> bool {
    ai_cfg.openai_tts_enabled && openai_api_key(ai_cfg).is_some()
}

fn openai_intent_stage_enabled(ai_cfg: &config::AiConfig) -> bool {
    ai_cfg.openai_intent_enabled && openai_api_key(ai_cfg).is_some()
}

fn asr_cloud_enabled(ai_cfg: &config::AiConfig) -> bool {
    openai_asr_stage_enabled(ai_cfg) || ai_cfg.use_aws_transcribe
}

fn gemini_llm_enabled(ai_cfg: &config::AiConfig) -> bool {
    ai_cfg
        .gemini_api_key
        .as_deref()
        .map(str::trim)
        .is_some_and(|key| !key.is_empty())
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

const ASR_FALLBACK_ORDER: [AsrStage; 3] = [AsrStage::Local, AsrStage::Cloud, AsrStage::Raspi];

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

const LLM_FALLBACK_ORDER: [LlmStage; 3] = [LlmStage::Local, LlmStage::Cloud, LlmStage::Raspi];

#[derive(Clone, Copy)]
enum TtsStage {
    Cloud,
    Local,
    Raspi,
}

impl TtsStage {
    fn as_str(self) -> &'static str {
        match self {
            Self::Cloud => "cloud",
            Self::Local => "local",
            Self::Raspi => "raspi",
        }
    }
}

const TTS_FALLBACK_ORDER: [TtsStage; 3] = [TtsStage::Local, TtsStage::Cloud, TtsStage::Raspi];

#[derive(Clone, Copy)]
enum IntentStage {
    Cloud,
    Local,
    Raspi,
}

impl IntentStage {
    fn as_str(self) -> &'static str {
        match self {
            Self::Cloud => "cloud",
            Self::Local => "local",
            Self::Raspi => "raspi",
        }
    }
}

fn asr_stage_count(ai_cfg: &config::AiConfig) -> usize {
    usize::from(asr_cloud_enabled(ai_cfg))
        + usize::from(ai_cfg.asr_local_server_enabled)
        + usize::from(ai_cfg.asr_raspi_enabled)
}

fn llm_cloud_enabled(ai_cfg: &config::AiConfig) -> bool {
    openai_llm_stage_enabled(ai_cfg) || gemini_llm_enabled(ai_cfg)
}

fn llm_stage_count(ai_cfg: &config::AiConfig) -> usize {
    usize::from(llm_cloud_enabled(ai_cfg))
        + usize::from(ai_cfg.llm_local_server_enabled)
        + usize::from(ai_cfg.llm_raspi_enabled)
}

fn tts_stage_count(ai_cfg: &config::AiConfig) -> usize {
    usize::from(openai_tts_stage_enabled(ai_cfg))
        + usize::from(ai_cfg.tts_local_server_enabled)
        + usize::from(ai_cfg.tts_raspi_enabled)
}

fn intent_stage_count(ai_cfg: &config::AiConfig) -> usize {
    usize::from(openai_intent_stage_enabled(ai_cfg))
        + usize::from(ai_cfg.intent_local_server_enabled)
        + usize::from(ai_cfg.intent_raspi_enabled)
}

fn log_asr_startup_warnings() {
    let ai_cfg = config::ai_config();
    if asr_stage_count(ai_cfg) == 0 {
        log::warn!(
            "[asr] no ASR stages enabled (OPENAI_ASR_ENABLED=0 or OPENAI_API_KEY missing, USE_AWS_TRANSCRIBE=0, ASR_LOCAL_SERVER_ENABLED=0, ASR_RASPI_ENABLED=0)"
        );
    }
    if ai_cfg.openai_asr_enabled && openai_api_key(ai_cfg).is_none() {
        log::warn!("[asr] OPENAI_ASR_ENABLED=1 but OPENAI_API_KEY is not set; openai cloud provider will be skipped");
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
            "[llm] no LLM stages enabled (OPENAI_LLM_ENABLED=0 or OPENAI_API_KEY missing, GEMINI_API_KEY missing, LLM_LOCAL_SERVER_ENABLED=0, LLM_RASPI_ENABLED=0)"
        );
    }
    if ai_cfg.openai_llm_enabled && openai_api_key(ai_cfg).is_none() {
        log::warn!("[llm] OPENAI_LLM_ENABLED=1 but OPENAI_API_KEY is not set; openai cloud provider will be skipped");
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
        log::warn!("[tts] no TTS stages enabled (OPENAI_TTS_ENABLED=0 or OPENAI_API_KEY missing, TTS_LOCAL_SERVER_ENABLED=0, TTS_RASPI_ENABLED=0)");
    }
    if ai_cfg.openai_tts_enabled && openai_api_key(ai_cfg).is_none() {
        log::warn!("[tts] OPENAI_TTS_ENABLED=1 but OPENAI_API_KEY is not set; openai cloud provider will be skipped");
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
            "[intent] no intent stages enabled (OPENAI_INTENT_ENABLED=0 or OPENAI_API_KEY missing, INTENT_LOCAL_SERVER_ENABLED=0, INTENT_RASPI_ENABLED=0)"
        );
    }
    if ai_cfg.openai_intent_enabled && openai_api_key(ai_cfg).is_none() {
        log::warn!("[intent] OPENAI_INTENT_ENABLED=1 but OPENAI_API_KEY is not set; openai cloud provider will be skipped");
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

const ASR_STREAM_AUDIO_CHANNEL_CAPACITY: usize = 32;

fn parse_whisper_stream_event_text(text: &str) -> std::result::Result<AsrStreamEvent, AsrError> {
    let parsed: WhisperStreamEventResponse = serde_json::from_str(text).map_err(|e| {
        AsrError::TranscriptionFailed(format!("invalid ASR stream event JSON: {e}"))
    })?;

    if let Some(err) = parsed.error {
        return Err(AsrError::TranscriptionFailed(err));
    }

    match parsed.kind.as_str() {
        "partial" => Ok(AsrStreamEvent::Partial(parsed.text.unwrap_or_default())),
        "final" => Ok(AsrStreamEvent::Final(parsed.text.unwrap_or_default())),
        other => Err(AsrError::TranscriptionFailed(format!(
            "unknown ASR stream event type: {other}"
        ))),
    }
}

async fn call_whisper_stream(
    call_id: &str,
    endpoint_url: &str,
) -> std::result::Result<AsrStreamHandle, AsrError> {
    let connect_timeout = config::asr_streaming_connect_timeout();
    let first_partial_timeout = config::asr_streaming_first_partial_timeout();
    let final_timeout = config::asr_streaming_final_timeout();

    let (ws, _) = timeout(connect_timeout, connect_async(endpoint_url))
        .await
        .map_err(|_| AsrError::Timeout)?
        .map_err(|e| AsrError::TranscriptionFailed(e.to_string()))?;

    let (mut ws_tx, mut ws_rx) = ws.split();
    let (audio_tx, audio_rx) = asr_audio_channel(ASR_STREAM_AUDIO_CHANNEL_CAPACITY);
    let (final_tx, final_rx) = oneshot::channel::<Result<String, AsrError>>();
    let call_id = call_id.to_string();

    tokio::spawn(async move {
        use std::future::pending;

        let mut first_partial_seen = false;
        let mut last_partial = String::new();
        let mut end_sent = false;
        let mut waiting_final = false;
        let mut first_partial_timer = Box::pin(sleep(first_partial_timeout));
        let mut final_timer: Option<Pin<Box<tokio::time::Sleep>>> = None;

        loop {
            tokio::select! {
                _ = &mut first_partial_timer, if !first_partial_seen => {
                    let _ = final_tx.send(Err(AsrError::Timeout));
                    let _ = ws_tx.close().await;
                    return;
                }
                _ = async {
                    if let Some(timer) = &mut final_timer {
                        timer.as_mut().await;
                    } else {
                        pending::<()>().await;
                    }
                }, if waiting_final => {
                    let _ = final_tx.send(Ok(last_partial));
                    let _ = ws_tx.close().await;
                    return;
                }
                maybe_audio = audio_rx.recv(), if !end_sent => {
                    match maybe_audio {
                        Some(AsrAudioMsg::Chunk(pcmu)) => {
                            if let Err(e) = ws_tx.send(WsMessage::Binary(pcmu.into())).await {
                                let _ = final_tx.send(Err(AsrError::TranscriptionFailed(e.to_string())));
                                return;
                            }
                        }
                        Some(AsrAudioMsg::End) => {
                            end_sent = true;
                            waiting_final = true;
                            if let Err(e) = ws_tx
                                .send(WsMessage::Text(
                                    serde_json::to_string(&WhisperStreamEndRequest { kind: "end" })
                                        .unwrap_or_else(|_| "{\"type\":\"end\"}".to_string())
                                        .into(),
                                ))
                                .await
                            {
                                let _ = final_tx.send(Err(AsrError::TranscriptionFailed(e.to_string())));
                                return;
                            }
                            final_timer = Some(Box::pin(sleep(final_timeout)));
                        }
                        None => {
                            let _ = final_tx.send(Err(AsrError::TranscriptionFailed(
                                "ASR audio channel closed".to_string(),
                            )));
                            let _ = ws_tx.close().await;
                            return;
                        }
                    }
                }
                maybe_msg = FuturesStreamExt::next(&mut ws_rx) => {
                    match maybe_msg {
                        Some(Ok(WsMessage::Text(text))) => {
                            match parse_whisper_stream_event_text(&text) {
                                Ok(AsrStreamEvent::Partial(text)) => {
                                    first_partial_seen = true;
                                    if !text.is_empty() {
                                        last_partial = text;
                                    }
                                }
                                Ok(AsrStreamEvent::Final(text)) => {
                                    if !text.is_empty() {
                                        last_partial = text.clone();
                                    }
                                    let out = if text.is_empty() { last_partial.clone() } else { text };
                                    let _ = final_tx.send(Ok(out));
                                    let _ = ws_tx.close().await;
                                    return;
                                }
                                Err(e) => {
                                    let _ = final_tx.send(Err(e));
                                    let _ = ws_tx.close().await;
                                    return;
                                }
                            }
                        }
                        Some(Ok(WsMessage::Close(_))) => {
                            if waiting_final {
                                let _ = final_tx.send(Ok(last_partial));
                            } else {
                                let _ = final_tx.send(Err(AsrError::TranscriptionFailed(
                                    "ASR stream closed before final".to_string(),
                                )));
                            }
                            return;
                        }
                        Some(Ok(WsMessage::Ping(data))) => {
                            let _ = ws_tx.send(WsMessage::Pong(data)).await;
                        }
                        Some(Ok(WsMessage::Pong(_))) => {}
                        Some(Ok(WsMessage::Binary(_))) => {}
                        Some(Ok(WsMessage::Frame(_))) => {}
                        Some(Err(e)) => {
                            log::warn!("[asr stream {}] websocket receive error: {}", call_id, e);
                            let _ = final_tx.send(Err(AsrError::TranscriptionFailed(e.to_string())));
                            return;
                        }
                        None => {
                            if waiting_final {
                                let _ = final_tx.send(Ok(last_partial));
                            } else {
                                let _ = final_tx.send(Err(AsrError::TranscriptionFailed(
                                    "ASR stream ended before final".to_string(),
                                )));
                            }
                            return;
                        }
                    }
                }
            }
        }
    });

    Ok(AsrStreamHandle { audio_tx, final_rx })
}

fn build_openai_chat_messages(
    messages: &[ChatMessage],
    system_prompt: &str,
) -> Vec<OpenAiChatMessage> {
    let mut openai_messages = Vec::with_capacity(messages.len() + 1);
    openai_messages.push(OpenAiChatMessage {
        role: "system".to_string(),
        content: system_prompt.to_string(),
    });
    for msg in messages {
        let role = match msg.role {
            Role::User => "user",
            Role::Assistant => "assistant",
        };
        openai_messages.push(OpenAiChatMessage {
            role: role.to_string(),
            content: msg.content.clone(),
        });
    }
    openai_messages
}

pub(super) async fn call_openai_chat_for_stage(
    messages: &[ChatMessage],
    system_prompt: &str,
    model: &str,
    base_url: &str,
    api_key: &str,
    http_timeout: Duration,
) -> Result<String> {
    let client = http_client(http_timeout)?;
    let url = openai_endpoint_url(base_url, "/chat/completions");
    let req = OpenAiChatCompletionsRequest {
        model: model.to_string(),
        messages: build_openai_chat_messages(messages, system_prompt),
    };

    let resp = client
        .post(url)
        .bearer_auth(api_key)
        .json(&req)
        .send()
        .await?;
    let status = resp.status();
    let body_text = resp.text().await?;
    if !status.is_success() {
        anyhow::bail!(
            "OpenAI chat HTTP error {} (body_len={})",
            status,
            body_text.len()
        );
    }

    let body: OpenAiChatCompletionsResponse = serde_json::from_str(&body_text)?;
    let answer = body
        .choices
        .first()
        .and_then(|choice| choice.message.content.clone())
        .unwrap_or_else(|| "<no response>".to_string());
    Ok(answer)
}

pub(crate) async fn call_openai_intent(
    text: &str,
    _call_id: &str,
    api_key: &str,
    base_url: &str,
    model: &str,
    http_timeout: Duration,
) -> Result<String> {
    let client = http_client(http_timeout)?;
    let url = openai_endpoint_url(base_url, "/chat/completions");
    let messages = vec![ChatMessage {
        role: Role::User,
        content: text.to_string(),
    }];
    let req = serde_json::json!({
        "model": model,
        "messages": build_openai_chat_messages(&messages, &intent::intent_prompt()),
        "response_format": { "type": "json_object" },
    });

    let resp = client
        .post(url)
        .bearer_auth(api_key)
        .json(&req)
        .send()
        .await?;
    let status = resp.status();
    let body_text = resp.text().await?;
    if !status.is_success() {
        anyhow::bail!(
            "OpenAI intent HTTP error {} (body_len={})",
            status,
            body_text.len()
        );
    }

    let body: OpenAiChatCompletionsResponse = serde_json::from_str(&body_text)?;
    let answer = body
        .choices
        .first()
        .and_then(|choice| choice.message.content.as_deref())
        .map(str::trim)
        .filter(|content| !content.is_empty())
        .ok_or_else(|| anyhow!("OpenAI intent response content missing"))?;
    Ok(answer.to_string())
}

async fn transcribe_with_openai_for_stage(
    base_url: &str,
    api_key: &str,
    wav_path: &str,
    http_timeout: Duration,
) -> Result<String> {
    let client = http_client(http_timeout)?;
    let url = openai_endpoint_url(base_url, "/audio/transcriptions");
    let bytes = tokio::fs::read(wav_path).await?;
    let part = multipart::Part::bytes(bytes)
        .file_name("question.wav")
        .mime_str("audio/wav")?;
    let form = multipart::Form::new()
        .part("file", part)
        .text("model", OPENAI_ASR_MODEL.to_string())
        .text("response_format", "json".to_string())
        .text("language", "ja".to_string());

    let resp = client
        .post(url)
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("OpenAI ASR HTTP error {} (body_len={})", status, body.len());
    }
    let result: WhisperResponse = resp.json().await?;
    Ok(result.text)
}

fn validate_openai_tts_wav_bytes(wav_bytes: &[u8]) -> Result<()> {
    let cursor = Cursor::new(wav_bytes);
    let reader = hound::WavReader::new(cursor)?;
    let spec = reader.spec();
    if spec.channels != 1 || spec.bits_per_sample != 16 {
        anyhow::bail!(
            "unsupported wav format {}ch/{}bit (expected mono 16bit)",
            spec.channels,
            spec.bits_per_sample
        );
    }
    match spec.sample_rate {
        8_000 | 24_000 => Ok(()),
        other => anyhow::bail!("unsupported sample rate {other}"),
    }
}

fn wrap_openai_tts_pcm_as_wav(pcm_bytes: &[u8]) -> Result<Vec<u8>> {
    if !pcm_bytes.len().is_multiple_of(2) {
        anyhow::bail!(
            "OpenAI TTS PCM byte length is not aligned to 16-bit samples: {}",
            pcm_bytes.len()
        );
    }

    let data_len =
        u32::try_from(pcm_bytes.len()).map_err(|_| anyhow!("OpenAI TTS PCM response too large"))?;
    let riff_chunk_len = data_len
        .checked_add(36)
        .ok_or_else(|| anyhow!("OpenAI TTS PCM response too large"))?;

    let channels: u16 = 1;
    let bits_per_sample: u16 = 16;
    let block_align: u16 = channels * (bits_per_sample / 8);
    let byte_rate: u32 = OPENAI_TTS_PCM_SAMPLE_RATE * u32::from(block_align);

    let mut wav = Vec::with_capacity(44 + pcm_bytes.len());
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&riff_chunk_len.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes()); // PCM fmt chunk size
    wav.extend_from_slice(&1u16.to_le_bytes()); // audio_format=PCM
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&OPENAI_TTS_PCM_SAMPLE_RATE.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&bits_per_sample.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.extend_from_slice(pcm_bytes);
    Ok(wav)
}

async fn synth_openai_tts_for_stage(
    base_url: &str,
    api_key: &str,
    text: &str,
    http_timeout: Duration,
) -> Result<Vec<u8>> {
    let client = http_client(http_timeout)?;
    let url = openai_endpoint_url(base_url, "/audio/speech");
    let req = OpenAiSpeechRequest {
        model: OPENAI_TTS_MODEL.to_string(),
        voice: OPENAI_TTS_VOICE.to_string(),
        input: text.to_string(),
        response_format: "pcm".to_string(),
    };

    let resp = client
        .post(url)
        .bearer_auth(api_key)
        .json(&req)
        .send()
        .await?;
    let status = resp.status();
    let audio_bytes = resp.bytes().await?;
    if !status.is_success() {
        let body_len = audio_bytes.len();
        anyhow::bail!("OpenAI TTS HTTP error {} (body_len={})", status, body_len);
    }

    let wav = wrap_openai_tts_pcm_as_wav(audio_bytes.as_ref())?;
    validate_openai_tts_wav_bytes(&wav)?;
    Ok(wav)
}

fn map_tts_response_stream(resp: reqwest::Response) -> TtsStream {
    Box::pin(resp.bytes_stream().map(|chunk| {
        chunk
            .map(|bytes| bytes.to_vec())
            .map_err(|e| TtsError::SynthesisFailed(e.to_string()))
    }))
}

async fn synth_openai_tts_stream_for_stage(
    base_url: &str,
    api_key: &str,
    text: &str,
    connect_timeout: Duration,
) -> Result<TtsStream> {
    let client = http_client_for_stream(connect_timeout)?;
    let url = openai_endpoint_url(base_url, "/audio/speech");
    let req = OpenAiSpeechRequest {
        model: OPENAI_TTS_MODEL.to_string(),
        voice: OPENAI_TTS_VOICE.to_string(),
        input: text.to_string(),
        response_format: "wav".to_string(),
    };

    let resp = client
        .post(url)
        .bearer_auth(api_key)
        .json(&req)
        .send()
        .await?;
    let status = resp.status();
    if !status.is_success() {
        let body_len = resp.bytes().await?.len();
        anyhow::bail!("OpenAI TTS HTTP error {} (body_len={})", status, body_len);
    }
    Ok(map_tts_response_stream(resp))
}

async fn synth_zundamon_stream_for_stage(
    base_url: &str,
    text: &str,
    startup_timeout: Duration,
    connect_timeout: Duration,
) -> Result<TtsStream> {
    let query_client = http_client(startup_timeout)?;
    let speaker_id = 3; // ずんだもん ノーマル

    let query_url = tts_endpoint_url(base_url, "/audio_query");
    let query_resp = query_client
        .post(query_url)
        .query(&[("text", text), ("speaker", &speaker_id.to_string())])
        .send()
        .await?;

    let status = query_resp.status();
    let query_body = query_resp.text().await?;
    if !status.is_success() {
        anyhow::bail!("audio_query error {} ({} bytes)", status, query_body.len());
    }

    let stream_client = http_client_for_stream(connect_timeout)?;
    let synth_url = tts_endpoint_url(base_url, "/synthesis");
    let synth_resp = stream_client
        .post(synth_url)
        .query(&[("speaker", &speaker_id.to_string())])
        .header("Content-Type", "application/json")
        .body(query_body)
        .send()
        .await?;

    let status = synth_resp.status();
    if !status.is_success() {
        let body_len = synth_resp.bytes().await?.len();
        anyhow::bail!("synthesis error {} ({} bytes)", status, body_len);
    }

    Ok(map_tts_response_stream(synth_resp))
}

async fn try_tts_stream_stage<F, Fut>(
    call_id: &str,
    stage: TtsStage,
    startup_timeout: Duration,
    run: F,
) -> Option<TtsStream>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<TtsStream>>,
{
    let stage_name = stage.as_str();
    log::debug!(
        "[tts {call_id}] TTS stream stage start: tts_stage={} timeout_ms={}",
        stage_name,
        startup_timeout.as_millis()
    );

    let stream = match timeout(startup_timeout, run()).await {
        Ok(Ok(stream)) => stream,
        Ok(Err(err)) => {
            log::warn!(
                "[tts {call_id}] TTS stream stage failed: tts_stage={} reason={}",
                stage_name,
                err
            );
            return None;
        }
        Err(_) => {
            log::warn!(
                "[tts {call_id}] TTS stream stage failed: tts_stage={} reason=timeout timeout_ms={}",
                stage_name,
                startup_timeout.as_millis()
            );
            return None;
        }
    };

    log::debug!(
        "[tts {call_id}] TTS stream stage connected: tts_stage={}",
        stage_name
    );
    Some(stream)
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

/// ASR 実行（現行実装を拡張）: local server -> cloud(OpenAI/AWS) -> raspi server のフォールバック。
pub async fn transcribe_and_log(call_id: &str, wav_path: &str) -> Result<String> {
    let ai_cfg = config::ai_config();

    if asr_stage_count(ai_cfg) == 0 {
        log::error!("[asr {call_id}] ASR failed: reason=all ASR stages disabled");
        anyhow::bail!("all ASR stages failed");
    }

    let openai_asr_enabled = openai_asr_stage_enabled(ai_cfg);
    let openai_api_key_owned = openai_api_key(ai_cfg).map(str::to_string);
    let openai_base_url = ai_cfg.openai_base_url.clone();

    for stage in ASR_FALLBACK_ORDER {
        match stage {
            AsrStage::Local => {
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
            }
            AsrStage::Cloud => {
                if asr_cloud_enabled(ai_cfg) {
                    if let Some(text) =
                        try_asr_stage(call_id, AsrStage::Cloud, ai_cfg.asr_cloud_timeout, || {
                            let openai_api_key_owned = openai_api_key_owned.clone();
                            let openai_base_url = openai_base_url.clone();
                            async move {
                                if openai_asr_enabled {
                                    let api_key = openai_api_key_owned
                                        .as_deref()
                                        .ok_or_else(|| anyhow!("OPENAI_API_KEY missing"))?;
                                    match transcribe_with_openai_for_stage(
                                        &openai_base_url,
                                        api_key,
                                        wav_path,
                                        ai_cfg.asr_cloud_timeout,
                                    )
                                    .await
                                    {
                                        Ok(text) => return Ok(text),
                                        Err(err) => {
                                            log::warn!(
                                                "[asr {call_id}] ASR cloud provider failed: provider=openai reason={}",
                                                err
                                            );
                                        }
                                    }
                                }

                                if ai_cfg.use_aws_transcribe {
                                    return transcribe_with_aws(wav_path).await;
                                }

                                anyhow::bail!("no cloud ASR providers enabled")
                            }
                        })
                        .await
                    {
                        return Ok(text);
                    }
                }
            }
            AsrStage::Raspi => {
                if ai_cfg.asr_raspi_enabled {
                    if let Some(raspi_url) = ai_cfg.asr_raspi_url.clone() {
                        if let Some(text) = try_asr_stage(
                            call_id,
                            AsrStage::Raspi,
                            ai_cfg.asr_raspi_timeout,
                            || {
                                transcribe_with_http_asr(
                                    &raspi_url,
                                    wav_path,
                                    ai_cfg.asr_raspi_timeout,
                                )
                            },
                        )
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
            }
        }
    }

    log::error!("[asr {call_id}] ASR failed: reason=all ASR stages failed");
    anyhow::bail!("all ASR stages failed")
}

/// LLM + TTS 実行（現行実装を拡張: local Ollama→cloud(OpenAI/Gemini)→raspi fallback→TTS）。挙動は従来I/F維持。
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

    let system_prompt = llm::system_prompt();
    let openai_llm_enabled = openai_llm_stage_enabled(ai_cfg);
    let openai_api_key_owned = openai_api_key(ai_cfg).map(str::to_string);
    let openai_base_url = ai_cfg.openai_base_url.clone();
    for stage in LLM_FALLBACK_ORDER {
        match stage {
            LlmStage::Local => {
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
            }
            LlmStage::Cloud => {
                if llm_cloud_enabled(ai_cfg) {
                    if let Some(text) =
                        try_llm_stage(call_id, LlmStage::Cloud, ai_cfg.llm_cloud_timeout, || {
                            let openai_api_key_owned = openai_api_key_owned.clone();
                            let openai_base_url = openai_base_url.clone();
                            let system_prompt = system_prompt.clone();
                            let cloud_messages = messages.clone();
                            async move {
                                if openai_llm_enabled {
                                    let api_key = openai_api_key_owned
                                        .as_deref()
                                        .ok_or_else(|| anyhow!("OPENAI_API_KEY missing"))?;
                                    match call_openai_chat_for_stage(
                                        &cloud_messages,
                                        &system_prompt,
                                        OPENAI_LLM_MODEL,
                                        &openai_base_url,
                                        api_key,
                                        ai_cfg.llm_cloud_timeout,
                                    )
                                    .await
                                    {
                                        Ok(text) => return Ok(text),
                                        Err(err) => {
                                            log::warn!(
                                                "[llm {call_id}] LLM cloud provider failed: provider=openai reason={}",
                                                err
                                            );
                                        }
                                    }
                                }

                                if gemini_llm_enabled(ai_cfg) {
                                    return call_gemini_with_http_timeout(
                                        &cloud_messages,
                                        ai_cfg.llm_cloud_timeout,
                                    )
                                    .await;
                                }

                                anyhow::bail!("no cloud LLM providers enabled")
                            }
                        })
                        .await
                    {
                        return Ok(text);
                    }
                }
            }
            LlmStage::Raspi => {
                if ai_cfg.llm_raspi_enabled {
                    if let Some(raspi_url) = ai_cfg.llm_raspi_url.clone() {
                        let raspi_model = ai_cfg.llm_raspi_model.clone();
                        if let Some(text) = try_llm_stage(
                            call_id,
                            LlmStage::Raspi,
                            ai_cfg.llm_raspi_timeout,
                            || {
                                call_ollama_for_stage(
                                    &messages,
                                    &system_prompt,
                                    &raspi_model,
                                    &raspi_url,
                                    ai_cfg.llm_raspi_timeout,
                                )
                            },
                        )
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
            }
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

const OLLAMA_STREAM_CHANNEL_CAPACITY: usize = 32;

fn parse_ollama_stream_line(line: &[u8]) -> std::result::Result<(Option<String>, bool), LlmError> {
    let parsed: OllamaChatStreamResponse = serde_json::from_slice(line)
        .map_err(|e| LlmError::GenerationFailed(format!("failed to parse Ollama NDJSON: {e}")))?;
    if let Some(err) = parsed.error {
        return Err(LlmError::GenerationFailed(format!(
            "Ollama stream returned error: {err}"
        )));
    }

    let token = parsed.message.and_then(|m| {
        if m.content.is_empty() {
            None
        } else {
            Some(m.content)
        }
    });
    let done = parsed.done.unwrap_or(false);
    Ok((token, done))
}

fn spawn_ollama_stream_parser(resp: reqwest::Response) -> LlmStream {
    let (tx, rx) =
        mpsc::channel::<Result<LlmStreamEvent, LlmError>>(OLLAMA_STREAM_CHANNEL_CAPACITY);
    tokio::spawn(async move {
        let mut bytes_stream = resp.bytes_stream();
        let mut buf = Vec::<u8>::new();
        // Parser task leak防止用の read timeout。
        // 上位の厳密なタイムアウト制御（first-token / chunk-idle / total）は consumer 側で行う。
        // ここで短すぎる値を使うと first-token timeout を実質的に短縮してしまうため、より緩い方を使う。
        let read_timeout =
            config::llm_streaming_first_token_timeout().max(config::sentence_max_wait());

        loop {
            if tx.is_closed() {
                return;
            }

            let next_result =
                timeout(read_timeout, FuturesStreamExt::next(&mut bytes_stream)).await;
            let Some(item) = (match next_result {
                Ok(item) => item,
                Err(_) => {
                    if tx.is_closed() {
                        return;
                    }
                    let _ = tx
                        .send(Err(LlmError::Timeout("ollama stream read timeout".into())))
                        .await;
                    return;
                }
            }) else {
                break;
            };
            let chunk = match item {
                Ok(chunk) => chunk,
                Err(e) => {
                    let _ = tx
                        .send(Err(LlmError::GenerationFailed(e.to_string())))
                        .await;
                    return;
                }
            };
            buf.extend_from_slice(&chunk);

            while let Some(pos) = buf.iter().position(|b| *b == b'\n') {
                if tx.is_closed() {
                    return;
                }
                let line = buf.drain(..=pos).collect::<Vec<u8>>();
                let line = line.trim_ascii();
                if line.is_empty() {
                    continue;
                }
                match parse_ollama_stream_line(line) {
                    Ok((token, done)) => {
                        if let Some(token) = token {
                            if tx.send(Ok(LlmStreamEvent::Token(token))).await.is_err() {
                                return;
                            }
                        }
                        if done {
                            let _ = tx.send(Ok(LlmStreamEvent::End)).await;
                            return;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e)).await;
                        return;
                    }
                }
            }
        }

        let tail = buf.trim_ascii();
        if tail.is_empty() {
            return;
        }
        match parse_ollama_stream_line(tail) {
            Ok((token, done)) => {
                if let Some(token) = token {
                    let _ = tx.send(Ok(LlmStreamEvent::Token(token))).await;
                }
                if done {
                    let _ = tx.send(Ok(LlmStreamEvent::End)).await;
                }
            }
            Err(e) => {
                let _ = tx.send(Err(e)).await;
            }
        }
    });
    Box::pin(ReceiverStream::new(rx))
}

pub(super) async fn call_ollama_for_chat_stream(
    messages: &[ChatMessage],
    system_prompt: &str,
    model: &str,
    endpoint_url: &str,
    connect_timeout: Duration,
    response_header_timeout: Duration,
) -> std::result::Result<LlmStream, LlmError> {
    let client = http_client_for_stream(connect_timeout)
        .map_err(|e| LlmError::GenerationFailed(e.to_string()))?;

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
        stream: true,
    };

    let send_fut = client.post(endpoint_url).json(&req).send();
    let resp = timeout(response_header_timeout, send_fut)
        .await
        .map_err(|_| LlmError::Timeout("response header timeout".into()))?
        .map_err(|e| LlmError::GenerationFailed(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(LlmError::GenerationFailed(format!(
            "Ollama HTTP error {}",
            resp.status()
        )));
    }

    Ok(spawn_ollama_stream_parser(resp))
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

pub(super) async fn call_ollama_for_weather(
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

impl AsrStreamPort for DefaultAiPort {
    fn transcribe_stream(
        &self,
        call_id: String,
        endpoint_url: String,
    ) -> AiFuture<Result<AsrStreamHandle, AsrError>> {
        Box::pin(async move { call_whisper_stream(&call_id, &endpoint_url).await })
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

impl LlmStreamPort for DefaultAiPort {
    fn generate_answer_stream(
        &self,
        call_id: String,
        messages: Vec<ChatMessage>,
    ) -> AiFuture<Result<LlmStream, LlmError>> {
        Box::pin(async move { llm::generate_answer_stream(&call_id, messages).await })
    }
}

impl WeatherPort for DefaultAiPort {
    fn handle_weather(
        &self,
        call_id: String,
        query: WeatherQuery,
    ) -> AiFuture<Result<String, WeatherError>> {
        Box::pin(async move {
            weather::handle_weather(&call_id, query)
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

impl TtsStreamPort for DefaultAiPort {
    fn synth_stream(&self, call_id: String, text: String) -> AiFuture<Result<TtsStream, TtsError>> {
        Box::pin(async move {
            let ai_cfg = config::ai_config();
            if tts_stage_count(ai_cfg) == 0 {
                log::error!("[tts {call_id}] TTS stream failed: reason=all TTS stages disabled");
                return Err(TtsError::SynthesisFailed(
                    "all TTS stages disabled".to_string(),
                ));
            }

            let startup_timeout = config::tts_streaming_connect_timeout();
            let openai_tts_enabled = openai_tts_stage_enabled(ai_cfg);
            let openai_api_key_owned = openai_api_key(ai_cfg).map(str::to_string);
            let openai_base_url = ai_cfg.openai_base_url.clone();

            for stage in TTS_FALLBACK_ORDER {
                match stage {
                    TtsStage::Local => {
                        if ai_cfg.tts_local_server_enabled {
                            let local_base_url = ai_cfg.tts_local_server_base_url.clone();
                            if let Some(stream) = try_tts_stream_stage(
                                &call_id,
                                TtsStage::Local,
                                startup_timeout,
                                || {
                                    let text = text.clone();
                                    async move {
                                        synth_zundamon_stream_for_stage(
                                            &local_base_url,
                                            &text,
                                            startup_timeout,
                                            startup_timeout,
                                        )
                                        .await
                                    }
                                },
                            )
                            .await
                            {
                                return Ok(stream);
                            }
                        }
                    }
                    TtsStage::Cloud => {
                        if openai_tts_enabled {
                            if let Some(stream) = try_tts_stream_stage(
                                &call_id,
                                TtsStage::Cloud,
                                startup_timeout,
                                || {
                                    let openai_api_key_owned = openai_api_key_owned.clone();
                                    let openai_base_url = openai_base_url.clone();
                                    let text = text.clone();
                                    async move {
                                        let api_key = openai_api_key_owned
                                            .as_deref()
                                            .ok_or_else(|| anyhow!("OPENAI_API_KEY missing"))?;
                                        synth_openai_tts_stream_for_stage(
                                            &openai_base_url,
                                            api_key,
                                            &text,
                                            startup_timeout,
                                        )
                                        .await
                                    }
                                },
                            )
                            .await
                            {
                                return Ok(stream);
                            }
                        }
                    }
                    TtsStage::Raspi => {
                        if ai_cfg.tts_raspi_enabled {
                            if let Some(raspi_base_url) = ai_cfg.tts_raspi_base_url.clone() {
                                if let Some(stream) = try_tts_stream_stage(
                                    &call_id,
                                    TtsStage::Raspi,
                                    startup_timeout,
                                    || {
                                        let text = text.clone();
                                        async move {
                                            synth_zundamon_stream_for_stage(
                                                &raspi_base_url,
                                                &text,
                                                startup_timeout,
                                                startup_timeout,
                                            )
                                            .await
                                        }
                                    },
                                )
                                .await
                                {
                                    return Ok(stream);
                                }
                            } else {
                                log::warn!(
                                    "[tts {call_id}] TTS stream stage failed: tts_stage=raspi reason=TTS_RASPI_BASE_URL missing"
                                );
                            }
                        }
                    }
                }
            }

            log::error!("[tts {call_id}] TTS stream failed: reason=all TTS stages failed");
            Err(TtsError::SynthesisFailed(
                "all TTS stages failed".to_string(),
            ))
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

/// TTS 呼び出し（VoiceVox local / OpenAI cloud / raspi）。I/F はテキストと出力 WAV パス（従来どおり）。
pub async fn synth_zundamon_wav(call_id: &str, text: &str, out_path: &str) -> Result<()> {
    let ai_cfg = config::ai_config();

    if tts_stage_count(ai_cfg) == 0 {
        log::error!("[tts {call_id}] TTS failed: reason=all TTS stages disabled");
        anyhow::bail!("all TTS stages failed");
    }

    let openai_tts_enabled = openai_tts_stage_enabled(ai_cfg);
    let openai_api_key_owned = openai_api_key(ai_cfg).map(str::to_string);
    let openai_base_url = ai_cfg.openai_base_url.clone();

    for stage in TTS_FALLBACK_ORDER {
        match stage {
            TtsStage::Local => {
                if ai_cfg.tts_local_server_enabled {
                    let local_base_url = ai_cfg.tts_local_server_base_url.clone();
                    if let Some(wav_bytes) =
                        try_tts_stage(call_id, TtsStage::Local, ai_cfg.tts_local_timeout, || {
                            synth_zundamon_for_stage(
                                &local_base_url,
                                call_id,
                                text,
                                ai_cfg.tts_local_timeout,
                            )
                        })
                        .await
                    {
                        tokio::fs::write(out_path, &wav_bytes).await?;
                        info!("[tts {call_id}] TTS written to {}", out_path);
                        return Ok(());
                    }
                }
            }
            TtsStage::Cloud => {
                if openai_tts_enabled {
                    if let Some(wav_bytes) =
                        try_tts_stage(call_id, TtsStage::Cloud, ai_cfg.tts_cloud_timeout, || {
                            let openai_api_key_owned = openai_api_key_owned.clone();
                            let openai_base_url = openai_base_url.clone();
                            async move {
                                let api_key = openai_api_key_owned
                                    .as_deref()
                                    .ok_or_else(|| anyhow!("OPENAI_API_KEY missing"))?;
                                synth_openai_tts_for_stage(
                                    &openai_base_url,
                                    api_key,
                                    text,
                                    ai_cfg.tts_cloud_timeout,
                                )
                                .await
                            }
                        })
                        .await
                    {
                        tokio::fs::write(out_path, &wav_bytes).await?;
                        info!("[tts {call_id}] TTS written to {}", out_path);
                        return Ok(());
                    }
                }
            }
            TtsStage::Raspi => {
                if ai_cfg.tts_raspi_enabled {
                    if let Some(raspi_base_url) = ai_cfg.tts_raspi_base_url.clone() {
                        if let Some(wav_bytes) = try_tts_stage(
                            call_id,
                            TtsStage::Raspi,
                            ai_cfg.tts_raspi_timeout,
                            || {
                                synth_zundamon_for_stage(
                                    &raspi_base_url,
                                    call_id,
                                    text,
                                    ai_cfg.tts_raspi_timeout,
                                )
                            },
                        )
                        .await
                        {
                            tokio::fs::write(out_path, &wav_bytes).await?;
                            info!("[tts {call_id}] TTS written to {}", out_path);
                            return Ok(());
                        }
                    } else {
                        log::warn!(
                            "[tts {call_id}] TTS stage failed: tts_stage=raspi reason=TTS_RASPI_BASE_URL missing"
                        );
                    }
                }
            }
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
    use std::io::Cursor;
    use std::time::Duration;

    use hound::{SampleFormat, WavSpec, WavWriter};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use super::{
        call_ollama_for_weather, call_openai_chat_for_stage, call_openai_intent, extract_base_url,
        openai_endpoint_url, parse_ollama_stream_line, probe_once, sanitized_display_url,
        synth_openai_tts_for_stage, transcribe_with_openai_for_stage, tts_endpoint_url,
        validate_openai_tts_wav_bytes, wrap_openai_tts_pcm_as_wav, ChatMessage, LlmError, Role,
        ASR_FALLBACK_ORDER, LLM_FALLBACK_ORDER, TTS_FALLBACK_ORDER,
    };

    async fn read_http_request(
        mut socket: tokio::net::TcpStream,
    ) -> (tokio::net::TcpStream, String) {
        let mut buf = [0u8; 1024];
        let mut request = Vec::new();
        let mut header_end = None;
        let mut content_length = 0usize;

        loop {
            let n = socket.read(&mut buf).await.expect("read request");
            if n == 0 {
                break;
            }
            request.extend_from_slice(&buf[..n]);
            if header_end.is_none() {
                if let Some(pos) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                    let end = pos + 4;
                    header_end = Some(end);
                    let headers = String::from_utf8_lossy(&request[..end]).to_ascii_lowercase();
                    for line in headers.lines() {
                        if let Some(value) = line.strip_prefix("content-length:") {
                            content_length = value.trim().parse::<usize>().unwrap_or(0);
                        }
                    }
                }
            }
            if let Some(end) = header_end {
                if request.len() >= end + content_length {
                    break;
                }
            }
        }

        (socket, String::from_utf8_lossy(&request).into_owned())
    }

    fn make_test_wav(sample_rate: u32) -> Vec<u8> {
        let spec = WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = WavWriter::new(&mut cursor, spec).expect("create wav");
            for _ in 0..160 {
                writer.write_sample::<i16>(0).expect("write sample");
            }
            writer.finalize().expect("finalize wav");
        }
        cursor.into_inner()
    }

    #[test]
    fn asr_fallback_order_is_local_cloud_raspi() {
        assert_eq!(
            ASR_FALLBACK_ORDER
                .iter()
                .map(|stage| stage.as_str())
                .collect::<Vec<_>>(),
            vec!["local", "cloud", "raspi"]
        );
    }

    #[test]
    fn llm_fallback_order_is_local_cloud_raspi() {
        assert_eq!(
            LLM_FALLBACK_ORDER
                .iter()
                .map(|stage| stage.as_str())
                .collect::<Vec<_>>(),
            vec!["local", "cloud", "raspi"]
        );
    }

    #[test]
    fn tts_fallback_order_is_local_cloud_raspi() {
        assert_eq!(
            TTS_FALLBACK_ORDER
                .iter()
                .map(|stage| stage.as_str())
                .collect::<Vec<_>>(),
            vec!["local", "cloud", "raspi"]
        );
    }

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

    #[test]
    fn parse_ollama_stream_line_emits_normal_token() {
        let line = br#"{"message":{"role":"assistant","content":"hello"},"done":false}"#;

        let parsed = parse_ollama_stream_line(line).expect("parse normal stream token");

        assert_eq!(parsed, (Some("hello".to_string()), false));
    }

    #[test]
    fn parse_ollama_stream_line_handles_done_chunk() {
        let line = br#"{"message":{"role":"assistant","content":""},"done":true}"#;

        let parsed = parse_ollama_stream_line(line).expect("parse final done chunk");

        assert_eq!(parsed, (None, true));
    }

    #[test]
    fn parse_ollama_stream_line_returns_error_for_ollama_error_payload() {
        let line = br#"{"error":"model not found","done":true}"#;

        let err = parse_ollama_stream_line(line).expect_err("ollama error payload should fail");

        match err {
            LlmError::GenerationFailed(msg) => {
                assert!(msg.contains("Ollama stream returned error"));
                assert!(msg.contains("model not found"));
            }
            other => panic!("unexpected error variant: {other:?}"),
        }
    }

    #[test]
    fn parse_ollama_stream_line_returns_error_for_invalid_json() {
        let line = br#"{"message":{"role":"assistant","content":"x"}"#;

        let err = parse_ollama_stream_line(line).expect_err("invalid ndjson should fail");

        match err {
            LlmError::GenerationFailed(msg) => {
                assert!(msg.contains("failed to parse Ollama NDJSON"));
            }
            other => panic!("unexpected error variant: {other:?}"),
        }
    }

    #[tokio::test]
    async fn call_ollama_for_weather_uses_supplied_endpoint_and_parses_response() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test server");
        let addr = listener.local_addr().expect("get local addr");
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept client");
            let mut buf = [0u8; 1024];
            let mut request = Vec::new();

            loop {
                let n = socket.read(&mut buf).await.expect("read request");
                if n == 0 {
                    break;
                }
                request.extend_from_slice(&buf[..n]);
                if request.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }

            let body = r#"{"message":{"role":"assistant","content":"晴れです"}}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket
                .write_all(response.as_bytes())
                .await
                .expect("write response");

            String::from_utf8_lossy(&request).into_owned()
        });

        let endpoint = format!("http://{addr}/api/chat");
        let messages = vec![ChatMessage {
            role: Role::User,
            content: "東京の天気".to_string(),
        }];

        let result = call_ollama_for_weather(
            &messages,
            "weather system prompt",
            "test-weather-model",
            &endpoint,
            Duration::from_secs(2),
        )
        .await
        .expect("weather helper should parse ollama response");

        let request_line = server.await.expect("join server task");
        assert_eq!(result, "晴れです");
        assert!(request_line.starts_with("POST /api/chat HTTP/1.1"));
    }

    #[test]
    fn openai_endpoint_url_handles_slashes() {
        assert_eq!(
            openai_endpoint_url("https://api.openai.com/v1/", "/chat/completions"),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            openai_endpoint_url("https://proxy.example/v1", "audio/speech"),
            "https://proxy.example/v1/audio/speech"
        );
    }

    #[test]
    fn extract_base_url_strips_path() {
        assert_eq!(
            extract_base_url("http://whisper:9000/transcribe"),
            "http://whisper:9000"
        );
        assert_eq!(
            extract_base_url("https://ollama.local:11434/api/chat"),
            "https://ollama.local:11434"
        );
    }

    #[test]
    fn extract_base_url_handles_existing_base() {
        assert_eq!(
            extract_base_url("http://voicevox:50021"),
            "http://voicevox:50021"
        );
        assert_eq!(
            extract_base_url("http://voicevox:50021/"),
            "http://voicevox:50021"
        );
    }

    #[test]
    fn extract_base_url_defensively_strips_path_without_scheme() {
        assert_eq!(
            extract_base_url("example.com:8080/path/to/probe"),
            "example.com:8080"
        );
    }

    #[test]
    fn extract_base_url_keeps_userinfo_for_probe_use() {
        assert_eq!(
            extract_base_url("http://user:pass@whisper:9000/transcribe"),
            "http://user:pass@whisper:9000"
        );
    }

    #[test]
    fn sanitized_display_url_strips_userinfo() {
        assert_eq!(
            sanitized_display_url("http://user:pass@whisper:9000"),
            "http://whisper:9000"
        );
        assert_eq!(
            sanitized_display_url("https://user@ollama.local:11434"),
            "https://ollama.local:11434"
        );
    }

    #[tokio::test]
    async fn probe_once_returns_true_for_http_200() {
        let url = spawn_status_server(200, "OK").await;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(500))
            .build()
            .expect("client");

        assert!(probe_once(&client, &url).await);
    }

    #[tokio::test]
    async fn probe_once_returns_false_for_non_200() {
        let url = spawn_status_server(204, "No Content").await;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(500))
            .build()
            .expect("client");

        assert!(!probe_once(&client, &url).await);
    }

    async fn spawn_status_server(status: u16, reason: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local addr");

        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let mut buf = [0u8; 1024];
            let _ = socket.read(&mut buf).await;
            let response = format!(
                "HTTP/1.1 {status} {reason}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            socket
                .write_all(response.as_bytes())
                .await
                .expect("write response");
        });

        format!("http://{addr}/probe")
    }

    #[tokio::test]
    async fn call_openai_chat_for_stage_uses_supplied_endpoint_and_parses_response() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test server");
        let addr = listener.local_addr().expect("get local addr");
        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.expect("accept client");
            let (mut socket, request) = read_http_request(socket).await;
            let body = r#"{"choices":[{"message":{"content":"承知しました"}}]}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket
                .write_all(response.as_bytes())
                .await
                .expect("write response");
            request
        });

        let endpoint_base = format!("http://{addr}");
        let messages = vec![ChatMessage {
            role: Role::User,
            content: "こんにちは".to_string(),
        }];
        let result = call_openai_chat_for_stage(
            &messages,
            "system prompt",
            "gpt-4o-mini",
            &endpoint_base,
            "sk-test",
            Duration::from_secs(2),
        )
        .await
        .expect("openai helper should parse response");

        let request = server.await.expect("join server task");
        assert_eq!(result, "承知しました");
        assert!(request.starts_with("POST /chat/completions HTTP/1.1"));
        assert!(request
            .to_ascii_lowercase()
            .contains("authorization: bearer sk-test"));
    }

    #[tokio::test]
    async fn call_openai_intent_requests_json_mode_and_returns_raw_json_string() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test server");
        let addr = listener.local_addr().expect("get local addr");
        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.expect("accept client");
            let (mut socket, request) = read_http_request(socket).await;
            let body = r#"{"choices":[{"message":{"content":"{\"intent\":\"general_chat\",\"query\":\"こんにちは\"}"}}]}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket
                .write_all(response.as_bytes())
                .await
                .expect("write response");
            request
        });

        let result = call_openai_intent(
            "こんにちは",
            "call-test",
            "sk-test",
            &format!("http://{addr}"),
            "gpt-4o-mini",
            Duration::from_secs(2),
        )
        .await
        .expect("openai intent helper should return raw json string");

        let request = server.await.expect("join server task");
        assert!(request.starts_with("POST /chat/completions HTTP/1.1"));
        assert!(request.contains("\"response_format\":{\"type\":\"json_object\"}"));
        assert!(request
            .to_ascii_lowercase()
            .contains("authorization: bearer sk-test"));
        assert!(result.contains("\"intent\":\"general_chat\""));
    }

    #[tokio::test]
    async fn transcribe_with_openai_for_stage_uses_supplied_endpoint_and_parses_response() {
        let tmp = tempfile::NamedTempFile::new().expect("tmp wav");
        std::fs::write(tmp.path(), make_test_wav(8_000)).expect("write wav");

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test server");
        let addr = listener.local_addr().expect("get local addr");
        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.expect("accept client");
            let (mut socket, request) = read_http_request(socket).await;
            let body = r#"{"text":"テスト転写"}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket
                .write_all(response.as_bytes())
                .await
                .expect("write response");
            request
        });

        let base_url = format!("http://{addr}");
        let result = transcribe_with_openai_for_stage(
            &base_url,
            "sk-test",
            tmp.path().to_str().expect("utf8 path"),
            Duration::from_secs(2),
        )
        .await
        .expect("openai asr helper should parse response");

        let request = server.await.expect("join server task");
        assert_eq!(result, "テスト転写");
        assert!(request.starts_with("POST /audio/transcriptions HTTP/1.1"));
        assert!(request
            .to_ascii_lowercase()
            .contains("content-type: multipart/form-data;"));
    }

    #[test]
    fn validate_openai_tts_wav_bytes_accepts_24khz_mono_16bit() {
        let wav = make_test_wav(24_000);
        validate_openai_tts_wav_bytes(&wav).expect("24k mono 16bit should be accepted");
    }

    #[test]
    fn validate_openai_tts_wav_bytes_rejects_unsupported_sample_rate() {
        let wav = make_test_wav(16_000);
        assert!(validate_openai_tts_wav_bytes(&wav).is_err());
    }

    #[test]
    fn wrap_openai_tts_pcm_as_wav_rejects_odd_length() {
        let pcm = vec![0x00, 0x01, 0x02];
        assert!(wrap_openai_tts_pcm_as_wav(&pcm).is_err());
    }

    #[tokio::test]
    async fn synth_openai_tts_for_stage_requests_pcm_and_wraps_response() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test server");
        let addr = listener.local_addr().expect("get local addr");
        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.expect("accept client");
            let (mut socket, request) = read_http_request(socket).await;
            let pcm: [u8; 8] = [0x00, 0x00, 0x10, 0x00, 0xf0, 0xff, 0x00, 0x00];
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/octet-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                pcm.len()
            );
            socket
                .write_all(response.as_bytes())
                .await
                .expect("write headers");
            socket.write_all(&pcm).await.expect("write pcm");
            request
        });

        let wav = synth_openai_tts_for_stage(
            &format!("http://{addr}"),
            "sk-test",
            "こんにちは",
            Duration::from_secs(2),
        )
        .await
        .expect("openai tts helper should wrap pcm as wav");

        let request = server.await.expect("join server task");
        assert!(request.starts_with("POST /audio/speech HTTP/1.1"));
        assert!(request
            .to_ascii_lowercase()
            .contains("authorization: bearer sk-test"));
        assert!(request.contains("\"response_format\":\"pcm\""));
        validate_openai_tts_wav_bytes(&wav).expect("wrapped wav should be valid");
    }
}
