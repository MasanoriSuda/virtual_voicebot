use crate::shared::error::ai::LlmError;

use super::{AiFuture, ChatMessage};

pub trait LlmPort: Send + Sync {
    fn generate_answer(
        &self,
        call_id: String,
        messages: Vec<ChatMessage>,
    ) -> AiFuture<Result<String, LlmError>>;
}
