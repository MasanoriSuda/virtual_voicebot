use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use log::{debug, info, warn};
use tokio::net::UdpSocket;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::time::interval;

use crate::config::rtp_config;
use crate::rtp::codec::{codec_from_pt, decode_to_mulaw};
use crate::rtp::dtmf::DtmfDetector;
use crate::rtp::parser::parse_rtp_packet;
use crate::rtp::rtcp::{
    build_rr, is_rtcp_packet, parse_rtcp_packets, RtcpEvent, RtcpEventTx, RtcpPacket,
    RtcpReceiverReport, RtcpReportBlock,
};
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
    jitter: Arc<Mutex<HashMap<String, JitterBuffer>>>,
    dtmf: Arc<Mutex<HashMap<String, DtmfDetector>>>,
    jitter_max_reorder: u16,
    rtcp_tx: Option<RtcpEventTx>,
    rtcp_reporter: RtcpReporter,
}

impl RtpReceiver {
    pub fn new(
        session_map: SessionMap,
        rtp_port_map: Arc<Mutex<HashMap<u16, String>>>,
        rtcp_tx: Option<RtcpEventTx>,
    ) -> Self {
        let config = rtp_config();
        let rtcp_reporter = RtcpReporter::new(config.rtcp_interval);
        Self {
            session_map,
            rtp_port_map,
            jitter: Arc::new(Mutex::new(HashMap::new())),
            dtmf: Arc::new(Mutex::new(HashMap::new())),
            jitter_max_reorder: config.jitter_max_reorder,
            rtcp_tx,
            rtcp_reporter,
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
            let call_id_opt = raw
                .dst_port
                .checked_sub(1)
                .and_then(|rtp_port| self.rtp_port_map.lock().unwrap().get(&rtp_port).cloned());
            for pkt in parse_rtcp_packets(&raw.data) {
                info!("[rtcp recv] packet {:?}", pkt);
                if let (Some(call_id), RtcpPacket::SenderReport(sr)) = (&call_id_opt, &pkt) {
                    self.rtcp_reporter
                        .update_sr(call_id, sr.ssrc, sr.ntp_timestamp, raw.src);
                }
            }
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
                        self.rtcp_reporter.update_rtp(
                            &call_id,
                            pkt.ssrc,
                            pkt.sequence_number,
                            pkt.timestamp,
                            raw.src,
                            Instant::now(),
                        );
                        if let Err(err) = codec_from_pt(pkt.payload_type) {
                            warn!(
                                "[rtp recv] unsupported payload type {} from {} (call_id={})",
                                err.0, raw.src, call_id
                            );
                            return;
                        }
                        let frames = self.reorder(
                            &call_id,
                            RtpFrame {
                                seq: pkt.sequence_number,
                                ts: pkt.timestamp,
                                pt: pkt.payload_type,
                                payload: pkt.payload,
                            },
                        );
                        if frames.is_empty() {
                            warn!(
                                "[rtp recv] drop late/dup seq={} from {} (call_id={})",
                                pkt.sequence_number, raw.src, call_id
                            );
                            return;
                        }
                        for frame in frames {
                            let codec = match codec_from_pt(frame.pt) {
                                Ok(codec) => codec,
                                Err(err) => {
                                    warn!(
                                        "[rtp recv] unsupported payload type {} from {} (call_id={})",
                                        err.0, raw.src, call_id
                                    );
                                    continue;
                                }
                            };
                            debug!(
                                "[rtp recv] len={} from {} mapped to call_id={} pt={} seq={}",
                                frame.payload.len(),
                                raw.src,
                                call_id,
                                frame.pt,
                                frame.seq
                            );
                            let payload = decode_to_mulaw(codec, &frame.payload);
                            let digit = {
                                let mut map = self.dtmf.lock().unwrap();
                                let detector = map
                                    .entry(call_id.clone())
                                    .or_insert_with(DtmfDetector::new);
                                detector.ingest_mulaw(&payload)
                            };
                            if let Some(digit) = digit {
                                info!(
                                    "[rtp recv] dtmf detected call_id={} digit={}",
                                    call_id, digit
                                );
                                let _ = sess_tx.send(SessionIn::Dtmf { digit });
                            }
                            let _ = sess_tx.send(SessionIn::MediaRtpIn {
                                ts: frame.ts,
                                payload,
                            });
                        }
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

    fn reorder(&self, call_id: &str, frame: RtpFrame) -> Vec<RtpFrame> {
        let mut map = self.jitter.lock().unwrap();
        let buffer = map.entry(call_id.to_string()).or_default();
        buffer.push(frame, self.jitter_max_reorder)
    }
}

#[derive(Debug, Clone)]
struct RtpFrame {
    seq: u16,
    ts: u32,
    pt: u8,
    payload: Vec<u8>,
}

#[derive(Default)]
struct JitterBuffer {
    expected: Option<u16>,
    buffer: BTreeMap<u16, RtpFrame>,
}

impl JitterBuffer {
    fn push(&mut self, frame: RtpFrame, max_reorder: u16) -> Vec<RtpFrame> {
        let mut out = Vec::new();

        let expected = match self.expected {
            None => {
                self.expected = Some(frame.seq.wrapping_add(1));
                out.push(frame);
                return out;
            }
            Some(expected) => expected,
        };

        let diff = frame.seq.wrapping_sub(expected);
        if diff == 0 {
            out.push(frame);
            self.expected = Some(expected.wrapping_add(1));
        } else if diff < 0x8000 {
            if diff > max_reorder {
                self.buffer.clear();
                self.expected = Some(frame.seq.wrapping_add(1));
                out.push(frame);
            } else if !self.buffer.contains_key(&frame.seq) {
                self.buffer.insert(frame.seq, frame);
            }
        } else {
            // 古すぎる/重複は捨てる
        }

        if let Some(mut next) = self.expected {
            while let Some(frame) = self.buffer.remove(&next) {
                out.push(frame);
                next = next.wrapping_add(1);
            }
            self.expected = Some(next);
        }

        if self.buffer.len() > max_reorder as usize {
            if let Some((&next, _)) = self.buffer.iter().next() {
                self.expected = Some(next);
                let mut seq = next;
                while let Some(frame) = self.buffer.remove(&seq) {
                    out.push(frame);
                    seq = seq.wrapping_add(1);
                }
                self.expected = Some(seq);
            }
        }

        out
    }
}

struct RtcpReporter {
    tx: UnboundedSender<RtcpReportUpdate>,
}

enum RtcpReportUpdate {
    RtpPacket {
        call_id: String,
        ssrc: u32,
        seq: u16,
        rtp_ts: u32,
        peer: SocketAddr,
        arrival: Instant,
    },
    SenderReport {
        call_id: String,
        ssrc: u32,
        ntp_timestamp: u64,
        peer: SocketAddr,
        received_at: Instant,
    },
}

impl RtcpReporter {
    fn new(rtcp_interval: Duration) -> Self {
        let (tx, rx) = unbounded_channel();
        tokio::spawn(async move {
            run_rtcp_rr_loop(rx, rtcp_interval).await;
        });
        Self { tx }
    }

    fn update_rtp(
        &self,
        call_id: &str,
        ssrc: u32,
        seq: u16,
        rtp_ts: u32,
        peer: SocketAddr,
        arrival: Instant,
    ) {
        let _ = self.tx.send(RtcpReportUpdate::RtpPacket {
            call_id: call_id.to_string(),
            ssrc,
            seq,
            rtp_ts,
            peer,
            arrival,
        });
    }

    fn update_sr(&self, call_id: &str, ssrc: u32, ntp_timestamp: u64, peer: SocketAddr) {
        let _ = self.tx.send(RtcpReportUpdate::SenderReport {
            call_id: call_id.to_string(),
            ssrc,
            ntp_timestamp,
            peer,
            received_at: Instant::now(),
        });
    }
}

struct RtcpRxState {
    ssrc: u32,
    base_seq: u32,
    max_seq: u32,
    received: u32,
    expected_prior: u32,
    received_prior: u32,
    jitter: u32,
    last_arrival: Option<Instant>,
    last_rtp_ts: Option<u32>,
    peer: SocketAddr,
    last_sr_mid_ntp: Option<u32>,
    last_sr_at: Option<Instant>,
}

async fn run_rtcp_rr_loop(mut rx: UnboundedReceiver<RtcpReportUpdate>, rtcp_interval: Duration) {
    let mut states: HashMap<String, RtcpRxState> = HashMap::new();
    let local_ssrc = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0) as u32)
        ^ 0xA5A5_5A5A;
    let sock = match UdpSocket::bind("0.0.0.0:0").await {
        Ok(sock) => sock,
        Err(e) => {
            warn!("[rtcp rr] failed to bind socket: {e:?}");
            return;
        }
    };
    let mut tick = interval(rtcp_interval);

