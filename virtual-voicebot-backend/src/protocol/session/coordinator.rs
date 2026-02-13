#![allow(dead_code)]
// session.rs
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::time::{interval, Duration, Instant, MissedTickBehavior};

use chrono::{DateTime, Utc};
#[path = "handlers/mod.rs"]
mod handlers;
#[path = "services/mod.rs"]
mod services;

use crate::protocol::session::state_machine::{SessionEvent, SessionStateMachine};
use crate::protocol::session::types::Sdp;
use crate::protocol::session::types::*;

use crate::protocol::rtp::tx::RtpTxHandle;
use crate::protocol::session::b2bua;
use crate::protocol::session::capture::AudioCapture;
use crate::protocol::session::timers::SessionTimers;
use crate::protocol::sip::{parse_name_addr, parse_uri};
use crate::shared::config::SessionRuntimeConfig;
use crate::shared::ports::app::AppEventTx;
use crate::shared::ports::call_log_port::{CallLogPort, EndedCallLog, EndedRecording};
use crate::shared::ports::ingest::IngestPort;
use crate::shared::ports::routing_port::RoutingPort;
use crate::shared::ports::storage::StoragePort;
use anyhow::Error;
use uuid::Uuid;
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
pub(crate) const ANNOUNCEMENT_FALLBACK_WAV_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/data/zundamon_sorry.wav");

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
    call_log_port: Arc<dyn CallLogPort>,
    routing_port: Arc<dyn RoutingPort>,
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
    no_response_mode: bool,
    announce_mode: bool,
    voicebot_direct_mode: bool,
    voicemail_mode: bool,
    recording_notice_pending: bool,
    transfer_after_answer_pending: bool,
    announcement_id: Option<Uuid>,
    announcement_audio_file_url: Option<String>,
    ivr_flow_id: Option<Uuid>,
    ivr_menu_audio_file_url: Option<String>,
    ivr_keypad_node_id: Option<Uuid>,
    ivr_retry_count: u32,
    ivr_max_retries: u32,
    ivr_timeout_override: Option<Duration>,
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
        call_log_port: Arc<dyn CallLogPort>,
        routing_port: Arc<dyn RoutingPort>,
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
            ingest: crate::protocol::session::ingest_manager::IngestManager::new(
                ingest_url,
                ingest_port,
            ),
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
            call_log_port,
            routing_port,
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
            no_response_mode: false,
            announce_mode: false,
            voicebot_direct_mode: false,
            voicemail_mode: false,
            recording_notice_pending: false,
            transfer_after_answer_pending: false,
            announcement_id: None,
            announcement_audio_file_url: None,
            ivr_flow_id: None,
            ivr_menu_audio_file_url: None,
            ivr_keypad_node_id: None,
            ivr_retry_count: 0,
            ivr_max_retries: 0,
            ivr_timeout_override: None,
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

    pub(crate) fn set_outbound_mode(&mut self, enabled: bool) {
        self.outbound_mode = enabled;
    }

    pub(crate) fn set_recording_enabled(&mut self, enabled: bool) {
        self.recording.set_enabled(enabled);
    }

    pub(crate) fn set_invite_rejected(&mut self, rejected: bool) {
        self.invite_rejected = rejected;
    }

    pub(crate) fn set_no_response_mode(&mut self, enabled: bool) {
        self.no_response_mode = enabled;
    }

    pub(crate) fn set_announce_mode(&mut self, enabled: bool) {
        self.announce_mode = enabled;
    }

    pub(crate) fn set_voicebot_direct_mode(&mut self, enabled: bool) {
        self.voicebot_direct_mode = enabled;
    }

    pub(crate) fn set_voicemail_mode(&mut self, enabled: bool) {
        self.voicemail_mode = enabled;
    }

    pub(crate) fn set_recording_notice_pending(&mut self, pending: bool) {
        self.recording_notice_pending = pending;
    }

    pub(crate) fn set_transfer_after_answer_pending(&mut self, pending: bool) {
        self.transfer_after_answer_pending = pending;
    }

    pub(crate) fn set_announcement_id(&mut self, announcement_id: Uuid) {
        self.announcement_id = Some(announcement_id);
    }

    pub(crate) fn set_announcement_audio_file_url(&mut self, audio_file_url: String) {
        self.announcement_audio_file_url = Some(audio_file_url);
    }

    pub(crate) fn set_ivr_flow_id(&mut self, ivr_flow_id: Uuid) {
        self.ivr_flow_id = Some(ivr_flow_id);
    }

    pub(crate) fn set_ivr_state(&mut self, ivr_state: IvrState) {
        self.ivr_state = ivr_state;
    }

    pub(crate) async fn send_sip_error(&mut self, code: u16, reason: &str) -> Result<(), Error> {
        self.session_out_tx.try_send((
            self.call_id.clone(),
            SessionOut::SipSendError {
                code,
                reason: reason.to_string(),
            },
        ))?;
        Ok(())
    }

    pub(crate) fn reset_action_modes(&mut self) {
        self.no_response_mode = false;
        self.announce_mode = false;
        self.voicebot_direct_mode = false;
        self.voicemail_mode = false;
        self.recording_notice_pending = false;
        self.transfer_after_answer_pending = false;
        self.announcement_id = None;
        self.announcement_audio_file_url = None;
        self.ivr_flow_id = None;
        self.ivr_menu_audio_file_url = None;
        self.ivr_keypad_node_id = None;
        self.ivr_retry_count = 0;
        self.ivr_max_retries = 0;
        self.ivr_timeout_override = None;
    }

    pub(crate) async fn resolve_announcement_playback_path(&self) -> Option<String> {
        if let Some(audio_file_url) = self.announcement_audio_file_url.clone() {
            return Some(map_audio_file_url_to_local_path(audio_file_url));
        }

        let announcement_id = self.announcement_id?;
        match self
            .routing_port
            .find_announcement_audio_file_url(announcement_id)
            .await
        {
            Ok(Some(audio_file_url)) => Some(map_audio_file_url_to_local_path(audio_file_url)),
            Ok(None) => {
                log::warn!(
                    "[session {}] announcement not found id={}",
                    self.call_id,
                    announcement_id
                );
                None
            }
            Err(err) => {
                log::warn!(
                    "[session {}] failed to read announcement id={} error={}",
                    self.call_id,
                    announcement_id,
                    err
                );
                None
            }
        }
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
        let (call_status, end_reason) = match status {
            "ended" => ("ended", "normal"),
            "failed" | "error" => ("error", "error"),
            _ => {
                log::warn!(
                    "[session {}] unsupported ingest status={}, skip call log persist",
                    self.call_id,
                    status
                );
                return;
            }
        };

        let started_wall = self.started_wall.unwrap_or_else(std::time::SystemTime::now);
        let started_at: DateTime<Utc> = started_wall.into();
        let ended_at = Utc::now();
        let duration_sec = self
            .started_at
            .map(|s| s.elapsed().as_secs().min(i32::MAX as u64) as i32);
        let call_log_id = Uuid::now_v7();
        let caller_number = extract_e164_caller_number(self.from_uri.as_str());

        let recording_path = self.recording.mixed_file_path();
        let recording = match tokio::fs::metadata(&recording_path).await {
            Ok(meta) if meta.is_file() => Some(EndedRecording {
                id: Uuid::now_v7(),
                file_path: recording_path.to_string_lossy().to_string(),
                duration_sec,
                format: "wav".to_string(),
                file_size_bytes: i64::try_from(meta.len()).ok(),
                started_at,
                ended_at: Some(ended_at),
            }),
            Ok(_) => None,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
            Err(err) => {
                log::warn!(
                    "[session {}] failed to inspect recording file {}: {}",
                    self.call_id,
                    recording_path.display(),
                    err
                );
                None
            }
        };

        let ended_call = EndedCallLog {
            id: call_log_id,
            started_at,
            ended_at,
            duration_sec,
            external_call_id: call_log_id.to_string(),
            sip_call_id: self.call_id.to_string(),
            caller_number,
            caller_category: "unknown".to_string(),
            action_code: "IV".to_string(),
            ivr_flow_id: self.ivr_flow_id,
            answered_at: None,
            end_reason: end_reason.to_string(),
            status: call_status.to_string(),
            recording,
        };

        if let Err(err) = self.call_log_port.persist_call_ended(ended_call).await {
            log::warn!(
                "[session {}] failed to persist call log/outbox: {}",
                self.call_id,
                err
            );
        }
    }
}

