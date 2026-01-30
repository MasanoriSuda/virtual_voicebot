use serde::{Deserialize, Serialize};

use crate::config;
use crate::ports::ai::{Emotion, SerError, SerInputPcm, SerResult};

#[derive(Serialize)]
struct SerRequest<'a> {
    session_id: &'a str,
    stream_id: &'a str,
    sample_rate: u32,
    channels: u8,
    pcm: &'a [i16],
}

#[derive(Deserialize)]
struct SerResponse {
    emotion: Option<String>,
    confidence: Option<f32>,
    arousal: Option<f32>,
    valence: Option<f32>,
}

/// Analyze PCM input for speech emotion recognition using the configured SER service.
///
/// If the configured SER URL is empty, returns the same result as `dummy_result(input)`.
/// If `input.pcm` is empty, returns a `SerResult` with `Emotion::Unknown` and `confidence` 0.0.
/// Otherwise, sends `input` to the configured SER HTTP endpoint and:
/// - on HTTP/network errors or JSON parse errors, returns a `SerError` containing the `session_id` and a descriptive `reason`,
/// - on non-success HTTP status, returns a `SerError` whose `reason` includes the status code and response body,
/// - on success, returns a `SerResult` with the mapped `Emotion`, `confidence` (defaulting to 0.0 if missing), and optional `arousal`/`valence`.
///
/// # Examples
///
/// ```
/// # // This example assumes a tokio runtime and that SerInputPcm and related types are in scope.
/// # use crate::{analyze, SerInputPcm, Emotion};
/// # tokio_test::block_on(async {
/// let input = SerInputPcm {
///     session_id: "sess1".to_string(),
///     stream_id: "stream1".to_string(),
///     sample_rate: 16000,
///     channels: 1,
///     pcm: vec![0i16; 16000],
/// };
///
/// match analyze(input).await {
///     Ok(result) => {
///         // result.emotion, result.confidence, result.arousal, result.valence
///         let _ = result;
///     }
///     Err(err) => {
///         // err.session_id and err.reason describe the failure
///         let _ = err;
///     }
/// }
/// # });
/// ```
pub async fn analyze(input: SerInputPcm) -> std::result::Result<SerResult, SerError> {
    let ser_url = config::ai_config().ser_url.as_deref().unwrap_or("");
    if ser_url.trim().is_empty() {
        return Ok(dummy_result(input));
    }
    if input.pcm.is_empty() {
        return Ok(SerResult {
            session_id: input.session_id,
            stream_id: input.stream_id,
            emotion: Emotion::Unknown,
            confidence: 0.0,
            arousal: None,
            valence: None,
        });
    }

    let request = SerRequest {
        session_id: input.session_id.as_str(),
        stream_id: input.stream_id.as_str(),
        sample_rate: input.sample_rate,
        channels: input.channels,
        pcm: input.pcm.as_slice(),
    };

    let client = super::http_client(config::timeouts().ai_http).map_err(|e| SerError {
        session_id: input.session_id.clone(),
        reason: format!("ser client error: {e}"),
    })?;

    let resp = client
        .post(ser_url)
        .json(&request)
        .send()
        .await
        .map_err(|e| SerError {
            session_id: input.session_id.clone(),
            reason: format!("ser http error: {e}"),
        })?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(SerError {
            session_id: input.session_id.clone(),
            reason: format!("ser http status {}: {}", status.as_u16(), body),
        });
    }

    let response: SerResponse = resp.json().await.map_err(|e| SerError {
        session_id: input.session_id.clone(),
        reason: format!("ser response parse error: {e}"),
    })?;

    let emotion = map_emotion(response.emotion.as_deref().unwrap_or("unknown"));
    let confidence = response.confidence.unwrap_or(0.0);

    Ok(SerResult {
        session_id: input.session_id,
        stream_id: input.stream_id,
        emotion,
        confidence,
        arousal: response.arousal,
        valence: response.valence,
    })
}

fn dummy_result(input: SerInputPcm) -> SerResult {
    let (emotion, confidence) = if input.pcm.is_empty() {
        (Emotion::Unknown, 0.0)
    } else {
        (Emotion::Neutral, 0.5)
    };
    SerResult {
        session_id: input.session_id,
        stream_id: input.stream_id,
        emotion,
        confidence,
        arousal: None,
        valence: None,
    }
}

fn map_emotion(raw: &str) -> Emotion {
    match raw.trim().to_ascii_lowercase().as_str() {
        "neutral" | "calm" => Emotion::Neutral,
        "happy" | "joy" | "joyful" => Emotion::Happy,
        "sad" | "sadness" => Emotion::Sad,
        "angry" | "anger" => Emotion::Angry,
        _ => Emotion::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_emotion_matches_expected_labels() {
        assert_eq!(map_emotion("neutral"), Emotion::Neutral);
        assert_eq!(map_emotion("Happy"), Emotion::Happy);
        assert_eq!(map_emotion("sadness"), Emotion::Sad);
        assert_eq!(map_emotion("ANGER"), Emotion::Angry);
        assert_eq!(map_emotion("unknown"), Emotion::Unknown);
    }
}