use chrono::{DateTime, Duration, Utc};

use crate::shared::entities::identifiers::{CallId, SessionId};
use crate::shared::entities::participant::Participant;
use crate::shared::entities::recording::RecordingRef;

#[derive(Debug, Clone)]
pub struct Call {
    id: CallId,
    session_id: SessionId,
    from: Participant,
    to: Participant,
    state: CallState,
    started_at: DateTime<Utc>,
    ended_at: Option<DateTime<Utc>>,
    recordings: Vec<RecordingRef>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CallState {
    Setup,
    Ringing,
    Active,
    Releasing,
    Ended(EndReason),
}

#[derive(Debug, Clone, PartialEq)]
pub enum EndReason {
    Normal,
    Cancelled,
    Rejected,
    Timeout,
    Error(String),
}

impl Call {
    pub fn new(id: CallId, from: Participant, to: Participant) -> Self {
        Self {
            id,
            session_id: SessionId::new(),
            from,
            to,
            state: CallState::Setup,
            started_at: Utc::now(),
            ended_at: None,
            recordings: Vec::new(),
        }
    }

    pub fn state(&self) -> &CallState {
        &self.state
    }

    pub fn transition(&mut self, to_state: CallState) -> Result<(), CallError> {
        match (&self.state, &to_state) {
            (CallState::Setup, CallState::Ringing) => Ok(()),
            (CallState::Setup, CallState::Active) => Ok(()),
            (CallState::Ringing, CallState::Active) => Ok(()),
            (CallState::Active, CallState::Releasing) => Ok(()),
            (CallState::Releasing, CallState::Ended(_)) => Ok(()),
            _ => Err(CallError::InvalidTransition {
                from: self.state.clone(),
                to: to_state.clone(),
            }),
        }?;
        self.state = to_state;
        Ok(())
    }

    pub fn duration(&self) -> Option<Duration> {
        self.ended_at.map(|e| e - self.started_at)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CallError {
    #[error("Invalid state transition from {from:?} to {to:?}")]
    InvalidTransition { from: CallState, to: CallState },
}