fn map_audio_file_url_to_local_path(audio_file_url: String) -> String {
    let trimmed = audio_file_url.trim();
    let url_path = if let Some(scheme_sep) = trimmed.find("://") {
        let after_scheme = &trimmed[scheme_sep + 3..];
        if let Some(path_pos) = after_scheme.find('/') {
            &after_scheme[path_pos..]
        } else {
            "/"
        }
    } else {
        trimmed
    };

    if let Some(rest) = url_path.strip_prefix("/audio/announcements/") {
        let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        let path = repo_root
            .join("virtual-voicebot-frontend")
            .join("public")
            .join("audio")
            .join("announcements")
            .join(rest);
        return path.to_string_lossy().to_string();
    }

    if let Some(path) = url_path.strip_prefix("file://") {
        return path.to_string();
    }

    url_path.to_string()
}

fn extract_e164_caller_number(value: &str) -> Option<String> {
    let candidate = extract_user_from_to(value)?;
    if is_e164(candidate.as_str()) {
        Some(candidate)
    } else {
        None
    }
}

fn extract_user_from_to(value: &str) -> Option<String> {
    if let Ok(name_addr) = parse_name_addr(value) {
        if name_addr.uri.scheme.eq_ignore_ascii_case("tel") {
            if !name_addr.uri.host.trim().is_empty() {
                return Some(name_addr.uri.host);
            }
        }
        if let Some(user) = name_addr.uri.user {
            return Some(user);
        }
    }

    let trimmed = value.trim();
    let addr = if let Some(start) = trimmed.find('<') {
        if let Some(end) = trimmed[start + 1..].find('>') {
            &trimmed[start + 1..start + 1 + end]
        } else {
            trimmed
        }
    } else {
        trimmed
    };
    let addr = addr.split(';').next().unwrap_or(addr).trim();
    let uri = parse_uri(addr).ok()?;
    if uri.scheme.eq_ignore_ascii_case("tel") {
        if !uri.host.trim().is_empty() {
            return Some(uri.host);
        }
    }
    uri.user
}

