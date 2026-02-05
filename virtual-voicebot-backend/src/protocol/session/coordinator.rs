#![allow(dead_code)]
// session.rs
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::time::{interval, Duration, Instant, MissedTickBehavior};

#[path = "handlers/mod.rs"]
mod handlers;
#[path = "services/mod.rs"]
mod services;

use crate::protocol::session::state_machine::{SessionEvent, SessionStateMachine};
use crate::protocol::session::types::Sdp;
use crate::protocol::session::types::*;

use crate::shared::config::{self, SessionRuntimeConfig};
use crate::shared::ports::app::{AppEvent, AppEventTx};
use crate::shared::ports::ingest::{IngestPayload, IngestPort, IngestRecording};
use crate::shared::ports::storage::StoragePort;
use crate::protocol::rtp::tx::RtpTxHandle;
use crate::protocol::session::b2bua;
use crate::protocol::session::capture::AudioCapture;
use crate::protocol::session::timers::SessionTimers;
use anyhow::Error;
// log macros used in handler/service modules
use services::playback_service::PlaybackState;

const KEEPALIVE_INTERVAL: Duration = Duration::from_millis(20);
const PLAYBACK_FRAME_INTERVAL: Duration = Duration::from_millis(20);
const TRANSFER_ANNOUNCE_INTERVAL: Duration = Duration::from_secs(5);
const SESSION_CONTROL_CHANNEL_CAPACITY: usize = 64;
const SESSION_MEDIA_CHANNEL_CAPACITY: usize = 64;

pub(crate) const INTRO_MORNING_WAV_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/data/zundamon_intro_morning.wav"
);
pub(crate) const INTRO_AFTERNOON_WAV_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/data/zundamon_intro_afternoon.wav"
);
pub(crate) const INTRO_EVENING_WAV_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/data/zundamon_intro_evening.wav"
);
pub(crate) const IVR_INTRO_WAV_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/data/zundamon_intro_ivr.wav");
pub(crate) const VOICEBOT_INTRO_WAV_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/data/zundamon_intro_ivr_1.wav");
pub(crate) const IVR_INTRO_AGAIN_WAV_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/data/zundamon_intro_ivr_again.wav"
);
pub(crate) const IVR_SENDAI_WAV_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/data/zundamon_sendai.wav");
pub(crate) const IVR_INVALID_WAV_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/data/zundamon_invalid.wav");
pub(crate) const TRANSFER_WAV_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/data/zundamon_try_transfer.wav"
);
pub(crate) const TRANSFER_FAIL_WAV_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/data/zundamon_transfer_fail.wav"
);

pub struct SessionCoordinator {
    state_machine: SessionStateMachine,
    call_id: CallId,
    from_uri: String,
    to_uri: String,
    ingest: crate::protocol::session::ingest_manager::IngestManager,
    recording_base_url: Option<String>,
    storage_port: Arc<dyn StoragePort>,
    peer_sdp: Option<Sdp>,
    local_sdp: Option<Sdp>,
    session_out_tx: mpsc::Sender<(CallId, SessionOut)>,
    control_tx: mpsc::Sender<SessionControlIn>,
    media_tx: mpsc::Sender<SessionMediaIn>,
    app_tx: AppEventTx,
    runtime_cfg: Arc<SessionRuntimeConfig>,
    media_cfg: MediaConfig,
    rtp: crate::protocol::session::rtp_stream_manager::RtpStreamManager,
    recording: crate::protocol::session::recording_manager::RecordingManager,
    started_at: Option<Instant>,
    started_wall: Option<std::time::SystemTime>,
    rtp_last_sent: Option<Instant>,
    a_leg_rtp_started: bool,
    timers: SessionTimers,
    sending_audio: bool,
    playback: Option<PlaybackState>,
    // バッファ/タイマ
    speaking: bool,
    capture: AudioCapture,
    intro_sent: bool,
    ivr_state: IvrState,
    ivr_timeout_stop: Option<oneshot::Sender<()>>,
    b_leg: Option<b2bua::BLeg>,
    transfer_cancel: Option<oneshot::Sender<()>>,
    transfer_announce_stop: Option<oneshot::Sender<()>>,
    ring_delay_cancel: Option<oneshot::Sender<()>>,
    pending_answer: Option<Sdp>,
    outbound_mode: bool,
    outbound_answered: bool,
    outbound_sent_180: bool,
    outbound_sent_183: bool,
    invite_rejected: bool,
    session_expires: Option<Duration>,
    session_refresher: Option<SessionRefresher>,
}

