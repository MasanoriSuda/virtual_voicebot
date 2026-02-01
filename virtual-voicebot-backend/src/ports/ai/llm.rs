use crate::error::ai::LlmError;

use super::{AiFuture, ChatMessage};

pub trait LlmPort: Send + Sync {
    fn generate_answer(&self, messages: Vec<ChatMessage>) -> AiFuture<Result<String, LlmError>>;
}
