use chrono::{Local, Timelike};
use tokio::sync::oneshot;

use super::super::SessionCoordinator;
use crate::session::types::{IvrState, SessionControlIn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IvrAction {
    EnterVoicebot,
    PlaySendai,
    Transfer,
    ReplayMenu,
    Invalid,
}

pub(crate) fn ivr_action_for_digit(digit: char) -> IvrAction {
    match digit {
        '1' => IvrAction::EnterVoicebot,
        '2' => IvrAction::PlaySendai,
        '3' => IvrAction::Transfer,
        '9' => IvrAction::ReplayMenu,
        _ => IvrAction::Invalid,
    }
}

pub(crate) fn ivr_state_after_action(state: IvrState, action: IvrAction) -> IvrState {
    match (state, action) {
        (IvrState::IvrMenuWaiting, IvrAction::EnterVoicebot) => IvrState::VoicebotIntroPlaying,
        _ => state,
    }
}

pub(crate) fn intro_wav_path_for_hour(hour: u32) -> &'static str {
    match hour {
        5..=11 => super::super::INTRO_MORNING_WAV_PATH,
        12..=16 => super::super::INTRO_AFTERNOON_WAV_PATH,
        _ => super::super::INTRO_EVENING_WAV_PATH,
    }
}

pub(crate) fn get_intro_wav_path() -> &'static str {
    let hour = Local::now().hour();
    intro_wav_path_for_hour(hour)
}

impl SessionCoordinator {
    pub(crate) fn start_ivr_timeout(&mut self) {
        let timeout = self.runtime_cfg.ivr_timeout;
        let (stop_tx, mut stop_rx) = oneshot::channel();
        let tx = self.control_tx.clone();
        self.ivr_timeout_stop = Some(stop_tx);
        tokio::spawn(async move {
            tokio::select! {
                _ = tokio::time::sleep(timeout) => {
                    let _ = tx.send(SessionControlIn::IvrTimeout).await;
                }
                _ = &mut stop_rx => {}
            }
        });
    }

    pub(crate) fn reset_ivr_timeout(&mut self) {
        self.stop_ivr_timeout();
        self.start_ivr_timeout();
    }

    pub(crate) fn stop_ivr_timeout(&mut self) {
        if let Some(stop) = self.ivr_timeout_stop.take() {
            let _ = stop.send(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::types::IvrState;

    #[test]
    fn intro_path_matches_time_window() {
        assert_eq!(
            intro_wav_path_for_hour(5),
            crate::session::coordinator::INTRO_MORNING_WAV_PATH
        );
        assert_eq!(
            intro_wav_path_for_hour(11),
            crate::session::coordinator::INTRO_MORNING_WAV_PATH
        );
        assert_eq!(
            intro_wav_path_for_hour(12),
            crate::session::coordinator::INTRO_AFTERNOON_WAV_PATH
        );
        assert_eq!(
            intro_wav_path_for_hour(16),
            crate::session::coordinator::INTRO_AFTERNOON_WAV_PATH
        );
        assert_eq!(
            intro_wav_path_for_hour(17),
            crate::session::coordinator::INTRO_EVENING_WAV_PATH
        );
        assert_eq!(
            intro_wav_path_for_hour(4),
            crate::session::coordinator::INTRO_EVENING_WAV_PATH
        );
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
}
