#![allow(dead_code)]
// session.rs
use std::net::SocketAddr;

use tokio::net::UdpSocket;
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    oneshot,
};
use tokio::time::{Duration, Instant};

use crate::session::types::Sdp;
use crate::session::types::*;

use crate::ai;
use crate::rtp::{build_rtp_packet, RtpPacket};
use anyhow::Error;
use log::{debug, info, warn};

const INTRO_WAV_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/test/simpletest/audio/zundamon_intro.wav"
);

#[derive(Clone)]
pub struct SessionHandle {
    pub tx_in: UnboundedSender<SessionIn>,
}

pub struct Session {
    state: SessState,
    call_id: String,
    peer_sdp: Option<Sdp>,
    local_sdp: Option<Sdp>,
    tx_up: UnboundedSender<SessionOut>,
    tx_in: UnboundedSender<SessionIn>,
    media_cfg: MediaConfig,
    // RTP送出用
    rtp_seq: u16,
    rtp_ts: u32,
    rtp_ssrc: u32,
    rtp_last_sent: Option<Instant>,
    rtp_socket: Option<UdpSocket>,
    keepalive_stop: Option<oneshot::Sender<()>>,
    sending_audio: bool,
    // バッファ/タイマ
    speaking: bool,
    capture_started: Option<Instant>,
    capture_payloads: Vec<u8>,
    intro_sent: bool,
}

impl Session {
    pub fn spawn(
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
            tx_up,
            tx_in: tx_in.clone(),
            media_cfg,
            rtp_seq: 0,
            rtp_ts: 0,
            rtp_ssrc: 0x12345678,
            rtp_last_sent: None,
            rtp_socket: None,
            keepalive_stop: None,
            sending_audio: false,
            speaking: false,
            capture_started: None,
            capture_payloads: Vec::new(),
            intro_sent: false,
        };
        tokio::spawn(async move {
            s.run(rx_in).await;
        });
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
                    if self.intro_sent {
                        continue;
                    }

                    let (ip, port) = self.peer_rtp_dst();
                    if self.rtp_socket.is_none() {
                        match UdpSocket::bind("0.0.0.0:0").await {
                            Ok(sock) => self.rtp_socket = Some(sock),
                            Err(e) => {
                                warn!(
                                    "[session {}] failed to bind RTP socket: {:?}",
                                    self.call_id, e
                                );
                                continue;
                            }
                        }
                    }

                    let _ = self.tx_up.send(SessionOut::StartRtpTx {
                        dst_ip: ip.clone(),
                        dst_port: port,
                        pt: 0,
                    }); // PCMU
                    self.state = SessState::Established;
                    self.capture_started = None;
                    self.capture_payloads.clear();
                    self.intro_sent = true;

                    self.align_rtp_clock();

                    if let Some(sock) = self.rtp_socket.as_ref() {
                        self.sending_audio = true;
                        match send_wav_as_rtp_pcmu(
                            INTRO_WAV_PATH,
                            (ip.as_str(), port),
                            sock,
                            self.rtp_seq,
                            self.rtp_ts,
                            self.rtp_ssrc,
                        )
                        .await
                        {
                            Ok((next_seq, next_ts)) => {
                                self.rtp_seq = next_seq;
                                self.rtp_ts = next_ts;
                                self.rtp_last_sent = Some(Instant::now());
                                info!(
                                    "[session {}] sent intro wav {} (next seq={} ts={})",
                                    self.call_id, INTRO_WAV_PATH, self.rtp_seq, self.rtp_ts
                                );
                            }
                            Err(e) => {
                                warn!(
                                    "[session {}] failed to send intro wav: {:?}",
                                    self.call_id, e
                                );
                            }
                        }
                        self.sending_audio = false;
                    } else {
                        warn!(
                            "[session {}] intro skipped because RTP socket missing",
                            self.call_id
                        );
                    }

                    self.capture_started = Some(Instant::now());
                    self.capture_payloads.clear();
                    info!(
                        "[session {}] capture window started after intro playback",
                        self.call_id
                    );

