use anyhow::{anyhow, Error};

use super::super::SessionCoordinator;
use log::{info, warn};
use tokio::task::spawn_blocking;
use tokio::time::timeout;

use crate::protocol::session::types::IvrState;
use crate::shared::config;

#[derive(Debug)]
pub(crate) struct PlaybackState {
    pub(crate) frames: Vec<Vec<u8>>,
    pub(crate) index: usize,
}

impl SessionCoordinator {
    pub(crate) async fn start_playback(&mut self, paths: &[&str]) -> Result<(), Error> {
        let Some(_dst) = self.peer_rtp_addr() else {
            warn!(
                "[session {}] start_playback skipped: no peer RTP address",
                self.call_id
            );
            return Ok(());
        };
        self.cancel_playback();
        self.stop_ivr_timeout();
        let io_timeout = config::timeouts().recording_io;
        let mut frames = Vec::new();
        for path in paths {
            let storage_port = self.storage_port.clone();
            let path = (*path).to_string();
            let load = spawn_blocking(move || storage_port.load_wav_as_pcmu_frames(&path));
            let mut loaded = match timeout(io_timeout, load).await {
                Ok(joined) => joined
                    .map_err(|e| anyhow!("load_wav_as_pcmu_frames task failed: {}", e))??,
                Err(_) => {
                    return Err(anyhow!("load_wav_as_pcmu_frames timed out"));
                }
            };
            frames.append(&mut loaded);
        }
        if frames.is_empty() {
            anyhow::bail!("no frames");
        }
        self.align_rtp_clock();
        self.playback = Some(PlaybackState { frames, index: 0 });
        self.sending_audio = true;
        Ok(())
    }

    pub(crate) fn step_playback(&mut self) {
        let Some(mut state) = self.playback.take() else {
            return;
        };
        if state.index >= state.frames.len() {
            self.finish_playback(true);
            return;
        }
        let frame = state.frames[state.index].clone();
        state.index += 1;
        self.recording.push_tx(&frame);
        self.rtp.send_payload(self.call_id.as_str(), frame);
        self.rtp_last_sent = Some(tokio::time::Instant::now());
        if state.index < state.frames.len() {
            self.playback = Some(state);
        } else {
            self.finish_playback(true);
        }
    }

    pub(crate) fn finish_playback(&mut self, restart_ivr_timeout: bool) {
        self.playback = None;
        self.sending_audio = false;
        if self.ivr_state == IvrState::VoicebotIntroPlaying {
            self.ivr_state = IvrState::VoicebotMode;
            self.capture.reset();
            self.capture.start();
        }
        if restart_ivr_timeout && self.ivr_state == IvrState::IvrMenuWaiting {
            self.reset_ivr_timeout();
        }
    }

    pub(crate) fn cancel_playback(&mut self) {
        if self.playback.is_some() {
            info!("[session {}] playback cancelled", self.call_id);
        }
        self.finish_playback(false);
    }
}
