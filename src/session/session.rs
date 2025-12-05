#![allow(dead_code)]
// session.rs
use std::net::SocketAddr;

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::time::{Duration, Instant};

use crate::session::types::*;
use crate::session::types::Sdp;

use anyhow::Error;
use crate::rtp::{build_rtp_packet, RtpPacket};
use crate::bot;
use log::{debug, info};
use tokio::time::sleep;

#[derive(Clone)]
pub struct SessionHandle {
    pub tx_in: UnboundedSender<SessionIn>,
}

pub struct Session {
    state: SessState,
    call_id: String,
    peer_sdp: Option<Sdp>,
    local_sdp: Option<Sdp>,
    peer_rtp_override: Option<SocketAddr>,
    tx_up: UnboundedSender<SessionOut>,
    media_cfg: MediaConfig,
    // RTP送出用
    rtp_seq: u16,
    rtp_ts: u32,
    // バッファ/タイマ
    speaking: bool,
    capture_started: Option<Instant>,
    capture_payloads: Vec<u8>,
}

impl Session {
    pub fn new(
        call_id: String,
        tx_up: UnboundedSender<SessionOut>,
        media_cfg: MediaConfig,
    ) -> SessionHandle {
        let (tx_in, rx_in) = tokio::sync::mpsc::unbounded_channel();
        let mut s = Self {
            state: SessState::Idle,
            call_id,
            peer_sdp: None,
            local_sdp: None,
            peer_rtp_override: None,
            tx_up,
            media_cfg,
            rtp_seq: 0,
            rtp_ts: 0,
            speaking: false,
            capture_started: None,
            capture_payloads: Vec::new(),
        };
        tokio::spawn(async move { s.run(rx_in).await; });
        SessionHandle { tx_in }
    }

    async fn run(&mut self, mut rx: UnboundedReceiver<SessionIn>) {
        while let Some(ev) = rx.recv().await {
            match (self.state, ev) {
                (SessState::Idle, SessionIn::Invite { offer, .. }) => {
                    self.peer_sdp = Some(offer);
                    let answer = self.build_answer_pcmu8k();
                    self.local_sdp = Some(answer.clone());
                    let _ = self.tx_up.send(SessionOut::SendSip180);
                    let _ = self.tx_up.send(SessionOut::SendSip200 { answer });
                    self.state = SessState::Early;
                }
                (SessState::Early, SessionIn::Ack) => {
                    // 相手SDPからRTP宛先を確定して送信開始
                    let (ip, port) = self.peer_rtp_dst();
                    let _ = self.tx_up.send(SessionOut::StartRtpTx { dst_ip: ip.clone(), dst_port: port, pt: 0 }); // PCMU
                    self.state = SessState::Established;
                    self.capture_started = Some(Instant::now());
                    self.capture_payloads.clear();
                    // ダミーRTPを数フレーム送ってメディア経路を確認
                    if port != 0 && ip != "0.0.0.0" {
                        tokio::spawn(send_dummy_rtp((ip, port)));
                    }
                    // 例: 最初の発話をキック（固定文でOK）
                    let _ = self.tx_up.send(SessionOut::BotSynthesize { text: "はじめまして、ずんだもんです。".into() });
                }
                (SessState::Established, SessionIn::RtpIn { payload, src, .. }) => {
                    if self.peer_rtp_override.is_none() {
                        self.peer_rtp_override = Some(src);
                        info!(
                            "[session {}] learned RTP peer from incoming packet: {}",
                            self.call_id, src
                        );
                    }
                    debug!(
                        "[session {}] RTP payload received len={}",
                        self.call_id,
                        payload.len()
                    );
                    if let Some(start) = self.capture_started {
                        self.capture_payloads.extend_from_slice(&payload);
                        if start.elapsed() >= Duration::from_secs(10) {
                            info!(
                                "[session {}] Starting bot pipeline ({} bytes buffered)",
                                self.call_id,
                                self.capture_payloads.len()
                            );
                            if let Err(e) = self.handle_bot_pipeline().await {
                                log::warn!("bot pipeline error: {e:?}");
                            }
                            self.capture_started = None;
                            self.capture_payloads.clear();
                        }
                    }
                    let _ = self.tx_up.send(SessionOut::Metrics { name: "rtp_in", value: payload.len() as i64 });
                }
                (SessState::Established, SessionIn::BotAudio { pcm48k: _ }) => {
                    // 48k→8k→μ-law→RTPパケット化は下位メディア層に委譲してOK
                    // ここではTS/Seqの進行のみ示唆
                    self.rtp_ts = self.rtp_ts.wrapping_add(160);
                }
                (_, SessionIn::Bye) => {
                    let _ = self.tx_up.send(SessionOut::StopRtpTx);
                    let _ = self.tx_up.send(SessionOut::SendSipBye200);
                    self.state = SessState::Terminated;
                }
                (_, SessionIn::Abort(e)) => {
                    eprintln!("call {} abort: {e:?}", self.call_id);
                    let _ = self.tx_up.send(SessionOut::StopRtpTx);
                    self.state = SessState::Terminated;
                }
                _ => { /* それ以外は無視 or ログ */ }
            }
        }
    }