impl SessionCoordinator {
    pub fn spawn(
        call_id: CallId,
        from_uri: String,
        to_uri: String,
        session_out_tx: mpsc::Sender<(CallId, SessionOut)>,
        app_tx: AppEventTx,
        media_cfg: MediaConfig,
        rtp_tx: RtpTxHandle,
        ingest_url: Option<String>,
        recording_base_url: Option<String>,
        ingest_port: Arc<dyn IngestPort>,
        storage_port: Arc<dyn StoragePort>,
        runtime_cfg: Arc<SessionRuntimeConfig>,
    ) -> SessionHandle {
        // Bounded channels: control is reliable, media is drop-on-full upstream.
        let (control_tx, control_rx) = mpsc::channel(SESSION_CONTROL_CHANNEL_CAPACITY);
        let (media_tx, media_rx) = mpsc::channel(SESSION_MEDIA_CHANNEL_CAPACITY);
        let call_id_clone = call_id.clone();
        let mut s = Self {
            state_machine: SessionStateMachine::new(),
            call_id,
            from_uri,
            to_uri,
            ingest: crate::protocol::session::ingest_manager::IngestManager::new(ingest_url, ingest_port),
            recording_base_url,
            storage_port,
            peer_sdp: None,
            local_sdp: None,
            session_out_tx,
            control_tx: control_tx.clone(),
            media_tx: media_tx.clone(),
            app_tx,
            runtime_cfg: runtime_cfg.clone(),
            media_cfg,
            rtp: crate::protocol::session::rtp_stream_manager::RtpStreamManager::new(rtp_tx),
            recording: crate::protocol::session::recording_manager::RecordingManager::new(
                call_id_clone.to_string(),
            ),
            started_at: None,
            started_wall: None,
            rtp_last_sent: None,
            a_leg_rtp_started: false,
            timers: SessionTimers::new(Duration::from_secs(0)),
            sending_audio: false,
            playback: None,
            speaking: false,
            capture: AudioCapture::new(runtime_cfg.vad.clone()),
            intro_sent: false,
            ivr_state: IvrState::default(),
            ivr_timeout_stop: None,
            b_leg: None,
            transfer_cancel: None,
            transfer_announce_stop: None,
            ring_delay_cancel: None,
            pending_answer: None,
            outbound_mode: false,
            outbound_answered: false,
            outbound_sent_180: false,
            outbound_sent_183: false,
            invite_rejected: false,
            session_expires: None,
            session_refresher: None,
        };
        tokio::spawn(async move {
            s.run(control_rx, media_rx).await;
        });
        SessionHandle {
            control_tx,
            media_tx,
        }
    }

    /// Runs the session's main event loop, processing incoming control/media events,
    /// periodic playback ticks, timers, media, SIP/B2BUA actions, IVR flow, and
    /// performing cleanup when the input channel closes or the session ends.
    ///
    /// This method drives the session state machine: it receives events from the
    /// provided control/media receivers, advances the internal `SessState`,
    /// handles playback and recording, manages RTP and SIP interactions, and emits
    /// outgoing actions via the session's configured channels. The loop exits when
    /// the receiver is closed; recorders are stopped after the loop finishes.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tokio::sync::mpsc;
    ///
    /// async fn run_session_example(mut session: crate::protocol::session::Session) {
    ///     let (_control_tx, control_rx) = mpsc::channel(1);
    ///     let (_media_tx, media_rx) = mpsc::channel(1);
    ///     // This will run until control_rx is closed or the session ends.
    ///     session.run(control_rx, media_rx).await;
    /// }
    /// ```
    async fn run(
        &mut self,
        mut control_rx: mpsc::Receiver<SessionControlIn>,
        mut media_rx: mpsc::Receiver<SessionMediaIn>,
    ) {
        let mut playback_tick = interval(PLAYBACK_FRAME_INTERVAL);
        playback_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let mut media_open = true;
        loop {
            tokio::select! {
                biased;
                maybe_ev = control_rx.recv() => {
                    let Some(ev) = maybe_ev else { break; };
                    let current_state = self.state_machine.state();
                    let commands = self.state_machine.process_event(SessionEvent::from(&ev));
                    let advance_state = self.handle_control_event(current_state, ev).await;
                    if advance_state {
                        self.state_machine.apply_commands(&commands);
                    }
                }
                _ = playback_tick.tick() => {
                    if self.playback.is_some() {
                        self.step_playback();
                    }
                }
                maybe_media = media_rx.recv(), if media_open => {
                    match maybe_media {
                        Some(ev) => {
                            self.handle_media_event(ev).await;
                        }
                        None => {
                            media_open = false;
                        }
                    }
                }
            }
        }
        self.stop_recorders();
    }

    fn stop_recorders(&mut self) {
        self.recording.stop_and_merge();
    }

    async fn send_silence_frame(&mut self) -> Result<(), Error> {
        if self.peer_sdp.is_none() {
            return Ok(());
        }
        if self.ivr_state == IvrState::B2buaMode {
            return Ok(());
        }
        if self.sending_audio {
            return Ok(());
        }

        self.align_rtp_clock();

        let frame = vec![0xFFu8; 160]; // μ-law silence
        self.rtp.send_payload(self.call_id.as_str(), frame);
        self.rtp_last_sent = Some(Instant::now());
        Ok(())
    }

