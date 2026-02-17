use crate::protocol::rtp::codec::mulaw_to_linear16;
use crate::shared::config::VadConfig;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CaptureState {
    Idle,
    InSpeech,
}

pub struct AudioCapture {
    vad_threshold: u32,
    start_silence_ms: u64,
    end_silence_ms: u64,
    min_speech_ms: u64,
    max_speech_ms: u64,
    active: bool,
    state: CaptureState,
    start_at: Option<Instant>,
    start_delay_active: bool,
    payloads: Vec<u8>,
    last_voice_len: usize,
    end_silence_ms_accum: u64,
    total_ms: u64,
}

impl AudioCapture {
    pub fn new(cfg: VadConfig) -> Self {
        Self {
            vad_threshold: cfg.rms_threshold,
            start_silence_ms: cfg.start_silence_ms,
            end_silence_ms: cfg.end_silence_ms,
            min_speech_ms: cfg.min_speech_ms,
            max_speech_ms: cfg.max_speech_ms,
            active: false,
            state: CaptureState::Idle,
            start_at: None,
            start_delay_active: cfg.start_silence_ms > 0,
            payloads: Vec::new(),
            last_voice_len: 0,
            end_silence_ms_accum: 0,
            total_ms: 0,
        }
    }

    pub fn start(&mut self) {
        self.active = true;
        self.reset_state();
        if self.start_delay_active && self.start_silence_ms > 0 {
            self.start_at = Some(Instant::now());
        } else {
            self.start_at = None;
        }
    }

    pub fn reset(&mut self) {
        self.active = false;
        self.start_at = None;
        self.reset_state();
    }

    /// Processes a single mu-law audio frame for voice activity detection, accumulating frames into a speech segment and emitting the captured speech when configured end conditions are met.
    ///
    /// This method uses the capture configuration (VAD threshold, start/end silence windows, and min/max speech durations) to decide whether a frame contains voice, to start or continue a speech segment, and to finish and return the collected payload when the segment ends and satisfies the minimum speech duration.
    ///
    /// # Returns
    ///
    /// `Some(Vec<u8>)` containing the captured speech payload (raw mu-law frames) when a speech segment finishes and meets the configured minimum duration; `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use virtual_voicebot_backend::protocol::session::capture::AudioCapture;
    /// use virtual_voicebot_backend::shared::config::VadConfig;
    ///
    /// let cfg = VadConfig {
    ///     rms_threshold: 50,
    ///     start_silence_ms: 0,
    ///     end_silence_ms: 200,
    ///     min_speech_ms: 100,
    ///     max_speech_ms: 5000,
    /// };
    /// let mut ac = AudioCapture::new(cfg);
    /// ac.start();
    /// let frame = vec![0u8; 160]; // one 20ms mu-law frame at 8kHz
    /// let result = ac.ingest(&frame);
    /// // result is `Some` only when a speech segment finishes and meets min_speech_ms
    /// ```
    pub fn ingest(&mut self, payload: &[u8]) -> Option<Vec<u8>> {
        if !self.active || payload.is_empty() {
            return None;
        };

        let frame_ms = (payload.len() as u64 * 1000) / 8000;
        if frame_ms == 0 {
            return None;
        }

        if let Some(start_at) = self.start_at {
            if start_at.elapsed() < Duration::from_millis(self.start_silence_ms) {
                return None;
            }
            self.start_at = None;
            self.start_delay_active = false;
        }

        let rms = rms_energy(payload);
        let is_voice = rms >= self.vad_threshold;

        match self.state {
            CaptureState::Idle => {
                if is_voice {
                    self.state = CaptureState::InSpeech;
                    self.payloads.extend_from_slice(payload);
                    self.last_voice_len = self.payloads.len();
                    self.end_silence_ms_accum = 0;
                    self.total_ms = frame_ms;
                }
            }
            CaptureState::InSpeech => {
                self.payloads.extend_from_slice(payload);
                self.total_ms = self.total_ms.saturating_add(frame_ms);
                if is_voice {
                    self.last_voice_len = self.payloads.len();
                    self.end_silence_ms_accum = 0;
                } else {
                    self.end_silence_ms_accum = self.end_silence_ms_accum.saturating_add(frame_ms);
                }

                if self.total_ms >= self.max_speech_ms
                    || self.end_silence_ms_accum >= self.end_silence_ms
                {
                    return self.finish_capture();
                }
            }
        }

        None
    }

    fn reset_state(&mut self) {
        self.state = CaptureState::Idle;
        self.payloads.clear();
        self.last_voice_len = 0;
        self.end_silence_ms_accum = 0;
        self.total_ms = 0;
    }

    fn finish_capture(&mut self) -> Option<Vec<u8>> {
        let speech_len = self.last_voice_len;
        let speech_ms = (speech_len as u64 * 1000) / 8000;
        let mut out = Vec::new();
        if speech_len > 0 && speech_ms >= self.min_speech_ms {
            out.extend_from_slice(&self.payloads[..speech_len]);
        }
        self.reset_state();
        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }
}

fn rms_energy(payload: &[u8]) -> u32 {
    let mut sum: u64 = 0;
    for &b in payload {
        let sample = mulaw_to_linear16(b) as i32;
        sum = sum.saturating_add((sample * sample) as u64);
    }
    if payload.is_empty() {
        return 0;
    }
    let mean = sum / payload.len() as u64;
    (mean as f64).sqrt() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vad_emits_buffer_after_silence() {
        let cfg = VadConfig {
            rms_threshold: 600,
            start_silence_ms: 0,
            end_silence_ms: 200,
            min_speech_ms: 100,
            max_speech_ms: 5_000,
        };
        let threshold = cfg.rms_threshold;
        let mut capture = AudioCapture::new(cfg);
        capture.start();

        let (voice, silence) = samples_for_threshold(threshold);
        let voice_frame = vec![voice; 160];
        let silence_frame = vec![silence; 160];

        for _ in 0..5 {
            assert!(capture.ingest(&voice_frame).is_none());
        }
        let mut out = None;
        for _ in 0..10 {
            if let Some(buf) = capture.ingest(&silence_frame) {
                out = Some(buf);
                break;
            }
        }
        let buf = out.expect("buffer");
        assert_eq!(buf.len(), 5 * 160);
    }

    #[test]
    fn short_speech_is_dropped() {
        let cfg = VadConfig {
            rms_threshold: 600,
            start_silence_ms: 0,
            end_silence_ms: 200,
            min_speech_ms: 300,
            max_speech_ms: 5_000,
        };
        let threshold = cfg.rms_threshold;
        let mut capture = AudioCapture::new(cfg);
        capture.start();

        let (voice, silence) = samples_for_threshold(threshold);
        let voice_frame = vec![voice; 160];
        let silence_frame = vec![silence; 160];

        for _ in 0..2 {
            assert!(capture.ingest(&voice_frame).is_none());
        }
        let mut out = None;
        for _ in 0..10 {
            if let Some(buf) = capture.ingest(&silence_frame) {
                out = Some(buf);
                break;
            }
        }
        assert!(out.is_none());
    }

    fn samples_for_threshold(threshold: u32) -> (u8, u8) {
        let mut loud = 0x00;
        let mut quiet = 0xff;
        for v in 0u8..=255 {
            if (mulaw_to_linear16(v) as i32).unsigned_abs() >= threshold {
                loud = v;
                break;
            }
        }
        for v in 0u8..=255 {
            if (mulaw_to_linear16(v) as i32).unsigned_abs() < threshold {
                quiet = v;
                break;
            }
        }
        (loud, quiet)
    }
}
