#![allow(dead_code)]

use anyhow::Result;
use crate::ports::ai::ChatMessage;

/// LLM 呼び出しの薄いI/F（挙動は ai::handle_user_question_from_whisper のLLM部分と同じ）
pub async fn generate_answer(messages: Vec<ChatMessage>) -> Result<String> {
    super::handle_user_question_from_whisper_llm_only(messages).await
}

/// LLM 呼び出しの薄いラッパ（挙動は ai::handle_user_question_from_whisper と同じ）。
pub async fn handle_user_question_from_whisper(messages: Vec<ChatMessage>) -> Result<String> {
    super::handle_user_question_from_whisper(messages).await
}