fn is_e164(value: &str) -> bool {
    if !value.starts_with('+') {
        return false;
    }
    let digits = &value[1..];
    if digits.len() < 2 || digits.len() > 15 {
        return false;
    }
    let mut chars = digits.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !('1'..='9').contains(&first) {
        return false;
    }
    chars.all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::session::types::SessionControlIn;
    use crate::shared::ports::ingest::IngestPayload;
    use crate::shared::ports::routing_port::NoopRoutingPort;

    struct DummyIngestPort;

    impl IngestPort for DummyIngestPort {
        fn post(
            &self,
            _url: String,
            _payload: IngestPayload,
        ) -> crate::shared::ports::ingest::IngestFuture<
            Result<(), crate::shared::ports::ingest::IngestError>,
        > {
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

    struct DummyCallLogPort;

    impl CallLogPort for DummyCallLogPort {
        fn persist_call_ended(
            &self,
            _call_log: EndedCallLog,
        ) -> crate::shared::ports::call_log_port::CallLogFuture<()> {
            Box::pin(async { Ok(()) })
        }
    }

    fn build_test_session_with_control(
        storage_port: Arc<dyn StoragePort>,
    ) -> (
        SessionCoordinator,
        tokio::sync::mpsc::Receiver<SessionControlIn>,
    ) {
        let (session_out_tx, _session_out_rx) = mpsc::channel(32);
        let (app_tx, _app_rx) = crate::shared::ports::app::app_event_channel(16);
        let (control_tx, control_rx) = mpsc::channel(SESSION_CONTROL_CHANNEL_CAPACITY);
        let (media_tx, _media_rx) = mpsc::channel(SESSION_MEDIA_CHANNEL_CAPACITY);
        let base_cfg = crate::shared::config::Config::from_env().expect("config loads");
        let runtime_cfg = Arc::new(SessionRuntimeConfig::from_env(&base_cfg));
        let session = SessionCoordinator {
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
            call_log_port: Arc::new(DummyCallLogPort),
            routing_port: Arc::new(NoopRoutingPort::new()),
            rtp: crate::protocol::session::rtp_stream_manager::RtpStreamManager::new(
                RtpTxHandle::new(crate::shared::config::rtp_config().clone()),
            ),
            recording: crate::protocol::session::recording_manager::RecordingManager::new(
                "test-call",
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
            no_response_mode: false,
            announce_mode: false,
            voicebot_direct_mode: false,
            voicemail_mode: false,
            recording_notice_pending: false,
            transfer_after_answer_pending: false,
            announcement_id: None,
            announcement_audio_file_url: None,
            ivr_flow_id: None,
            ivr_menu_audio_file_url: None,
            ivr_keypad_node_id: None,
            ivr_retry_count: 0,
            ivr_max_retries: 0,
            ivr_timeout_override: None,
            session_expires: None,
            session_refresher: None,
        };
        (session, control_rx)
    }

    fn build_test_session(storage_port: Arc<dyn StoragePort>) -> SessionCoordinator {
        let (session, _control_rx) = build_test_session_with_control(storage_port);
        session
    }

    #[tokio::test]
    async fn cancel_playback_clears_state() {
        let mut session = build_test_session(Arc::new(DummyStoragePort));
        session.start_playback(&["dummy.wav"]).await.unwrap();
        assert!(session.playback.is_some());
        assert!(session.sending_audio);
        session.cancel_playback();
        assert!(session.playback.is_none());
        assert!(!session.sending_audio);
    }

    #[tokio::test]
    async fn finish_playback_requests_transfer_after_recording_notice() {
        let (mut session, mut control_rx) =
            build_test_session_with_control(Arc::new(DummyStoragePort));
        session.set_announce_mode(true);
        session.set_recording_notice_pending(true);

        session.finish_playback(false);

        assert!(!session.announce_mode);
        assert!(!session.recording_notice_pending);
        let control = tokio::time::timeout(Duration::from_millis(50), control_rx.recv())
            .await
            .expect("control message should be sent")
            .expect("control channel should stay open");
        match control {
            SessionControlIn::AppTransferRequest { person } => {
                assert_eq!(person, "recording_notice");
            }
            other => panic!("unexpected control message: {:?}", other),
        }
    }

    #[tokio::test]
    async fn reset_action_modes_clears_voicebot_direct_mode() {
        let mut session = build_test_session(Arc::new(DummyStoragePort));
        session.set_voicebot_direct_mode(true);
        session.set_transfer_after_answer_pending(true);
        session.reset_action_modes();
        assert!(!session.voicebot_direct_mode);
        assert!(!session.transfer_after_answer_pending);
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

    #[test]
    fn audio_announcement_url_is_mapped_to_frontend_public_file() {
        let mapped = super::map_audio_file_url_to_local_path(
            "http://localhost:3000/audio/announcements/abc.wav".to_string(),
        );
        assert!(mapped.ends_with("virtual-voicebot-frontend/public/audio/announcements/abc.wav"));
    }

    #[test]
    fn relative_audio_announcement_url_is_mapped_to_frontend_public_file() {
        let mapped = super::map_audio_file_url_to_local_path("/audio/announcements/xyz.wav".into());
        assert!(mapped.ends_with("virtual-voicebot-frontend/public/audio/announcements/xyz.wav"));
    }
}
