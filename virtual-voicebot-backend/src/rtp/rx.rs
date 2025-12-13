use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use log::{debug, warn};

use crate::rtp::parser::parse_rtp_packet;
use crate::rtp::payload::{classify_payload, PayloadKind};
use crate::rtp::rtcp::{is_rtcp_packet, RtcpEvent, RtcpEventTx};
use crate::session::{SessionIn, SessionMap};

/// transport 層から受け取る RTP 生パケット
#[derive(Debug, Clone)]
pub struct RawRtp {
    pub src: SocketAddr,
    pub dst_port: u16,
    pub data: Vec<u8>,
}

/// transport → rtp → session の受信経路ハンドラ。
/// 役割: 生パケットをパースし、call_id を引いて session へ MediaRtpIn を送る。
pub struct RtpReceiver {
    session_map: SessionMap,
    rtp_port_map: Arc<Mutex<HashMap<u16, String>>>,
    jitter: Arc<Mutex<HashMap<String, JitterState>>>,
    rtcp_tx: Option<RtcpEventTx>,
}

impl RtpReceiver {
    pub fn new(
        session_map: SessionMap,
        rtp_port_map: Arc<Mutex<HashMap<u16, String>>>,
        rtcp_tx: Option<RtcpEventTx>,
    ) -> Self {
        Self {
            session_map,
            rtp_port_map,
            jitter: Arc::new(Mutex::new(HashMap::new())),
            rtcp_tx,
        }
    }

    pub fn handle_raw(&self, raw: RawRtp) {
        // RTCP簡易判定
        if is_rtcp_packet(&raw.data) {
            warn!(
                "[rtcp recv] from {} len={} (dst_port={})",
                raw.src,
                raw.data.len(),
                raw.dst_port
            );
            if let Some(tx) = &self.rtcp_tx {
                let _ = tx.send(RtcpEvent {
                    raw: raw.data.clone(),
                    src: raw.src,
                    dst_port: raw.dst_port,
                });
            }
            return;
        }

        let call_id_opt = {
            let map = self.rtp_port_map.lock().unwrap();
            map.get(&raw.dst_port).cloned()
        };

        if let Some(call_id) = call_id_opt {
            // 対応するセッションを探して RTP入力イベントを投げる
            let sess_tx_opt = {
                let map = self.session_map.lock().unwrap();
                map.get(&call_id).cloned()
            };

            if let Some(sess_tx) = sess_tx_opt {
                match parse_rtp_packet(&raw.data) {
                    Ok(pkt) => {
                        match classify_payload(pkt.payload_type) {
                            Ok(PayloadKind::Pcmu) => {}
                            Err(err) => {
                                warn!(
                                    "[rtp recv] unsupported payload type {} from {} (call_id={})",
                                    err.0, raw.src, call_id
                                );
                                return;
                            }
                        }
                        if !self.should_accept(&call_id, pkt.sequence_number) {
                            warn!(
                                "[rtp recv] drop late/dup seq={} from {} (call_id={})",
                                pkt.sequence_number, raw.src, call_id
                            );
                            return;
                        }
                        debug!(
                            "[rtp recv] len={} from {} mapped to call_id={} pt={} seq={}",
                            raw.data.len(),
                            raw.src,
                            call_id,
                            pkt.payload_type,
                            pkt.sequence_number
                        );
                        let _ = sess_tx.send(SessionIn::MediaRtpIn {
                            ts: pkt.timestamp,
                            payload: pkt.payload,
                        });
                    }
                    Err(e) => {
                        warn!(
                            "[rtp recv] RTP parse error for call_id={} from {}: {:?}",
                            call_id, raw.src, e
                        );
                    }
                }
            } else {
                warn!(
                    "[rtp recv] RTP for unknown session (call_id={}), from {}",
                    call_id, raw.src
                );
            }
        } else {
            // 未登録ポート → いまはログだけ
            warn!(
                "[rtp recv] RTP on port {} without call_id mapping, from {}",
                raw.dst_port, raw.src
            );
        }
    }

    fn should_accept(&self, call_id: &str, seq: u16) -> bool {
        const MAX_REORDER: u16 = 50; // 遅延廃棄の簡易しきい値
        let mut map = self.jitter.lock().unwrap();
        let state = map.entry(call_id.to_string()).or_default();
        state.accept(seq, MAX_REORDER)
    }
}

#[derive(Default)]
struct JitterState {
    last_seq: Option<u16>,
}

impl JitterState {
    fn accept(&mut self, seq: u16, max_reorder: u16) -> bool {
        match self.last_seq {
            None => {
                self.last_seq = Some(seq);
                true
            }
            Some(last) => {
                if seq == last {
                    return false;
                }
                let diff_forward = seq.wrapping_sub(last);
                if diff_forward == 0 {
                    return false;
                }
                // 大きく逆行するものは廃棄（wrap-aroundは許容）
                if diff_forward > max_reorder && last.wrapping_sub(seq) < max_reorder {
                    return false;
                }
                self.last_seq = Some(seq);
                true
            }
        }
    }
}