    fn build_answer_pcmu8k(&self) -> Sdp {
        // PCMU/8000 でローカル SDP を組み立て
        Sdp::pcmu(self.media_cfg.local_ip.clone(), self.media_cfg.local_port)
    }

    fn peer_rtp_dst(&self) -> (String, u16) {
        if let Some(addr) = self.peer_rtp_override {
            return (addr.ip().to_string(), addr.port());
        }
        if let Some(sdp) = &self.peer_sdp {
            (sdp.ip.clone(), sdp.port)
        } else {
            ("0.0.0.0".to_string(), 0)
        }
    }

    async fn handle_bot_pipeline(&self) -> Result<(), Error> {
        // 1) μ-law payload を WAV に保存
        let wav_path = "/tmp/input_from_peer.wav";
        write_mulaw_to_wav(&self.capture_payloads, wav_path)?;

        // 2) ASR+LLM+TTS (main.txt 由来の処理を bot モジュールに集約)
        let user_text = match bot::transcribe_and_log(wav_path).await {
            Ok(t) => t,
            Err(e) => {
                log::warn!("ASR failed: {e:?}");
                "すみません、聞き取れませんでした。".to_string()
            }
        };

        let bot_wav = match bot::handle_user_question_from_whisper(&user_text).await {
            Ok(p) => p,
            Err(e) => {
                log::warn!("LLM/TTS failed: {e:?}");
                return Ok(());
            }
        };

        // 5) RTP 送信
        let (dst_ip, dst_port) = self.peer_rtp_dst();
        if dst_ip != "0.0.0.0" && dst_port != 0 {
            if let Err(e) = send_wav_as_rtp_pcmu(&bot_wav, (&dst_ip, dst_port)).await {
                log::warn!("RTP send failed: {e:?}");
            }
        } else {
            log::warn!("RTP send skipped: unknown peer");
        }

        Ok(())
    }
}