    loop {
        tokio::select! {
            Some(update) = rx.recv() => {
                match update {
                    RtcpReportUpdate::RtpPacket { call_id, ssrc, seq, rtp_ts, peer, arrival } => {
                        let state = states.entry(call_id).or_insert_with(|| RtcpRxState {
                            ssrc,
                            base_seq: seq as u32,
                            max_seq: seq as u32,
                            received: 0,
                            expected_prior: 0,
                            received_prior: 0,
                            jitter: 0,
                            last_arrival: None,
                            last_rtp_ts: None,
                            peer,
                            last_sr_mid_ntp: None,
                            last_sr_at: None,
                        });
                        if ssrc != state.ssrc {
                            let last_sr_mid_ntp = state.last_sr_mid_ntp.take();
                            let last_sr_at = state.last_sr_at.take();
                            *state = RtcpRxState {
                                ssrc,
                                base_seq: seq as u32,
                                max_seq: seq as u32,
                                received: 0,
                                expected_prior: 0,
                                received_prior: 0,
                                jitter: 0,
                                last_arrival: None,
                                last_rtp_ts: None,
                                peer,
                                last_sr_mid_ntp,
                                last_sr_at,
                            };
                        }
                        state.peer = peer;
                        state.received = state.received.saturating_add(1);
                        state.max_seq = extend_highest_seq(state.max_seq, seq);
                        update_jitter(state, arrival, rtp_ts);
                    }
                    RtcpReportUpdate::SenderReport { call_id, ssrc, ntp_timestamp, peer, received_at } => {
                        let state = states.entry(call_id).or_insert_with(|| RtcpRxState {
                            ssrc,
                            base_seq: 0,
                            max_seq: 0,
                            received: 0,
                            expected_prior: 0,
                            received_prior: 0,
                            jitter: 0,
                            last_arrival: None,
                            last_rtp_ts: None,
                            peer,
                            last_sr_mid_ntp: None,
                            last_sr_at: None,
                        });
                        state.ssrc = ssrc;
                        state.peer = peer;
                        state.last_sr_mid_ntp = Some(mid_ntp(ntp_timestamp));
                        state.last_sr_at = Some(received_at);
                    }
                }
            }
            _ = tick.tick() => {
                for state in states.values_mut() {
                    let expected = if state.received == 0 {
                        0
                    } else if state.max_seq >= state.base_seq {
                        state.max_seq - state.base_seq + 1
                    } else {
                        0
                    };
                    let expected_interval = expected.saturating_sub(state.expected_prior);
                    let received_interval = state.received.saturating_sub(state.received_prior);
                    let lost_interval = expected_interval as i64 - received_interval as i64;
                    let fraction_lost = if expected_interval == 0 {
                        0
                    } else {
                        let lost = lost_interval.max(0) as u64;
                        ((lost << 8) / expected_interval as u64) as u8
                    };
                    let cumulative_lost = clamp_loss(expected, state.received);
                    let (lsr, dlsr) = match (state.last_sr_mid_ntp, state.last_sr_at) {
                        (Some(lsr), Some(at)) => (lsr, dlsr_from(at)),
                        _ => (0, 0),
                    };
                    let report = RtcpReceiverReport {
                        ssrc: local_ssrc,
                        report: Some(RtcpReportBlock {
                            ssrc: state.ssrc,
                            fraction_lost,
                            cumulative_lost,
                            highest_seq: state.max_seq,
                            jitter: state.jitter,
                            lsr,
                            dlsr,
                        }),
                    };
                    let payload = build_rr(&report);
                    let dst = SocketAddr::new(state.peer.ip(), state.peer.port() + 1);
                    let _ = sock.send_to(&payload, dst).await;
                    state.expected_prior = expected;
                    state.received_prior = state.received;
                }
            }
            else => break,
        }
    }
}

