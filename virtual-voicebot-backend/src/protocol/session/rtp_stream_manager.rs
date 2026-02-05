use std::net::SocketAddr;

use crate::protocol::rtp::tx::RtpTxHandle;

#[derive(Clone)]
pub struct RtpStreamManager {
    rtp_tx: RtpTxHandle,
}

impl RtpStreamManager {
    pub fn new(rtp_tx: RtpTxHandle) -> Self {
        Self { rtp_tx }
    }

    pub fn start(&self, key: String, dst: SocketAddr, pt: u8, ssrc: u32, seq: u16, ts: u32) {
        self.rtp_tx.start(key, dst, pt, ssrc, seq, ts);
    }

    pub fn stop(&self, key: &str) {
        self.rtp_tx.stop(key);
    }

    pub fn send_payload(&self, key: &str, payload: Vec<u8>) {
        self.rtp_tx.send_payload(key, payload);
    }

    pub fn adjust_timestamp(&self, key: &str, delta: u32) {
        self.rtp_tx.adjust_timestamp(key, delta);
    }
}
