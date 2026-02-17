use crate::protocol::session::types::{next_session_state, SessState, SessionControlIn};

#[derive(Debug, Clone, Copy)]
pub enum SessionEvent<'a> {
    Input(&'a SessionControlIn),
}

impl<'a> From<&'a SessionControlIn> for SessionEvent<'a> {
    fn from(value: &'a SessionControlIn) -> Self {
        SessionEvent::Input(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionCommand {
    Transition(SessState),
}

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

    pub fn process_event(&self, event: SessionEvent<'_>) -> Vec<SessionCommand> {
        let next = match event {
            SessionEvent::Input(input) => next_session_state(self.state, input),
        };
        if next == self.state {
            Vec::new()
        } else {
            vec![SessionCommand::Transition(next)]
        }
    }

    pub fn apply_commands(&mut self, commands: &[SessionCommand]) {
        for command in commands {
            let SessionCommand::Transition(next) = command;
            self.state = *next;
        }
    }
}

impl Default for SessionStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::session::types::CallId;

    #[test]
    fn process_event_emits_transition() {
        let sm = SessionStateMachine::new();
        let event = SessionControlIn::SipInvite {
            call_id: CallId::new("call".to_string()).expect("valid test call id"),
            from: "from".to_string(),
            to: "to".to_string(),
            offer: super::super::types::Sdp::pcmu("127.0.0.1", 10000),
            session_timer: None,
        };
        let commands = sm.process_event(SessionEvent::from(&event));
        assert_eq!(commands, vec![SessionCommand::Transition(SessState::Early)]);
    }
}