fn extend_highest_seq(current: u32, seq: u16) -> u32 {
    let last_seq = (current & 0xFFFF) as u16;
    let mut cycles = current & 0xFFFF_0000;
    if seq < last_seq && last_seq.wrapping_sub(seq) > 0x8000 {
        cycles = cycles.wrapping_add(0x1_0000);
    }
    let extended = cycles | seq as u32;
    if extended > current {
        extended
    } else {
        current
    }
}

fn update_jitter(state: &mut RtcpRxState, arrival: Instant, rtp_ts: u32) {
    const RTP_CLOCK_RATE: u32 = 8_000;
    if let (Some(prev_arrival), Some(prev_rtp_ts)) = (state.last_arrival, state.last_rtp_ts) {
        if arrival >= prev_arrival {
            let arrival_delta = arrival.duration_since(prev_arrival);
            let arrival_units = duration_to_rtp_units(arrival_delta, RTP_CLOCK_RATE);
            let rtp_delta = rtp_ts.wrapping_sub(prev_rtp_ts);
            let d = arrival_units as i64 - rtp_delta as i64;
            let d_abs = d.abs() as u32;
            let jitter = state.jitter as i64 + (d_abs as i64 - state.jitter as i64) / 16;
            state.jitter = jitter.max(0) as u32;
        }
    }
    state.last_arrival = Some(arrival);
    state.last_rtp_ts = Some(rtp_ts);
}

fn duration_to_rtp_units(duration: Duration, clock_rate: u32) -> u32 {
    let secs = duration.as_secs().saturating_mul(clock_rate as u64);
    let frac = duration.subsec_nanos() as u64 * clock_rate as u64 / 1_000_000_000u64;
    (secs + frac) as u32
}

fn mid_ntp(ntp_timestamp: u64) -> u32 {
    ((ntp_timestamp >> 16) & 0xFFFF_FFFF) as u32
}

fn dlsr_from(at: Instant) -> u32 {
    let elapsed = at.elapsed();
    let secs = elapsed.as_secs().saturating_mul(65_536);
    let frac = elapsed.subsec_nanos() as u64 * 65_536 / 1_000_000_000u64;
    (secs + frac) as u32
}

fn clamp_loss(expected: u32, received: u32) -> u32 {
    let lost = expected as i64 - received as i64;
    if lost <= 0 {
        0
    } else if lost > 0x7F_FFFF {
        0x7F_FFFF
    } else {
        lost as u32
    }
}
