use crate::session::types::{next_session_state, SessState, SessionIn};

#[derive(Debug, Clone, Copy)]
pub enum SessionEvent<'a> {
    Input(&'a SessionIn),
}

impl<'a> From<&'a SessionIn> for SessionEvent<'a> {
    fn from(value: &'a SessionIn) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_event_emits_transition() {
        let sm = SessionStateMachine::new();
        let event = SessionIn::SipInvite {
            call_id: "call".to_string(),
            from: "from".to_string(),
            to: "to".to_string(),
            offer: super::super::types::Sdp::pcmu("127.0.0.1", 10000),
            session_timer: None,
        };
        let commands = sm.process_event(SessionEvent::from(&event));
        assert_eq!(commands, vec![SessionCommand::Transition(SessState::Early)]);
    }
}
