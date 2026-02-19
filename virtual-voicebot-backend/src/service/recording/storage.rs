use hound::WavReader;

use crate::protocol::rtp::codec::linear16_to_mulaw;
use crate::shared::ports::storage::{StorageError, StoragePort};

pub struct FileStoragePort;

impl FileStoragePort {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileStoragePort {
    fn default() -> Self {
        Self::new()
    }
}

impl StoragePort for FileStoragePort {
    fn load_wav_as_pcmu_frames(&self, path: &str) -> Result<Vec<Vec<u8>>, StorageError> {
        load_wav_as_pcmu_frames(path)
    }
}

fn load_wav_as_pcmu_frames(path: &str) -> Result<Vec<Vec<u8>>, StorageError> {
    let mut reader = WavReader::open(path).map_err(|e| StorageError::Io(e.to_string()))?;
    let spec = reader.spec();
    if spec.channels != 1 || spec.bits_per_sample != 16 {
        return Err(StorageError::UnsupportedFormat(
            "expected mono 16bit wav".to_string(),
        ));
    }
    let mut samples: Vec<i16> = Vec::new();
    for s in reader.samples::<i16>() {
        samples.push(s.map_err(|e| StorageError::Io(e.to_string()))?);
    }
    let base_samples: Vec<i16> = match spec.sample_rate {
        8000 => samples,
        24000 => samples.iter().step_by(3).copied().collect(),
        other => {
            return Err(StorageError::UnsupportedFormat(format!(
                "unsupported sample rate {other}"
            )))
        }
    };
    let mut frames = Vec::new();
    let mut cur = Vec::with_capacity(160);
    for s in base_samples {
        cur.push(linear16_to_mulaw(s));
        if cur.len() == 160 {
            frames.push(cur.clone());
            cur.clear();
        }
    }
    if !cur.is_empty() {
        while cur.len() < 160 {
            cur.push(0xFF);
        }
        frames.push(cur);
    }
    Ok(frames)
}