    async fn send_ingest(&mut self, status: &str) {
        if !self.ingest.should_post() {
            return;
        }
        let started_at = self.started_wall.unwrap_or_else(std::time::SystemTime::now);
        let ended_at = std::time::SystemTime::now();
        let duration_sec = self.started_at.map(|s| s.elapsed().as_secs()).unwrap_or(0);
        let recording_dir = self.recording.relative_path();
        let recording_url = self
            .recording_base_url
            .as_ref()
            .map(|base| recording_url(base, &recording_dir));
        let recording = recording_url.map(|url| IngestRecording {
            recording_url: url,
            duration_sec,
            sample_rate: self.recording.sample_rate(),
            channels: self.recording.channels(),
        });
        let payload = IngestPayload {
            call_id: self.call_id.clone(),
            from: self.from_uri.clone(),
            to: self.to_uri.clone(),
            started_at,
            ended_at,
            status: status.to_string(),
            summary: String::new(),
            duration_sec,
            recording,
        };
        self.ingest.post_once(payload).await;
    }
}

fn recording_url(base_url: &str, call_id: &str) -> String {
    let base = base_url.trim_end_matches('/');
    format!("{}/recordings/{}/mixed.wav", base, call_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyIngestPort;

    impl IngestPort for DummyIngestPort {
        fn post(
            &self,
            _url: String,
            _payload: IngestPayload,
        ) -> crate::shared::ports::ingest::IngestFuture<Result<(), crate::shared::ports::ingest::IngestError>>
        {
            Box::pin(async { Ok(()) })
        }
    }

    struct DummyStoragePort;

    impl StoragePort for DummyStoragePort {
        fn load_wav_as_pcmu_frames(
            &self,
            _path: &str,
        ) -> Result<Vec<Vec<u8>>, crate::shared::ports::storage::StorageError> {
            Ok(vec![vec![0xFF; 160]])
        }
    }

    fn build_test_session(storage_port: Arc<dyn StoragePort>) -> SessionCoordinator {
        let (session_out_tx, _session_out_rx) = mpsc::channel(32);
        let (app_tx, _app_rx) = crate::shared::ports::app::app_event_channel(16);
        let (control_tx, _control_rx) = mpsc::channel(SESSION_CONTROL_CHANNEL_CAPACITY);
        let (media_tx, _media_rx) = mpsc::channel(SESSION_MEDIA_CHANNEL_CAPACITY);
        let base_cfg = config::Config::from_env().expect("config loads");
        let runtime_cfg = Arc::new(SessionRuntimeConfig::from_env(&base_cfg));
        SessionCoordinator {
            state_machine: SessionStateMachine::new(),
            call_id: CallId::new("test-call".to_string()).expect("valid test call id"),
            from_uri: "sip:from@example.com".to_string(),
            to_uri: "sip:to@example.com".to_string(),
            ingest: crate::protocol::session::ingest_manager::IngestManager::new(
                None,
                Arc::new(DummyIngestPort),
            ),
            recording_base_url: None,
            storage_port,
            peer_sdp: Some(Sdp::pcmu("127.0.0.1", 10000)),
            local_sdp: None,
            session_out_tx,
            control_tx,
            media_tx,
            app_tx,
            runtime_cfg: runtime_cfg.clone(),
            media_cfg: MediaConfig::pcmu("127.0.0.1", 10000),
            rtp: crate::protocol::session::rtp_stream_manager::RtpStreamManager::new(RtpTxHandle::new(
                config::rtp_config().clone(),
            )),
            recording: crate::protocol::session::recording_manager::RecordingManager::new("test-call"),
            started_at: None,
            started_wall: None,
            rtp_last_sent: None,
            a_leg_rtp_started: false,
            timers: SessionTimers::new(Duration::from_secs(0)),
            sending_audio: false,
            playback: None,
            speaking: false,
            capture: AudioCapture::new(runtime_cfg.vad.clone()),
            intro_sent: false,
            ivr_state: IvrState::default(),
            ivr_timeout_stop: None,
            b_leg: None,
            transfer_cancel: None,
            transfer_announce_stop: None,
            ring_delay_cancel: None,
            pending_answer: None,
            outbound_mode: false,
            outbound_answered: false,
            outbound_sent_180: false,
            outbound_sent_183: false,
            invite_rejected: false,
            session_expires: None,
            session_refresher: None,
        }
    }

    #[tokio::test]
    async fn cancel_playback_clears_state() {
        let mut session = build_test_session(Arc::new(DummyStoragePort));
        session.start_playback(&["dummy.wav"]).unwrap();
        assert!(session.playback.is_some());
        assert!(session.sending_audio);
        session.cancel_playback();
        assert!(session.playback.is_none());
        assert!(!session.sending_audio);
    }

    #[tokio::test]
    async fn keepalive_silence_skipped_in_b2bua() {
        let mut session = build_test_session(Arc::new(DummyStoragePort));
        assert!(session.rtp_last_sent.is_none());
        session.send_silence_frame().await.unwrap();
        assert!(session.rtp_last_sent.is_some());

        session.rtp_last_sent = None;
        session.ivr_state = IvrState::B2buaMode;
        session.send_silence_frame().await.unwrap();
        assert!(session.rtp_last_sent.is_none());
    }
}
