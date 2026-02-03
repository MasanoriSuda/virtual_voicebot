#![allow(dead_code)]
// session.rs
use std::sync::Arc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;
use tokio::time::{interval, Duration, Instant, MissedTickBehavior};

#[path = "handlers/mod.rs"]
mod handlers;
#[path = "services/mod.rs"]
mod services;

use crate::session::state_machine::{SessionEvent, SessionStateMachine};
use crate::session::types::Sdp;
use crate::session::types::*;

use crate::app::AppEvent;
use crate::config;
use crate::http::ingest::IngestPort;
use crate::recording;
use crate::recording::storage::StoragePort;
use crate::rtp::tx::RtpTxHandle;
use crate::session::b2bua;
use crate::session::capture::AudioCapture;
use crate::session::timers::SessionTimers;
use anyhow::Error;
// log macros used in handler/service modules
use serde_json::json;
use services::playback_service::PlaybackState;

const KEEPALIVE_INTERVAL: Duration = Duration::from_millis(20);
const PLAYBACK_FRAME_INTERVAL: Duration = Duration::from_millis(20);
const TRANSFER_ANNOUNCE_INTERVAL: Duration = Duration::from_secs(5);

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

#[derive(Clone)]
pub struct SessionHandle {
    pub tx_in: UnboundedSender<SessionIn>,
}

