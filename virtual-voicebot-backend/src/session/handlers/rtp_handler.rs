use std::net::SocketAddr;

use log::warn;

use super::super::SessionCoordinator;
use crate::session::types::SessionOut;

impl SessionCoordinator {
    pub(crate) fn peer_rtp_dst(&self) -> (String, u16) {
        if let Some(sdp) = &self.peer_sdp {
            (sdp.ip.clone(), sdp.port)
        } else {
            ("0.0.0.0".to_string(), 0)
        }
    }

    pub(crate) fn ensure_a_leg_rtp_started(&mut self) -> bool {
        if self.a_leg_rtp_started {
            return true;
        }
        let (ip, port) = self.peer_rtp_dst();
        let dst_addr: SocketAddr = match format!("{ip}:{port}").parse() {
            Ok(addr) => addr,
            Err(e) => {
                warn!(
                    "[session {}] invalid RTP destination {}:{} ({:?})",
                    self.call_id, ip, port, e
                );
                return false;
            }
        };
        self.rtp
            .start(self.call_id.to_string(), dst_addr, 0, 0x12345678, 0, 0);
        let _ = self.session_out_tx.send((
            self.call_id.clone(),
            SessionOut::RtpStartTx {
                dst_ip: ip,
                dst_port: port,
                pt: 0,
            },
        ));
        self.a_leg_rtp_started = true;
        true
    }

    pub(crate) fn align_rtp_clock(&mut self) {
        if let Some(last) = self.rtp_last_sent {
            let gap_samples = (last.elapsed().as_secs_f64() * 8000.0) as u32;
            self.rtp.adjust_timestamp(self.call_id.as_str(), gap_samples);
        }
    }

    pub(crate) fn peer_rtp_addr(&self) -> Option<SocketAddr> {
        let peer = self.peer_sdp.as_ref()?;
        format!("{}:{}", peer.ip, peer.port).parse().ok()
    }
}