fn write_mulaw_to_wav(payloads: &[u8], path: &str) -> Result<(), Error> {
    use hound::{SampleFormat, WavSpec, WavWriter};
    let spec = WavSpec {
        channels: 1,
        sample_rate: 8000,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut writer = WavWriter::create(path, spec)?;
    for &b in payloads {
        writer.write_sample(mulaw_to_linear16(b))?;
    }
    writer.finalize()?;
    Ok(())
}

fn mulaw_to_linear16(mu: u8) -> i16 {
    const BIAS: i16 = 0x84;
    let mu = !mu;
    let sign = (mu & 0x80) != 0;
    let segment = (mu & 0x70) >> 4;
    let mantissa = mu & 0x0F;

    let mut value = ((mantissa as i16) << 4) + 0x08;
    value <<= segment as i16;
    value -= BIAS;
    if sign { -value } else { value }
}

async fn send_wav_as_rtp_pcmu(
    wav_path: &str,
    dst: (&str, u16),
) -> Result<(), Error> {
    let frames = load_wav_as_pcmu_frames(wav_path)?;
    if frames.is_empty() {
        anyhow::bail!("no frames");
    }

    let socket = tokio::net::UdpSocket::bind("0.0.0.0:0").await?;
    let remote: SocketAddr = format!("{}:{}", dst.0, dst.1).parse()?;
    let mut seq = 0u16;
    let mut ts = 0u32;
    let ssrc = 0x12345678;

    log::info!(
        "[rtp tx] sending {} frames ({} samples) to {}",
        frames.len(),
        frames.len() * 160,
        remote
    );

    for frame in frames {
        let pkt = RtpPacket::new(0, seq, ts, ssrc, frame);
        let bytes = build_rtp_packet(&pkt);
        log::debug!(
            "[rtp tx] seq={} ts={} len={} first_bytes={:02x?}",
            seq,
            ts,
            bytes.len(),
            &bytes[..bytes.len().min(16)]
        );
        socket.send_to(&bytes, remote).await?;
        seq = seq.wrapping_add(1);
        ts = ts.wrapping_add(160);
        sleep(Duration::from_millis(20)).await;
    }
    Ok(())
}

/// ACK直後に疎通確認用のダミーRTPを少量送る
async fn send_dummy_rtp(dst: (String, u16)) {
    use tokio::net::UdpSocket;

    let (ip, port) = dst;
    let remote: SocketAddr = match format!("{}:{}", ip, port).parse() {
        Ok(a) => a,
        Err(e) => {
            log::warn!("[rtp tx] invalid dummy dst: {e:?}");
            return;
        }
    };

    let socket = match UdpSocket::bind("0.0.0.0:0").await {
        Ok(s) => s,
        Err(e) => {
            log::warn!("[rtp tx] failed to bind dummy socket: {e:?}");
            return;
        }
    };

    let payload = vec![0xFFu8; 160]; // μ-law静音相当
    let ssrc = 0x87654321;
    for i in 0..10u16 {
        let ts = i as u32 * 160;
        let pkt = RtpPacket::new(0, i, ts, ssrc, payload.clone());
        let bytes = build_rtp_packet(&pkt);
        if let Err(e) = socket.send_to(&bytes, remote).await {
            log::warn!("[rtp tx] dummy send failed: {e:?}");
            break;
        }
        log::info!("[rtp tx] dummy seq={} ts={} dst={}", i, ts, remote);
        sleep(Duration::from_millis(20)).await;
    }
}

fn load_wav_as_pcmu_frames(path: &str) -> Result<Vec<Vec<u8>>, Error> {
    use hound::WavReader;
    let mut reader = WavReader::open(path)?;
    let spec = reader.spec();
    if spec.channels != 1 || spec.bits_per_sample != 16 {
        anyhow::bail!("expected mono 16bit wav");
    }
    let mut samples: Vec<i16> = Vec::new();
    for s in reader.samples::<i16>() {
        samples.push(s?);
    }
    let base_samples: Vec<i16> = match spec.sample_rate {
        8000 => samples,
        24000 => samples.iter().step_by(3).copied().collect(),
        other => anyhow::bail!("unsupported sample rate {other}"),
    };
    let mut frames = Vec::new();
    let mut cur = Vec::with_capacity(160);
    for s in base_samples {
        cur.push(linear16_to_mulaw(s));
        if cur.len() == 160 {
            frames.push(cur.clone());
            cur.clear();
        }
    }
    if !cur.is_empty() {
        while cur.len() < 160 { cur.push(0xFF); }
        frames.push(cur);
    }
    Ok(frames)
}

fn linear16_to_mulaw(sample: i16) -> u8 {
    const BIAS: i16 = 0x84;
    const CLIP: i16 = 32635;
    let mut s = sample;
    let mut sign = 0u8;
    if s < 0 { s = -s; sign = 0x80; }
    if s > CLIP { s = CLIP; }
    s += BIAS;
    let mut segment: u8 = 0;
    let mut value = (s as u16) >> 7;
    while value > 0 {
        segment += 1;
        value >>= 1;
        if segment >= 8 { break; }
    }
    let mantissa = ((s >> (segment + 3)) & 0x0F) as u8;
    !(sign | (segment << 4) | mantissa)
}
