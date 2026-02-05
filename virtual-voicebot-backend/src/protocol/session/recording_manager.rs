use std::sync::{Arc, Mutex};

use thiserror::Error;

use crate::shared::media::merge::merge_stereo_files;
use crate::shared::media::Recorder;

#[derive(Debug, Error, Clone)]
pub enum RecordingError {
    #[error("recording start failed: {0}")]
    Start(String),
    #[error("recording stop failed: {0}")]
    Stop(String),
    #[error("recording copy failed: {0}")]
    Copy(String),
    #[error("recording merge failed: {0}")]
    Merge(String),
}

#[derive(Clone, Default)]
struct RecordingErrorSink {
    errors: Arc<Mutex<Vec<RecordingError>>>,
}

impl RecordingErrorSink {
    fn push(&self, err: RecordingError) {
        let mut errors = self.errors.lock().unwrap();
        errors.push(err);
    }

    fn drain(&self) -> Vec<RecordingError> {
        let mut errors = self.errors.lock().unwrap();
        std::mem::take(&mut *errors)
    }
}

pub struct RecordingManager {
    call_id: String,
    recorder: Recorder,
    b_leg_recorder: Option<Recorder>,
    error_sink: RecordingErrorSink,
}

impl RecordingManager {
    pub fn new(call_id: impl Into<String>) -> Self {
        let call_id = call_id.into();
        Self {
            recorder: Recorder::new(call_id.clone()),
            b_leg_recorder: None,
            call_id,
            error_sink: RecordingErrorSink::default(),
        }
    }

    pub fn start_main(&mut self) -> Result<(), RecordingError> {
        self.recorder
            .start()
            .map_err(|e| RecordingError::Start(e.to_string()))
    }

    pub fn start_b_leg(&mut self) -> Result<(), RecordingError> {
        if let Some(recorder) = self.b_leg_recorder.as_mut() {
            recorder
                .start()
                .map_err(|e| RecordingError::Start(e.to_string()))?;
        }
        Ok(())
    }

    pub fn ensure_b_leg(&mut self) {
        if self.b_leg_recorder.is_none() {
            self.b_leg_recorder = Some(Recorder::with_file(
                self.call_id.clone(),
                "b_leg.wav",
                false,
            ));
        }
    }

    pub fn is_started(&self) -> bool {
        self.recorder.is_started()
    }

    pub fn push_rx(&mut self, payload: &[u8]) {
        self.recorder.push_rx_mulaw(payload);
    }

    pub fn push_tx(&mut self, payload: &[u8]) {
        self.recorder.push_tx_mulaw(payload);
    }

    pub fn push_b_leg_rx(&mut self, payload: &[u8]) {
        if let Some(recorder) = self.b_leg_recorder.as_mut() {
            recorder.push_rx_mulaw(payload);
        }
    }

    pub fn push_b_leg_tx(&mut self, payload: &[u8]) {
        if let Some(recorder) = self.b_leg_recorder.as_mut() {
            recorder.push_tx_mulaw(payload);
        }
    }

    pub fn flush_tick(&mut self) {
        self.recorder.flush_tick();
        if let Some(recorder) = self.b_leg_recorder.as_mut() {
            recorder.flush_tick();
        }
    }

    pub fn stop_and_merge(&mut self) {
        let a_path = self.recorder.file_path();
        let dir_path = self.recorder.dir_path().to_path_buf();
        if let Err(e) = self.recorder.stop() {
            self.error_sink
                .push(RecordingError::Stop(e.to_string()));
            log::warn!(
                "[session {}] failed to finalize recording: {:?}",
                self.call_id,
                e
            );
        }

        let Some(mut b_recorder) = self.b_leg_recorder.take() else {
            return;
        };
        let b_path = b_recorder.file_path();
        if let Err(e) = b_recorder.stop() {
            self.error_sink
                .push(RecordingError::Stop(e.to_string()));
            log::warn!(
                "[session {}] failed to finalize b-leg recording: {:?}",
                self.call_id,
                e
            );
        }

        let call_id = self.call_id.clone();
        let error_sink = self.error_sink.clone();
        tokio::spawn(async move {
            let merge_call_id = call_id.clone();
            let error_sink_blocking = error_sink.clone();
            let merge_task = tokio::task::spawn_blocking(move || {
                if !a_path.exists() || !b_path.exists() {
                    log::warn!(
                        "[session {}] merge skipped (missing recording file)",
                        merge_call_id
                    );
                    return Ok(());
                }
                let a_leg_path = dir_path.join("a_leg.wav");
                if let Err(e) = std::fs::copy(&a_path, &a_leg_path) {
                    error_sink_blocking.push(RecordingError::Copy(e.to_string()));
                    log::warn!(
                        "[session {}] failed to copy a-leg wav: {:?}",
                        merge_call_id,
                        e
                    );
                }
                let merged_path = dir_path.join("merged.wav");
                merge_stereo_files(&a_path, &b_path, &merged_path)
                    .map_err(|e| RecordingError::Merge(e.to_string()))
            });

            match merge_task.await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    error_sink.push(e.clone());
                    log::warn!("[session {}] failed to merge recordings: {:?}", call_id, e)
                }
                Err(e) => {
                    error_sink.push(RecordingError::Merge(e.to_string()));
                    log::warn!("[session {}] merge task failed: {:?}", call_id, e)
                }
            }
        });
    }

    pub fn relative_path(&self) -> String {
        self.recorder.relative_path()
    }

    pub fn sample_rate(&self) -> u32 {
        self.recorder.sample_rate()
    }

    pub fn channels(&self) -> u16 {
        self.recorder.channels()
    }

    pub fn take_errors(&self) -> Vec<RecordingError> {
        self.error_sink.drain()
    }
}
