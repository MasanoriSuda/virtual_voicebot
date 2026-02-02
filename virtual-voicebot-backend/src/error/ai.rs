use thiserror::Error;

#[derive(Debug, Error)]
pub enum AsrError {
    #[error("Transcription failed: {0}")]
    TranscriptionFailed(String),
    #[error("Audio too short")]
    AudioTooShort,
    #[error("Service unavailable")]
    ServiceUnavailable,
    #[error("Timeout")]
    Timeout,
}

#[derive(Debug, Error)]
pub enum IntentError {
    #[error("Classification failed: {0}")]
    ClassificationFailed(String),
    #[error("Unknown intent")]
    UnknownIntent,
}

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("Generation failed: {0}")]
    GenerationFailed(String),
    #[error("Context too long")]
    ContextTooLong,
    #[error("Rate limited")]
    RateLimited,
}

#[derive(Debug, Error)]
pub enum TtsError {
    #[error("Synthesis failed: {0}")]
    SynthesisFailed(String),
    #[error("Text too long")]
    TextTooLong,
    #[error("Voice not found")]
    VoiceNotFound,
}

#[derive(Debug, Error)]
pub enum WeatherError {
    #[error("Weather query failed: {0}")]
    QueryFailed(String),
    #[error("Location not found")]
    LocationNotFound,
    #[error("Service unavailable")]
    ServiceUnavailable,
}

#[derive(Debug, Error)]
pub enum SerError {
    #[error("SER analysis failed: {0}")]
    AnalysisFailed(String),
    #[error("Audio format invalid")]
    InvalidFormat,
    #[error("Model not loaded")]
    ModelNotLoaded,
}
