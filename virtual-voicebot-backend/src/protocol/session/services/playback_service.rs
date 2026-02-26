use std::io::ErrorKind;

use anyhow::{anyhow, Error};

use super::super::SessionCoordinator;
use log::{info, warn};
use tokio::task::spawn_blocking;
use tokio::time::timeout;

use crate::protocol::session::types::{IvrState, PlaybackGenerationId};
use crate::shared::config;

#[derive(Debug)]
pub(crate) struct PlaybackState {
    pub(crate) frames: Vec<Vec<u8>>,
    pub(crate) index: usize,
}

#[derive(Debug)]
pub(crate) struct PendingUtterance {
    pub(crate) generation_id: PlaybackGenerationId,
    pub(crate) frames: Vec<Vec<u8>>,
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
        let mut frames = Vec::new();
        for path in paths {
            let mut loaded = self.load_frames_with_timeout(path).await?;
            frames.append(&mut loaded);
        }
        if frames.is_empty() {
            anyhow::bail!("no frames");
        }
        self.begin_playback_frames(frames, None)
    }

    pub(crate) async fn enqueue_playback(
        &mut self,
        path: &str,
        generation_id: PlaybackGenerationId,
    ) -> Result<(), Error> {
        let frames = match self.load_frames_with_timeout(path).await {
            Ok(frames) => {
                self.cleanup_tts_temp_wav(path).await;
                frames
            }
            Err(e) => {
                self.cleanup_tts_temp_wav(path).await;
                return Err(e);
            }
        };
        if frames.is_empty() {
            anyhow::bail!("no frames");
        }

        if self.playback.is_some() {
            if self.playback_generation_id == Some(generation_id) {
                self.playback_queue.push_back(PendingUtterance {
                    generation_id,
                    frames,
                });
                return Ok(());
            }
            info!(
                "[session {}] interrupt-first enqueue: replace playback old_generation={:?} new_generation={}",
                self.call_id, self.playback_generation_id, generation_id
            );
            self.cancel_playback();
        } else if self
            .playback_queue
            .front()
            .map(|p| p.generation_id != generation_id)
            .unwrap_or(false)
        {
            info!(
                "[session {}] dropping stale queued playback for new generation={}",
                self.call_id, generation_id
            );
            self.playback_queue.clear();
        }

        self.stop_ivr_timeout();
        self.begin_playback_frames(frames, Some(generation_id))
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
        if let Some(next) = self.playback_queue.pop_front() {
            if let Err(e) = self.begin_playback_frames(next.frames, Some(next.generation_id)) {
                warn!(
                    "[session {}] failed to start queued playback generation={}: {:?}",
                    self.call_id, next.generation_id, e
                );
                self.finish_playback(restart_ivr_timeout);
            }
            return;
        }
        self.clear_playback_state();

        if self.announce_mode {
            if self.voicemail_mode {
                self.announce_mode = false;
                info!(
                    "[session {}] voicemail announcement finished, recording continues",
                    self.call_id
                );
            } else if self.recording_notice_pending {
                self.announce_mode = false;
                self.recording_notice_pending = false;
                info!(
                    "[session {}] recording notice finished, requesting transfer",
                    self.call_id
                );
                let _ = self.control_tx.try_send(
                    crate::protocol::session::types::SessionControlIn::AppTransferRequest {
                        person: "recording_notice".to_string(),
                    },
                );
                return;
            } else {
                self.announce_mode = false;
                info!(
                    "[session {}] announcement finished, requesting hangup",
                    self.call_id
                );
                let _ = self
                    .control_tx
                    .try_send(crate::protocol::session::types::SessionControlIn::AppHangup);
                return;
            }
        }

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
        if self.playback.is_none() && self.playback_queue.is_empty() {
            self.sending_audio = false;
            return;
        }
        info!("[session {}] playback cancelled", self.call_id);
        self.clear_playback_state();
        self.playback_queue.clear();
        self.announce_mode = false;
        self.recording_notice_pending = false;
    }

    async fn load_frames_with_timeout(&self, path: &str) -> Result<Vec<Vec<u8>>, Error> {
        let io_timeout = config::timeouts().recording_io;
        let storage_port = self.storage_port.clone();
        let path = path.to_string();
        let load = spawn_blocking(move || storage_port.load_wav_as_pcmu_frames(&path));
        match timeout(io_timeout, load).await {
            Ok(joined) => {
                Ok(joined.map_err(|e| anyhow!("load_wav_as_pcmu_frames task failed: {}", e))??)
            }
            Err(_) => Err(anyhow!("load_wav_as_pcmu_frames timed out")),
        }
    }

    async fn cleanup_tts_temp_wav(&self, path: &str) {
        if !is_tts_temp_wav_path(path) {
            return;
        }
        match tokio::fs::remove_file(path).await {
            Ok(()) => {}
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => warn!(
                "[session {}] failed to remove TTS temp wav path={}: {:?}",
                self.call_id, path, err
            ),
        }
    }

    fn clear_playback_state(&mut self) {
        self.playback = None;
        self.playback_generation_id = None;
        self.sending_audio = false;
    }

    fn begin_playback_frames(
        &mut self,
        frames: Vec<Vec<u8>>,
        generation_id: Option<PlaybackGenerationId>,
    ) -> Result<(), Error> {
        if frames.is_empty() {
            anyhow::bail!("no frames");
        }
        self.align_rtp_clock();
        self.playback = Some(PlaybackState { frames, index: 0 });
        self.playback_generation_id = generation_id;
        self.sending_audio = true;
        Ok(())
    }
}

fn is_tts_temp_wav_path(path: &str) -> bool {
    path.starts_with("/tmp/tts_output_") || path.starts_with("/tmp/tts_stream_output_")
}
