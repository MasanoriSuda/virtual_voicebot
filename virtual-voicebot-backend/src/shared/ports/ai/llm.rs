use std::pin::Pin;

use tokio_stream::Stream;

use crate::shared::error::ai::LlmError;

use super::{AiFuture, ChatMessage};

pub type LlmStream = Pin<Box<dyn Stream<Item = Result<String, LlmError>> + Send>>;

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
