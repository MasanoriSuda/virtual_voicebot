use crate::error::ai::AsrError;

use super::{AiFuture, AsrChunk};

pub trait AsrPort: Send + Sync {
    fn transcribe_chunks(
        &self,
        call_id: String,
        chunks: Vec<AsrChunk>,
    ) -> AiFuture<Result<String, AsrError>>;
}
