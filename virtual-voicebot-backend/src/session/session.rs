#![allow(dead_code)]
// session.rs
use std::net::SocketAddr;
use std::sync::Arc;

use chrono::{Local, Timelike};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;
use tokio::time::{interval, Duration, Instant, MissedTickBehavior};

use crate::session::types::Sdp;
use crate::session::types::*;
use crate::session::b2bua;
use crate::sip::{parse_name_addr, parse_uri};

use crate::app::AppEvent;
use crate::http::ingest::IngestPort;
use crate::media::Recorder;
use crate::recording;
use crate::recording::storage::StoragePort;
use crate::rtp::codec::mulaw_to_linear16;
use crate::rtp::tx::RtpTxHandle;
use crate::session::capture::AudioCapture;
use crate::config;
use crate::session::timers::SessionTimers;
use anyhow::Error;
use log::{debug, info, warn};
use serde_json::json;

const KEEPALIVE_INTERVAL: Duration = Duration::from_millis(20);
const PLAYBACK_FRAME_INTERVAL: Duration = Duration::from_millis(20);
const TRANSFER_ANNOUNCE_INTERVAL: Duration = Duration::from_secs(5);

const INTRO_MORNING_WAV_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/data/zundamon_intro_morning.wav");
const INTRO_AFTERNOON_WAV_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/data/zundamon_intro_afternoon.wav");
const INTRO_EVENING_WAV_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/data/zundamon_intro_evening.wav");
const IVR_INTRO_WAV_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/data/zundamon_intro_ivr.wav");
const VOICEBOT_INTRO_WAV_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/data/zundamon_intro_ivr_1.wav");
const IVR_INTRO_AGAIN_WAV_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/data/zundamon_intro_ivr_again.wav");
const IVR_SENDAI_WAV_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/data/zundamon_sendai.wav");
const IVR_INVALID_WAV_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/data/zundamon_invalid.wav");
const TRANSFER_WAV_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/data/zundamon_try_transfer.wav");
const TRANSFER_FAIL_WAV_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/data/zundamon_transfer_fail.wav");

fn intro_wav_path_for_hour(hour: u32) -> &'static str {
    match hour {
        5..=11 => INTRO_MORNING_WAV_PATH,
        12..=16 => INTRO_AFTERNOON_WAV_PATH,
        _ => INTRO_EVENING_WAV_PATH,
    }
}

fn get_intro_wav_path() -> &'static str {
    let hour = Local::now().hour();
    intro_wav_path_for_hour(hour)
}

