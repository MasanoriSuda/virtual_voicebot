use crate::session::types::{next_session_state, SessState, SessionIn};

/// Pure session state machine: transitions only, no I/O.
pub struct SessionStateMachine {
    state: SessState,
}

impl SessionStateMachine {
    pub fn new() -> Self {
        Self {
            state: SessState::Idle,
        }
    }

    pub fn state(&self) -> SessState {
        self.state
    }

    pub fn next_state(&self, event: &SessionIn) -> SessState {
        next_session_state(self.state, event)
    }

    pub fn apply(&mut self, next: SessState) {
        self.state = next;
    }

    pub fn advance(&mut self, event: &SessionIn) -> SessState {
        let next = self.next_state(event);
        self.state = next;
        next
    }
}
