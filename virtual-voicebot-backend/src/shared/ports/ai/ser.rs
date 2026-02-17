use crate::shared::error::ai::SerError;

use super::{AiFuture, SerInputPcm, SerOutcome};

pub trait SerPort: Send + Sync {
    fn analyze(&self, input: SerInputPcm) -> AiFuture<Result<SerOutcome, SerError>>;
}
