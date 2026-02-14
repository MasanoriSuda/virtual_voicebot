use anyhow::Result;
use log::{info, warn};

use super::ActionConfig;
use crate::protocol::session::types::IvrState;
use crate::protocol::session::SessionCoordinator;

pub struct ActionExecutor;

impl ActionExecutor {
    pub fn new() -> Self {
        Self
    }

    pub async fn execute(
        &self,
        action: &ActionConfig,
        call_id: &str,
        session: &mut SessionCoordinator,
    ) -> Result<()> {
        info!(
            "[ActionExecutor] call_id={} action_code={} recording_enabled={} announce_enabled={}",
            call_id, action.action_code, action.recording_enabled, action.announce_enabled
        );
        let action_code = action.action_code.trim().to_ascii_uppercase();
        if action.caller_category != "unknown" || session.caller_category_is_unknown() {
            session.set_caller_category(action.caller_category.as_str());
        }
        session.reset_action_modes();
        match action_code.as_str() {
            "VR" => {
                session.register_action_for_call_log("VR");
                self.execute_vr(action, call_id, session).await
            }
            "VB" => {
                session.register_action_for_call_log("VB");
                self.execute_vb(action, call_id, session).await
            }
            "BZ" => {
                session.register_action_for_call_log("BZ");
                self.execute_bz(call_id, session).await
            }
            "NR" => {
                session.register_action_for_call_log("NR");
                self.execute_nr(call_id, session).await
            }
            "AN" => {
                session.register_action_for_call_log("AN");
                self.execute_an(action, call_id, session).await
            }
            "VM" => {
                session.register_action_for_call_log("VM");
                self.execute_vm(action, call_id, session).await
            }
            "IV" => {
                session.register_action_for_call_log("IV");
                self.execute_iv(action, call_id, session).await
            }
            unknown => {
                warn!(
                    "[ActionExecutor] call_id={} unknown ActionCode: {}, fallback=VR",
                    call_id, unknown
                );
                session.register_action_for_call_log("VR");
                self.execute_vr(action, call_id, session).await
            }
        }
    }

    async fn execute_vr(
        &self,
        action: &ActionConfig,
        call_id: &str,
        session: &mut SessionCoordinator,
    ) -> Result<()> {
        info!(
            "[ActionExecutor] call_id={} executing VR (B2BUA transfer mode, recording_enabled={})",
            call_id, action.recording_enabled
        );
        session.set_outbound_mode(false);
        session.set_recording_enabled(action.recording_enabled);
        session.set_transfer_after_answer_pending(!action.announce_enabled);
        if action.announce_enabled {
            let recording_announcement_id =
                action.recording_announcement_id.or(action.announcement_id);
            // Route through announce flow so recording notice is handled
            // separately from legacy IVR intro playback.
            session.set_announce_mode(true);
            session.set_recording_notice_pending(true);
            if let Some(announcement_id) = recording_announcement_id {
                session.set_announcement_id(announcement_id);
                info!(
                    "[ActionExecutor] call_id={} recording_announcement_id={}",
                    call_id, announcement_id
                );
            } else {
                info!(
                    "[ActionExecutor] call_id={} recording notice uses fallback audio",
                    call_id
                );
            }
        } else {
            info!(
                "[ActionExecutor] call_id={} transfer will start immediately after answer",
                call_id
            );
        }
        Ok(())
    }

    async fn execute_vb(
        &self,
        action: &ActionConfig,
        call_id: &str,
        session: &mut SessionCoordinator,
    ) -> Result<()> {
        info!(
            "[ActionExecutor] call_id={} executing VB (voicebot mode, recording_enabled=false)",
            call_id
        );
        session.set_outbound_mode(false);
        session.set_voicebot_direct_mode(true);
        session.set_recording_enabled(false);
        if action.announce_enabled {
            // VB should remain on voicebot path; prepend notice in legacy IVR path
            // and then continue to voicebot mode.
            session.set_recording_notice_pending(true);
            if let Some(announcement_id) = action.announcement_id {
                session.set_announcement_id(announcement_id);
                info!(
                    "[ActionExecutor] call_id={} welcome_announcement_id={}",
                    call_id, announcement_id
                );
            } else {
                info!(
                    "[ActionExecutor] call_id={} welcome announcement uses fallback audio",
                    call_id
                );
            }
        }
        Ok(())
    }

