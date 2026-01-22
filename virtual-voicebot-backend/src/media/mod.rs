use std::fs::{create_dir_all, File};
use std::io::BufWriter;
use std::path::PathBuf;
use std::time::SystemTime;

use anyhow::Result;
use hound::{SampleFormat, WavSpec, WavWriter};
use serde::Serialize;

use crate::recording;
use crate::rtp::codec::mulaw_to_linear16;

pub struct Recorder {
    call_id: String,
    dir: PathBuf,
    dir_name: String,
    writer: Option<WavWriter<BufWriter<File>>>,
    sample_rate: u32,
    channels: u16,
    samples_written: u64,
    started_at: Option<SystemTime>,
}

impl Recorder {
    pub fn new(call_id: impl Into<String>) -> Self {
        let call_id = call_id.into();
        let dir_name = recording::recording_dir_name(&call_id);
        let dir = recording::recording_dir(&call_id);
        Self {
            call_id,
            dir,
            dir_name,
            writer: None,
            sample_rate: 8000,
            channels: 1,
            samples_written: 0,
            started_at: None,
        }
    }

    pub fn relative_path(&self) -> String {
        self.dir_name.clone()
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
        let writer = WavWriter::create(self.dir.join("mixed.wav"), spec)?;
        self.writer = Some(writer);
        self.samples_written = 0;
        self.started_at = Some(SystemTime::now());
        Ok(())
    }

    /// μ-law の受信PCMを追記する
    pub fn push_rx_mulaw(&mut self, pcm_mulaw: &[u8]) {
        self.push_mulaw(pcm_mulaw);
    }

    /// 送信側PCM（μ-law）を追記する
    pub fn push_tx_mulaw(&mut self, pcm_mulaw: &[u8]) {
        self.push_mulaw(pcm_mulaw);
    }

    fn push_mulaw(&mut self, pcm_mulaw: &[u8]) {
        #[cfg(debug_assertions)]
        dump_raw_mulaw(pcm_mulaw);
        if let Some(w) = self.writer.as_mut() {
            for &b in pcm_mulaw {
                let _ = w.write_sample(mulaw_to_linear16(b));
                self.samples_written += 1;
            }
        }
    }

    /// 録音を終了し、mixed.wav と meta.json を確定する
    pub fn stop(&mut self) -> Result<()> {
        if let Some(writer) = self.writer.take() {
            writer.finalize()?;
        } else {
            return Ok(());
        }
        self.write_meta()
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
            files: MetaFiles { mixed: "mixed.wav" },
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
