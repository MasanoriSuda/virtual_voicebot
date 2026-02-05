use tokio::time::Duration;

use super::super::SessionCoordinator;
use crate::session::types::{SessionRefresher, SessionTimerInfo};

impl SessionCoordinator {
    pub(crate) fn start_keepalive_timer(&mut self) {
        self.timers
            .start_keepalive(self.control_tx.clone(), super::super::KEEPALIVE_INTERVAL);
    }

    pub(crate) fn stop_keepalive_timer(&mut self) {
        self.timers.stop_keepalive();
    }

    pub(crate) fn start_session_timer_if_needed(&mut self) {
        let Some(expires) = self.session_expires else {
            return;
        };
        if expires.is_zero() {
            return;
        }
        let refresh_after = self.refresh_after(expires);
        self.timers
            .start_session_timer(self.control_tx.clone(), expires, refresh_after);
    }

    pub(crate) fn stop_session_timer(&mut self) {
        self.timers.stop_session_timer();
    }

    pub(crate) fn update_session_expires(&mut self, timer: SessionTimerInfo) {
        self.session_expires = Some(timer.expires);
        self.session_refresher = Some(timer.refresher);
        let refresh_after = self.refresh_after(timer.expires);
        self.timers
            .update_session_expires(timer.expires, self.control_tx.clone(), refresh_after);
    }

    pub(crate) fn refresh_after(&self, expires: Duration) -> Option<Duration> {
        if self.session_refresher != Some(SessionRefresher::Uas) {
            return None;
        }
        let total_ms = expires.as_millis();
        if total_ms == 0 {
            return None;
        }
        let refresh_ms = total_ms.saturating_mul(8) / 10;
        if refresh_ms == 0 {
            return None;
        }
        let refresh_ms = std::cmp::min(refresh_ms, u64::MAX as u128) as u64;
        Some(Duration::from_millis(refresh_ms))
    }

    pub(crate) fn start_ring_delay(&mut self, duration: Duration) {
        if let Some(cancel) = self.ring_delay_cancel.take() {
            let _ = cancel.send(());
        }
        let tx = self.control_tx.clone();
        let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel();
        self.ring_delay_cancel = Some(cancel_tx);
        tokio::spawn(async move {
            tokio::select! {
                _ = tokio::time::sleep(duration) => {
                    let _ = tx.send(crate::session::types::SessionControlIn::RingDurationElapsed).await;
                }
                _ = &mut cancel_rx => {}
            }
        });
    }

    pub(crate) fn stop_ring_delay(&mut self) {
        if let Some(cancel) = self.ring_delay_cancel.take() {
            let _ = cancel.send(());
        }
        self.pending_answer = None;
    }
}
