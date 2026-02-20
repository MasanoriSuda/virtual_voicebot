use log::{debug, info, warn};
use tokio::sync::oneshot;
use tokio::time::{interval, MissedTickBehavior};

use super::super::SessionCoordinator;
use crate::protocol::session::types::{IvrState, SessionControlIn};

impl SessionCoordinator {
    pub(crate) fn cancel_transfer(&mut self) {
        if let Some(cancel) = self.transfer_cancel.take() {
            let _ = cancel.send(());
        }
        self.stop_transfer_announce();
    }

    pub(crate) fn start_transfer_announce(&mut self) {
        self.stop_transfer_announce();
        let (stop_tx, mut stop_rx) = oneshot::channel();
        let tx = self.control_tx.clone();
        self.transfer_announce_stop = Some(stop_tx);
        tokio::spawn(async move {
            let mut tick = interval(super::super::TRANSFER_ANNOUNCE_INTERVAL);
            tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
            tick.tick().await;
            loop {
                tokio::select! {
                    _ = tick.tick() => {
                        let _ = tx.send(SessionControlIn::TransferAnnounce).await;
                    }
                    _ = &mut stop_rx => {
                        break;
                    }
                }
            }
        });
    }

    pub(crate) fn stop_transfer_announce(&mut self) {
        if let Some(stop) = self.transfer_announce_stop.take() {
            let _ = stop.send(());
        }
    }

    pub(crate) async fn shutdown_b_leg(&mut self, send_bye: bool) {
        if let Some(mut b_leg) = self.b_leg.take() {
            info!(
                "[session {}] shutting down B-leg (send_bye={})",
                self.call_id, send_bye
            );
            if send_bye {
                match b_leg.send_bye().await {
                    Ok(()) => {
                        info!("[session {}] B-leg BYE enqueued successfully", self.call_id);
                    }
                    Err(e) => {
                        warn!(
                            "[session {}] B-leg BYE enqueue failed: {:?}",
                            self.call_id, e
                        );
                    }
                }
            }
            b_leg.shutdown();
            self.rtp.stop(&b_leg.rtp_key);
        } else if self.ivr_state == IvrState::B2buaMode {
            warn!(
                "[session {}] shutdown_b_leg called but b_leg is None (send_bye={}) in B2buaMode",
                self.call_id, send_bye
            );
        } else {
            debug!(
                "[session {}] shutdown_b_leg called but b_leg is None (send_bye={})",
                self.call_id, send_bye
            );
        }
    }
}
