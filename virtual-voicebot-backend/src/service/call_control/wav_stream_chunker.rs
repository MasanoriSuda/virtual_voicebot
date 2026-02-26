#[derive(Clone, Debug)]
struct WavPcmFormat {
    audio_format: u16,
    channels: u16,
    sample_rate: u32,
    byte_rate: u32,
    block_align: u16,
    bits_per_sample: u16,
}

#[derive(Debug)]
pub(super) struct WavStreamChunker {
    header_buf: Vec<u8>,
    pcm_buf: Vec<u8>,
    fmt: Option<WavPcmFormat>,
    data_start: Option<usize>,
    segment_pcm_bytes: usize,
}

impl WavStreamChunker {
    pub(super) fn new(segment_pcm_bytes: usize) -> Self {
        Self {
            header_buf: Vec::new(),
            pcm_buf: Vec::new(),
            fmt: None,
            data_start: None,
            segment_pcm_bytes: segment_pcm_bytes.max(1),
        }
    }

    pub(super) fn push(&mut self, bytes: &[u8]) -> Result<Vec<Vec<u8>>, String> {
        if bytes.is_empty() {
            return Ok(Vec::new());
        }

        if self.fmt.is_none() {
            self.header_buf.extend_from_slice(bytes);
            self.try_parse_header()?;
        } else {
            self.pcm_buf.extend_from_slice(bytes);
        }

        self.take_ready_segments()
    }

    pub(super) fn finish(&mut self) -> Result<Option<Vec<u8>>, String> {
        if self.fmt.is_none() {
            if self.header_buf.is_empty() {
                return Err("empty wav stream".to_string());
            }
            return Err("incomplete wav header".to_string());
        }

        if self.pcm_buf.is_empty() {
            return Ok(None);
        }

        let align = self.block_align_bytes();
        if !self.pcm_buf.len().is_multiple_of(align) {
            return Err(format!(
                "pcm payload size is not aligned: len={} align={align}",
                self.pcm_buf.len()
            ));
        }

        let pcm = std::mem::take(&mut self.pcm_buf);
        Ok(Some(self.build_pcm_wav(&pcm)?))
    }

    fn try_parse_header(&mut self) -> Result<(), String> {
        if self.fmt.is_some() {
            return Ok(());
        }

        let b = &self.header_buf;
        if b.len() < 12 {
            return Ok(());
        }
        if &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
            return Err("unsupported wav header (expected RIFF/WAVE)".to_string());
        }

        let mut offset = 12usize;
        let mut fmt: Option<WavPcmFormat> = None;

        while offset + 8 <= b.len() {
            let chunk_id = &b[offset..offset + 4];
            let chunk_size = le_u32(&b[offset + 4..offset + 8]) as usize;
            let chunk_data_start = offset + 8;

            if chunk_id == b"data" {
                let Some(fmt) = fmt else {
                    return Err("wav data chunk appeared before fmt chunk".to_string());
                };
                validate_fmt(&fmt)?;
                self.fmt = Some(fmt);
                self.data_start = Some(chunk_data_start);
                if b.len() > chunk_data_start {
                    self.pcm_buf.extend_from_slice(&b[chunk_data_start..]);
                }
                self.header_buf.clear();
                return Ok(());
            }

            let chunk_data_end = chunk_data_start
                .checked_add(chunk_size)
                .ok_or_else(|| "wav chunk size overflow".to_string())?;
            let padded_end = chunk_data_end + (chunk_size % 2);
            if padded_end > b.len() {
                return Ok(());
            }

            if chunk_id == b"fmt " {
                if chunk_size < 16 {
                    return Err("wav fmt chunk too small".to_string());
                }
                let data = &b[chunk_data_start..chunk_data_end];
                fmt = Some(WavPcmFormat {
                    audio_format: le_u16(&data[0..2]),
                    channels: le_u16(&data[2..4]),
                    sample_rate: le_u32(&data[4..8]),
                    byte_rate: le_u32(&data[8..12]),
                    block_align: le_u16(&data[12..14]),
                    bits_per_sample: le_u16(&data[14..16]),
                });
            }

            offset = padded_end;
        }

