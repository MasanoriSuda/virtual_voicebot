#![allow(dead_code)]

use anyhow::Result;

/// TTS 呼び出しの薄いI/F（挙動は ai::handle_user_question_from_whisper のTTS部分と同じ）
pub async fn synth_to_wav(text: &str, path: Option<&str>) -> Result<String> {
    let out = path
        .map(|p| p.to_string())
        .unwrap_or_else(|| "/tmp/tts_output.wav".to_string());
    super::synth_zundamon_wav(text, &out).await?;
    Ok(out)
}

/// TTS 呼び出しの薄いラッパ（挙動は ai::synth_zundamon_wav と同じ）。
pub async fn synth_zundamon_wav(text: &str, out_path: &str) -> Result<()> {
    super::synth_zundamon_wav(text, out_path).await
}
