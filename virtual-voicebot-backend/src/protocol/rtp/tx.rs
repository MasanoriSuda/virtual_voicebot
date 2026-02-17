use std::net::SocketAddr;

use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::time::{interval, MissedTickBehavior};

use crate::protocol::rtp::codec::{codec_from_pt, encode_from_mulaw};
use crate::protocol::rtp::rtcp::{build_sr, ntp_timestamp_now, RtcpSenderReport};
use crate::protocol::rtp::stream_manager::StreamManager;
use crate::protocol::rtp::{build_rtp_packet, RtpPacket};
use crate::shared::config::RtpConfig;

#[derive(Debug)]
pub enum RtpTxCommand {
    Start {
        key: String,
        dst: SocketAddr,
        pt: u8,
        ssrc: u32,
        seq: u16,
        ts: u32,
    },
    Stop {
        key: String,
    },
    SendPayload {
        key: String,
        payload: Vec<u8>,
    },
    AdjustTimestamp {
        key: String,
        delta: u32,
    },
}

#[derive(Clone)]
pub struct RtpTxHandle {
    tx: mpsc::Sender<RtpTxCommand>,
}

impl RtpTxHandle {
    pub fn new(rtp_cfg: RtpConfig) -> Self {
        const RTP_TX_CHANNEL_CAPACITY: usize = 256;
        let (tx, rx) = mpsc::channel(RTP_TX_CHANNEL_CAPACITY);
        let streams = StreamManager::new();
        tokio::spawn(async move { run_tx(streams, rx, rtp_cfg.rtcp_interval).await });
        Self { tx }
    }

    pub fn start(&self, key: String, dst: SocketAddr, pt: u8, ssrc: u32, seq: u16, ts: u32) {
        if let Err(err) = self.tx.try_send(RtpTxCommand::Start {
            key,
            dst,
            pt,
            ssrc,
            seq,
            ts,
        }) {
            log::warn!("[rtp tx] drop Start command (channel full): {:?}", err);
        }
    }

    pub fn stop(&self, key: &str) {
        if let Err(err) = self.tx.try_send(RtpTxCommand::Stop {
            key: key.to_string(),
        }) {
            log::warn!("[rtp tx] drop Stop command (channel full): {:?}", err);
        }
    }

    pub fn send_payload(&self, key: &str, payload: Vec<u8>) {
        let _ = self.tx.try_send(RtpTxCommand::SendPayload {
            key: key.to_string(),
            payload,
        });
    }

    pub fn adjust_timestamp(&self, key: &str, delta: u32) {
        if delta == 0 {
            return;
        }
        let _ = self.tx.try_send(RtpTxCommand::AdjustTimestamp {
            key: key.to_string(),
            delta,
        });
    }
}

async fn run_tx(
    streams: StreamManager,
    mut rx: mpsc::Receiver<RtpTxCommand>,
    rtcp_interval: std::time::Duration,
) {
    let mut sock: Option<UdpSocket> = None;
    let mut rtcp_tick = interval(rtcp_interval);
    rtcp_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            cmd = rx.recv() => {
                let Some(cmd) = cmd else { break; };
                match cmd {
                    RtpTxCommand::Start {
                        key,
                        dst,
                        pt,
                        ssrc,
                        seq,
                        ts,
                    } => {
                        if codec_from_pt(pt).is_err() {
                            log::warn!("[rtp tx] unsupported payload type {}", pt);
                            continue;
                        }
                        streams.upsert(key, dst, pt, ssrc, seq, ts).await;
                        if sock.is_none() {
                            match UdpSocket::bind("0.0.0.0:0").await {
                                Ok(s) => sock = Some(s),
                                Err(e) => {
                                    log::warn!("[rtp tx] failed to bind RTP socket: {e:?}");
                                }
                            }
                        }
                    }
                    RtpTxCommand::Stop { key } => {
                        streams.remove(&key).await;
                        if streams.is_empty().await {
                            sock = None;
                        }
                    }
                    RtpTxCommand::SendPayload { key, payload } => {
                        if let Some(s) = sock.as_ref() {
                            let sent = streams
                                .with_mut(&key, |stream| {
                                    let codec = match codec_from_pt(stream.pt) {
                                        Ok(codec) => codec,
                                        Err(err) => {
                                            log::warn!("[rtp tx] unsupported payload type {}", err.0);
                                            return None;
                                        }
                                    };
                                    let pcm_len = payload.len() as u32;
                                    let encoded = encode_from_mulaw(codec, &payload);
                                    let pkt = RtpPacket::new(
                                        stream.pt,
                                        stream.seq,
                                        stream.ts,
                                        stream.ssrc,
                                        encoded,
                                    );
                                    let bytes = build_rtp_packet(&pkt);
                                    stream.packet_count = stream.packet_count.saturating_add(1);
                                    stream.octet_count = stream.octet_count.saturating_add(pcm_len);
                                    stream.last_rtp_ts = stream.ts;
                                    // 送信後に進める
                                    stream.seq = stream.seq.wrapping_add(1);
                                    stream.ts = stream.ts.wrapping_add(pcm_len);
                                    Some((stream.dst, bytes))
                                })
                                .await;
                            if let Some((dst, bytes)) = sent.flatten() {
                                let _ = s.send_to(&bytes, dst).await.ok();
                            } else {
                                log::warn!("[rtp tx] send requested but stream key not found");
                            }
                        }
                    }
                    RtpTxCommand::AdjustTimestamp { key, delta } => {
                        let _ = streams
                            .with_mut(&key, |stream| {
                                stream.ts = stream.ts.wrapping_add(delta);
                            })
                            .await;
                    }
                }
            }
            _ = rtcp_tick.tick() => {
                if let Some(s) = sock.as_ref() {
                    let list = streams.list().await;
                    for (_, stream) in list {
                        let report = RtcpSenderReport {
                            ssrc: stream.ssrc,
                            ntp_timestamp: ntp_timestamp_now(),
                            rtp_timestamp: stream.last_rtp_ts,
                            packet_count: stream.packet_count,
                            octet_count: stream.octet_count,
                        };
                        let payload = build_sr(&report);
                        let dst = SocketAddr::new(stream.dst.ip(), stream.dst.port() + 1);
                        let _ = s.send_to(&payload, dst).await;
                    }
                }
            }
        }
    }
}
