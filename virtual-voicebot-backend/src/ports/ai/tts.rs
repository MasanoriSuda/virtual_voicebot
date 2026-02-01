use std::path::PathBuf;

use crate::error::ai::TtsError;

use super::AiFuture;

pub trait TtsPort: Send + Sync {
    fn synth_to_wav(
        &self,
        text: String,
        path: Option<String>,
    ) -> AiFuture<Result<PathBuf, TtsError>>;
}
