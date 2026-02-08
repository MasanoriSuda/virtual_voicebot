use anyhow::Result;
use log::{info, warn};

use super::ActionConfig;
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
        match action.action_code.as_str() {
            "VR" => self.execute_vr(action, call_id, session).await,
            "IV" => self.execute_iv(action, call_id, session).await,
            unknown => {
                warn!(
                    "[ActionExecutor] call_id={} action_code={} not supported in phase-2, fallback=VR",
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

    async fn execute_iv(
        &self,
        action: &ActionConfig,
        call_id: &str,
        session: &mut SessionCoordinator,
    ) -> Result<()> {
        if let Some(ivr_flow_id) = action.ivr_flow_id {
            warn!(
                "[ActionExecutor] call_id={} ivr_flow_id={} specified but IV execution is phase-4",
                call_id, ivr_flow_id
            );
        } else {
            warn!(
                "[ActionExecutor] call_id={} action_code=IV without ivr_flow_id, fallback=VR",
                call_id
            );
        }
        self.execute_vr(action, call_id, session).await
    }
}
