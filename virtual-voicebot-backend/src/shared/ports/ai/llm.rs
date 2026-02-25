use std::pin::Pin;

use tokio_stream::Stream;

use crate::shared::error::ai::LlmError;

use super::{AiFuture, ChatMessage};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmStreamEvent {
    Token(String),
    End,
}

pub type LlmStream = Pin<Box<dyn Stream<Item = Result<LlmStreamEvent, LlmError>> + Send>>;

pub trait LlmPort: Send + Sync {
    fn generate_answer(
        &self,
        call_id: String,
        messages: Vec<ChatMessage>,
    ) -> AiFuture<Result<String, LlmError>>;
}

pub trait LlmStreamPort: Send + Sync {
    fn generate_answer_stream(
        &self,
        call_id: String,
        messages: Vec<ChatMessage>,
    ) -> AiFuture<Result<LlmStream, LlmError>>;
}