        Ok(())
    }

    fn take_ready_segments(&mut self) -> Result<Vec<Vec<u8>>, String> {
        if self.fmt.is_none() {
            return Ok(Vec::new());
        }
        let align = self.block_align_bytes();
        let mut threshold = self.segment_pcm_bytes;
        if threshold < align {
            threshold = align;
        }
        threshold -= threshold % align;
        if threshold == 0 {
            threshold = align;
        }

        let mut out = Vec::new();
        while self.pcm_buf.len() >= threshold {
            let rest = self.pcm_buf.split_off(threshold);
            let pcm = std::mem::replace(&mut self.pcm_buf, rest);
            out.push(self.build_pcm_wav(&pcm)?);
        }
        Ok(out)
    }

    fn block_align_bytes(&self) -> usize {
        self.fmt
            .as_ref()
            .map(|f| usize::from(f.block_align.max(1)))
            .unwrap_or(2)
    }

    fn build_pcm_wav(&self, pcm: &[u8]) -> Result<Vec<u8>, String> {
        let fmt = self
            .fmt
            .as_ref()
            .ok_or_else(|| "wav header not parsed".to_string())?;
        validate_fmt(fmt)?;

        let data_len =
            u32::try_from(pcm.len()).map_err(|_| "pcm payload too large for wav".to_string())?;
        let riff_size = 36u32
            .checked_add(data_len)
            .ok_or_else(|| "wav riff size overflow".to_string())?;

        let mut out = Vec::with_capacity(44 + pcm.len());
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&riff_size.to_le_bytes());
        out.extend_from_slice(b"WAVE");
        out.extend_from_slice(b"fmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&fmt.audio_format.to_le_bytes());
        out.extend_from_slice(&fmt.channels.to_le_bytes());
        out.extend_from_slice(&fmt.sample_rate.to_le_bytes());
        out.extend_from_slice(&fmt.byte_rate.to_le_bytes());
        out.extend_from_slice(&fmt.block_align.to_le_bytes());
        out.extend_from_slice(&fmt.bits_per_sample.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&data_len.to_le_bytes());
        out.extend_from_slice(pcm);
        Ok(out)
    }
}

fn validate_fmt(fmt: &WavPcmFormat) -> Result<(), String> {
    if fmt.audio_format != 1 {
        return Err(format!("unsupported wav audio format {}", fmt.audio_format));
    }
    if fmt.channels != 1 {
        return Err(format!("unsupported wav channels {}", fmt.channels));
    }
    if fmt.bits_per_sample != 16 {
        return Err(format!(
            "unsupported wav bits_per_sample {}",
            fmt.bits_per_sample
        ));
    }
    if fmt.block_align == 0 {
        return Err("invalid wav block_align 0".to_string());
    }
    Ok(())
}

fn le_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

fn le_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

#[cfg(test)]
mod tests {
    use super::WavStreamChunker;

    fn pcm16_mono_wav_bytes(samples: &[i16], sample_rate: u32) -> Vec<u8> {
        let mut pcm = Vec::with_capacity(samples.len() * 2);
        for s in samples {
            pcm.extend_from_slice(&s.to_le_bytes());
        }
        let data_len = pcm.len() as u32;
        let byte_rate = sample_rate * 2;
        let mut out = Vec::with_capacity(44 + pcm.len());
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data_len).to_le_bytes());
        out.extend_from_slice(b"WAVE");
        out.extend_from_slice(b"fmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&sample_rate.to_le_bytes());
        out.extend_from_slice(&byte_rate.to_le_bytes());
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&16u16.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&data_len.to_le_bytes());
        out.extend_from_slice(&pcm);
        out
    }

    #[test]
    fn emits_segments_and_tail_from_streamed_wav() {
        let wav = pcm16_mono_wav_bytes(&[1, 2, 3, 4, 5, 6], 24_000);
        let mut chunker = WavStreamChunker::new(8); // 4 samples (8 bytes)

        let out0 = chunker.push(&wav[..10]).unwrap();
        assert!(out0.is_empty());

        let out1 = chunker.push(&wav[10..47]).unwrap();
        assert!(out1.is_empty());

        let out2 = chunker.push(&wav[47..]).unwrap();
        assert_eq!(out2.len(), 1);

        let tail = chunker.finish().unwrap().unwrap();
        assert!(tail.len() > 44);
    }

    #[test]
    fn rejects_non_pcm_fmt() {
        let mut wav = pcm16_mono_wav_bytes(&[1, 2], 24_000);
        // audio_format in fmt chunk
        wav[20] = 3;
        wav[21] = 0;
        let mut chunker = WavStreamChunker::new(4);
        let err = chunker.push(&wav).unwrap_err();
        assert!(err.contains("unsupported wav audio format"));
    }

    #[test]
    fn finish_errors_on_incomplete_header() {
        let mut chunker = WavStreamChunker::new(4);
        chunker.push(b"RIFF").unwrap();
        let err = chunker.finish().unwrap_err();
        assert!(err.contains("incomplete wav header"));
    }
}
