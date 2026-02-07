use std::future::Future;
use std::pin::Pin;

pub mod asr;
pub mod intent;
pub mod llm;
pub mod ser;
pub mod tts;
pub mod types;
pub mod weather;

pub use asr::AsrPort;
pub use intent::IntentPort;
pub use llm::LlmPort;
pub use ser::SerPort;
pub use tts::TtsPort;
pub use types::{
    AsrChunk, ChatMessage, Emotion, Intent, Role, SerInputPcm, SerOutcome, SerResult, WeatherQuery,
    WeatherResponse,
};
pub use weather::WeatherPort;

pub use crate::shared::error::ai::{
    AsrError, IntentError, LlmError, SerError, TtsError, WeatherError,
};

pub type AiFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// Aggregate trait for bundling AI services.
pub trait AiServices: AsrPort + IntentPort + LlmPort + WeatherPort + TtsPort + SerPort {}

impl<T> AiServices for T where T: AsrPort + IntentPort + LlmPort + WeatherPort + TtsPort + SerPort {}
