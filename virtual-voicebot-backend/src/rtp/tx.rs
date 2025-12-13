use std::net::SocketAddr;

use tokio::net::UdpSocket;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use crate::rtp::payload::{classify_payload, PayloadKind};
use crate::rtp::stream_manager::StreamManager;
use crate::rtp::{build_rtp_packet, RtpPacket};

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
    tx: UnboundedSender<RtpTxCommand>,
}

impl RtpTxHandle {
    pub fn new() -> Self {
        let (tx, rx) = unbounded_channel();
        let streams = StreamManager::new();
        tokio::spawn(async move { run_tx(streams, rx).await });
        Self { tx }
    }

    pub fn start(&self, key: String, dst: SocketAddr, pt: u8, ssrc: u32, seq: u16, ts: u32) {
        let _ = self.tx.send(RtpTxCommand::Start {
            key,
            dst,
            pt,
            ssrc,
            seq,
            ts,
        });
    }

    pub fn stop(&self, key: &str) {
        let _ = self.tx.send(RtpTxCommand::Stop {
            key: key.to_string(),
        });
    }

    pub fn send_payload(&self, key: &str, payload: Vec<u8>) {
        let _ = self.tx.send(RtpTxCommand::SendPayload {
            key: key.to_string(),
            payload,
        });
    }

    pub fn adjust_timestamp(&self, key: &str, delta: u32) {
        if delta == 0 {
            return;
        }
        let _ = self.tx.send(RtpTxCommand::AdjustTimestamp {
            key: key.to_string(),
            delta,
        });
    }
}

async fn run_tx(streams: StreamManager, mut rx: UnboundedReceiver<RtpTxCommand>) {
    let mut sock: Option<UdpSocket> = None;

    while let Some(cmd) = rx.recv().await {
        match cmd {
            RtpTxCommand::Start {
                key,
                dst,
                pt,
                ssrc,
                seq,
                ts,
            } => {
                if classify_payload(pt) != Ok(PayloadKind::Pcmu) {
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
                            let pkt = RtpPacket::new(
                                stream.pt,
                                stream.seq,
                                stream.ts,
                                stream.ssrc,
                                payload.clone(),
                            );
                            let bytes = build_rtp_packet(&pkt);
                            // 送信後に進める
                            stream.seq = stream.seq.wrapping_add(1);
                            stream.ts = stream.ts.wrapping_add(payload.len() as u32);
                            (stream.dst, bytes)
                        })
                        .await;
                    if let Some((dst, bytes)) = sent {
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
}
