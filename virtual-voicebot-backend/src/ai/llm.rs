#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::OnceLock;

use anyhow::Result;

use crate::ports::ai::ChatMessage;

const DEFAULT_SYSTEM_PROMPT: &str = "あなたはボイスボットです。120文字以内で回答してください。";
const PROMPT_FILE_NAME: &str = "prompt.local.txt";

static SYSTEM_PROMPT_CACHE: OnceLock<String> = OnceLock::new();

pub fn init_system_prompt() {
    let _ = system_prompt();
}

pub fn system_prompt() -> String {
    SYSTEM_PROMPT_CACHE
        .get_or_init(|| read_prompt_file().unwrap_or_else(|| DEFAULT_SYSTEM_PROMPT.to_string()))
        .clone()
}

fn read_prompt_file() -> Option<String> {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let path = base.join(PROMPT_FILE_NAME);
    let text = std::fs::read_to_string(path).ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

/// LLM 呼び出しの薄いI/F（挙動は ai::handle_user_question_from_whisper のLLM部分と同じ）
pub async fn generate_answer(messages: Vec<ChatMessage>) -> Result<String> {
    super::handle_user_question_from_whisper_llm_only(messages).await
}

/// LLM 呼び出しの薄いラッパ（挙動は ai::handle_user_question_from_whisper と同じ）。
pub async fn handle_user_question_from_whisper(messages: Vec<ChatMessage>) -> Result<String> {
    super::handle_user_question_from_whisper(messages).await
}
