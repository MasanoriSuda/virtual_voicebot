use std::time::{Duration, Instant};

use tokio::sync::{mpsc::UnboundedSender, oneshot};

use super::types::SessionIn;

pub struct SessionTimers {
    keepalive_stop: Option<oneshot::Sender<()>>,
    session_timer_stop: Option<oneshot::Sender<()>>,
    session_timer_deadline: Option<Instant>,
    session_expires: Duration,
}

impl SessionTimers {
    pub fn new(session_expires: Duration) -> Self {
        Self {
            keepalive_stop: None,
            session_timer_stop: None,
            session_timer_deadline: None,
            session_expires,
        }
    }

    pub fn start_keepalive(&mut self, tx: UnboundedSender<SessionIn>, interval: Duration) {
        if self.keepalive_stop.is_some() {
            return;
        }
        let (stop_tx, mut stop_rx) = oneshot::channel();
        self.keepalive_stop = Some(stop_tx);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(interval) => {
                        let _ = tx.send(SessionIn::MediaTimerTick);
                    }
                    _ = &mut stop_rx => {
                        break;
                    }
                }
            }
        });
    }

    pub fn stop_keepalive(&mut self) {
        if let Some(stop) = self.keepalive_stop.take() {
            let _ = stop.send(());
        }
    }

    pub fn start_session_timer(&mut self, tx: UnboundedSender<SessionIn>) {
        if self.session_timer_stop.is_some() {
            return;
        }
        let (stop_tx, mut stop_rx) = oneshot::channel();
        let timeout = self.session_expires;
        self.session_timer_deadline = Some(Instant::now() + timeout);
        self.session_timer_stop = Some(stop_tx);
        tokio::spawn(async move {
            tokio::select! {
                _ = tokio::time::sleep(timeout) => {
                    let _ = tx.send(SessionIn::SessionTimerFired);
                }
                _ = &mut stop_rx => {}
            }
        });
    }

    pub fn stop_session_timer(&mut self) {
        if let Some(stop) = self.session_timer_stop.take() {
            let _ = stop.send(());
        }
        self.session_timer_deadline = None;
    }

    pub fn update_session_expires(&mut self, expires: Duration, tx: UnboundedSender<SessionIn>) {
        self.session_expires = expires;
        if self.session_timer_stop.is_some() {
            self.stop_session_timer();
            self.start_session_timer(tx);
        }
    }
}
