#![allow(dead_code)]

use anyhow::Result;

/// LLM 呼び出しの薄いラッパ（挙動は ai::handle_user_question_from_whisper と同じ）。
pub async fn handle_user_question_from_whisper(text: &str) -> Result<String> {
    super::handle_user_question_from_whisper(text).await
}
