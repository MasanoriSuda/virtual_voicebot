pub(super) mod rtp_handler;
pub(super) mod sip_handler;
pub(super) mod timer_handler;

use log::{debug, error, info, warn};
use serde::Deserialize;
use tokio::time::{Duration, Instant};
use uuid::Uuid;

use super::services::ivr_service::{ivr_action_for_digit, ivr_state_after_action, IvrAction};
use super::SessionCoordinator;
use crate::protocol::rtp::codec::mulaw_to_linear16;
use crate::protocol::session::b2bua;
use crate::protocol::session::types::{
    IvrState, SessState, SessionControlIn, SessionMediaIn, SessionOut, SessionRefresher,
    SessionTimerInfo,
};
use crate::service::routing::{ActionConfig, ActionExecutor, RuleEvaluator};
use crate::shared::ports::app::{AppEvent, EndReason};

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IvrDestinationMetadata {
    #[serde(default)]
    ivr_flow_id: Option<Uuid>,
    #[serde(default)]
    scenario_id: Option<String>,
    #[serde(default)]
    recording_enabled: Option<bool>,
    #[serde(default)]
    include_announcement: Option<bool>,
}

impl SessionCoordinator {
    pub(crate) async fn handle_control_event(
        &mut self,
        current_state: SessState,
        ev: SessionControlIn,
    ) -> bool {
        let mut advance_state = true;
        match (current_state, ev) {
            (
                SessState::Idle,
                SessionControlIn::SipInvite {
                    offer,
                    session_timer,
                    ..
                },
            ) => {
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
                self.reset_action_modes();
                self.stop_ring_delay();

                let caller_id =
                    sip_handler::extract_user_from_to(self.from_uri.as_str()).unwrap_or_default();
                let call_id_str = self.call_id.to_string();
                let evaluator = RuleEvaluator::new(self.routing_port.clone());
                match evaluator.evaluate(&caller_id, &call_id_str).await {
                    Ok(action) => {
                        info!(
                            "[SessionCoordinator] call_id={} evaluated action_code={}",
                            self.call_id, action.action_code
                        );
                        let executor = ActionExecutor::new();
                        if let Err(err) = executor.execute(&action, &call_id_str, self).await {
                            error!(
                                "[SessionCoordinator] call_id={} action execution failed: {}",
                                self.call_id, err
                            );
                            self.set_outbound_mode(false);
                        }
                    }
                    Err(err) => {
                        error!(
                            "[SessionCoordinator] call_id={} rule evaluation failed: {}",
                            self.call_id, err
                        );
                        self.set_outbound_mode(false);
                    }
                }

                if self.invite_rejected {
                    info!(
                        "[SessionCoordinator] call_id={} invite_rejected=true, skipping SIP responses",
                        self.call_id
                    );
                    self.pending_answer = None;
                    return false;
                }

                if self.no_response_mode {
                    info!(
                        "[SessionCoordinator] call_id={} NR mode active, skipping SIP responses",
                        self.call_id
                    );
                    self.pending_answer = None;
                    return false;
                }

                if self.runtime_cfg.outbound.enabled {
                    let outbound_cfg = &self.runtime_cfg.outbound;
                    let registrar = self.runtime_cfg.registrar.as_ref();
                    let user =
                        sip_handler::extract_user_from_to(self.to_uri.as_str()).unwrap_or_default();
                    let skip_outbound = registrar.map(|cfg| cfg.user == user).unwrap_or(false);
                    if !skip_outbound {
                        let target = outbound_cfg.resolve_number(user.as_str());
                        if outbound_cfg.domain.is_empty() || registrar.is_none() || target.is_none()
                        {
                            warn!(
                                "[session {}] outbound disabled (missing config)",
                                self.call_id
                            );
                            let _ = self.session_out_tx.try_send((
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
                                    self.control_tx.clone(),
                                    self.media_tx.clone(),
                                    self.runtime_cfg.clone(),
                                ));
                            }
                        }
                    }
                }
                if advance_state {
                    if !self.no_response_mode {
                        if let Err(err) = self
                            .session_out_tx
                            .try_send((self.call_id.clone(), SessionOut::SipSend100))
                        {
                            warn!(
                                "[session {}] dropped SipSend100 (channel full): {:?}",
                                self.call_id, err
                            );
                        }
                    } else {
                        info!(
                            "[SessionCoordinator] call_id={} NR mode: skipping 100 Trying",
                            self.call_id
                        );
                    }
                    if !self.outbound_mode {
                        if let Err(err) = self
                            .session_out_tx
                            .try_send((self.call_id.clone(), SessionOut::SipSend180))
                        {
                            warn!(
                                "[session {}] dropped SipSend180 (channel full): {:?}",
                                self.call_id, err
                            );
                        }
                        let from = sip_handler::extract_notify_from(self.from_uri.as_str());
                        let _ = self
                            .app_tx
                            .send(AppEvent::CallRinging {
                                call_id: self.call_id.clone(),
                                from,
                                timestamp: sip_handler::now_jst(),
                            })
                            .await;
                        let ring_duration = self.runtime_cfg.ring_duration;
                        if ring_duration.is_zero() {
                            if let Err(err) = self
                                .session_out_tx
                                .try_send((self.call_id.clone(), SessionOut::SipSend200 { answer }))
                            {
                                warn!(
                                    "[session {}] dropped SipSend200 (channel full): {:?}",
                                    self.call_id, err
                                );
                            }
                        } else {
                            self.pending_answer = Some(answer);
                            self.start_ring_delay(ring_duration);
                        }
                    }
                }
            }
            (_, SessionControlIn::RingDurationElapsed) => {
                if self.outbound_mode || self.invite_rejected {
                    return false;
                }
                if self.no_response_mode {
                    info!(
                        "[SessionCoordinator] call_id={} NR mode: skipping 200 OK (RingDurationElapsed)",
                        self.call_id
                    );
                    self.pending_answer = None;
                    return false;
                }
                if let Some(answer) = self.pending_answer.take() {
                    let _ = self
                        .session_out_tx
                        .send((self.call_id.clone(), SessionOut::SipSend200 { answer }))
                        .await;
                }
            }
            (SessState::Early, SessionControlIn::SipAck) => {
                if self.intro_sent {
                    advance_state = false;
                }
                if self.invite_rejected {
                    advance_state = false;
                }
                if self.no_response_mode {
                    advance_state = false;
                }
                if !advance_state {
                    return false;
                }
                self.started_at = Some(Instant::now());
                self.started_wall = Some(std::time::SystemTime::now());
                if let Err(e) = self.recording.start_main() {
                    warn!(
                        "[session {}] failed to start recorder: {:?}",
                        self.call_id, e
                    );
                }
                if let Err(e) = self.recording.start_b_leg() {
                    warn!(
                        "[session {}] failed to start b-leg recorder: {:?}",
                        self.call_id, e
                    );
                }
                if !self.ensure_a_leg_rtp_started() {
                    return false;
                }
                self.capture.reset();
                self.intro_sent = true;

                self.align_rtp_clock();

                let caller = sip_handler::extract_user_from_to(self.from_uri.as_str());
                let _ = self
                    .app_tx
                    .send(AppEvent::CallStarted {
                        call_id: self.call_id.clone(),
                        caller,
                    })
                    .await;

                if !self.outbound_mode {
                    if self.announce_mode {
                        self.ivr_state = IvrState::Transferring;
                        let announcement_path = self
                            .resolve_announcement_playback_path()
                            .await
                            .unwrap_or_else(|| super::ANNOUNCEMENT_FALLBACK_WAV_PATH.to_string());
                        if self.voicemail_mode {
                            info!(
                                "[session {}] playing voicemail announcement path={}",
                                self.call_id, announcement_path
                            );
                        } else {
                            info!(
                                "[session {}] playing announcement path={}",
                                self.call_id, announcement_path
                            );
                        }
                        if let Err(e) = self.start_playback(&[announcement_path.as_str()]).await {
                            warn!(
                                "[session {}] failed to play announcement: {:?}",
                                self.call_id, e
                            );
                            if !self.voicemail_mode {
                                let _ = self.control_tx.try_send(SessionControlIn::AppHangup);
                            }
                        }
                    } else if self.voicebot_direct_mode {
                        let intro_path = if self.recording_notice_pending {
                            self.recording_notice_pending = false;
                            let path = self
                                .resolve_announcement_playback_path()
                                .await
                                .unwrap_or_else(|| {
                                    super::ANNOUNCEMENT_FALLBACK_WAV_PATH.to_string()
                                });
                            self.announcement_id = None;
                            self.announcement_audio_file_url = None;
                            Some(path)
                        } else {
                            None
                        };
                        self.transition_to_voicebot_mode(intro_path).await;
                    } else {
                        if let Some(ivr_flow_id) = self.ivr_flow_id {
                            if !self.enter_db_ivr_flow(ivr_flow_id).await {
                                warn!(
                                    "[session {}] failed to start DB IVR flow id={}, fallback to legacy IVR menu",
                                    self.call_id, ivr_flow_id
                                );
                                self.start_legacy_ivr_menu().await;
                            }
                        } else {
                            self.start_legacy_ivr_menu().await;
                        }
                    }
                }

                self.start_keepalive_timer();
                self.start_session_timer_if_needed();
            }
            (_, SessionControlIn::B2buaEstablished { b_leg }) => {
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
                self.recording.ensure_b_leg();
                if self.recording.is_started() {
                    if let Err(e) = self.recording.start_b_leg() {
                        warn!(
                            "[session {}] failed to start b-leg recorder: {:?}",
                            self.call_id, e
                        );
                    }
                }
                if let Some(b_leg) = &self.b_leg {
                    let payload_type = 0; // PCMU
                    let ssrc = rand::random::<u32>();
                    self.rtp.start(
                        b_leg.rtp_key.clone(),
                        b_leg.remote_rtp_addr,
                        payload_type,
                        ssrc,
                        0,
                        0,
                    );
                }
                let _ = self.ensure_a_leg_rtp_started();
                if self.outbound_mode && !self.outbound_answered {
                    if let Some(answer) = self.local_sdp.clone() {
                        let _ = self
                            .session_out_tx
                            .try_send((self.call_id.clone(), SessionOut::SipSend200 { answer }));
                        self.outbound_answered = true;
                    }
                }
            }
            (_, SessionControlIn::B2buaFailed { reason, status }) => {
                warn!("[session {}] transfer failed: {}", self.call_id, reason);
                self.transfer_cancel = None;
                self.stop_transfer_announce();
                if self.outbound_mode {
                    let code = status.unwrap_or(503);
                    let _ = self.session_out_tx.try_send((
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
                    if let Err(e) = self
                        .start_playback(&[
                            super::TRANSFER_FAIL_WAV_PATH,
                            super::IVR_INTRO_AGAIN_WAV_PATH,
                        ])
                        .await
                    {
                        warn!(
                            "[session {}] failed to play transfer fail flow: {:?}",
                            self.call_id, e
                        );
                        self.reset_ivr_timeout();
                    }
                }
            }
            (_, SessionControlIn::BLegBye) => {
                info!("[session {}] B-leg BYE received, ending call", self.call_id);
                self.cancel_transfer();
                self.shutdown_b_leg(false).await;
                self.cancel_playback();
                self.stop_keepalive_timer();
                self.stop_session_timer();
                self.stop_ivr_timeout();
                self.send_bye_to_a_leg();
                self.stop_recorders();
                self.send_ingest("ended").await;
                self.rtp.stop(self.call_id.as_str());
                let _ = self
                    .session_out_tx
                    .send((self.call_id.clone(), SessionOut::RtpStopTx));
                self.send_call_ended(EndReason::Bye);
            }
            (_, SessionControlIn::B2buaRinging) => {
                if self.outbound_mode && !self.outbound_sent_180 && !self.outbound_sent_183 {
                    let _ = self
                        .session_out_tx
                        .send((self.call_id.clone(), SessionOut::SipSend180));
                    self.outbound_sent_180 = true;
                }
            }
            (_, SessionControlIn::B2buaEarlyMedia) => {
                if !self.outbound_mode || self.invite_rejected {
                    return false;
                }
                self.ivr_state = IvrState::B2buaMode;
                if !self.ensure_a_leg_rtp_started() {
                    return false;
                }
                if !self.outbound_sent_183 {
                    if let Some(answer) = self.local_sdp.clone() {
                        let _ = self
                            .session_out_tx
                            .send((self.call_id.clone(), SessionOut::SipSend183 { answer }));
                        self.outbound_sent_183 = true;
                    }
                }
            }
            (_, SessionControlIn::TransferAnnounce) => {
                if self.ivr_state == IvrState::Transferring && self.playback.is_none() {
                    if let Err(e) = self.start_playback(&[super::TRANSFER_WAV_PATH]).await {
                        warn!(
                            "[session {}] failed to replay transfer wav: {:?}",
                            self.call_id, e
                        );
                    }
                }
            }
            (SessState::Established, SessionControlIn::SipReInvite { session_timer, .. }) => {
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
            (SessState::Established, SessionControlIn::MediaTimerTick) => {
                self.recording.flush_tick();
                if let Err(e) = self.send_silence_frame().await {
                    warn!("[session {}] silence send failed: {:?}", self.call_id, e);
                }
            }
            (_, SessionControlIn::SipCancel) => {
                info!(
                    "[session {}] CANCEL received, terminating early",
                    self.call_id
                );
                self.invite_rejected = true;
                self.stop_ring_delay();
                self.cancel_transfer();
                self.shutdown_b_leg(true).await;
                self.cancel_playback();
                self.stop_keepalive_timer();
                self.stop_session_timer();
                self.stop_ivr_timeout();
                self.rtp.stop(self.call_id.as_str());
                let _ = self
                    .session_out_tx
                    .try_send((self.call_id.clone(), SessionOut::RtpStopTx));
                let _ = self.session_out_tx.try_send((
                    self.call_id.clone(),
                    SessionOut::SipSendError {
                        code: 487,
                        reason: "Request Terminated".to_string(),
                    },
                ));
                self.stop_recorders();
                self.send_call_ended(EndReason::Cancel);
            }
            (_, SessionControlIn::SipBye) => {
                self.stop_ring_delay();
                self.cancel_transfer();
                self.shutdown_b_leg(true).await;
                self.cancel_playback();
                self.stop_keepalive_timer();
                self.stop_session_timer();
                self.stop_ivr_timeout();
                self.rtp.stop(self.call_id.as_str());
                let _ = self
                    .session_out_tx
                    .try_send((self.call_id.clone(), SessionOut::RtpStopTx));
                let _ = self
                    .session_out_tx
                    .try_send((self.call_id.clone(), SessionOut::SipSendBye200));
                self.stop_recorders();
                self.send_ingest("ended").await;
                self.send_call_ended(EndReason::Bye);
            }
            (_, SessionControlIn::SipTransactionTimeout { call_id: _ }) => {
                warn!("[session {}] transaction timeout notified", self.call_id);
            }
            (SessState::Established, SessionControlIn::AppBotAudioFile { path }) => {
                if let Err(e) = self.start_playback(&[path.as_str()]).await {
                    warn!(
                        "[session {}] failed to send app audio: {:?}",
                        self.call_id, e
                    );
                }
            }
            (_, SessionControlIn::AppHangup) => {
                warn!("[session {}] app requested hangup", self.call_id);
                self.stop_ring_delay();
                self.cancel_transfer();
                self.shutdown_b_leg(true).await;
                self.cancel_playback();
                self.stop_keepalive_timer();
                self.stop_session_timer();
                self.stop_ivr_timeout();
                self.stop_recorders();
                self.send_ingest("ended").await;
                self.rtp.stop(self.call_id.as_str());
                let _ = self
                    .session_out_tx
                    .try_send((self.call_id.clone(), SessionOut::RtpStopTx));
                let _ = self
                    .session_out_tx
                    .try_send((self.call_id.clone(), SessionOut::SipSendBye));
                self.send_call_ended(EndReason::AppHangup);
            }
            (_, SessionControlIn::AppTransferRequest { person }) => {
                if self.transfer_cancel.is_some() || self.b_leg.is_some() {
                    warn!(
                        "[session {}] transfer already active (person={})",
                        self.call_id, person
                    );
                    return false;
                }
                info!(
                    "[session {}] transfer requested by app (person={})",
                    self.call_id, person
                );
                self.cancel_playback();
                self.stop_ivr_timeout();
                self.ivr_state = IvrState::Transferring;
                self.transfer_cancel = Some(b2bua::spawn_transfer(
                    self.call_id.clone(),
                    self.control_tx.clone(),
                    self.media_tx.clone(),
                    self.runtime_cfg.clone(),
                ));
            }
            (_, SessionControlIn::SipSessionExpires { timer }) => {
                self.update_session_expires(timer);
            }
            (_, SessionControlIn::IvrTimeout) => {
                if self.ivr_state == IvrState::IvrMenuWaiting {
                    if self.ivr_keypad_node_id.is_some() {
                        self.handle_db_ivr_timeout().await;
                    } else {
                        info!("[session {}] IVR timeout, replaying menu", self.call_id);
                        self.stop_ivr_timeout();
                        if let Err(e) = self
                            .start_playback(&[super::IVR_INTRO_AGAIN_WAV_PATH])
                            .await
                        {
                            warn!(
                                "[session {}] failed to replay IVR menu: {:?}",
                                self.call_id, e
                            );
                            self.reset_ivr_timeout();
                        }
                    }
                }
            }
            (_, SessionControlIn::SessionRefreshDue) => {
                if let (Some(expires), Some(SessionRefresher::Uas)) =
                    (self.session_expires, self.session_refresher)
                {
                    let _ = self
                        .session_out_tx
                        .try_send((self.call_id.clone(), SessionOut::SipSendUpdate { expires }));
                    self.update_session_expires(SessionTimerInfo {
                        expires,
                        refresher: SessionRefresher::Uas,
                    });
                }
            }
            (_, SessionControlIn::SessionTimerFired) => {
                warn!("[session {}] session timer fired", self.call_id);
                self.stop_ring_delay();
                self.cancel_transfer();
                self.shutdown_b_leg(true).await;
                self.cancel_playback();
                self.stop_keepalive_timer();
                self.stop_session_timer();
                self.stop_ivr_timeout();
                self.stop_recorders();
                self.send_ingest("ended").await;
                let _ = self
                    .session_out_tx
                    .send((self.call_id.clone(), SessionOut::RtpStopTx));
                let _ = self
                    .session_out_tx
                    .send((self.call_id.clone(), SessionOut::AppSessionTimeout));
                self.send_call_ended(EndReason::Timeout);
            }
            (_, SessionControlIn::Abort(e)) => {
                warn!("call {} abort: {e:?}", self.call_id);
                self.stop_ring_delay();
                self.cancel_transfer();
                self.shutdown_b_leg(true).await;
                self.cancel_playback();
                self.stop_keepalive_timer();
                self.stop_session_timer();
                self.stop_ivr_timeout();
                self.stop_recorders();
                self.send_ingest("failed").await;
                self.rtp.stop(self.call_id.as_str());
                let _ = self
                    .session_out_tx
                    .send((self.call_id.clone(), SessionOut::RtpStopTx));
                self.send_call_ended(EndReason::Error);
            }
            _ => { /* それ以外は無視 or ログ */ }
        }
        advance_state
    }

    pub(crate) async fn handle_media_event(&mut self, ev: SessionMediaIn) {
        match ev {
            SessionMediaIn::MediaRtpIn {
                call_id,
                stream_id: _,
                payload,
                ..
            } => {
                if call_id != self.call_id {
                    warn!(
                        "[session {}] media event for different call_id={}",
                        self.call_id, call_id
                    );
                    return;
                }
                if self.state_machine.state() != SessState::Established {
                    return;
                }
                debug!(
                    "[session {}] RTP payload received len={}",
                    self.call_id,
                    payload.len()
                );
                let payload_len = payload.len();
                self.recording.push_rx(&payload);
                if self.ivr_state == IvrState::B2buaMode {
                    if let Some(b_leg) = &self.b_leg {
                        self.rtp.send_payload(&b_leg.rtp_key, payload.clone());
                    }
                    self.recording.push_b_leg_tx(&payload);
                } else if self.ivr_state == IvrState::VoicebotMode {
                    if let Some(buffer) = self.capture.ingest(&payload) {
                        info!(
                            "[session {}] buffered audio ready for app ({} bytes)",
                            self.call_id,
                            buffer.len()
                        );
                        let pcm_linear16: Vec<i16> =
                            buffer.iter().map(|&b| mulaw_to_linear16(b)).collect();
                        if let Err(err) = self.app_tx.try_send_latest(AppEvent::AudioBuffered {
                            call_id: self.call_id.clone(),
                            pcm_mulaw: buffer,
                            pcm_linear16,
                        }) {
                            warn!(
                                "[session {}] dropped AudioBuffered event (channel full): {:?}",
                                self.call_id, err
                            );
                        }
                        self.capture.start();
                    }
                }
                let _ = self.session_out_tx.try_send((
                    self.call_id.clone(),
                    SessionOut::Metrics {
                        name: "rtp_in",
                        value: payload_len as i64,
                    },
                ));
            }
            SessionMediaIn::Dtmf {
                call_id,
                stream_id: _,
                digit,
            } => {
                if call_id != self.call_id {
                    warn!(
                        "[session {}] DTMF event for different call_id={}",
                        self.call_id, call_id
                    );
                    return;
                }
                if self.state_machine.state() != SessState::Established {
                    return;
                }
                info!("[session {}] DTMF received: '{}'", self.call_id, digit);
                if self.ivr_state == IvrState::VoicebotIntroPlaying {
                    info!(
                        "[session {}] ignoring DTMF during voicebot intro",
                        self.call_id
                    );
                    return;
                }
                if self.ivr_state != IvrState::IvrMenuWaiting {
                    debug!(
                        "[session {}] ignoring DTMF in {:?}",
                        self.call_id, self.ivr_state
                    );
                    return;
                }
                self.cancel_playback();
                self.stop_ivr_timeout();
                if self.ivr_keypad_node_id.is_some() {
                    self.handle_db_ivr_dtmf(digit).await;
                    return;
                }
                let action = ivr_action_for_digit(digit);
                match action {
                    IvrAction::EnterVoicebot => {
                        info!("[session {}] starting voicebot intro", self.call_id);
                        if let Err(e) = self.start_playback(&[super::VOICEBOT_INTRO_WAV_PATH]).await
                        {
                            warn!("[session {}] voicebot intro failed: {:?}", self.call_id, e);
                            self.ivr_state = IvrState::VoicebotMode;
                            self.capture.reset();
                            self.capture.start();
                        } else {
                            self.ivr_state = ivr_state_after_action(self.ivr_state, action);
                        }
                    }
                    IvrAction::PlaySendai => {
                        info!("[session {}] playing sendai info", self.call_id);
                        if let Err(e) = self
                            .start_playback(&[
                                super::IVR_SENDAI_WAV_PATH,
                                super::IVR_INTRO_AGAIN_WAV_PATH,
                            ])
                            .await
                        {
                            warn!(
                                "[session {}] failed to play sendai flow: {:?}",
                                self.call_id, e
                            );
                            self.reset_ivr_timeout();
                        }
                    }
                    IvrAction::Transfer => {
                        if self.transfer_cancel.is_some() || self.b_leg.is_some() {
                            warn!("[session {}] transfer already active", self.call_id);
                            self.reset_ivr_timeout();
                            return;
                        }
                        info!("[session {}] initiating transfer to B-leg", self.call_id);
                        self.ivr_state = IvrState::Transferring;
                        if let Err(e) = self.start_playback(&[super::TRANSFER_WAV_PATH]).await {
                            warn!(
                                "[session {}] failed to play transfer wav: {:?}",
                                self.call_id, e
                            );
                        }
                        self.start_transfer_announce();
                        self.transfer_cancel = Some(b2bua::spawn_transfer(
                            self.call_id.clone(),
                            self.control_tx.clone(),
                            self.media_tx.clone(),
                            self.runtime_cfg.clone(),
                        ));
                    }
                    IvrAction::ReplayMenu => {
                        info!("[session {}] replaying IVR menu", self.call_id);
                        if let Err(e) = self
                            .start_playback(&[super::IVR_INTRO_AGAIN_WAV_PATH])
                            .await
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
                        if let Err(e) = self
                            .start_playback(&[
                                super::IVR_INVALID_WAV_PATH,
                                super::IVR_INTRO_AGAIN_WAV_PATH,
                            ])
                            .await
                        {
                            warn!(
                                "[session {}] failed to play invalid flow: {:?}",
                                self.call_id, e
                            );
                            self.reset_ivr_timeout();
                        }
                    }
                }
            }
            SessionMediaIn::BLegRtp {
                call_id,
                stream_id: _,
                payload,
            } => {
                if call_id != self.call_id {
                    warn!(
                        "[session {}] b-leg media event for different call_id={}",
                        self.call_id, call_id
                    );
                    return;
                }
                if self.ivr_state == IvrState::B2buaMode {
                    self.align_rtp_clock();
                    self.recording.push_tx(&payload);
                    self.recording.push_b_leg_rx(&payload);
                    self.rtp.send_payload(self.call_id.as_str(), payload);
                    self.rtp_last_sent = Some(Instant::now());
                }
            }
        }
    }

    async fn start_legacy_ivr_menu(&mut self) {
        self.ivr_state = IvrState::IvrMenuWaiting;
        self.ivr_keypad_node_id = None;
        self.ivr_menu_audio_file_url = Some(super::IVR_INTRO_WAV_PATH.to_string());
        self.ivr_retry_count = 0;
        self.ivr_max_retries = 0;
        self.ivr_timeout_override = None;

        let mut playback_paths: Vec<String> = Vec::with_capacity(2);
        if self.recording_notice_pending {
            self.recording_notice_pending = false;
            let recording_notice_path = self
                .resolve_announcement_playback_path()
                .await
                .unwrap_or_else(|| super::ANNOUNCEMENT_FALLBACK_WAV_PATH.to_string());
            info!(
                "[session {}] prepending recording notice path={}",
                self.call_id, recording_notice_path
            );
            playback_paths.push(recording_notice_path);
            self.announcement_id = None;
            self.announcement_audio_file_url = None;
        }
        playback_paths.push(super::IVR_INTRO_WAV_PATH.to_string());
        let playback_refs: Vec<&str> = playback_paths.iter().map(String::as_str).collect();

        if let Err(err) = self.start_playback(playback_refs.as_slice()).await {
            warn!(
                "[session {}] failed to send IVR intro wav: {:?}",
                self.call_id, err
            );
            self.reset_ivr_timeout();
        } else {
            info!(
                "[session {}] sent IVR intro wav {}",
                self.call_id,
                super::IVR_INTRO_WAV_PATH
            );
        }
    }

    async fn enter_db_ivr_flow(&mut self, ivr_flow_id: Uuid) -> bool {
        let menu = match self.routing_port.find_ivr_menu(ivr_flow_id).await {
            Ok(Some(row)) => row,
            Ok(None) => {
                warn!(
                    "[session {}] IVR flow not found or inactive id={}",
                    self.call_id, ivr_flow_id
                );
                return false;
            }
            Err(err) => {
                warn!(
                    "[session {}] failed to read IVR flow id={} error={}",
                    self.call_id, ivr_flow_id, err
                );
                return false;
            }
        };

        self.ivr_state = IvrState::IvrMenuWaiting;
        self.ivr_flow_id = Some(ivr_flow_id);
        self.ivr_keypad_node_id = Some(menu.keypad_node_id);
        self.ivr_retry_count = 0;
        self.ivr_max_retries = normalize_max_retries(menu.max_retries);
        self.ivr_timeout_override =
            Some(Duration::from_secs(normalize_timeout_sec(menu.timeout_sec)));
        self.ivr_menu_audio_file_url = Some(
            menu.audio_file_url
                .map(super::map_audio_file_url_to_local_path)
                .unwrap_or_else(|| super::IVR_INTRO_WAV_PATH.to_string()),
        );
        let menu_path = self
            .ivr_menu_audio_file_url
            .as_ref()
            .cloned()
            .unwrap_or_else(|| super::IVR_INTRO_WAV_PATH.to_string());

        info!(
            "[session {}] starting DB IVR flow id={} root_node_id={} keypad_node_id={} timeout_sec={} max_retries={}",
            self.call_id,
            ivr_flow_id,
            menu.root_node_id,
            menu.keypad_node_id,
            normalize_timeout_sec(menu.timeout_sec),
            self.ivr_max_retries
        );

        if let Err(err) = self.start_playback(&[menu_path.as_str()]).await {
            warn!(
                "[session {}] failed to start IVR menu playback path={} error={:?}",
                self.call_id, menu_path, err
            );
            self.reset_ivr_timeout();
            return false;
        }

        true
    }

    async fn replay_current_ivr_menu(&mut self) {
        let replay_path = self
            .ivr_menu_audio_file_url
            .as_ref()
            .cloned()
            .unwrap_or_else(|| super::IVR_INTRO_AGAIN_WAV_PATH.to_string());
        if let Err(err) = self.start_playback(&[replay_path.as_str()]).await {
            warn!(
                "[session {}] failed to replay IVR menu path={} error={:?}",
                self.call_id, replay_path, err
            );
            self.reset_ivr_timeout();
        }
    }

    async fn handle_db_ivr_timeout(&mut self) {
        info!("[session {}] IVR timeout detected", self.call_id);
        self.handle_db_ivr_retry("TIMEOUT").await;
    }

    async fn handle_db_ivr_dtmf(&mut self, digit: char) {
        let Some(keypad_node_id) = self.ivr_keypad_node_id else {
            warn!(
                "[session {}] DB IVR keypad node missing while handling DTMF '{}'",
                self.call_id, digit
            );
            self.reset_ivr_timeout();
            return;
        };

        let dtmf_key = digit.to_string();
        match self
            .routing_port
            .find_ivr_dtmf_destination(keypad_node_id, dtmf_key.as_str())
            .await
        {
            Ok(Some(destination)) => {
                info!(
                    "[session {}] IVR DTMF matched key={} destination_node_id={}",
                    self.call_id, dtmf_key, destination.node_id
                );
                self.ivr_retry_count = 0;
                self.execute_db_ivr_destination(destination).await;
            }
            Ok(None) => {
                info!(
                    "[session {}] IVR invalid DTMF key={} (no transition)",
                    self.call_id, dtmf_key
                );
                self.handle_db_ivr_retry("INVALID").await;
            }
            Err(err) => {
                warn!(
                    "[session {}] failed to read IVR DTMF transition key={} error={}",
                    self.call_id, dtmf_key, err
                );
                self.reset_ivr_timeout();
            }
        }
    }

    async fn handle_db_ivr_retry(&mut self, input_type: &'static str) {
        self.ivr_retry_count = self.ivr_retry_count.saturating_add(1);
        if self.ivr_retry_count <= self.ivr_max_retries {
            info!(
                "[session {}] IVR retry input_type={} retry={}/{}",
                self.call_id, input_type, self.ivr_retry_count, self.ivr_max_retries
            );
            self.replay_current_ivr_menu().await;
            return;
        }

        let Some(keypad_node_id) = self.ivr_keypad_node_id else {
            warn!(
                "[session {}] IVR fallback cannot resolve because keypad node is missing",
                self.call_id
            );
            self.replay_current_ivr_menu().await;
            return;
        };

        info!(
            "[session {}] IVR retries exhausted input_type={} retry={} max_retries={}",
            self.call_id, input_type, self.ivr_retry_count, self.ivr_max_retries
        );
        let fallback_result = match input_type {
            "TIMEOUT" => {
                self.routing_port
                    .find_ivr_timeout_destination(keypad_node_id)
                    .await
            }
            _ => {
                self.routing_port
                    .find_ivr_invalid_destination(keypad_node_id)
                    .await
            }
        };

        match fallback_result {
            Ok(Some(destination)) => self.execute_db_ivr_destination(destination).await,
            Ok(None) => {
                warn!(
                    "[session {}] IVR fallback transition missing input_type={}",
                    self.call_id, input_type
                );
                self.replay_current_ivr_menu().await;
            }
            Err(err) => {
                warn!(
                    "[session {}] failed to read IVR fallback transition input_type={} error={}",
                    self.call_id, input_type, err
                );
                self.replay_current_ivr_menu().await;
            }
        }
    }

    async fn execute_db_ivr_destination(
        &mut self,
        destination: crate::shared::ports::routing_port::IvrDestinationRow,
    ) {
        let action_code = destination.action_code.trim().to_ascii_uppercase();
        let metadata = parse_ivr_destination_metadata(
            self.call_id.as_str(),
            destination.node_id,
            destination.metadata_json.as_deref(),
        );
        let mut action = ActionConfig::default_vr();
        action.action_code = action_code.clone();
        action.ivr_flow_id = metadata.ivr_flow_id;
        action.recording_enabled = metadata.recording_enabled.unwrap_or(true);
        action.announcement_audio_file_url = destination.audio_file_url.clone();
        action.scenario_id = metadata.scenario_id;
        action.include_announcement = metadata.include_announcement;
        let previous_ivr_flow_id = self.ivr_flow_id;
        let previous_ivr_menu_audio_file_url = self.ivr_menu_audio_file_url.clone();
        let previous_ivr_keypad_node_id = self.ivr_keypad_node_id;
        let previous_ivr_retry_count = self.ivr_retry_count;
        let previous_ivr_max_retries = self.ivr_max_retries;
        let previous_ivr_timeout_override = self.ivr_timeout_override;
        let call_id = self.call_id.to_string();

        info!(
            "[session {}] executing IVR destination node_id={} action_code={}",
            self.call_id, destination.node_id, action_code
        );
        if let Err(err) = ActionExecutor::new()
            .execute(&action, call_id.as_str(), self)
            .await
        {
            warn!(
                "[session {}] failed to execute IVR destination action_code={} error={}",
                self.call_id, action_code, err
            );
            self.ivr_flow_id = previous_ivr_flow_id;
            self.ivr_menu_audio_file_url = previous_ivr_menu_audio_file_url;
            self.ivr_keypad_node_id = previous_ivr_keypad_node_id;
            self.ivr_retry_count = previous_ivr_retry_count;
            self.ivr_max_retries = previous_ivr_max_retries;
            self.ivr_timeout_override = previous_ivr_timeout_override;
            self.replay_current_ivr_menu().await;
            return;
        }

        if action_code != "IV" && self.ivr_flow_id.is_none() {
            self.ivr_flow_id = previous_ivr_flow_id;
        }

        match action_code.as_str() {
            "IV" => {
                if let Some(next_flow_id) = self.ivr_flow_id {
                    if !self.enter_db_ivr_flow(next_flow_id).await {
                        warn!(
                            "[session {}] failed to start nested IVR flow id={}, replaying current menu",
                            self.call_id, next_flow_id
                        );
                        self.ivr_flow_id = previous_ivr_flow_id;
                        self.ivr_menu_audio_file_url = previous_ivr_menu_audio_file_url;
                        self.ivr_keypad_node_id = previous_ivr_keypad_node_id;
                        self.ivr_retry_count = previous_ivr_retry_count;
                        self.ivr_max_retries = previous_ivr_max_retries;
                        self.ivr_timeout_override = previous_ivr_timeout_override;
                        self.replay_current_ivr_menu().await;
                    }
                } else {
                    warn!(
                        "[session {}] IVR destination missing ivrFlowId metadata, fallback to voicebot",
                        self.call_id
                    );
                    self.transition_to_voicebot_mode(Some(
                        super::VOICEBOT_INTRO_WAV_PATH.to_string(),
                    ))
                    .await;
                }
            }
            "VR" => {
                self.transition_to_voicebot_mode(Some(super::VOICEBOT_INTRO_WAV_PATH.to_string()))
                    .await;
            }
            "VB" => {
                let intro_path = if action.include_announcement.unwrap_or(false) {
                    action
                        .announcement_audio_file_url
                        .as_ref()
                        .cloned()
                        .map(super::map_audio_file_url_to_local_path)
                        .or_else(|| Some(super::VOICEBOT_INTRO_WAV_PATH.to_string()))
                } else {
                    None
                };
                self.transition_to_voicebot_mode(intro_path).await;
            }
            "AN" | "VM" => {
                self.play_announcement_for_current_mode(action_code.as_str())
                    .await;
            }
            _ => {
                warn!(
                    "[session {}] unsupported IVR destination action_code={}, replaying menu",
                    self.call_id, action_code
                );
                self.ivr_flow_id = previous_ivr_flow_id;
                self.ivr_menu_audio_file_url = previous_ivr_menu_audio_file_url;
                self.ivr_keypad_node_id = previous_ivr_keypad_node_id;
                self.ivr_retry_count = previous_ivr_retry_count;
                self.ivr_max_retries = previous_ivr_max_retries;
                self.ivr_timeout_override = previous_ivr_timeout_override;
                self.replay_current_ivr_menu().await;
            }
        }
    }

    async fn transition_to_voicebot_mode(&mut self, intro_path: Option<String>) {
        self.stop_ivr_timeout();
        self.ivr_state = IvrState::VoicebotMode;
        self.ivr_keypad_node_id = None;
        self.ivr_menu_audio_file_url = None;
        self.ivr_timeout_override = None;
        self.ivr_retry_count = 0;
        self.ivr_max_retries = 0;

        if let Some(path) = intro_path {
            info!(
                "[session {}] transitioning to voicebot intro path={}",
                self.call_id, path
            );
            if let Err(err) = self.start_playback(&[path.as_str()]).await {
                warn!(
                    "[session {}] voicebot intro playback failed path={} error={:?}",
                    self.call_id, path, err
                );
                self.capture.reset();
                self.capture.start();
            } else {
                self.ivr_state = IvrState::VoicebotIntroPlaying;
            }
            return;
        }

        info!(
            "[session {}] transitioning to voicebot mode without intro announcement",
            self.call_id
        );
        self.capture.reset();
        self.capture.start();
    }

    async fn play_announcement_for_current_mode(&mut self, action_code: &str) {
        self.stop_ivr_timeout();
        self.ivr_state = IvrState::Transferring;
        let announcement_path = self
            .resolve_announcement_playback_path()
            .await
            .unwrap_or_else(|| super::ANNOUNCEMENT_FALLBACK_WAV_PATH.to_string());
        info!(
            "[session {}] playing IVR destination announcement action_code={} path={}",
            self.call_id, action_code, announcement_path
        );
        if let Err(err) = self.start_playback(&[announcement_path.as_str()]).await {
            warn!(
                "[session {}] failed to play IVR destination announcement action_code={} error={:?}",
                self.call_id, action_code, err
            );
            if action_code == "AN" {
                let _ = self.control_tx.try_send(SessionControlIn::AppHangup);
            }
        }
    }
}

fn parse_ivr_destination_metadata(
    call_id: &str,
    node_id: Uuid,
    metadata_json: Option<&str>,
) -> IvrDestinationMetadata {
    let Some(raw) = metadata_json
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return IvrDestinationMetadata::default();
    };

    match serde_json::from_str::<IvrDestinationMetadata>(raw) {
        Ok(metadata) => metadata,
        Err(err) => {
            warn!(
                "[session {}] invalid IVR destination metadata node_id={} error={} raw={}",
                call_id, node_id, err, raw
            );
            IvrDestinationMetadata::default()
        }
    }
}

fn normalize_timeout_sec(timeout_sec: i32) -> u64 {
    if timeout_sec <= 0 {
        10
    } else {
        timeout_sec as u64
    }
}

fn normalize_max_retries(max_retries: i32) -> u32 {
    if max_retries < 0 {
        2
    } else {
        max_retries as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ivr_destination_metadata_reads_known_fields() {
        let metadata = parse_ivr_destination_metadata(
            "test-call",
            Uuid::nil(),
            Some(
                r#"{"ivrFlowId":"00000000-0000-0000-0000-000000000010","recordingEnabled":false,"includeAnnouncement":true}"#,
            ),
        );
        assert_eq!(metadata.ivr_flow_id, Some(Uuid::from_u128(0x10)));
        assert_eq!(metadata.recording_enabled, Some(false));
        assert_eq!(metadata.include_announcement, Some(true));
    }

    #[test]
    fn parse_ivr_destination_metadata_returns_default_on_invalid_json() {
        let metadata = parse_ivr_destination_metadata("test-call", Uuid::nil(), Some("{invalid"));
        assert_eq!(metadata.ivr_flow_id, None);
        assert_eq!(metadata.recording_enabled, None);
        assert_eq!(metadata.include_announcement, None);
    }

    #[test]
    fn normalize_timeout_sec_uses_default_for_non_positive_values() {
        assert_eq!(normalize_timeout_sec(-1), 10);
        assert_eq!(normalize_timeout_sec(0), 10);
        assert_eq!(normalize_timeout_sec(15), 15);
    }

    #[test]
    fn normalize_max_retries_uses_default_for_negative_values() {
        assert_eq!(normalize_max_retries(-1), 2);
        assert_eq!(normalize_max_retries(0), 0);
        assert_eq!(normalize_max_retries(3), 3);
    }
}
