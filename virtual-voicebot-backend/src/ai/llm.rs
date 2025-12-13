#![allow(dead_code)]

use anyhow::Result;

/// LLM 呼び出しの薄いI/F（挙動は ai::handle_user_question_from_whisper のLLM部分と同じ）
pub async fn generate_answer(text: &str) -> Result<String> {
    super::handle_user_question_from_whisper_llm_only(text).await
}

/// LLM 呼び出しの薄いラッパ（挙動は ai::handle_user_question_from_whisper と同じ）。
pub async fn handle_user_question_from_whisper(text: &str) -> Result<String> {
    super::handle_user_question_from_whisper(text).await
}
