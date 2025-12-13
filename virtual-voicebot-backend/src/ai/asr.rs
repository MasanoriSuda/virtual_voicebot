#![allow(dead_code)]

use anyhow::Result;

/// ASR 呼び出しの薄いラッパ（挙動は ai::transcribe_and_log と同じ）。
/// app からはこの関数を経由させる想定だが、現状の呼び出し順・回数は変えない。
pub async fn transcribe_and_log(wav_path: &str) -> Result<String> {
    super::transcribe_and_log(wav_path).await
}
