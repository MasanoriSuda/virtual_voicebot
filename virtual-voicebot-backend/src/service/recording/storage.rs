use hound::WavReader;

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

fn linear16_to_mulaw(sample: i16) -> u8 {
    const BIAS: i16 = 0x84;
    const CLIP: i16 = 32635;
    let mut s = sample;
    let mut sign = 0u8;
    if s < 0 {
        s = -s;
        sign = 0x80;
    }
    if s > CLIP {
        s = CLIP;
    }
    s += BIAS;
    let mut segment: u8 = 0;
    let mut value = (s as u16) >> 7;
    while value > 0 {
        segment += 1;
        value >>= 1;
        if segment >= 8 {
            break;
        }
    }
    let mantissa = ((s >> (segment + 3)) & 0x0F) as u8;
    !(sign | (segment << 4) | mantissa)
}
