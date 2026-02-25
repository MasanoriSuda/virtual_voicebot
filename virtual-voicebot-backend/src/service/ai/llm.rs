#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::OnceLock;

use anyhow::Result;

use crate::shared::config;
use crate::shared::error::ai::LlmError;
use crate::shared::ports::ai::{ChatMessage, LlmStream};

const DEFAULT_SYSTEM_PROMPT: &str = "あなたはボイスボットです。120文字以内で回答してください。";
const PROMPT_FILE_NAME: &str = "prompt.local.txt";

static SYSTEM_PROMPT_CACHE: OnceLock<String> = OnceLock::new();

pub fn init_system_prompt() {
    let _ = system_prompt();
}

/// Provides the active system prompt used by the AI assistant.
///
/// The returned string comes from a cached value that prefers a local override file when present;
/// if no override is available, the built-in default prompt is returned.
///
/// # Examples
///
/// ```
/// use virtual_voicebot_backend::service::ai::llm::system_prompt;
///
/// let prompt = system_prompt();
/// assert!(!prompt.is_empty());
/// ```
pub fn system_prompt() -> String {
    SYSTEM_PROMPT_CACHE
        .get_or_init(|| read_prompt_file().unwrap_or_else(|| DEFAULT_SYSTEM_PROMPT.to_string()))
        .clone()
}

fn read_prompt_file() -> Option<String> {
    // Try current working directory first, then executable directory.
    let paths = [
        PathBuf::from(PROMPT_FILE_NAME),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join(PROMPT_FILE_NAME)))
            .unwrap_or_default(),
    ];
    for path in paths {
        if let Ok(text) = std::fs::read_to_string(&path) {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

/// LLM 呼び出しの薄いI/F（挙動は ai::handle_user_question_from_whisper のLLM部分と同じ）
pub async fn generate_answer(call_id: &str, messages: Vec<ChatMessage>) -> Result<String> {
    super::handle_user_question_from_whisper_llm_only(call_id, messages).await
}

/// LLM 呼び出しのストリーム版ラッパ（初回スコープは Ollama local のみ）。
pub async fn generate_answer_stream(
    call_id: &str,
    messages: Vec<ChatMessage>,
) -> std::result::Result<LlmStream, LlmError> {
    let ai_cfg = config::ai_config();
    if !ai_cfg.llm_local_server_enabled {
        return Err(LlmError::GenerationFailed(
            "LLM local streaming is disabled".to_string(),
        ));
    }

    let system_prompt = system_prompt();
    let first_token_timeout = config::llm_streaming_first_token_timeout();
    super::call_ollama_for_chat_stream(
        &messages,
        &system_prompt,
        &ai_cfg.llm_local_model,
        &ai_cfg.llm_local_server_url,
        config::llm_streaming_connect_timeout(),
        first_token_timeout,
    )
    .await
    .map_err(|e| {
        log::warn!("[llm {call_id}] local streaming failed to start: {e}");
        e
    })
}

/// LLM 呼び出しの薄いラッパ（挙動は ai::handle_user_question_from_whisper と同じ）。
pub async fn handle_user_question_from_whisper(messages: Vec<ChatMessage>) -> Result<String> {
    super::handle_user_question_from_whisper(messages).await
}
