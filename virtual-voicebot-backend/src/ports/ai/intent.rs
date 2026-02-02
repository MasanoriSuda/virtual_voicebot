use crate::error::ai::IntentError;

use super::{AiFuture, Intent};

pub trait IntentPort: Send + Sync {
    fn classify_intent(&self, text: String) -> AiFuture<Result<Intent, IntentError>>;
}
