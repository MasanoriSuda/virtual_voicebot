use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("io error: {0}")]
    Io(String),
    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),
}

pub trait StoragePort: Send + Sync {
    fn load_wav_as_pcmu_frames(&self, path: &str) -> Result<Vec<Vec<u8>>, StorageError>;
}