fn extract_user_from_to(value: &str) -> Option<String> {
    if let Ok(name_addr) = parse_name_addr(value) {
        return name_addr.uri.user;
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
    parse_uri(addr).ok().and_then(|uri| uri.user)
}

#[derive(Clone)]
pub struct SessionHandle {
    pub tx_in: UnboundedSender<SessionIn>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IvrAction {
    EnterVoicebot,
    PlaySendai,
    Transfer,
    ReplayMenu,
    Invalid,
}

fn ivr_action_for_digit(digit: char) -> IvrAction {
    match digit {
        '1' => IvrAction::EnterVoicebot,
        '2' => IvrAction::PlaySendai,
        '3' => IvrAction::Transfer,
        '9' => IvrAction::ReplayMenu,
        _ => IvrAction::Invalid,
    }
}

fn ivr_state_after_action(state: IvrState, action: IvrAction) -> IvrState {
    match (state, action) {
        (IvrState::IvrMenuWaiting, IvrAction::EnterVoicebot) => IvrState::VoicebotIntroPlaying,
        _ => state,
    }
}

#[derive(Debug)]
struct PlaybackState {
    frames: Vec<Vec<u8>>,
    index: usize,
}

pub struct Session {
    state: SessState,
    call_id: CallId,
    from_uri: String,
    to_uri: String,
    ingest_url: Option<String>,
    recording_base_url: Option<String>,
    ingest_sent: bool,
    ingest_port: Arc<dyn IngestPort>,
    storage_port: Arc<dyn StoragePort>,
    peer_sdp: Option<Sdp>,
    local_sdp: Option<Sdp>,
    session_out_tx: UnboundedSender<(CallId, SessionOut)>,
    tx_in: UnboundedSender<SessionIn>,
    app_tx: UnboundedSender<AppEvent>,
    media_cfg: MediaConfig,
    rtp_tx: RtpTxHandle,
    recorder: Recorder,
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
    outbound_mode: bool,
    outbound_answered: bool,
    outbound_sent_180: bool,
    outbound_sent_183: bool,
    invite_rejected: bool,
    session_expires: Option<Duration>,
    session_refresher: Option<SessionRefresher>,
}

impl Session {
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
            state: SessState::Idle,
            call_id,
            from_uri,
            to_uri,
            ingest_url,
            recording_base_url,
            ingest_sent: false,
            ingest_port,
            storage_port,
            peer_sdp: None,
            local_sdp: None,
            session_out_tx,
            tx_in: tx_in.clone(),
            app_tx,
            media_cfg,
            rtp_tx,
            recorder: Recorder::new(call_id_clone),
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
                    let next_state = next_session_state(self.state, &ev);
                    let mut advance_state = true;
                    match (self.state, ev) {
                        (SessState::Idle, SessionIn::SipInvite { offer, session_timer, .. }) => {
                            self.peer_sdp = Some(offer);
                            if let Some(timer) = session_timer {
                                self.update_session_expires(timer);
                            }
                            let answer = self.build_answer_pcmu8k();
                            self.local_sdp = Some(answer.clone());
                            self.outbound_mode = false;
                            self.outbound_answered = false;
                            self.outbound_sent_180 = false;
                            self.outbound_sent_183 = false;
                            self.invite_rejected = false;
                            if config::outbound_config().enabled {
                                let outbound_cfg = config::outbound_config();
                                let registrar = config::registrar_config();
                                let user =
                                    extract_user_from_to(self.to_uri.as_str()).unwrap_or_default();
                                let skip_outbound = registrar
                                    .map(|cfg| cfg.user == user)
                                    .unwrap_or(false);
                                if !skip_outbound {
                                    let target = outbound_cfg.resolve_number(user.as_str());
                                    if outbound_cfg.domain.is_empty()
                                        || registrar.is_none()
                                        || target.is_none()
                                    {
                                        warn!(
                                            "[session {}] outbound disabled (missing config)",
                                            self.call_id
                                        );
                                        let _ = self.session_out_tx.send((
                                            self.call_id.clone(),
                                            SessionOut::SipSendError {
                                                code: 503,
                                                reason: "Service Unavailable".to_string(),
                                            },
                                        ));
                                        self.invite_rejected = true;
                                        advance_state = false;
                                    } else {
                                        self.outbound_mode = true;
                                        self.ivr_state = IvrState::Transferring;
                                        if let Some(number) = target {
                                            self.transfer_cancel = Some(b2bua::spawn_outbound(
                                                self.call_id.clone(),
                                                number,
                                                self.tx_in.clone(),
                                            ));
                                        }
                                    }
                                }
                            }
                            if advance_state {
                                let _ = self
                                    .session_out_tx
                                    .send((self.call_id.clone(), SessionOut::SipSend100));
                                if !self.outbound_mode {
                                    let _ = self
                                        .session_out_tx
                                        .send((self.call_id.clone(), SessionOut::SipSend180));
                                    let _ = self.session_out_tx.send((
                                        self.call_id.clone(),
                                        SessionOut::SipSend200 { answer },
                                    ));
                                }
                            }
                        }
                        (SessState::Early, SessionIn::SipAck) => {
                            // 相手SDPからRTP宛先を確定して送信開始
                            if self.intro_sent {
                                advance_state = false;
                            }
                            if self.invite_rejected {
                                advance_state = false;
                            }
                            if !advance_state {
                                continue;
                            }
                            self.started_at = Some(Instant::now());
                            self.started_wall = Some(std::time::SystemTime::now());
                            if let Err(e) = self.recorder.start() {
                                warn!(
                                    "[session {}] failed to start recorder: {:?}",
                                    self.call_id, e
                                );
                            }
                            if !self.ensure_a_leg_rtp_started() {
                                advance_state = false;
                                continue;
                            }
                            self.capture.reset();
                            self.intro_sent = true;

                            self.align_rtp_clock();

                            let caller = extract_user_from_to(self.from_uri.as_str());
                            let _ = self.app_tx.send(AppEvent::CallStarted {
                                call_id: self.call_id.clone(),
                                caller,
                            });

                            if !self.outbound_mode {
                                self.ivr_state = IvrState::IvrMenuWaiting;
                                if let Err(e) = self.start_playback(&[IVR_INTRO_WAV_PATH]) {
                                    warn!(
                                        "[session {}] failed to send IVR intro wav: {:?}",
                                        self.call_id, e
                                    );
                                    self.reset_ivr_timeout();
                                } else {
                                    info!(
                                        "[session {}] sent IVR intro wav {}",
                                        self.call_id, IVR_INTRO_WAV_PATH
                                    );
                                }
                            }

                            self.start_keepalive_timer();
                            self.start_session_timer_if_needed();
                        }
                        (SessState::Established, SessionIn::MediaRtpIn { payload, .. }) => {
                            debug!(
                                "[session {}] RTP payload received len={}",
                                self.call_id,
                                payload.len()
                            );
                            let payload_len = payload.len();
                            self.recorder.push_rx_mulaw(&payload);
                            if self.ivr_state == IvrState::B2buaMode {
                                if let Some(b_leg) = &self.b_leg {
                                    self.rtp_tx
                                        .send_payload(&b_leg.rtp_key, payload.clone());
                                }
                            } else if self.ivr_state == IvrState::VoicebotMode {
                                if let Some(buffer) = self.capture.ingest(&payload) {
                                    info!(
                                        "[session {}] buffered audio ready for app ({} bytes)",
                                        self.call_id,
                                        buffer.len()
                                    );
                                    let pcm_linear16: Vec<i16> = buffer
                                        .iter()
                                        .map(|&b| mulaw_to_linear16(b))
                                        .collect();
                                    let _ = self.app_tx.send(AppEvent::AudioBuffered {
                                        call_id: self.call_id.clone(),
                                        pcm_mulaw: buffer,
                                        pcm_linear16,
                                    });
                                    self.capture.start();
                                }
                            }
                            let _ = self.session_out_tx.send((
                                self.call_id.clone(),
                                SessionOut::Metrics {
                                    name: "rtp_in",
                                    value: payload_len as i64,
                                },
                            ));
                        }
                        (SessState::Established, SessionIn::Dtmf { digit }) => {
                            info!("[session {}] DTMF received: '{}'", self.call_id, digit);
                            if self.ivr_state == IvrState::VoicebotIntroPlaying {
                                info!(
                                    "[session {}] ignoring DTMF during voicebot intro",
                                    self.call_id
                                );
                                continue;
                            }
                            if self.ivr_state != IvrState::IvrMenuWaiting {
                                debug!(
                                    "[session {}] ignoring DTMF in {:?}",
                                    self.call_id, self.ivr_state
                                );
                                continue;
                            }
                            self.cancel_playback();
                            self.stop_ivr_timeout();
                            let action = ivr_action_for_digit(digit);
                            match action {
                                IvrAction::EnterVoicebot => {
                                    info!(
                                        "[session {}] starting voicebot intro",
                                        self.call_id
                                    );
                                    if let Err(e) = self.start_playback(&[VOICEBOT_INTRO_WAV_PATH])
                                    {
                                        warn!(
                                            "[session {}] voicebot intro failed: {:?}",
                                            self.call_id, e
                                        );
                                        self.ivr_state = IvrState::VoicebotMode;
                                        self.capture.reset();
                                        self.capture.start();
                                    } else {
                                        self.ivr_state =
                                            ivr_state_after_action(self.ivr_state, action);
                                    }
                                }
                                IvrAction::PlaySendai => {
                                    info!("[session {}] playing sendai info", self.call_id);
                                    if let Err(e) = self.start_playback(&[
                                        IVR_SENDAI_WAV_PATH,
                                        IVR_INTRO_AGAIN_WAV_PATH,
                                    ]) {
                                        warn!(
                                            "[session {}] failed to play sendai flow: {:?}",
                                            self.call_id, e
                                        );
                                        self.reset_ivr_timeout();
                                    }
                                }
                                IvrAction::Transfer => {
                                    if self.transfer_cancel.is_some() || self.b_leg.is_some() {
                                        warn!(
                                            "[session {}] transfer already active",
                                            self.call_id
                                        );
                                        self.reset_ivr_timeout();
                                        continue;
                                    }
                                    info!(
                                        "[session {}] initiating transfer to B-leg",
                                        self.call_id
                                    );
                                    self.ivr_state = IvrState::Transferring;
                                    if let Err(e) = self.start_playback(&[TRANSFER_WAV_PATH]) {
                                        warn!(
                                            "[session {}] failed to play transfer wav: {:?}",
                                            self.call_id, e
                                        );
                                    }
                                    self.start_transfer_announce();
                                    self.transfer_cancel = Some(b2bua::spawn_transfer(
                                        self.call_id.clone(),
                                        self.tx_in.clone(),
                                    ));
                                }
                                IvrAction::ReplayMenu => {
                                    info!("[session {}] replaying IVR menu", self.call_id);
                                    if let Err(e) =
                                        self.start_playback(&[IVR_INTRO_AGAIN_WAV_PATH])
                                    {
                                        warn!(
                                            "[session {}] failed to replay IVR menu: {:?}",
                                            self.call_id, e
                                        );
                                        self.reset_ivr_timeout();
                                    }
                                }
                                IvrAction::Invalid => {
                                    info!("[session {}] invalid DTMF: '{}'", self.call_id, digit);
                                    if let Err(e) = self.start_playback(&[
                                        IVR_INVALID_WAV_PATH,
                                        IVR_INTRO_AGAIN_WAV_PATH,
                                    ]) {
                                        warn!(
                                            "[session {}] failed to play invalid flow: {:?}",
                                            self.call_id, e
                                        );
                                        self.reset_ivr_timeout();
                                    }
                                }
                            }
                        }
                        (_, SessionIn::B2buaEstablished { b_leg }) => {
                            info!(
                                "[session {}] B-leg established, entering B2BUA mode",
                                self.call_id
                            );
                            self.transfer_cancel = None;
                            self.stop_transfer_announce();
                            self.cancel_playback();
                            self.stop_ivr_timeout();
                            self.ivr_state = IvrState::B2buaMode;
                            self.b_leg = Some(b_leg);
                            if let Some(b_leg) = &self.b_leg {
                                self.rtp_tx.start(
                                    b_leg.rtp_key.clone(),
                                    b_leg.remote_rtp_addr,
                                    0,
                                    0x22334455,
                                    0,
                                    0,
                                );
                            }
                            let _ = self.ensure_a_leg_rtp_started();
                            if self.outbound_mode && !self.outbound_answered {
                                if let Some(answer) = self.local_sdp.clone() {
                                    let _ = self.session_out_tx.send((
                                        self.call_id.clone(),
                                        SessionOut::SipSend200 { answer },
                                    ));
                                    self.outbound_answered = true;
                                }
                            }
                        }
                        (_, SessionIn::B2buaFailed { reason, status }) => {
                            warn!(
                                "[session {}] transfer failed: {}",
                                self.call_id, reason
                            );
                            self.transfer_cancel = None;
                            self.stop_transfer_announce();
                            if self.outbound_mode {
                                let code = status.unwrap_or(503);
                                let _ = self.session_out_tx.send((
                                    self.call_id.clone(),
                                    SessionOut::SipSendError {
                                        code,
                                        reason: "Service Unavailable".to_string(),
                                    },
                                ));
                                self.outbound_mode = false;
                                self.invite_rejected = true;
                            } else {
                                self.ivr_state = IvrState::IvrMenuWaiting;
                                self.b_leg = None;
                                if let Err(e) = self.start_playback(&[
                                    TRANSFER_FAIL_WAV_PATH,
                                    IVR_INTRO_AGAIN_WAV_PATH,
                                ]) {
                                    warn!(
                                        "[session {}] failed to play transfer fail flow: {:?}",
                                        self.call_id, e
                                    );
                                    self.reset_ivr_timeout();
                                }
                            }
                        }
                        (_, SessionIn::BLegRtp { payload }) => {
                            if self.ivr_state == IvrState::B2buaMode {
                                self.align_rtp_clock();
                                self.recorder.push_tx_mulaw(&payload);
                                self.rtp_tx
                                    .send_payload(&self.call_id, payload);
                                self.rtp_last_sent = Some(Instant::now());
                            }
                        }
                        (_, SessionIn::BLegBye) => {
                            info!(
                                "[session {}] B-leg BYE received, ending call",
                                self.call_id
                            );
                            self.cancel_transfer();
                            self.shutdown_b_leg(false).await;
                            self.cancel_playback();
                            self.stop_keepalive_timer();
                            self.stop_session_timer();
                            self.stop_ivr_timeout();
                            self.send_bye_to_a_leg();
                            if let Err(e) = self.recorder.stop() {
                                warn!(
                                    "[session {}] failed to finalize recording: {:?}",
                                    self.call_id, e
                                );
                            }
                            self.send_ingest("ended").await;
                            self.rtp_tx.stop(&self.call_id);
                            let _ = self
                                .session_out_tx
                                .send((self.call_id.clone(), SessionOut::RtpStopTx));
                            let _ = self.app_tx.send(AppEvent::CallEnded {
                                call_id: self.call_id.clone(),
                            });
                        }
                        (_, SessionIn::B2buaRinging) => {
                            if self.outbound_mode
                                && !self.outbound_sent_180
                                && !self.outbound_sent_183
                            {
                                let _ = self.session_out_tx.send((
                                    self.call_id.clone(),
                                    SessionOut::SipSend180,
                                ));
                                self.outbound_sent_180 = true;
                            }
                        }
                        (_, SessionIn::B2buaEarlyMedia) => {
                            if !self.outbound_mode || self.invite_rejected {
                                continue;
                            }
                            self.ivr_state = IvrState::B2buaMode;
                            if !self.ensure_a_leg_rtp_started() {
                                continue;
                            }
                            if !self.outbound_sent_183 {
                                if let Some(answer) = self.local_sdp.clone() {
                                    let _ = self.session_out_tx.send((
                                        self.call_id.clone(),
                                        SessionOut::SipSend183 { answer },
                                    ));
                                    self.outbound_sent_183 = true;
                                }
                            }
                        }
                        (_, SessionIn::TransferAnnounce) => {
                            if self.ivr_state == IvrState::Transferring && self.playback.is_none()
                            {
                                if let Err(e) = self.start_playback(&[TRANSFER_WAV_PATH]) {
                                    warn!(
                                        "[session {}] failed to replay transfer wav: {:?}",
                                        self.call_id, e
                                    );
                                }
                            }
                        }
                        (SessState::Established, SessionIn::SipReInvite { session_timer, .. }) => {
                            if let Some(timer) = session_timer {
                                self.update_session_expires(timer);
                            }
                            let answer = match self.local_sdp.clone() {
                                Some(answer) => answer,
                                None => {
                                    let answer = self.build_answer_pcmu8k();
                                    self.local_sdp = Some(answer.clone());
                                    answer
                                }
                            };
                            let _ = self
                                .session_out_tx
                                .send((self.call_id.clone(), SessionOut::SipSend200 { answer }));
                        }
                        (SessState::Established, SessionIn::MediaTimerTick) => {
                            if let Err(e) = self.send_silence_frame().await {
                                warn!("[session {}] silence send failed: {:?}", self.call_id, e);
                            }
                        }
                        (_, SessionIn::SipCancel) => {
                            info!("[session {}] CANCEL received, terminating early", self.call_id);
                            self.invite_rejected = true;
                            self.cancel_transfer();
                            self.shutdown_b_leg(true).await;
                            self.cancel_playback();
                            self.stop_keepalive_timer();
                            self.stop_session_timer();
                            self.stop_ivr_timeout();
                            self.rtp_tx.stop(&self.call_id);
                            let _ = self
                                .session_out_tx
                                .send((self.call_id.clone(), SessionOut::RtpStopTx));
                            let _ = self.session_out_tx.send((
                                self.call_id.clone(),
                                SessionOut::SipSendError {
                                    code: 487,
                                    reason: "Request Terminated".to_string(),
                                },
                            ));
                            if let Err(e) = self.recorder.stop() {
                                warn!(
                                    "[session {}] failed to finalize recording: {:?}",
                                    self.call_id, e
                                );
                            }
                            let _ = self.app_tx.send(AppEvent::CallEnded {
                                call_id: self.call_id.clone(),
                            });
                        }
                        (_, SessionIn::SipBye) => {
                            self.cancel_transfer();
                            self.shutdown_b_leg(true).await;
                            self.cancel_playback();
                            self.stop_keepalive_timer();
                            self.stop_session_timer();
                            self.stop_ivr_timeout();
                            self.rtp_tx.stop(&self.call_id);
                            let _ = self
                                .session_out_tx
                                .send((self.call_id.clone(), SessionOut::RtpStopTx));
                            let _ = self
                                .session_out_tx
                                .send((self.call_id.clone(), SessionOut::SipSendBye200));
                            if let Err(e) = self.recorder.stop() {
                                warn!(
                                    "[session {}] failed to finalize recording: {:?}",
                                    self.call_id, e
                                );
                            }
                            self.send_ingest("ended").await;
                            let _ = self.app_tx.send(AppEvent::CallEnded {
                                call_id: self.call_id.clone(),
                            });
                        }
                        (_, SessionIn::SipTransactionTimeout { call_id: _ }) => {
                            warn!("[session {}] transaction timeout notified", self.call_id);
                        }
                        (SessState::Established, SessionIn::AppBotAudioFile { path }) => {
                            if let Err(e) = self.start_playback(&[path.as_str()]) {
                                warn!(
                                    "[session {}] failed to send app audio: {:?}",
                                    self.call_id, e
                                );
                            }
                        }
                        (_, SessionIn::AppHangup) => {
                            warn!("[session {}] app requested hangup", self.call_id);
                            self.cancel_transfer();
                            self.shutdown_b_leg(true).await;
                            self.cancel_playback();
                            self.stop_keepalive_timer();
                            self.stop_session_timer();
                            self.stop_ivr_timeout();
                            if let Err(e) = self.recorder.stop() {
                                warn!(
                                    "[session {}] failed to finalize recording: {:?}",
                                    self.call_id, e
                                );
                            }
                            self.send_ingest("ended").await;
                            self.rtp_tx.stop(&self.call_id);
                            let _ = self
                                .session_out_tx
                                .send((self.call_id.clone(), SessionOut::RtpStopTx));
                            let _ = self
                                .session_out_tx
                                .send((self.call_id.clone(), SessionOut::SipSendBye200));
                            let _ = self.app_tx.send(AppEvent::CallEnded {
                                call_id: self.call_id.clone(),
                            });
                        }
                        (_, SessionIn::SipSessionExpires { timer }) => {
                            self.update_session_expires(timer);
                        }
                        (_, SessionIn::IvrTimeout) => {
                            if self.ivr_state == IvrState::IvrMenuWaiting {
                                info!(
                                    "[session {}] IVR timeout, replaying menu",
                                    self.call_id
                                );
                                self.stop_ivr_timeout();
                                if let Err(e) = self.start_playback(&[IVR_INTRO_AGAIN_WAV_PATH]) {
                                    warn!(
                                        "[session {}] failed to replay IVR menu: {:?}",
                                        self.call_id, e
                                    );
                                    self.reset_ivr_timeout();
                                }
                            }
                        }
                        (_, SessionIn::SessionRefreshDue) => {
                            if let (Some(expires), Some(SessionRefresher::Uas)) =
                                (self.session_expires, self.session_refresher)
                            {
                                let _ = self.session_out_tx.send((
                                    self.call_id.clone(),
                                    SessionOut::SipSendUpdate { expires },
                                ));
                                self.update_session_expires(SessionTimerInfo {
                                    expires,
                                    refresher: SessionRefresher::Uas,
                                });
                            }
                        }
                        (_, SessionIn::SessionTimerFired) => {
                            warn!("[session {}] session timer fired", self.call_id);
                            self.cancel_transfer();
                            self.shutdown_b_leg(true).await;
                            self.cancel_playback();
                            self.stop_keepalive_timer();
                            self.stop_session_timer();
                            self.stop_ivr_timeout();
                            if let Err(e) = self.recorder.stop() {
                                warn!(
                                    "[session {}] failed to finalize recording: {:?}",
                                    self.call_id, e
                                );
                            }
                            self.send_ingest("ended").await;
                            let _ = self
                                .session_out_tx
                                .send((self.call_id.clone(), SessionOut::RtpStopTx));
                            let _ = self
                                .session_out_tx
                                .send((self.call_id.clone(), SessionOut::AppSessionTimeout));
                            let _ = self.app_tx.send(AppEvent::CallEnded {
                                call_id: self.call_id.clone(),
                            });
                        }
                        (_, SessionIn::Abort(e)) => {
                            warn!("call {} abort: {e:?}", self.call_id);
                            self.cancel_transfer();
                            self.shutdown_b_leg(true).await;
                            self.cancel_playback();
                            self.stop_keepalive_timer();
                            self.stop_session_timer();
                            self.stop_ivr_timeout();
                            if let Err(e) = self.recorder.stop() {
                                warn!(
                                    "[session {}] failed to finalize recording: {:?}",
                                    self.call_id, e
                                );
                            }
                            self.send_ingest("failed").await;
                            self.rtp_tx.stop(&self.call_id);
                            let _ = self
                                .session_out_tx
                                .send((self.call_id.clone(), SessionOut::RtpStopTx));
                            let _ = self.app_tx.send(AppEvent::CallEnded {
                                call_id: self.call_id.clone(),
                            });
                        }
                        _ => { /* それ以外は無視 or ログ */ }
                    }
                    if advance_state {
                        self.state = next_state;
                    }
                }
            }
        }
        if let Err(e) = self.recorder.stop() {
            warn!(
                "[session {}] failed to finalize recording on shutdown: {:?}",
                self.call_id, e
            );
        }
    }

    fn build_answer_pcmu8k(&self) -> Sdp {
        // PCMU/8000 でローカル SDP を組み立て
        Sdp::pcmu(self.media_cfg.local_ip.clone(), self.media_cfg.local_port)
    }

    fn peer_rtp_dst(&self) -> (String, u16) {
        if let Some(sdp) = &self.peer_sdp {
            (sdp.ip.clone(), sdp.port)
        } else {
            ("0.0.0.0".to_string(), 0)
        }
    }

    fn ensure_a_leg_rtp_started(&mut self) -> bool {
        if self.a_leg_rtp_started {
            return true;
        }
        let (ip, port) = self.peer_rtp_dst();
        let dst_addr: SocketAddr = match format!("{ip}:{port}").parse() {
            Ok(addr) => addr,
            Err(e) => {
                warn!(
                    "[session {}] invalid RTP destination {}:{} ({:?})",
                    self.call_id, ip, port, e
                );
                return false;
            }
        };
        self.rtp_tx
            .start(self.call_id.clone(), dst_addr, 0, 0x12345678, 0, 0);
        let _ = self.session_out_tx.send((
            self.call_id.clone(),
            SessionOut::RtpStartTx {
                dst_ip: ip,
                dst_port: port,
                pt: 0,
            },
        ));
        self.a_leg_rtp_started = true;
        true
    }

    fn align_rtp_clock(&mut self) {
        if let Some(last) = self.rtp_last_sent {
            let gap_samples = (last.elapsed().as_secs_f64() * 8000.0) as u32;
            self.rtp_tx.adjust_timestamp(&self.call_id, gap_samples);
        }
    }

    /// keepalive タイマを開始する（挙動は従来と同じ。20ms ごとに TimerTick を送る）
    fn start_keepalive_timer(&mut self) {
        self.timers
            .start_keepalive(self.tx_in.clone(), KEEPALIVE_INTERVAL);
    }

    /// keepalive タイマを停止する（存在する場合のみ）
    fn stop_keepalive_timer(&mut self) {
        self.timers.stop_keepalive();
    }

    /// Session Timer（RFC 4028 準拠）。設定がある場合のみ開始する。
    fn start_session_timer_if_needed(&mut self) {
        let Some(expires) = self.session_expires else {
            return;
        };
        if expires.is_zero() {
            return;
        }
        let refresh_after = self.refresh_after(expires);
        self.timers
            .start_session_timer(self.tx_in.clone(), expires, refresh_after);
    }

    fn stop_session_timer(&mut self) {
        self.timers.stop_session_timer();
    }

    fn update_session_expires(&mut self, timer: SessionTimerInfo) {
        self.session_expires = Some(timer.expires);
        self.session_refresher = Some(timer.refresher);
        let refresh_after = self.refresh_after(timer.expires);
        self.timers
            .update_session_expires(timer.expires, self.tx_in.clone(), refresh_after);
    }

    fn refresh_after(&self, expires: Duration) -> Option<Duration> {
        if self.session_refresher != Some(SessionRefresher::Uas) {
            return None;
        }
        let total_ms = expires.as_millis();
        if total_ms == 0 {
            return None;
        }
        let refresh_ms = total_ms.saturating_mul(8) / 10;
        if refresh_ms == 0 {
            return None;
        }
        let refresh_ms = std::cmp::min(refresh_ms, u64::MAX as u128) as u64;
        Some(Duration::from_millis(refresh_ms))
    }

    fn start_ivr_timeout(&mut self) {
        let timeout = config::ivr_timeout();
        let (stop_tx, mut stop_rx) = oneshot::channel();
        let tx = self.tx_in.clone();
        self.ivr_timeout_stop = Some(stop_tx);
        tokio::spawn(async move {
            tokio::select! {
                _ = tokio::time::sleep(timeout) => {
                    let _ = tx.send(SessionIn::IvrTimeout);
                }
                _ = &mut stop_rx => {}
            }
        });
    }

    fn reset_ivr_timeout(&mut self) {
        self.stop_ivr_timeout();
        self.start_ivr_timeout();
    }

    fn stop_ivr_timeout(&mut self) {
        if let Some(stop) = self.ivr_timeout_stop.take() {
            let _ = stop.send(());
        }
    }

    fn cancel_transfer(&mut self) {
        if let Some(cancel) = self.transfer_cancel.take() {
            let _ = cancel.send(());
        }
        self.stop_transfer_announce();
    }

    fn start_transfer_announce(&mut self) {
        self.stop_transfer_announce();
        let (stop_tx, mut stop_rx) = oneshot::channel();
        let tx = self.tx_in.clone();
        self.transfer_announce_stop = Some(stop_tx);
        tokio::spawn(async move {
            let mut tick = interval(TRANSFER_ANNOUNCE_INTERVAL);
            tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
            tick.tick().await;
            loop {
                tokio::select! {
                    _ = tick.tick() => {
                        let _ = tx.send(SessionIn::TransferAnnounce);
                    }
                    _ = &mut stop_rx => {
                        break;
                    }
                }
            }
        });
    }

    fn stop_transfer_announce(&mut self) {
        if let Some(stop) = self.transfer_announce_stop.take() {
            let _ = stop.send(());
        }
    }

    async fn shutdown_b_leg(&mut self, send_bye: bool) {
        if let Some(mut b_leg) = self.b_leg.take() {
            if send_bye {
                if let Err(e) = b_leg.send_bye().await {
                    warn!("[session {}] B-leg BYE failed: {:?}", self.call_id, e);
                }
            }
            b_leg.shutdown();
            self.rtp_tx.stop(&b_leg.rtp_key);
        }
    }

    fn send_bye_to_a_leg(&self) {
        let _ = self
            .session_out_tx
            .send((self.call_id.clone(), SessionOut::SipSendBye));
    }

    fn peer_rtp_addr(&self) -> Option<SocketAddr> {
        let peer = self.peer_sdp.as_ref()?;
        format!("{}:{}", peer.ip, peer.port).parse().ok()
    }

    fn start_playback(&mut self, paths: &[&str]) -> Result<(), Error> {
        let Some(_dst) = self.peer_rtp_addr() else {
            return Ok(());
        };
        self.cancel_playback();
        self.stop_ivr_timeout();
        let mut frames = Vec::new();
        for path in paths {
            let mut loaded = self.storage_port.load_wav_as_pcmu_frames(path)?;
            frames.append(&mut loaded);
        }
        if frames.is_empty() {
            anyhow::bail!("no frames");
        }
        self.align_rtp_clock();
        self.sending_audio = true;
        self.playback = Some(PlaybackState { frames, index: 0 });
        Ok(())
    }

    fn step_playback(&mut self) {
        let Some(mut state) = self.playback.take() else {
            return;
        };
        if state.index >= state.frames.len() {
            self.finish_playback(true);
            return;
        }
        let frame = state.frames[state.index].clone();
        state.index += 1;
        self.recorder.push_tx_mulaw(&frame);
        self.rtp_tx.send_payload(&self.call_id, frame);
        self.rtp_last_sent = Some(Instant::now());
        if state.index < state.frames.len() {
            self.playback = Some(state);
        } else {
            self.finish_playback(true);
        }
    }

    fn finish_playback(&mut self, restart_ivr_timeout: bool) {
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

    fn cancel_playback(&mut self) {
        if self.playback.is_some() {
            info!("[session {}] playback cancelled", self.call_id);
        }
        self.finish_playback(false);
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
        self.rtp_tx.send_payload(&self.call_id, frame);
        self.rtp_last_sent = Some(Instant::now());
        Ok(())
    }

    async fn send_ingest(&mut self, status: &str) {
        if self.ingest_sent {
            return;
        }
        let ingest_url = match &self.ingest_url {
            Some(u) => u.clone(),
            None => return,
        };
        let started_at = self.started_wall.unwrap_or_else(std::time::SystemTime::now);
        let ended_at = std::time::SystemTime::now();
        let duration_sec = self.started_at.map(|s| s.elapsed().as_secs()).unwrap_or(0);
        let recording_dir = self.recorder.relative_path();
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
                "sampleRate": 8000,
                "channels": 1
            })),
        });

        let url = ingest_url.clone();
        let call_id = self.call_id.clone();
        let ingest_port = self.ingest_port.clone();
        self.ingest_sent = true;
        tokio::spawn(async move {
            if let Err(e) = ingest_port.post(url, payload).await {
                log::warn!("[ingest] failed to post call {}: {:?}", call_id, e);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intro_path_matches_time_window() {
        assert_eq!(intro_wav_path_for_hour(5), INTRO_MORNING_WAV_PATH);
        assert_eq!(intro_wav_path_for_hour(11), INTRO_MORNING_WAV_PATH);
        assert_eq!(intro_wav_path_for_hour(12), INTRO_AFTERNOON_WAV_PATH);
        assert_eq!(intro_wav_path_for_hour(16), INTRO_AFTERNOON_WAV_PATH);
        assert_eq!(intro_wav_path_for_hour(17), INTRO_EVENING_WAV_PATH);
        assert_eq!(intro_wav_path_for_hour(4), INTRO_EVENING_WAV_PATH);
    }

    #[test]
    fn ivr_action_maps_digit() {
        assert_eq!(ivr_action_for_digit('1'), IvrAction::EnterVoicebot);
        assert_eq!(ivr_action_for_digit('2'), IvrAction::PlaySendai);
        assert_eq!(ivr_action_for_digit('3'), IvrAction::Transfer);
        assert_eq!(ivr_action_for_digit('9'), IvrAction::ReplayMenu);
        assert_eq!(ivr_action_for_digit('5'), IvrAction::Invalid);
    }

    #[test]
    fn ivr_state_transitions() {
        assert_eq!(
            ivr_state_after_action(IvrState::IvrMenuWaiting, IvrAction::EnterVoicebot),
            IvrState::VoicebotIntroPlaying
        );
        assert_eq!(
            ivr_state_after_action(IvrState::IvrMenuWaiting, IvrAction::ReplayMenu),
            IvrState::IvrMenuWaiting
        );
    }

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

    fn build_test_session(storage_port: Arc<dyn StoragePort>) -> Session {
        let (session_out_tx, _session_out_rx) = tokio::sync::mpsc::unbounded_channel();
        let (app_tx, _app_rx) = tokio::sync::mpsc::unbounded_channel();
        let (tx_in, _rx_in) = tokio::sync::mpsc::unbounded_channel();
        Session {
            state: SessState::Idle,
            call_id: "test-call".to_string(),
            from_uri: "sip:from@example.com".to_string(),
            to_uri: "sip:to@example.com".to_string(),
            ingest_url: None,
            recording_base_url: None,
            ingest_sent: false,
            ingest_port: Arc::new(DummyIngestPort),
            storage_port,
            peer_sdp: Some(Sdp::pcmu("127.0.0.1", 10000)),
            local_sdp: None,
            session_out_tx,
            tx_in,
            app_tx,
            media_cfg: MediaConfig::pcmu("127.0.0.1", 10000),
            rtp_tx: RtpTxHandle::new(),
            recorder: Recorder::new("test-call"),
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
