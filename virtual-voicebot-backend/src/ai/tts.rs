#![allow(dead_code)]

use anyhow::Result;

/// TTS 呼び出しの薄いラッパ（挙動は ai::synth_zundamon_wav と同じ）。
pub async fn synth_zundamon_wav(text: &str, out_path: &str) -> Result<()> {
    super::synth_zundamon_wav(text, out_path).await
}