pub struct SessionCoordinator {
    state_machine: SessionStateMachine,
    call_id: CallId,
    from_uri: String,
    to_uri: String,
    ingest: crate::session::ingest_manager::IngestManager,
    recording_base_url: Option<String>,
    storage_port: Arc<dyn StoragePort>,
    peer_sdp: Option<Sdp>,
    local_sdp: Option<Sdp>,
    session_out_tx: UnboundedSender<(CallId, SessionOut)>,
    tx_in: UnboundedSender<SessionIn>,
    app_tx: UnboundedSender<AppEvent>,
    media_cfg: MediaConfig,
    rtp: crate::session::rtp_stream_manager::RtpStreamManager,
    recording: crate::session::recording_manager::RecordingManager,
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
        session_out_tx: UnboundedSender<(CallId, SessionOut)>,
        app_tx: UnboundedSender<AppEvent>,
        media_cfg: MediaConfig,
        rtp_tx: RtpTxHandle,
        ingest_url: Option<String>,
        recording_base_url: Option<String>,
        ingest_port: Arc<dyn IngestPort>,
        storage_port: Arc<dyn StoragePort>,
    ) -> SessionHandle {
        let (tx_in, rx_in) = tokio::sync::mpsc::unbounded_channel();
        let call_id_clone = call_id.clone();
        let mut s = Self {
            state_machine: SessionStateMachine::new(),
            call_id,
            from_uri,
            to_uri,
            ingest: crate::session::ingest_manager::IngestManager::new(ingest_url, ingest_port),
            recording_base_url,
            storage_port,
            peer_sdp: None,
            local_sdp: None,
            session_out_tx,
            tx_in: tx_in.clone(),
            app_tx,
            media_cfg,
            rtp: crate::session::rtp_stream_manager::RtpStreamManager::new(rtp_tx),
            recording: crate::session::recording_manager::RecordingManager::new(call_id_clone),
            started_at: None,
            started_wall: None,
            rtp_last_sent: None,
            a_leg_rtp_started: false,
            timers: SessionTimers::new(Duration::from_secs(0)),
            sending_audio: false,
            playback: None,
            speaking: false,
            capture: AudioCapture::new(config::vad_config().clone()),
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
            s.run(rx_in).await;
        });
        SessionHandle { tx_in }
    }

    /// Runs the session's main event loop, processing incoming `SessionIn` events,
    /// periodic playback ticks, timers, media, SIP/B2BUA actions, IVR flow, and
    /// performing cleanup when the input channel closes or the session ends.
    ///
    /// This method drives the session state machine: it receives events from the
    /// provided `UnboundedReceiver<SessionIn>`, advances the internal `SessState`,
    /// handles playback and recording, manages RTP and SIP interactions, and emits
    /// outgoing actions via the session's configured channels. The loop exits when
    /// the receiver is closed; recorders are stopped after the loop finishes.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio::sync::mpsc::UnboundedReceiver;
    ///
    /// // In an async context where `session` and `rx` are available:
    /// async fn run_session_example(mut session: crate::session::Session, rx: UnboundedReceiver<crate::session::SessionIn>) {
    ///     // This will run until `rx` is closed or the session ends.
    ///     session.run(rx).await;
    /// }
    /// ```
    async fn run(&mut self, mut rx: UnboundedReceiver<SessionIn>) {
        let mut playback_tick = interval(PLAYBACK_FRAME_INTERVAL);
        playback_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                biased;
                _ = playback_tick.tick() => {
                    if self.playback.is_some() {
                        self.step_playback();
                    }
                }
                maybe_ev = rx.recv() => {
                    let Some(ev) = maybe_ev else { break; };
                    let current_state = self.state_machine.state();
                    let commands = self.state_machine.process_event(SessionEvent::from(&ev));
                    let advance_state = self.handle_event(current_state, ev).await;
                    if advance_state {
                        self.state_machine.apply_commands(&commands);
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
        self.rtp.send_payload(&self.call_id, frame);
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
            .map(|base| recording::recording_url(base, &recording_dir));

        let payload = json!({
            "callId": self.call_id,
            "from": self.from_uri,
            "to": self.to_uri,
            "startedAt": humantime::format_rfc3339(started_at).to_string(),
            "endedAt": humantime::format_rfc3339(ended_at).to_string(),
            "status": status,
            "summary": "",
            "durationSec": duration_sec,
            "recording": recording_url.as_ref().map(|url| json!({
                "recordingUrl": url,
                "durationSec": duration_sec,
                "sampleRate": self.recording.sample_rate(),
                "channels": self.recording.channels()
            })),
        });

        self.ingest.post_once(payload).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyIngestPort;

    impl IngestPort for DummyIngestPort {
        fn post(
            &self,
            _url: String,
            _payload: serde_json::Value,
        ) -> crate::http::ingest::IngestFuture<anyhow::Result<()>> {
            Box::pin(async { Ok(()) })
        }
    }

    struct DummyStoragePort;

    impl StoragePort for DummyStoragePort {
        fn load_wav_as_pcmu_frames(&self, _path: &str) -> anyhow::Result<Vec<Vec<u8>>> {
            Ok(vec![vec![0xFF; 160]])
        }
    }

    fn build_test_session(storage_port: Arc<dyn StoragePort>) -> SessionCoordinator {
        let (session_out_tx, _session_out_rx) = tokio::sync::mpsc::unbounded_channel();
        let (app_tx, _app_rx) = tokio::sync::mpsc::unbounded_channel();
        let (tx_in, _rx_in) = tokio::sync::mpsc::unbounded_channel();
        SessionCoordinator {
            state_machine: SessionStateMachine::new(),
            call_id: "test-call".to_string(),
            from_uri: "sip:from@example.com".to_string(),
            to_uri: "sip:to@example.com".to_string(),
            ingest: crate::session::ingest_manager::IngestManager::new(
                None,
                Arc::new(DummyIngestPort),
            ),
            recording_base_url: None,
            storage_port,
            peer_sdp: Some(Sdp::pcmu("127.0.0.1", 10000)),
            local_sdp: None,
            session_out_tx,
            tx_in,
            app_tx,
            media_cfg: MediaConfig::pcmu("127.0.0.1", 10000),
            rtp: crate::session::rtp_stream_manager::RtpStreamManager::new(RtpTxHandle::new()),
            recording: crate::session::recording_manager::RecordingManager::new("test-call"),
            started_at: None,
            started_wall: None,
            rtp_last_sent: None,
            a_leg_rtp_started: false,
            timers: SessionTimers::new(Duration::from_secs(0)),
            sending_audio: false,
            playback: None,
            speaking: false,
            capture: AudioCapture::new(config::vad_config().clone()),
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
