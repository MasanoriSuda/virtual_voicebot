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
                self.reset_call_log_tracking();
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
                    self.send_ingest("ended").await;
                    return false;
                }

                if self.no_response_mode {
                    info!(
                        "[SessionCoordinator] call_id={} NR mode active, skipping SIP responses",
                        self.call_id
                    );
                    self.pending_answer = None;
                    self.send_ingest("ended").await;
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
                                    self.from_uri.clone(),
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
                self.ensure_call_log_id();
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
                    if self.transfer_after_answer_pending {
                        self.transfer_after_answer_pending = false;
                        self.start_b2bua_transfer("vr_initial");
                    } else if self.announce_mode {
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
                    } else if let Some(ivr_flow_id) = self.ivr_flow_id {
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
                self.mark_transfer_answered();
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
                self.cancel_transfer();
                self.mark_transfer_failed();
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
                    info!(
                        "[session {}] transfer failed in IVR mode, ending call",
                        self.call_id
                    );
                    if self.b_leg.is_some() {
                        self.shutdown_b_leg(false).await;
                    }
                    self.cancel_playback();
                    self.stop_keepalive_timer();
                    self.stop_session_timer();
                    self.stop_ivr_timeout();
                    self.mark_transfer_ended();
                    self.send_bye_to_a_leg();
                    self.stop_recorders();
                    self.send_ingest("ended").await;
                    self.rtp.stop(self.call_id.as_str());
                    let _ = self
                        .session_out_tx
                        .send((self.call_id.clone(), SessionOut::RtpStopTx))
                        .await;
                    self.send_call_ended(EndReason::Error);
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
                self.mark_transfer_ended();
                self.send_bye_to_a_leg();
                self.stop_recorders();
                self.send_ingest("ended").await;
                self.rtp.stop(self.call_id.as_str());
                let _ = self
                    .session_out_tx
                    .send((self.call_id.clone(), SessionOut::RtpStopTx))
                    .await;
                self.send_call_ended(EndReason::Bye);
            }
            (_, SessionControlIn::B2buaRinging) => {
                if self.outbound_mode && !self.outbound_sent_180 && !self.outbound_sent_183 {
                    let _ = self
                        .session_out_tx
                        .send((self.call_id.clone(), SessionOut::SipSend180))
                        .await;
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
                            .send((self.call_id.clone(), SessionOut::SipSend183 { answer }))
                            .await;
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
            (_, SessionControlIn::SipReInvite { session_timer, .. }) => {
                info!(
                    "[session {}] SipReInvite received state={:?}",
                    self.call_id,
                    self.state_machine.state()
                );
                if self.state_machine.state() != SessState::Established {
                    info!(
                        "[session {}] re-INVITE received in non-established state={:?}, sending 200 defensively",
                        self.call_id,
                        self.state_machine.state()
                    );
                }
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
                if let Err(err) = self
                    .session_out_tx
                    .send((self.call_id.clone(), SessionOut::SipSend200 { answer }))
                    .await
                {
                    warn!(
                        "[session {}] failed to emit SipSend200 for re-INVITE: {:?}",
                        self.call_id, err
                    );
                } else {
                    info!(
                        "[session {}] SipSend200 emitted for re-INVITE",
                        self.call_id
                    );
                }
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
                let already_rejected = self.invite_rejected;
                self.invite_rejected = true;
                self.stop_ring_delay();
                self.cancel_transfer();
                self.shutdown_b_leg(true).await;
                self.cancel_playback();
                self.stop_keepalive_timer();
                self.stop_session_timer();
                self.stop_ivr_timeout();
                self.mark_transfer_ended();
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
                if !already_rejected {
                    self.send_ingest("ended").await;
                }
                self.send_call_ended(EndReason::Cancel);
            }
            (_, SessionControlIn::SipBye) => {
                info!("[session {}] A-leg BYE received", self.call_id);
                self.stop_ring_delay();
                self.cancel_transfer();
                self.shutdown_b_leg(true).await;
                self.cancel_playback();
                self.stop_keepalive_timer();
                self.stop_session_timer();
                self.stop_ivr_timeout();
                self.mark_transfer_ended();
                self.rtp.stop(self.call_id.as_str());
                let _ = self
                    .session_out_tx
                    .try_send((self.call_id.clone(), SessionOut::RtpStopTx));
                if let Err(e) = self
                    .session_out_tx
                    .try_send((self.call_id.clone(), SessionOut::SipSendBye200))
                {
                    warn!(
                        "[session {}] failed to send BYE 200 OK: {:?}",
                        self.call_id, e
                    );
                } else {
                    info!("[session {}] BYE 200 OK sent to A-leg", self.call_id);
                }
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
                self.mark_transfer_ended();
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
                if !self.start_b2bua_transfer(person.as_str()) {
                    return false;
                }
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
                self.mark_transfer_ended();
                self.stop_recorders();
                self.send_ingest("ended").await;
                let _ = self
                    .session_out_tx
                    .send((self.call_id.clone(), SessionOut::RtpStopTx))
                    .await;
                let _ = self
                    .session_out_tx
                    .send((self.call_id.clone(), SessionOut::AppSessionTimeout))
                    .await;
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
                self.mark_transfer_ended();
                self.stop_recorders();
                self.send_ingest("failed").await;
                self.rtp.stop(self.call_id.as_str());
                let _ = self
                    .session_out_tx
                    .send((self.call_id.clone(), SessionOut::RtpStopTx))
                    .await;
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
                        self.mark_transfer_trying();
                        if let Err(e) = self.start_playback(&[super::TRANSFER_WAV_PATH]).await {
                            warn!(
                                "[session {}] failed to play transfer wav: {:?}",
                                self.call_id, e
                            );
                        }
                        self.start_transfer_announce();
                        self.transfer_cancel = Some(b2bua::spawn_transfer(
                            self.call_id.clone(),
                            self.from_uri.clone(),
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

    fn start_b2bua_transfer(&mut self, person: &str) -> bool {
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
        self.mark_transfer_trying();
        self.transfer_cancel = Some(b2bua::spawn_transfer(
            self.call_id.clone(),
            self.from_uri.clone(),
            self.control_tx.clone(),
            self.media_tx.clone(),
            self.runtime_cfg.clone(),
        ));
        true
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
        self.record_ivr_event(
            "node_enter",
            Some(menu.root_node_id),
            None,
            None,
            None,
            None,
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
        self.record_ivr_event("dtmf_input", None, Some(digit), None, None, None);
        let Some(ivr_flow_id) = self.ivr_flow_id else {
            warn!(
                "[session {}] DB IVR flow missing while handling DTMF '{}'",
                self.call_id, digit
            );
            self.reset_ivr_timeout();
            return;
        };

        let dtmf_key = digit.to_string();
        match self
            .routing_port
            .find_ivr_dtmf_destination_by_flow(ivr_flow_id, dtmf_key.as_str())
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
        let event_type = if input_type == "TIMEOUT" {
            "timeout"
        } else {
            "invalid_input"
        };
        self.record_ivr_event(event_type, self.ivr_keypad_node_id, None, None, None, None);
        self.ivr_retry_count = self.ivr_retry_count.saturating_add(1);
        if self.ivr_retry_count <= self.ivr_max_retries {
            info!(
                "[session {}] IVR retry input_type={} retry={}/{}",
                self.call_id, input_type, self.ivr_retry_count, self.ivr_max_retries
            );
            self.replay_current_ivr_menu().await;
            return;
        }

        let Some(ivr_flow_id) = self.ivr_flow_id else {
            warn!(
                "[session {}] IVR fallback cannot resolve because flow_id is missing",
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
                    .find_ivr_timeout_destination_by_flow(ivr_flow_id)
                    .await
            }
            _ => {
                self.routing_port
                    .find_ivr_invalid_destination_by_flow(ivr_flow_id)
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
        self.record_ivr_event(
            "transition",
            None,
            None,
            Some(destination.transition_id),
            None,
            None,
        );
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
        self.record_ivr_event(
            "node_enter",
            Some(destination.node_id),
            None,
            None,
            None,
            None,
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
                self.record_ivr_event(
                    "exit",
                    None,
                    None,
                    None,
                    Some("VR"),
                    Some("transfer_initiated"),
                );
                self.set_transfer_after_answer_pending(false);
                self.start_b2bua_transfer("ivr_vr");
            }
            "VB" => {
                self.record_ivr_event(
                    "exit",
                    None,
                    None,
                    None,
                    Some("VB"),
                    Some("voicebot_started"),
                );
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
                let exit_reason = if action_code == "AN" {
                    "announcement_started"
                } else {
                    "voicemail_started"
                };
                self.record_ivr_event(
                    "exit",
                    None,
                    None,
                    None,
                    Some(action_code.as_str()),
                    Some(exit_reason),
                );
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
    use crate::protocol::rtp::tx::RtpTxHandle;
    use crate::protocol::session::capture::AudioCapture;
    use crate::protocol::session::state_machine::SessionStateMachine;
    use crate::protocol::session::timers::SessionTimers;
    use crate::protocol::session::types::{
        CallId, IvrState, MediaConfig, Sdp, SessState, SessionControlIn, SessionOut,
    };
    use crate::shared::config::SessionRuntimeConfig;
    use crate::shared::ports::app::app_event_channel;
    use crate::shared::ports::call_log_port::{CallLogPort, EndedCallLog};
    use crate::shared::ports::ingest::{IngestError, IngestFuture, IngestPayload, IngestPort};
    use crate::shared::ports::routing_port::{
        CallActionRuleRow, IvrDestinationRow, IvrMenuRow, NoopRoutingPort, RegisteredNumberRow,
        RoutingFuture, RoutingPort, RoutingRuleRow,
    };
    use crate::shared::ports::storage::{StorageError, StoragePort};
    use serde_json::Value;
    use std::sync::{Arc, Mutex};
    use tokio::sync::mpsc;
    use tokio::time::Duration;

    struct DummyIngestPort;

    impl IngestPort for DummyIngestPort {
        fn post(
            &self,
            _url: String,
            _payload: IngestPayload,
        ) -> IngestFuture<Result<(), IngestError>> {
            Box::pin(async { Ok(()) })
        }
    }

    struct DummyStoragePort;

    impl StoragePort for DummyStoragePort {
        fn load_wav_as_pcmu_frames(&self, _path: &str) -> Result<Vec<Vec<u8>>, StorageError> {
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

    #[derive(Default, Debug)]
    struct RoutingLookupState {
        old_dtmf_calls: usize,
        old_timeout_calls: usize,
        old_invalid_calls: usize,
        by_flow_dtmf: Vec<(Uuid, String)>,
        by_flow_timeout: Vec<Uuid>,
        by_flow_invalid: Vec<Uuid>,
    }

    struct FlowAwareRoutingPort {
        state: Arc<Mutex<RoutingLookupState>>,
        dtmf_destination: Option<IvrDestinationRow>,
        timeout_destination: Option<IvrDestinationRow>,
        invalid_destination: Option<IvrDestinationRow>,
        noop: NoopRoutingPort,
    }

    impl FlowAwareRoutingPort {
        fn new(
            state: Arc<Mutex<RoutingLookupState>>,
            dtmf_destination: Option<IvrDestinationRow>,
            timeout_destination: Option<IvrDestinationRow>,
            invalid_destination: Option<IvrDestinationRow>,
        ) -> Self {
            Self {
                state,
                dtmf_destination,
                timeout_destination,
                invalid_destination,
                noop: NoopRoutingPort::new(),
            }
        }
    }

    impl RoutingPort for FlowAwareRoutingPort {
        fn find_registered_number(
            &self,
            phone_number: &str,
        ) -> RoutingFuture<Option<RegisteredNumberRow>> {
            self.noop.find_registered_number(phone_number)
        }

        fn find_caller_group(&self, phone_number: &str) -> RoutingFuture<Option<Uuid>> {
            self.noop.find_caller_group(phone_number)
        }

        fn find_call_action_rule(
            &self,
            group_id: Uuid,
        ) -> RoutingFuture<Option<CallActionRuleRow>> {
            self.noop.find_call_action_rule(group_id)
        }

        fn is_spam(&self, phone_number: &str) -> RoutingFuture<bool> {
            self.noop.is_spam(phone_number)
        }

        fn is_registered(&self, phone_number: &str) -> RoutingFuture<bool> {
            self.noop.is_registered(phone_number)
        }

        fn find_routing_rule(&self, category: &str) -> RoutingFuture<Option<RoutingRuleRow>> {
            self.noop.find_routing_rule(category)
        }

        fn get_system_settings_extra(&self) -> RoutingFuture<Option<Value>> {
            self.noop.get_system_settings_extra()
        }

        fn find_announcement_audio_file_url(
            &self,
            announcement_id: Uuid,
        ) -> RoutingFuture<Option<String>> {
            self.noop.find_announcement_audio_file_url(announcement_id)
        }

        fn find_ivr_menu(&self, flow_id: Uuid) -> RoutingFuture<Option<IvrMenuRow>> {
            self.noop.find_ivr_menu(flow_id)
        }

        fn find_ivr_dtmf_destination(
            &self,
            _keypad_node_id: Uuid,
            _dtmf_key: &str,
        ) -> RoutingFuture<Option<IvrDestinationRow>> {
            let state = self.state.clone();
            Box::pin(async move {
                let mut guard = state.lock().expect("routing state lock");
                guard.old_dtmf_calls += 1;
                Ok(None)
            })
        }

        fn find_ivr_dtmf_destination_by_flow(
            &self,
            flow_id: Uuid,
            dtmf_key: &str,
        ) -> RoutingFuture<Option<IvrDestinationRow>> {
            let state = self.state.clone();
            let dtmf_key = dtmf_key.to_string();
            let destination = self.dtmf_destination.clone();
            Box::pin(async move {
                let mut guard = state.lock().expect("routing state lock");
                guard.by_flow_dtmf.push((flow_id, dtmf_key));
                Ok(destination)
            })
        }

        fn find_ivr_timeout_destination(
            &self,
            _keypad_node_id: Uuid,
        ) -> RoutingFuture<Option<IvrDestinationRow>> {
            let state = self.state.clone();
            Box::pin(async move {
                let mut guard = state.lock().expect("routing state lock");
                guard.old_timeout_calls += 1;
                Ok(None)
            })
        }

        fn find_ivr_timeout_destination_by_flow(
            &self,
            flow_id: Uuid,
        ) -> RoutingFuture<Option<IvrDestinationRow>> {
            let state = self.state.clone();
            let destination = self.timeout_destination.clone();
            Box::pin(async move {
                let mut guard = state.lock().expect("routing state lock");
                guard.by_flow_timeout.push(flow_id);
                Ok(destination)
            })
        }

        fn find_ivr_invalid_destination(
            &self,
            _keypad_node_id: Uuid,
        ) -> RoutingFuture<Option<IvrDestinationRow>> {
            let state = self.state.clone();
            Box::pin(async move {
                let mut guard = state.lock().expect("routing state lock");
                guard.old_invalid_calls += 1;
                Ok(None)
            })
        }

        fn find_ivr_invalid_destination_by_flow(
            &self,
            flow_id: Uuid,
        ) -> RoutingFuture<Option<IvrDestinationRow>> {
            let state = self.state.clone();
            let destination = self.invalid_destination.clone();
            Box::pin(async move {
                let mut guard = state.lock().expect("routing state lock");
                guard.by_flow_invalid.push(flow_id);
                Ok(destination)
            })
        }
    }

    fn build_test_session(
        routing_port: Arc<dyn RoutingPort>,
    ) -> (SessionCoordinator, mpsc::Receiver<(CallId, SessionOut)>) {
        let (session_out_tx, session_out_rx) = mpsc::channel(32);
        let (app_tx, _app_rx) = app_event_channel(16);
        let (control_tx, _control_rx) =
            mpsc::channel(super::super::SESSION_CONTROL_CHANNEL_CAPACITY);
        let (media_tx, _media_rx) = mpsc::channel(super::super::SESSION_MEDIA_CHANNEL_CAPACITY);
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
            storage_port: Arc::new(DummyStoragePort),
            peer_sdp: Some(Sdp::pcmu("127.0.0.1", 10000)),
            local_sdp: None,
            session_out_tx,
            control_tx,
            media_tx,
            app_tx,
            runtime_cfg: runtime_cfg.clone(),
            media_cfg: MediaConfig::pcmu("127.0.0.1", 10000),
            call_log_port: Arc::new(DummyCallLogPort),
            routing_port,
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
            call_log_id: None,
            initial_action_code: None,
            caller_category: "unknown".to_string(),
            call_disposition: "allowed".to_string(),
            final_action: None,
            transfer_status: "no_transfer".to_string(),
            transfer_started_at: None,
            transfer_answered_at: None,
            transfer_ended_at: None,
            ivr_event_sequence: 0,
            ivr_events: Vec::new(),
            ingest_persisted: false,
            session_expires: None,
            session_refresher: None,
        };
        (session, session_out_rx)
    }

    fn voicebot_destination() -> IvrDestinationRow {
        IvrDestinationRow {
            transition_id: Uuid::from_u128(0x10),
            node_id: Uuid::from_u128(0x20),
            action_code: "VB".to_string(),
            audio_file_url: None,
            metadata_json: Some(r#"{"includeAnnouncement":false}"#.to_string()),
        }
    }

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

    #[tokio::test]
    async fn db_ivr_dtmf_uses_flow_lookup_not_keypad_lookup() {
        let flow_id = Uuid::from_u128(0x101);
        let keypad_node_id = Uuid::from_u128(0x202);
        let state = Arc::new(Mutex::new(RoutingLookupState::default()));
        let routing_port = Arc::new(FlowAwareRoutingPort::new(
            state.clone(),
            Some(voicebot_destination()),
            None,
            None,
        ));
        let (mut session, _session_out_rx) = build_test_session(routing_port);
        session.ivr_flow_id = Some(flow_id);
        session.ivr_keypad_node_id = Some(keypad_node_id);

        session.handle_db_ivr_dtmf('1').await;

        let guard = state.lock().expect("routing state lock");
        assert_eq!(guard.old_dtmf_calls, 0);
        assert_eq!(guard.by_flow_dtmf, vec![(flow_id, "1".to_string())]);
    }

    #[tokio::test]
    async fn db_ivr_timeout_retry_uses_flow_lookup_after_retry_exhaustion() {
        let flow_id = Uuid::from_u128(0x303);
        let state = Arc::new(Mutex::new(RoutingLookupState::default()));
        let routing_port = Arc::new(FlowAwareRoutingPort::new(
            state.clone(),
            None,
            Some(voicebot_destination()),
            None,
        ));
        let (mut session, _session_out_rx) = build_test_session(routing_port);
        session.ivr_flow_id = Some(flow_id);
        session.ivr_max_retries = 0;

        session.handle_db_ivr_retry("TIMEOUT").await;

        let guard = state.lock().expect("routing state lock");
        assert_eq!(guard.old_timeout_calls, 0);
        assert_eq!(guard.by_flow_timeout, vec![flow_id]);
    }

    #[tokio::test]
    async fn db_ivr_invalid_retry_uses_flow_lookup_after_retry_exhaustion() {
        let flow_id = Uuid::from_u128(0x404);
        let state = Arc::new(Mutex::new(RoutingLookupState::default()));
        let routing_port = Arc::new(FlowAwareRoutingPort::new(
            state.clone(),
            None,
            None,
            Some(voicebot_destination()),
        ));
        let (mut session, _session_out_rx) = build_test_session(routing_port);
        session.ivr_flow_id = Some(flow_id);
        session.ivr_max_retries = 0;

        session.handle_db_ivr_retry("INVALID").await;

        let guard = state.lock().expect("routing state lock");
        assert_eq!(guard.old_invalid_calls, 0);
        assert_eq!(guard.by_flow_invalid, vec![flow_id]);
    }

    #[tokio::test]
    async fn b2bua_failed_in_ivr_mode_sends_bye_and_rtp_stop() {
        let routing_port = Arc::new(NoopRoutingPort::new());
        let (mut session, mut session_out_rx) = build_test_session(routing_port);
        session.outbound_mode = false;
        session.ivr_state = IvrState::Transferring;

        let _ = session
            .handle_control_event(
                SessState::Established,
                SessionControlIn::B2buaFailed {
                    reason: "transfer failed status 486".to_string(),
                    status: None,
                },
            )
            .await;

        let mut saw_bye = false;
        let mut saw_rtp_stop = false;
        while let Ok((_call_id, out)) = session_out_rx.try_recv() {
            match out {
                SessionOut::SipSendBye => saw_bye = true,
                SessionOut::RtpStopTx => saw_rtp_stop = true,
                _ => {}
            }
        }

        assert!(saw_bye, "IVR mode B2buaFailed should send SipSendBye");
        assert!(saw_rtp_stop, "IVR mode B2buaFailed should send RtpStopTx");
    }

    #[tokio::test]
    async fn b2bua_failed_in_outbound_mode_keeps_error_response_behavior() {
        let routing_port = Arc::new(NoopRoutingPort::new());
        let (mut session, mut session_out_rx) = build_test_session(routing_port);
        session.outbound_mode = true;

        let _ = session
            .handle_control_event(
                SessState::Established,
                SessionControlIn::B2buaFailed {
                    reason: "connection refused".to_string(),
                    status: Some(503),
                },
            )
            .await;

        let mut saw_error = false;
        let mut saw_bye = false;
        while let Ok((_call_id, out)) = session_out_rx.try_recv() {
            match out {
                SessionOut::SipSendError { code: 503, .. } => saw_error = true,
                SessionOut::SipSendBye => saw_bye = true,
                _ => {}
            }
        }

        assert!(saw_error, "outbound mode should keep SipSendError behavior");
        assert!(
            !saw_bye,
            "outbound mode should not send SipSendBye on B2buaFailed"
        );
        assert!(
            session.invite_rejected,
            "outbound mode should mark invite_rejected"
        );
    }
}
