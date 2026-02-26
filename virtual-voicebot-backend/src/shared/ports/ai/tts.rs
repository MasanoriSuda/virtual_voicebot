use std::path::PathBuf;
use std::pin::Pin;

use tokio_stream::Stream;

use crate::shared::error::ai::TtsError;

use super::AiFuture;

pub type TtsStream = Pin<Box<dyn Stream<Item = Result<Vec<u8>, TtsError>> + Send>>;

pub trait TtsPort: Send + Sync {
    fn synth_to_wav(
        &self,
        call_id: String,
        text: String,
        path: Option<String>,
    ) -> AiFuture<Result<PathBuf, TtsError>>;
}

pub trait TtsStreamPort: Send + Sync {
    fn synth_stream(&self, call_id: String, text: String) -> AiFuture<Result<TtsStream, TtsError>>;
}
