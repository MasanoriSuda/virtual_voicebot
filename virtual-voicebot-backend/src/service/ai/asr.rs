#![allow(dead_code)]

use anyhow::{anyhow, Result};
use hound::{SampleFormat, WavSpec, WavWriter};
use std::path::{Component, Path};

use crate::protocol::rtp::codec::mulaw_to_linear16;
use crate::shared::ports::ai::AsrChunk;

/// ASR 呼び出しの薄いラッパ（挙動は ai::transcribe_and_log と同じ）。
/// app からはこの関数を経由させる想定だが、現状の呼び出し順・回数は変えない。
pub async fn transcribe_and_log(call_id: &str, wav_path: &str) -> Result<String> {
    super::transcribe_and_log(call_id, wav_path).await
}

/// μ-law チャンクを WAV にまとめ、既存ASRを呼ぶ（挙動は従来と同じ）
pub async fn transcribe_chunks(call_id: &str, chunks: &[AsrChunk]) -> Result<String> {
    let mut pcmu: Vec<u8> = Vec::new();
    for ch in chunks {
        pcmu.extend_from_slice(&ch.pcm_mulaw);
        if ch.end {
            break;
        }
    }
    let safe_call_id = sanitize_call_id_for_tmp_filename(call_id)?;
    let wav_path = format!("/tmp/asr_input_{}.wav", safe_call_id);
    write_mulaw_to_wav(&pcmu, &wav_path)?;
    super::transcribe_and_log(call_id, &wav_path).await
}

fn sanitize_call_id_for_tmp_filename(call_id: &str) -> Result<String> {
    let mut parts = Vec::new();
    for component in Path::new(call_id).components() {
        match component {
            Component::Normal(part) => {
                let part = part.to_string_lossy();
                if part.is_empty() {
                    return Err(anyhow!("invalid call_id for tmp filename"));
                }
                parts.push(part.into_owned());
            }
            _ => return Err(anyhow!("invalid call_id for tmp filename")),
        }
    }

    if parts.is_empty() {
        return Err(anyhow!("invalid call_id for tmp filename"));
    }

    Ok(parts.join("_"))
}

const HALLUCINATION_PATTERNS: &[&str] = &[
    "ご視聴ありがとうございました",
    "チャンネル登録",
    "高評価",
    "いいね",
    "お願いします",
    "ありがとうございました",
];

pub(crate) fn is_hallucination(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return true;
    }
    HALLUCINATION_PATTERNS
        .iter()
        .any(|pattern| trimmed.contains(pattern))
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;

    #[test]
    fn hallucination_patterns_match() {
        assert!(is_hallucination("ご視聴ありがとうございました"));
        assert!(is_hallucination("チャンネル登録よろしくお願いします"));
        assert!(is_hallucination("高評価お願いします"));
    }

    #[test]
    fn non_hallucination_passes() {
        assert!(!is_hallucination("こんにちは、元気ですか？"));
    }

    #[test]
    fn sanitize_call_id_rejects_parent_dir() {
        assert!(sanitize_call_id_for_tmp_filename("../escape").is_err());
    }

    #[test]
    fn sanitize_call_id_flattens_normal_components() {
        let got = sanitize_call_id_for_tmp_filename("call/part-1").unwrap();
        assert_eq!(got, "call_part-1");
    }
}

fn write_mulaw_to_wav(payloads: &[u8], path: impl AsRef<Path>) -> Result<()> {
    let spec = WavSpec {
        channels: 1,
        sample_rate: 8000,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut writer = WavWriter::create(path, spec)?;
    for &b in payloads {
        writer.write_sample(mulaw_to_linear16(b))?;
    }
    writer.finalize()?;
    Ok(())
}
