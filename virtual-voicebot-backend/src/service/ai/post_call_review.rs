use std::path::Path;

use anyhow::{anyhow, Result};
use chrono::Utc;
use reqwest::multipart;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::shared::config;
use crate::shared::ports::ai::{ChatMessage, Role};

#[derive(Clone, Debug)]
pub struct PostCallReviewOutput {
    pub summary_text: Option<String>,
    pub transcript_json: Option<Value>,
    pub review_json: Option<Value>,
}

#[derive(Deserialize)]
struct AmiVoiceResponse {
    #[serde(default)]
    results: Vec<AmiVoiceResult>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

#[derive(Deserialize)]
struct AmiVoiceResult {
    #[serde(default)]
    confidence: Option<f64>,
    #[serde(default)]
    starttime: Option<f64>,
    #[serde(default)]
    endtime: Option<f64>,
    #[serde(default)]
    text: Option<String>,
}

const REVIEW_SYSTEM_PROMPT: &str = r#"
あなたは電話応対品質をレビューする業務監査エージェントです。
入力される通話文字起こしを読み、必ずJSONオブジェクトだけを返してください。
Markdown、説明文、コードブロックは禁止です。

JSON schema:
{
  "version": 1,
  "summary": "短い通話概要",
  "customerIntent": "顧客の主目的",
  "responseEvaluation": {
    "status": "good | needs_attention | poor | unknown",
    "notes": "応対品質の評価"
  },
  "unresolvedItems": ["未解決事項"],
  "nextActions": [
    {
      "type": "follow_up | confirm | escalate | none | other",
      "priority": "low | medium | high",
      "label": "次にやること"
    }
  ],
  "riskSignals": [
    {
      "type": "complaint_risk | confusion | urgent | other",
      "severity": "low | medium | high",
      "label": "リスク内容"
    }
  ],
  "evidence": [
    {
      "label": "判断根拠のラベル",
      "speaker": "caller | bot | system | unknown",
      "startSec": 0.0,
      "endSec": 1.0,
      "text": "根拠発話"
    }
  ]
}
"#;

pub async fn generate_post_call_review(
    call_log_id: &str,
    audio_path: &Path,
) -> Result<PostCallReviewOutput> {
    let review_cfg = config::post_call_review_config();
    if !review_cfg.enabled {
        return Err(anyhow!("post-call review is disabled"));
    }

    let transcript = tokio::time::timeout(
        review_cfg.timeout,
        transcribe_with_amivoice(call_log_id, audio_path),
    )
    .await
    .map_err(|_| anyhow!("post-call review timeout"))??;

    let review_json = tokio::time::timeout(review_cfg.timeout, generate_review_json(&transcript))
        .await
        .map_err(|_| anyhow!("post-call review LLM timeout"))??;

    let summary_text = review_json
        .get("summary")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

    Ok(PostCallReviewOutput {
        summary_text,
        transcript_json: Some(transcript),
        review_json: Some(review_json),
    })
}

async fn transcribe_with_amivoice(call_log_id: &str, audio_path: &Path) -> Result<Value> {
    let review_cfg = config::post_call_review_config();
    let api_key = review_cfg
        .amivoice_api_key
        .as_deref()
        .ok_or_else(|| anyhow!("AMIVOICE_API_KEY is not set"))?;
    let bytes = tokio::fs::read(audio_path)
        .await
        .map_err(|e| anyhow!("recording read failed: {e}"))?;
    if bytes.is_empty() {
        return Err(anyhow!("recording file is empty"));
    }

    let client = reqwest::Client::builder()
        .timeout(review_cfg.amivoice_timeout)
        .build()?;
    let url = format!(
        "{}/recognize",
        review_cfg.amivoice_base_url.trim_end_matches('/')
    );
    let part = multipart::Part::bytes(bytes)
        .file_name("mixed.wav")
        .mime_str("audio/wav")?;
    let form = multipart::Form::new()
        .text("d", review_cfg.amivoice_engine.clone())
        .text("u", api_key.to_string())
        .part("a", part);

    let response = client.post(url).multipart(form).send().await?;
    let status = response.status();
    let body_text = response.text().await?;
    if !status.is_success() {
        return Err(anyhow!(
            "AmiVoice HTTP error {} (body_len={})",
            status,
            body_text.len()
        ));
    }

    let body: AmiVoiceResponse = serde_json::from_str(&body_text)
        .map_err(|e| anyhow!("AmiVoice response parse failed: {e}"))?;
    if body.code.as_deref().unwrap_or_default() != "" {
        return Err(anyhow!(
            "AmiVoice recognition failed: code={} message_len={}",
            body.code.unwrap_or_default(),
            body.message.unwrap_or_default().len()
        ));
    }

    Ok(normalize_amivoice_response(
        call_log_id,
        &review_cfg.amivoice_engine,
        body,
    ))
}

fn normalize_amivoice_response(call_log_id: &str, engine: &str, body: AmiVoiceResponse) -> Value {
    let now = Utc::now().to_rfc3339();
    let mut utterances = Vec::new();
    for (index, result) in body.results.into_iter().enumerate() {
        let text = result.text.unwrap_or_default();
        let text = text.trim();
        if text.is_empty() {
            continue;
        }
        utterances.push(json!({
            "seq": index + 1,
            "speaker": "unknown",
            "text": text,
            "timestamp": now,
            "isFinal": true,
            "startSec": result.starttime.map(|value| value / 1000.0),
            "endSec": result.endtime.map(|value| value / 1000.0),
            "confidence": result.confidence,
        }));
    }

    let full_text = body.text.unwrap_or_default();
    if utterances.is_empty() && !full_text.trim().is_empty() {
        utterances.push(json!({
            "seq": 1,
            "speaker": "unknown",
            "text": full_text.trim(),
            "timestamp": now,
            "isFinal": true,
            "startSec": Value::Null,
            "endSec": Value::Null,
            "confidence": Value::Null,
        }));
    }

    json!({
        "provider": "amivoice",
        "language": "ja-JP",
        "callLogId": call_log_id,
        "text": full_text,
        "utterances": utterances,
        "rawProvider": {
            "name": "amivoice",
            "engine": engine,
        },
    })
}

async fn generate_review_json(transcript: &Value) -> Result<Value> {
    let transcript_text = transcript_text_for_prompt(transcript);
    if transcript_text.trim().is_empty() {
        return Err(anyhow!("transcript text is empty"));
    }

    let user_prompt = format!(
        "以下の電話応対文字起こしをレビューしてください。\n\n{}",
        transcript_text
    );
    let messages = vec![ChatMessage {
        role: Role::User,
        content: user_prompt,
    }];
    let raw = generate_review_llm_text(&messages).await?;
    parse_review_json(&raw)
}

async fn generate_review_llm_text(messages: &[ChatMessage]) -> Result<String> {
    let ai_cfg = config::ai_config();
    let review_cfg = config::post_call_review_config();

    if ai_cfg.openai_llm_enabled {
        if let Some(api_key) = ai_cfg.openai_api_key.as_deref() {
            return call_openai_review_json(
                messages,
                &review_cfg.llm_model,
                &ai_cfg.openai_base_url,
                api_key,
                ai_cfg.llm_cloud_timeout,
            )
            .await;
        }
    }

    if ai_cfg.llm_local_server_enabled {
        return call_ollama_review(
            messages,
            &ai_cfg.llm_local_model,
            &ai_cfg.llm_local_server_url,
            ai_cfg.llm_local_timeout,
        )
        .await;
    }

    if ai_cfg.llm_raspi_enabled {
        if let Some(url) = ai_cfg.llm_raspi_url.as_deref() {
            return call_ollama_review(
                messages,
                &ai_cfg.llm_raspi_model,
                url,
                ai_cfg.llm_raspi_timeout,
            )
            .await;
        }
    }

    Err(anyhow!("no LLM stage available for post-call review"))
}

async fn call_openai_review_json(
    messages: &[ChatMessage],
    model: &str,
    base_url: &str,
    api_key: &str,
    timeout: std::time::Duration,
) -> Result<String> {
    let client = reqwest::Client::builder().timeout(timeout).build()?;
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let body = json!({
        "model": model,
        "messages": build_chat_messages(messages, REVIEW_SYSTEM_PROMPT, "user", "assistant"),
        "response_format": { "type": "json_object" },
    });

    let response = client
        .post(url)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await?;
    let status = response.status();
    let body_text = response.text().await?;
    if !status.is_success() {
        return Err(anyhow!(
            "OpenAI review HTTP error {} (body_len={})",
            status,
            body_text.len()
        ));
    }
    parse_chat_content(&body_text, "OpenAI review")
}

async fn call_ollama_review(
    messages: &[ChatMessage],
    model: &str,
    endpoint_url: &str,
    timeout: std::time::Duration,
) -> Result<String> {
    let client = reqwest::Client::builder().timeout(timeout).build()?;
    let body = json!({
        "model": model,
        "messages": build_chat_messages(messages, REVIEW_SYSTEM_PROMPT, "user", "assistant"),
        "stream": false,
        "format": "json",
    });

    let response = client.post(endpoint_url).json(&body).send().await?;
    let status = response.status();
    let body_text = response.text().await?;
    if !status.is_success() {
        return Err(anyhow!(
            "Ollama review HTTP error {} (body_len={})",
            status,
            body_text.len()
        ));
    }
    parse_chat_content(&body_text, "Ollama review")
}

fn build_chat_messages(
    messages: &[ChatMessage],
    system_prompt: &str,
    user_role: &str,
    assistant_role: &str,
) -> Vec<Value> {
    let mut out = Vec::with_capacity(messages.len() + 1);
    out.push(json!({
        "role": "system",
        "content": system_prompt,
    }));
    for message in messages {
        let role = match message.role {
            Role::User => user_role,
            Role::Assistant => assistant_role,
        };
        out.push(json!({
            "role": role,
            "content": message.content.clone(),
        }));
    }
    out
}

fn parse_chat_content(body_text: &str, provider: &str) -> Result<String> {
    let value: Value = serde_json::from_str(body_text)
        .map_err(|e| anyhow!("{provider} response parse failed: {e}"))?;
    let content = value
        .pointer("/choices/0/message/content")
        .or_else(|| value.pointer("/message/content"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("{provider} response content missing"))?;
    Ok(content.to_string())
}

fn transcript_text_for_prompt(transcript: &Value) -> String {
    let utterances = transcript
        .get("utterances")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if utterances.is_empty() {
        return transcript
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
    }

    utterances
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            let text = item.get("text").and_then(Value::as_str)?.trim();
            if text.is_empty() {
                return None;
            }
            let start = item.get("startSec").and_then(Value::as_f64);
            let end = item.get("endSec").and_then(Value::as_f64);
            let speaker = item
                .get("speaker")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            Some(match (start, end) {
                (Some(start), Some(end)) => {
                    format!(
                        "{}. [{:.1}-{:.1}s] {}: {}",
                        index + 1,
                        start,
                        end,
                        speaker,
                        text
                    )
                }
                _ => format!("{}. {}: {}", index + 1, speaker, text),
            })
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_review_json(raw: &str) -> Result<Value> {
    let candidate = extract_json_object(raw);
    let value: Value =
        serde_json::from_str(candidate).map_err(|e| anyhow!("review JSON parse failed: {e}"))?;
    if !value.is_object() {
        return Err(anyhow!("review JSON must be object"));
    }
    let summary = value
        .get("summary")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    if summary.is_empty() {
        return Err(anyhow!("review summary is missing"));
    }
    Ok(value)
}

fn extract_json_object(raw: &str) -> &str {
    let trimmed = raw.trim();
    if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
        if start <= end {
            return &trimmed[start..=end];
        }
    }
    trimmed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_json_object_accepts_markdown_fence() {
        let raw = "```json\n{\"summary\":\"ok\"}\n```";
        assert_eq!(extract_json_object(raw), "{\"summary\":\"ok\"}");
    }

    #[test]
    fn parse_review_json_requires_summary() {
        assert!(parse_review_json(r#"{"summary":"通話概要"}"#).is_ok());
        assert!(parse_review_json(r#"{"summary":""}"#).is_err());
    }

    #[test]
    fn transcript_text_prefers_utterances() {
        let transcript = json!({
            "text": "fallback",
            "utterances": [
                {"speaker":"unknown","text":"配送状況を確認したい","startSec":1.0,"endSec":3.0}
            ]
        });
        let text = transcript_text_for_prompt(&transcript);
        assert!(text.contains("配送状況"));
        assert!(text.contains("1.0-3.0s"));
    }
}
