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
use std::io::Cursor;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;

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

/// ASR 実行（現行実装: AWS Transcribe→Whisper fallback）。呼び出し順・ポリシーはそのまま。
pub async fn transcribe_and_log(wav_path: &str) -> Result<String> {
    if aws_transcribe_enabled() {
        match transcribe_with_aws(wav_path).await {
            Ok(text) => {
                if text.trim().is_empty() {
                    log::warn!(
                        "AWS Transcribe returned empty text, falling back to local Whisper."
                    );
                } else {
                    info!("User question (aws): {}", text);
                    return Ok(text);
                }
            }
            Err(e) => {
                log::error!("AWS Transcribe failed: {e:?}. Falling back to local Whisper.");
            }
        }
    }

    let client = http_client(config::timeouts().ai_http)?;
    let bytes = tokio::fs::read(wav_path).await?;

    let part = multipart::Part::bytes(bytes)
        .file_name("question.wav")
        .mime_str("audio/wav")?;

    let form = multipart::Form::new().part("file", part);

    let resp = client
        .post("http://localhost:9000/transcribe")
        .multipart(form)
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("whisper error: {} - {}", status, body);
    }

    let result: WhisperResponse = resp.json().await?;
    let text = result.text;
    info!("User question (whisper): {}", text);

    Ok(text)
}

/// LLM + TTS 実行（現行実装: Gemini→Ollama fallback→ずんだもんTTS）。挙動は変更なし。
/// I/F はテキスト入力→WAVパス出力（将来はチャネル/PCM化予定、現状は一時ファイルのまま）。
pub async fn handle_user_question_from_whisper(messages: Vec<ChatMessage>) -> Result<String> {
    let answer = handle_user_question_from_whisper_llm_only(messages).await?;

    // 一時WAVファイル経由のまま（責務は ai モジュール内に閉じ込める）
    let answer_wav = "/tmp/ollama_answer.wav";
    synth_zundamon_wav(&answer, answer_wav).await?;

    Ok(answer_wav.to_string())
}

/// LLM 部分のみを切り出した I/F（app→ai で分離できるようにする）
pub async fn handle_user_question_from_whisper_llm_only(
    messages: Vec<ChatMessage>,
) -> Result<String> {
    if let Some(last_user) = messages.iter().rev().find(|m| m.role == Role::User) {
        info!("User question (whisper): {}", last_user.content);
    }

    let answer = match call_gemini(&messages).await {
        Ok(ans) => {
            info!("LLM answer (gemini): {}", ans);
            ans
        }
        Err(gemini_err) => {
            log::error!("call_gemini failed: {gemini_err:?}, falling back to ollama");
            match call_ollama(&messages).await {
                Ok(fallback) => {
                    info!("LLM answer (ollama fallback): {}", fallback);
                    fallback
                }
                Err(ollama_err) => {
                    log::error!(
                        "call_ollama also failed: {ollama_err:?}. Using default apology message."
                    );
                    "すみません、うまく答えを用意できませんでした。".to_string()
                }
            }
        }
    };

    Ok(answer)
}

async fn call_ollama(messages: &[ChatMessage]) -> Result<String> {
    let model = config::ai_config().ollama_model.clone();
    let system_prompt = llm::system_prompt();
    call_ollama_with_prompt(messages, &system_prompt, &model).await
}

pub(crate) async fn call_ollama_with_prompt(
    messages: &[ChatMessage],
    system_prompt: &str,
    model: &str,
) -> Result<String> {
    let client = http_client(config::timeouts().ai_http)?;

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

    let resp = client
        .post("http://localhost:11434/api/chat")
        .json(&req)
        .send()
        .await?;

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
    /// ```
    /// use virtual_voicebot_backend::service::ai::DefaultAiPort;
    /// let port = DefaultAiPort::new();
    /// let transcript = futures::executor::block_on(port.transcribe_chunks("call-1".to_string(), vec![])).unwrap();
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
    fn classify_intent(&self, text: String) -> AiFuture<Result<String, IntentError>> {
        Box::pin(async move {
            intent::classify_intent(text)
                .await
                .map_err(|e| IntentError::ClassificationFailed(e.to_string()))
        })
    }
}

impl LlmPort for DefaultAiPort {
    fn generate_answer(&self, messages: Vec<ChatMessage>) -> AiFuture<Result<String, LlmError>> {
        Box::pin(async move {
            llm::generate_answer(messages)
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
        text: String,
        path: Option<String>,
    ) -> AiFuture<Result<std::path::PathBuf, TtsError>> {
        Box::pin(async move {
            tts::synth_to_wav(&text, path.as_deref())
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
pub async fn synth_zundamon_wav(text: &str, out_path: &str) -> Result<()> {
    let client = http_client(config::timeouts().ai_http)?;
    let speaker_id = 3; // ずんだもん ノーマル

    let query_resp = client
        .post("http://localhost:50021/audio_query")
        .query(&[("text", text), ("speaker", &speaker_id.to_string())])
        .send()
        .await?;

    let status = query_resp.status();
    let query_body = query_resp.text().await?;
    if !status.is_success() {
        anyhow::bail!("audio_query error {}: {}", status, query_body);
    }

    let synth_resp = client
        .post("http://localhost:50021/synthesis")
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

    tokio::fs::write(out_path, &wav_bytes).await?;
    info!("Zundamon TTS written to {}", out_path);

    Ok(())
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
/// ```no_run
/// # use crate::service::ai::llm::{call_gemini, ChatMessage, Role};
/// # tokio_test::block_on(async {
/// let msgs = vec![ChatMessage { role: Role::User, content: "Hello".into() }];
/// let reply = call_gemini(&msgs).await.unwrap();
/// println!("{}", reply);
/// # });
/// ```
async fn call_gemini(messages: &[ChatMessage]) -> Result<String> {
    let client = http_client(config::timeouts().ai_http)?;

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

fn aws_transcribe_enabled() -> bool {
    config::ai_config().use_aws_transcribe
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
/// ```
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
