use std::collections::VecDeque;
use std::fs::{create_dir_all, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::Result;
use hound::{SampleFormat, WavSpec, WavWriter};
use serde::Serialize;

use crate::service::recording;
use crate::protocol::rtp::codec::mulaw_to_linear16;

pub mod merge;

pub struct Recorder {
    call_id: String,
    dir: PathBuf,
    dir_name: String,
    file_name: String,
    writer: Option<WavWriter<BufWriter<File>>>,
    sample_rate: u32,
    channels: u16,
    samples_written: u64,
    started_at: Option<SystemTime>,
    write_meta: bool,
    rx_samples: VecDeque<i16>,
    tx_samples: VecDeque<i16>,
}

impl Recorder {
    pub fn new(call_id: impl Into<String>) -> Self {
        Self::with_file(call_id, "mixed.wav", true)
    }

    pub fn with_file(call_id: impl Into<String>, file_name: &str, write_meta: bool) -> Self {
        let call_id = call_id.into();
        let dir_name = recording::recording_dir_name(&call_id);
        let dir = recording::recording_dir(&call_id);
        Self {
            call_id,
            dir,
            dir_name,
            file_name: file_name.to_string(),
            writer: None,
            sample_rate: 8000,
            channels: 2,
            samples_written: 0,
            started_at: None,
            write_meta,
            rx_samples: VecDeque::new(),
            tx_samples: VecDeque::new(),
        }
    }

    pub fn relative_path(&self) -> String {
        self.dir_name.clone()
    }

    pub fn dir_path(&self) -> &Path {
        &self.dir
    }

    pub fn file_path(&self) -> PathBuf {
        self.dir.join(self.file_name.as_str())
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn is_started(&self) -> bool {
        self.writer.is_some()
    }

    /// 録音を開始する（多重呼び出しは無視）
    pub fn start(&mut self) -> Result<()> {
        if self.writer.is_some() {
            return Ok(());
        }
        create_dir_all(&self.dir)?;
        let spec = WavSpec {
            channels: self.channels,
            sample_rate: self.sample_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let writer = WavWriter::create(self.file_path(), spec)?;
        self.writer = Some(writer);
        self.samples_written = 0;
        self.started_at = Some(SystemTime::now());
        self.rx_samples.clear();
        self.tx_samples.clear();
        Ok(())
    }

    /// μ-law の受信PCMを追記する
    pub fn push_rx_mulaw(&mut self, pcm_mulaw: &[u8]) {
        #[cfg(debug_assertions)]
        dump_raw_mulaw(pcm_mulaw);
        Self::push_mulaw(&mut self.rx_samples, pcm_mulaw);
    }

    /// 送信側PCM（μ-law）を追記する
    pub fn push_tx_mulaw(&mut self, pcm_mulaw: &[u8]) {
        #[cfg(debug_assertions)]
        dump_raw_mulaw(pcm_mulaw);
        Self::push_mulaw(&mut self.tx_samples, pcm_mulaw);
    }

    fn push_mulaw(queue: &mut VecDeque<i16>, pcm_mulaw: &[u8]) {
        for &b in pcm_mulaw {
            queue.push_back(mulaw_to_linear16(b));
        }
    }

    pub fn flush_tick(&mut self) {
        const FRAME_SAMPLES: usize = 160;
        if self.writer.is_none() {
            return;
        }
        let mut rx_frame = [0i16; FRAME_SAMPLES];
        let mut tx_frame = [0i16; FRAME_SAMPLES];
        for i in 0..FRAME_SAMPLES {
            if let Some(sample) = self.rx_samples.pop_front() {
                rx_frame[i] = sample;
            }
            if let Some(sample) = self.tx_samples.pop_front() {
                tx_frame[i] = sample;
            }
        }
        if let Some(w) = self.writer.as_mut() {
            for i in 0..FRAME_SAMPLES {
                let _ = w.write_sample(rx_frame[i]);
                let _ = w.write_sample(tx_frame[i]);
            }
            self.samples_written += FRAME_SAMPLES as u64;
        }
    }

    /// 録音を終了し、mixed.wav と meta.json を確定する
    pub fn stop(&mut self) -> Result<()> {
        if self.writer.is_none() {
            return Ok(());
        }
        while !self.rx_samples.is_empty() || !self.tx_samples.is_empty() {
            self.flush_tick();
        }
        if let Some(writer) = self.writer.take() {
            writer.finalize()?;
        }
        self.rx_samples.clear();
        self.tx_samples.clear();
        if self.write_meta {
            self.write_meta()?;
        }
        Ok(())
    }

    fn write_meta(&self) -> Result<()> {
        #[derive(Serialize)]
        struct MetaFiles<'a> {
            mixed: &'a str,
        }
        #[allow(non_snake_case)]
        #[derive(Serialize)]
        struct Meta<'a> {
            callId: &'a str,
            recordingStartedAt: Option<String>,
            sampleRate: u32,
            channels: u16,
            durationSec: f64,
            files: MetaFiles<'a>,
        }
        let started_at = self
            .started_at
            .map(|t| humantime::format_rfc3339(t).to_string());
        let duration_sec = self.samples_written as f64 / self.sample_rate as f64;
        let meta = Meta {
            callId: &self.call_id,
            recordingStartedAt: started_at,
            sampleRate: self.sample_rate,
            channels: self.channels,
            durationSec: duration_sec,
            files: MetaFiles {
                mixed: self.file_name.as_str(),
            },
        };
        let meta_path = self.dir.join("meta.json");
        let json = serde_json::to_vec_pretty(&meta)?;
        std::fs::write(meta_path, json)?;
        Ok(())
    }
}

#[cfg(debug_assertions)]
fn dump_raw_mulaw(pcm_mulaw: &[u8]) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/raw_mulaw.bin")
    {
        let _ = f.write_all(pcm_mulaw);
    }
}
