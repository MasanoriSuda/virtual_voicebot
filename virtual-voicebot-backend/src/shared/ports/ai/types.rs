/// Chunked μ-law audio input for ASR.
#[derive(Debug, Clone)]
pub struct AsrChunk {
    pub pcm_mulaw: Vec<u8>,
    pub end: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

pub type Intent = String;

pub type WeatherResponse = String;

#[derive(Debug, Clone)]
pub struct WeatherQuery {
    pub location: String,
    pub date: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SerInputPcm {
    pub session_id: String,
    pub stream_id: String,
    pub pcm: Vec<i16>,
    pub sample_rate: u32,
    pub channels: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Emotion {
    Neutral,
    Happy,
    Sad,
    Angry,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct SerResult {
    pub session_id: String,
    pub stream_id: String,
    pub emotion: Emotion,
    pub confidence: f32,
    pub arousal: Option<f32>,
    pub valence: Option<f32>,
}

pub type SerOutcome = SerResult;
