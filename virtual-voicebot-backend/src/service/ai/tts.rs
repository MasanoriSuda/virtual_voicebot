#![allow(dead_code)]

use anyhow::{anyhow, Result};
use std::path::{Component, Path};
use std::time::{SystemTime, UNIX_EPOCH};

/// TTS 呼び出しの薄いI/F（挙動は ai::handle_user_question_from_whisper のTTS部分と同じ）
pub async fn synth_to_wav(call_id: &str, text: &str, path: Option<&str>) -> Result<String> {
    let out = path
        .map(|p| p.to_string())
        .map(Ok)
        .unwrap_or_else(|| default_tts_output_path(call_id))?;
    super::synth_zundamon_wav(call_id, text, &out).await?;
    Ok(out)
}

/// TTS 呼び出しの薄いラッパ（挙動は ai::synth_zundamon_wav と同じ）。
pub async fn synth_zundamon_wav(call_id: &str, text: &str, out_path: &str) -> Result<()> {
    super::synth_zundamon_wav(call_id, text, out_path).await
}

fn default_tts_output_path(call_id: &str) -> Result<String> {
    let safe_call_id = sanitize_call_id_for_tmp_filename(call_id)?;
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    Ok(format!(
        "/tmp/tts_output_{}_{}.wav",
        safe_call_id, unique_suffix
    ))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_call_id_rejects_parent_dir() {
        assert!(sanitize_call_id_for_tmp_filename("../escape").is_err());
    }

    #[test]
    fn default_tts_output_path_uses_sanitized_call_id() {
        let path = default_tts_output_path("call/part-1").unwrap();
        assert!(path.starts_with("/tmp/tts_output_call_part-1_"));
        assert!(path.ends_with(".wav"));
    }
}
