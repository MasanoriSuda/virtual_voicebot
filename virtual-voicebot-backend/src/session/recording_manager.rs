use crate::media::merge::merge_stereo_files;
use crate::media::Recorder;

pub struct RecordingManager {
    call_id: String,
    recorder: Recorder,
    b_leg_recorder: Option<Recorder>,
}

impl RecordingManager {
    pub fn new(call_id: impl Into<String>) -> Self {
        let call_id = call_id.into();
        Self {
            recorder: Recorder::new(call_id.clone()),
            b_leg_recorder: None,
            call_id,
        }
    }

    pub fn start_main(&mut self) -> anyhow::Result<()> {
        self.recorder.start()
    }

    pub fn start_b_leg(&mut self) -> anyhow::Result<()> {
        if let Some(recorder) = self.b_leg_recorder.as_mut() {
            recorder.start()?;
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
            log::warn!(
                "[session {}] failed to finalize b-leg recording: {:?}",
                self.call_id,
                e
            );
        }

        let call_id = self.call_id.clone();
        tokio::spawn(async move {
            let merge_call_id = call_id.clone();
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
                    log::warn!(
                        "[session {}] failed to copy a-leg wav: {:?}",
                        merge_call_id,
                        e
                    );
                }
                let merged_path = dir_path.join("merged.wav");
                merge_stereo_files(&a_path, &b_path, &merged_path)
            });

            match merge_task.await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    log::warn!("[session {}] failed to merge recordings: {:?}", call_id, e)
                }
                Err(e) => {
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
}
