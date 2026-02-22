use std::path::PathBuf;

use crate::shared::error::ai::TtsError;

use super::AiFuture;

pub trait TtsPort: Send + Sync {
    fn synth_to_wav(
        &self,
        call_id: String,
        text: String,
        path: Option<String>,
    ) -> AiFuture<Result<PathBuf, TtsError>>;
}
