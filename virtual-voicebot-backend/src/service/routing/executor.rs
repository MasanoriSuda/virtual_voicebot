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
        session.reset_action_modes();
        match action.action_code.as_str() {
            "VR" | "VB" => self.execute_vr(action, call_id, session).await,
            "BZ" => self.execute_bz(call_id, session).await,
            "NR" => self.execute_nr(call_id, session).await,
            "AN" => self.execute_an(action, call_id, session).await,
            "VM" => self.execute_vm(action, call_id, session).await,
            "IV" => self.execute_iv(action, call_id, session).await,
            unknown => {
                warn!(
                    "[ActionExecutor] call_id={} unknown ActionCode: {}, fallback=VR",
                    call_id, unknown
                );
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
            "[ActionExecutor] call_id={} executing VR (voicebot mode, recording_enabled={})",
            call_id, action.recording_enabled
        );
        session.set_outbound_mode(false);
        session.set_recording_enabled(action.recording_enabled);
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
        if let Some(announcement_id) = action.announcement_id {
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
        if let Some(announcement_id) = action.announcement_id {
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
