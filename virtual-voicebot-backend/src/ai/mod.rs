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

use crate::config;

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
pub mod llm;
pub mod tts;

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
pub async fn handle_user_question_from_whisper(text: &str) -> Result<String> {
    let answer = handle_user_question_from_whisper_llm_only(text).await?;

    // 一時WAVファイル経由のまま（責務は ai モジュール内に閉じ込める）
    let answer_wav = "/tmp/ollama_answer.wav";
    synth_zundamon_wav(&answer, answer_wav).await?;

    Ok(answer_wav.to_string())
}

/// LLM 部分のみを切り出した I/F（app→ai で分離できるようにする）
pub async fn handle_user_question_from_whisper_llm_only(text: &str) -> Result<String> {
    info!("User question (whisper): {}", text);
    let llm_prompt = build_llm_prompt(text);

    let answer = match call_gemini(&llm_prompt).await {
        Ok(ans) => {
            info!("LLM answer (gemini): {}", ans);
            ans
        }
        Err(gemini_err) => {
            log::error!("call_gemini failed: {gemini_err:?}, falling back to ollama");
            match call_ollama(&llm_prompt).await {
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

fn build_llm_prompt(user_text: &str) -> String {
    format!(
        "以下の質問に「はい」または「いいえ」で回答し、回答全体を30文字以内にまとめてください。質問: {}",
        user_text
    )
}

async fn call_ollama(question: &str) -> Result<String> {
    let client = http_client(config::timeouts().ai_http)?;

    let req = OllamaChatRequest {
        model: "gemma3:4b".to_string(),
        messages: vec![OllamaMessage {
            role: "user".to_string(),
            content: question.to_string(),
        }],
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

async fn call_gemini(question: &str) -> Result<String> {
    let client = http_client(config::timeouts().ai_http)?;

    let api_key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY must be set");

    let model =
        std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-2.5-flash-lite".to_string());

    let url = format!(
        "https://generativelanguage.googleapis.com/v1/models/{}:generateContent?key={}",
        model, api_key
    );

    let req_body = GeminiRequest {
        contents: vec![GeminiContent {
            parts: vec![GeminiPart {
                text: question.to_string(),
            }],
        }],
    };

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
    std::env::var("USE_AWS_TRANSCRIBE")
        .map(|v| {
            let lower = v.to_ascii_lowercase();
            lower == "1" || lower == "true" || lower == "yes"
        })
        .unwrap_or(false)
}

async fn transcribe_with_aws(wav_path: &str) -> Result<String> {
    let bucket = std::env::var("AWS_TRANSCRIBE_BUCKET")
        .map_err(|_| anyhow!("AWS_TRANSCRIBE_BUCKET must be set when USE_AWS_TRANSCRIBE=1"))?;
    let prefix = std::env::var("AWS_TRANSCRIBE_PREFIX").unwrap_or_else(|_| "voicebot".to_string());

    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let job_name = format!("voicebot-{}", timestamp);

    let normalized_prefix = if prefix.is_empty() {
        String::new()
    } else if prefix.ends_with('/') {
        prefix
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
        .bucket(&bucket)
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
    let http = http_client(crate::config::timeouts().ai_http)?;
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

                        log::info!("AWS transcript raw JSON: {}", body_text);

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
