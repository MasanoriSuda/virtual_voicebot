use crate::shared::error::ai::IntentError;

use super::{AiFuture, Intent};

pub trait IntentPort: Send + Sync {
    fn classify_intent(
        &self,
        call_id: String,
        text: String,
    ) -> AiFuture<Result<Intent, IntentError>>;
}