                    if self.keepalive_stop.is_none() {
                        let (stop_tx, mut stop_rx) = oneshot::channel();
                        self.keepalive_stop = Some(stop_tx);
                        let tx = self.tx_in.clone();
                        tokio::spawn(async move {
                            loop {
                                tokio::select! {
                                    _ = tokio::time::sleep(Duration::from_millis(20)) => {
                                        let _ = tx.send(SessionIn::TimerTick);
                                    }
                                    _ = &mut stop_rx => break,
                                }
                            }
                        });
                    }
                }
                (SessState::Established, SessionIn::RtpIn { payload, .. }) => {
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
                    let _ = self.tx_up.send(SessionOut::Metrics {
                        name: "rtp_in",
                        value: payload.len() as i64,
                    });
                }
                (SessState::Established, SessionIn::TimerTick) => {
                    if let Err(e) = self.send_silence_frame().await {
                        warn!("[session {}] silence send failed: {:?}", self.call_id, e);
                    }
                }
                (SessState::Established, SessionIn::BotAudio { pcm48k: _ }) => {
                    // 48k→8k→μ-law→RTPパケット化は下位メディア層に委譲してOK
                    // ここではTS/Seqの進行のみ示唆
                    self.rtp_ts = self.rtp_ts.wrapping_add(160);
                }
                (_, SessionIn::Bye) => {
                    if let Some(stop) = self.keepalive_stop.take() {
                        let _ = stop.send(());
                    }
                    let _ = self.tx_up.send(SessionOut::StopRtpTx);
                    let _ = self.tx_up.send(SessionOut::SendSipBye200);
                    self.state = SessState::Terminated;
                }
                (_, SessionIn::Abort(e)) => {
                    eprintln!("call {} abort: {e:?}", self.call_id);
                    if let Some(stop) = self.keepalive_stop.take() {
                        let _ = stop.send(());
                    }
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
        if let Some(sdp) = &self.peer_sdp {
            (sdp.ip.clone(), sdp.port)
        } else {
            ("0.0.0.0".to_string(), 0)
        }
    }

    fn align_rtp_clock(&mut self) {
        if let Some(last) = self.rtp_last_sent {
            let gap_samples = (last.elapsed().as_secs_f64() * 8000.0) as u32;
            self.rtp_ts = self.rtp_ts.wrapping_add(gap_samples);
        }
    }

    async fn send_silence_frame(&mut self) -> Result<(), Error> {
        let peer = match self.peer_sdp.clone() {
            Some(p) => p,
            None => return Ok(()),
        };
        if self.sending_audio {
            return Ok(());
        }

        if self.rtp_socket.is_none() {
            match UdpSocket::bind("0.0.0.0:0").await {
                Ok(sock) => self.rtp_socket = Some(sock),
                Err(e) => {
                    warn!(
                        "[session {}] failed to bind RTP socket for silence: {:?}",
                        self.call_id, e
                    );
                    return Err(e.into());
                }
            }
        }

        self.align_rtp_clock();

        let frame = vec![0xFFu8; 160]; // μ-law silence
        let pkt = RtpPacket::new(0, self.rtp_seq, self.rtp_ts, self.rtp_ssrc, frame);
        let bytes = build_rtp_packet(&pkt);
        if let Some(sock) = &self.rtp_socket {
            let remote: SocketAddr = format!("{}:{}", peer.ip, peer.port).parse()?;
            sock.send_to(&bytes, remote).await?;
            self.rtp_seq = self.rtp_seq.wrapping_add(1);
            self.rtp_ts = self.rtp_ts.wrapping_add(160);
            self.rtp_last_sent = Some(Instant::now());
        }
        Ok(())
    }

    async fn handle_bot_pipeline(&mut self) -> Result<(), Error> {
        // 1) μ-law payload を WAV に保存
        let wav_path = "/tmp/input_from_peer.wav";
        write_mulaw_to_wav(&self.capture_payloads, wav_path)?;

        // 2) ASR+LLM+TTS (main.txt 由来の処理を bot モジュールに集約)
        let user_text = match ai::transcribe_and_log(wav_path).await {
            Ok(t) => t,
            Err(e) => {
                log::warn!("ASR failed: {e:?}");
                "すみません、聞き取れませんでした。".to_string()
            }
        };

        let bot_wav = match ai::handle_user_question_from_whisper(&user_text).await {
            Ok(p) => p,
            Err(e) => {
                log::warn!("LLM/TTS failed: {e:?}");
                return Ok(());
            }
        };

        // 5) RTP 送信
        if let Some(peer) = self.peer_sdp.clone() {
            info!(
                "[session {}] sending bot reply wav {} to {}:{}",
                self.call_id, bot_wav, peer.ip, peer.port
            );
            self.align_rtp_clock();
            if self.rtp_socket.is_none() {
                match UdpSocket::bind("0.0.0.0:0").await {
                    Ok(sock) => self.rtp_socket = Some(sock),
                    Err(e) => {
                        log::warn!(
                            "[session {}] failed to bind RTP socket for bot reply: {:?}",
                            self.call_id,
                            e
                        );
                        return Ok(());
                    }
                }
            }
            if let Some(sock) = self.rtp_socket.as_ref() {
                self.sending_audio = true;
                match send_wav_as_rtp_pcmu(
                    &bot_wav,
                    (&peer.ip, peer.port),
                    sock,
                    self.rtp_seq,
                    self.rtp_ts,
                    self.rtp_ssrc,
                )
                .await
                {
                    Ok((next_seq, next_ts)) => {
                        self.rtp_seq = next_seq;
                        self.rtp_ts = next_ts;
                        self.rtp_last_sent = Some(Instant::now());
                    }
                    Err(e) => {
                        log::warn!("RTP send failed: {e:?}");
                    }
                }
                self.sending_audio = false;
            }
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
    if sign {
        -value
    } else {
        value
    }
}

async fn send_wav_as_rtp_pcmu(
    wav_path: &str,
    dst: (&str, u16),
    sock: &UdpSocket,
    seq_start: u16,
    ts_start: u32,
    ssrc: u32,
) -> Result<(u16, u32), Error> {
    use tokio::time::sleep;

    let frames = load_wav_as_pcmu_frames(wav_path)?;
    if frames.is_empty() {
        anyhow::bail!("no frames");
    }

    let remote: SocketAddr = format!("{}:{}", dst.0, dst.1).parse()?;
    let mut seq = seq_start;
    let mut ts = ts_start;

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
        sock.send_to(&bytes, remote).await?;
        seq = seq.wrapping_add(1);
        ts = ts.wrapping_add(160);
        sleep(Duration::from_millis(20)).await;
    }
    Ok((seq, ts))
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
        while cur.len() < 160 {
            cur.push(0xFF);
        }
        frames.push(cur);
    }
    Ok(frames)
}

fn linear16_to_mulaw(sample: i16) -> u8 {
    const BIAS: i16 = 0x84;
    const CLIP: i16 = 32635;
    let mut s = sample;
    let mut sign = 0u8;
    if s < 0 {
        s = -s;
        sign = 0x80;
    }
    if s > CLIP {
        s = CLIP;
    }
    s += BIAS;
    let mut segment: u8 = 0;
    let mut value = (s as u16) >> 7;
    while value > 0 {
        segment += 1;
        value >>= 1;
        if segment >= 8 {
            break;
        }
    }
    let mantissa = ((s >> (segment + 3)) & 0x0F) as u8;
    !(sign | (segment << 4) | mantissa)
}