    async fn execute_bz(&self, call_id: &str, session: &mut SessionCoordinator) -> Result<()> {
        info!("[ActionExecutor] call_id={} executing BZ (busy)", call_id);
        session.set_invite_rejected(true);
        session.send_sip_error(486, "Busy Here").await?;
        info!("[ActionExecutor] call_id={} sent 486 Busy Here", call_id);
        Ok(())
    }

    async fn execute_nr(&self, call_id: &str, session: &mut SessionCoordinator) -> Result<()> {
        info!(
            "[ActionExecutor] call_id={} executing NR (no response)",
            call_id
        );
        session.set_no_response_mode(true);
        session.set_outbound_mode(false);
        Ok(())
    }

    async fn execute_an(
        &self,
        action: &ActionConfig,
        call_id: &str,
        session: &mut SessionCoordinator,
    ) -> Result<()> {
        info!(
            "[ActionExecutor] call_id={} executing AN (announcement)",
            call_id
        );
        session.set_outbound_mode(false);
        session.set_recording_enabled(false);
        session.set_announce_mode(true);
        session.set_voicemail_mode(false);
        if let Some(audio_file_url) = action.announcement_audio_file_url.as_ref() {
            session.set_announcement_audio_file_url(audio_file_url.clone());
            info!(
                "[ActionExecutor] call_id={} announcement_audio_file_url={}",
                call_id, audio_file_url
            );
        } else if let Some(announcement_id) = action.announcement_id {
            session.set_announcement_id(announcement_id);
            info!(
                "[ActionExecutor] call_id={} announcement_id={}",
                call_id, announcement_id
            );
        } else {
            warn!(
                "[ActionExecutor] call_id={} action_code=AN without announcement_id (fallback audio)",
                call_id
            );
        }
        Ok(())
    }

    async fn execute_vm(
        &self,
        action: &ActionConfig,
        call_id: &str,
        session: &mut SessionCoordinator,
    ) -> Result<()> {
        info!(
            "[ActionExecutor] call_id={} executing VM (voicemail)",
            call_id
        );
        session.set_outbound_mode(false);
        session.set_recording_enabled(true);
        session.set_announce_mode(true);
        session.set_voicemail_mode(true);
        if let Some(audio_file_url) = action.announcement_audio_file_url.as_ref() {
            session.set_announcement_audio_file_url(audio_file_url.clone());
            info!(
                "[ActionExecutor] call_id={} voicemail_announcement_audio_file_url={}",
                call_id, audio_file_url
            );
        } else if let Some(announcement_id) = action.announcement_id {
            session.set_announcement_id(announcement_id);
            info!(
                "[ActionExecutor] call_id={} voicemail_announcement_id={}",
                call_id, announcement_id
            );
        } else {
            warn!(
                "[ActionExecutor] call_id={} action_code=VM without announcement_id (fallback audio)",
                call_id
            );
        }
        Ok(())
    }

    async fn execute_iv(
        &self,
        action: &ActionConfig,
        call_id: &str,
        session: &mut SessionCoordinator,
    ) -> Result<()> {
        if let Some(ivr_flow_id) = action.ivr_flow_id {
            info!(
                "[ActionExecutor] call_id={} executing IV (ivr flow), ivr_flow_id={}",
                call_id, ivr_flow_id
            );
            session.set_outbound_mode(false);
            session.set_ivr_flow_id(ivr_flow_id);
            session.set_ivr_state(IvrState::IvrMenuWaiting);
            session.set_recording_enabled(action.recording_enabled);
            return Ok(());
        } else {
            warn!(
                "[ActionExecutor] call_id={} action_code=IV without ivr_flow_id, fallback=VR",
                call_id
            );
        }
        self.execute_vr(action, call_id, session).await
    }
}
