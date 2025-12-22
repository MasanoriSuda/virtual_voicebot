#![allow(dead_code)]
// session.rs
use std::net::SocketAddr;

use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    oneshot,
};
use tokio::time::{Duration, Instant};

use crate::session::types::Sdp;
use crate::session::types::*;

use crate::app::{self, AppEvent};
use crate::config;
use crate::media::Recorder;
use crate::recording;
use crate::rtp::tx::RtpTxHandle;
use anyhow::Error;
use log::{debug, info, warn};
use reqwest::Client;
use serde_json::json;

const KEEPALIVE_INTERVAL: Duration = Duration::from_millis(20);
// MVPのシンプルなSession Timer。必要に応じて設計に合わせて短縮/設定化する。
const SESSION_TIMEOUT: Duration = Duration::from_secs(120);

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
    call_id: CallId,
    from_uri: String,
    to_uri: String,
    ingest_url: Option<String>,
    recording_base_url: Option<String>,
    ingest_sent: bool,
    peer_sdp: Option<Sdp>,
    local_sdp: Option<Sdp>,
    tx_up: UnboundedSender<SessionOut>,
    tx_in: UnboundedSender<SessionIn>,
    app_tx: UnboundedSender<AppEvent>,
    media_cfg: MediaConfig,
    rtp_tx: RtpTxHandle,
    recorder: Recorder,
    started_at: Option<Instant>,
    started_wall: Option<std::time::SystemTime>,
    rtp_last_sent: Option<Instant>,
    keepalive_stop: Option<oneshot::Sender<()>>,
    session_timer_stop: Option<oneshot::Sender<()>>,
    session_timer_deadline: Option<Instant>,
    session_expires: Duration,
    sending_audio: bool,
    // バッファ/タイマ
    speaking: bool,
    capture_started: Option<Instant>,
    capture_payloads: Vec<u8>,
    intro_sent: bool,
}

impl Session {
    pub fn spawn(
        call_id: CallId,
        from_uri: String,
        to_uri: String,
        tx_up: UnboundedSender<SessionOut>,
        media_cfg: MediaConfig,
        rtp_tx: RtpTxHandle,
        ingest_url: Option<String>,
        recording_base_url: Option<String>,
    ) -> SessionHandle {
        let (tx_in, rx_in) = tokio::sync::mpsc::unbounded_channel();
        let call_id_clone = call_id.clone();
        let (app_tx, app_rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
        let mut s = Self {
            state: SessState::Idle,
            call_id,
            from_uri,
            to_uri,
            ingest_url,
            recording_base_url,
            ingest_sent: false,
            peer_sdp: None,
            local_sdp: None,
            tx_up,
            tx_in: tx_in.clone(),
            app_tx,
            media_cfg,
            rtp_tx,
            recorder: Recorder::new(call_id_clone.clone()),
            started_at: None,
            started_wall: None,
            rtp_last_sent: None,
            keepalive_stop: None,
            session_timer_stop: None,
            session_timer_deadline: None,
            session_expires: SESSION_TIMEOUT,
            sending_audio: false,
            speaking: false,
            capture_started: None,
            capture_payloads: Vec::new(),
            intro_sent: false,
        };
        app::spawn_app_worker(s.call_id.clone(), app_rx, s.tx_up.clone());
        tokio::spawn(async move {
            s.run(rx_in).await;
        });
        SessionHandle { tx_in }
    }

    async fn run(&mut self, mut rx: UnboundedReceiver<SessionIn>) {
        while let Some(ev) = rx.recv().await {
            match (self.state, ev) {
                (SessState::Idle, SessionIn::SipInvite { offer, session_expires, .. }) => {
                    self.peer_sdp = Some(offer);
                    if let Some(expires) = session_expires {
                        self.update_session_expires(expires);
                    }
                    let answer = self.build_answer_pcmu8k();
                    self.local_sdp = Some(answer.clone());
                    let _ = self.tx_up.send(SessionOut::SipSend100);
                    let _ = self.tx_up.send(SessionOut::SipSend180);
                    let _ = self.tx_up.send(SessionOut::SipSend200 { answer });
                    self.state = SessState::Early;
                }
                (SessState::Early, SessionIn::SipAck) => {
                    // 相手SDPからRTP宛先を確定して送信開始
                    if self.intro_sent {
                        continue;
                    }
                    self.started_at = Some(Instant::now());
                    self.started_wall = Some(std::time::SystemTime::now());
                    if let Err(e) = self.recorder.start() {
                        warn!(
                            "[session {}] failed to start recorder: {:?}",
                            self.call_id, e
                        );
                    }

                    let (ip, port) = self.peer_rtp_dst();
                    let dst_addr: SocketAddr = match format!("{ip}:{port}").parse() {
                        Ok(a) => a,
                        Err(e) => {
                            warn!(
                                "[session {}] invalid RTP destination {}:{} ({:?})",
                                self.call_id, ip, port, e
                            );
                            continue;
                        }
                    };

                    // rtp側でソケットを持つように変更
                    // RTP送信開始（rtp 側で Seq/TS/SSRC を管理）
                    self.rtp_tx
                        .start(self.call_id.clone(), dst_addr, 0, 0x12345678, 0, 0);

                    let _ = self.tx_up.send(SessionOut::RtpStartTx {
                        dst_ip: ip.clone(),
                        dst_port: port,
                        pt: 0,
                    }); // PCMU
                    self.state = SessState::Established;
                    self.capture_started = None;
                    self.capture_payloads.clear();
                    self.intro_sent = true;

                    self.align_rtp_clock();

                    let _ = self.app_tx.send(AppEvent::CallStarted {
                        call_id: self.call_id.clone(),
                    });

                    self.sending_audio = true;
                    match send_wav_as_rtp_pcmu(
                        INTRO_WAV_PATH,
                        dst_addr,
                        &self.rtp_tx,
                        &self.call_id,
                        &mut self.recorder,
                    )
                    .await
                    {
                        Ok(()) => {
                            self.rtp_last_sent = Some(Instant::now());
                            info!(
                                "[session {}] sent intro wav {}",
                                self.call_id, INTRO_WAV_PATH
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

                    self.capture_started = Some(Instant::now());
                    self.capture_payloads.clear();
                    info!(
                        "[session {}] capture window started after intro playback",
                        self.call_id
                    );

                    self.start_keepalive_timer();
                    self.start_session_timer();
                }
                (SessState::Established, SessionIn::MediaRtpIn { payload, .. }) => {
                    debug!(
                        "[session {}] RTP payload received len={}",
                        self.call_id,
                        payload.len()
                    );
                    self.recorder.push_rx_mulaw(&payload);
                    if let Some(start) = self.capture_started {
                        self.capture_payloads.extend_from_slice(&payload);
                        if start.elapsed() >= Duration::from_secs(10) {
                            info!(
                                "[session {}] buffered audio ready for app ({} bytes)",
                                self.call_id,
                                self.capture_payloads.len()
                            );
                            let _ = self.app_tx.send(AppEvent::AudioBuffered {
                                call_id: self.call_id.clone(),
                                pcm_mulaw: self.capture_payloads.clone(),
                            });
                            self.capture_started = None;
                            self.capture_payloads.clear();
                        }
                    }
                    let _ = self.tx_up.send(SessionOut::Metrics {
                        name: "rtp_in",
                        value: payload.len() as i64,
                    });
                }
                (SessState::Established, SessionIn::MediaTimerTick) => {
                    if let Err(e) = self.send_silence_frame().await {
                        warn!("[session {}] silence send failed: {:?}", self.call_id, e);
                    }
                }
                (_, SessionIn::SipBye) => {
                    self.stop_keepalive_timer();
                    self.stop_session_timer();
                    if let Err(e) = self.recorder.stop() {
                        warn!(
                            "[session {}] failed to finalize recording: {:?}",
                            self.call_id, e
                        );
                    }
                    self.send_ingest("ended").await;
                    self.rtp_tx.stop(&self.call_id);
                    let _ = self.tx_up.send(SessionOut::RtpStopTx);
                    let _ = self.tx_up.send(SessionOut::SipSendBye200);
                    let _ = self.app_tx.send(AppEvent::CallEnded {
                        call_id: self.call_id.clone(),
                    });
                    self.state = SessState::Terminated;
                }
                (_, SessionIn::SipTransactionTimeout { call_id: _ }) => {
                    warn!("[session {}] transaction timeout notified", self.call_id);
                }
                (SessState::Established, SessionIn::AppBotAudioFile { path }) => {
                    if let Some(peer) = self.peer_sdp.clone() {
                        self.align_rtp_clock();
                        let dst: SocketAddr = match format!("{}:{}", peer.ip, peer.port).parse() {
                            Ok(a) => a,
                            Err(e) => {
                                warn!(
                                    "[session {}] invalid RTP destination {}:{} ({:?})",
                                    self.call_id, peer.ip, peer.port, e
                                );
                                return;
                            }
                        };
                        self.sending_audio = true;
                        match send_wav_as_rtp_pcmu(
                            &path,
                            dst,
                            &self.rtp_tx,
                            &self.call_id,
                            &mut self.recorder,
                        )
                        .await
                        {
                            Ok(()) => {
                                self.rtp_last_sent = Some(Instant::now());
                            }
                            Err(e) => {
                                warn!(
                                    "[session {}] failed to send app audio: {:?}",
                                    self.call_id, e
                                );
                            }
                        }
                        self.sending_audio = false;
                    }
                }
                (_, SessionIn::AppHangup) => {
                    warn!("[session {}] app requested hangup", self.call_id);
                    self.stop_keepalive_timer();
                    self.stop_session_timer();
                    if let Err(e) = self.recorder.stop() {
                        warn!(
                            "[session {}] failed to finalize recording: {:?}",
                            self.call_id, e
                        );
                    }
                    self.send_ingest("ended").await;
                    self.rtp_tx.stop(&self.call_id);
                    let _ = self.tx_up.send(SessionOut::RtpStopTx);
                    let _ = self.tx_up.send(SessionOut::SipSendBye200);
                    let _ = self.app_tx.send(AppEvent::CallEnded {
                        call_id: self.call_id.clone(),
                    });
                    self.state = SessState::Terminated;
                }
                (_, SessionIn::SipSessionExpires { expires }) => {
                    self.update_session_expires(expires);
                }
                (_, SessionIn::SessionTimerFired) => {
                    warn!("[session {}] session timer fired", self.call_id);
                    self.stop_keepalive_timer();
                    self.stop_session_timer();
                    if let Err(e) = self.recorder.stop() {
                        warn!(
                            "[session {}] failed to finalize recording: {:?}",
                            self.call_id, e
                        );
                    }
                    self.send_ingest("ended").await;
                    let _ = self.tx_up.send(SessionOut::RtpStopTx);
                    let _ = self.tx_up.send(SessionOut::AppSessionTimeout);
                    let _ = self.app_tx.send(AppEvent::CallEnded {
                        call_id: self.call_id.clone(),
                    });
                    self.state = SessState::Terminated;
                }
                (_, SessionIn::Abort(e)) => {
                    warn!("call {} abort: {e:?}", self.call_id);
                    self.stop_keepalive_timer();
                    self.stop_session_timer();
                    if let Err(e) = self.recorder.stop() {
                        warn!(
                            "[session {}] failed to finalize recording: {:?}",
                            self.call_id, e
                        );
                    }
                    self.send_ingest("failed").await;
                    self.rtp_tx.stop(&self.call_id);
                    let _ = self.tx_up.send(SessionOut::RtpStopTx);
                    let _ = self.app_tx.send(AppEvent::CallEnded {
                        call_id: self.call_id.clone(),
                    });
                    self.state = SessState::Terminated;
                }
                _ => { /* それ以外は無視 or ログ */ }
            }
        }
        if let Err(e) = self.recorder.stop() {
            warn!(
                "[session {}] failed to finalize recording on shutdown: {:?}",
                self.call_id, e
            );
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
            self.rtp_tx.adjust_timestamp(&self.call_id, gap_samples);
        }
    }

    /// keepalive タイマを開始する（挙動は従来と同じ。20ms ごとに TimerTick を送る）
    fn start_keepalive_timer(&mut self) {
        if self.keepalive_stop.is_some() {
            return;
        }
        let (stop_tx, mut stop_rx) = oneshot::channel();
        self.keepalive_stop = Some(stop_tx);
        let tx = self.tx_in.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(KEEPALIVE_INTERVAL) => {
                        let _ = tx.send(SessionIn::MediaTimerTick);
                    }
                    _ = &mut stop_rx => break,
                }
            }
        });
    }

    /// keepalive タイマを停止する（存在する場合のみ）
    fn stop_keepalive_timer(&mut self) {
        if let Some(stop) = self.keepalive_stop.take() {
            let _ = stop.send(());
        }
    }

    /// Session Timer（簡易 keepalive 含む）の発火を監視し、失効時に SessionIn を送る
    fn start_session_timer(&mut self) {
        if self.session_timer_stop.is_some() {
            return;
        }
        let (stop_tx, mut stop_rx) = oneshot::channel();
        let timeout = self.session_expires;
        self.session_timer_deadline = Some(Instant::now() + timeout);
        self.session_timer_stop = Some(stop_tx);
        let tx = self.tx_in.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = tokio::time::sleep(timeout) => {
                    let _ = tx.send(SessionIn::SessionTimerFired);
                }
                _ = &mut stop_rx => {}
            }
        });
    }

    fn stop_session_timer(&mut self) {
        if let Some(stop) = self.session_timer_stop.take() {
            let _ = stop.send(());
        }
        self.session_timer_deadline = None;
    }

    fn update_session_expires(&mut self, expires: Duration) {
        self.session_expires = expires;
        if self.session_timer_stop.is_some() {
            self.stop_session_timer();
            self.start_session_timer();
        }
    }

    async fn send_silence_frame(&mut self) -> Result<(), Error> {
        if self.peer_sdp.is_none() {
            return Ok(());
        }
        if self.sending_audio {
            return Ok(());
        }

        self.align_rtp_clock();

        let frame = vec![0xFFu8; 160]; // μ-law silence
        self.rtp_tx.send_payload(&self.call_id, frame);
        self.rtp_last_sent = Some(Instant::now());
        Ok(())
    }

    async fn send_ingest(&mut self, status: &str) {
        if self.ingest_sent {
            return;
        }
        let ingest_url = match &self.ingest_url {
            Some(u) => u.clone(),
            None => return,
        };
        let started_at = self.started_wall.unwrap_or_else(std::time::SystemTime::now);
        let ended_at = std::time::SystemTime::now();
        let duration_sec = self.started_at.map(|s| s.elapsed().as_secs()).unwrap_or(0);
        let recording_dir = self.recorder.relative_path();
        let recording_url = self
            .recording_base_url
            .as_ref()
            .map(|base| recording::recording_url(base, &recording_dir));

        let payload = json!({
            "callId": self.call_id,
            "from": self.from_uri,
            "to": self.to_uri,
            "startedAt": humantime::format_rfc3339(started_at).to_string(),
            "endedAt": humantime::format_rfc3339(ended_at).to_string(),
            "status": status,
            "summary": "",
            "durationSec": duration_sec,
            "recording": recording_url.as_ref().map(|url| json!({
                "recordingUrl": url,
                "durationSec": duration_sec,
                "sampleRate": 8000,
                "channels": 1
            })),
        });

        let timeout = config::timeouts().ingest_http;
        let url = ingest_url.clone();
        let call_id = self.call_id.clone();
        self.ingest_sent = true;
        tokio::spawn(async move {
            let client = match Client::builder().timeout(timeout).build() {
                Ok(c) => c,
                Err(e) => {
                    log::warn!(
                        "[ingest] failed to build client for call {}: {:?}",
                        call_id,
                        e
                    );
                    return;
                }
            };
            if let Err(e) = client.post(url).json(&payload).send().await {
                log::warn!("[ingest] failed to post call {}: {:?}", call_id, e);
            }
        });
    }
}

async fn send_wav_as_rtp_pcmu(
    wav_path: &str,
    dst: SocketAddr,
    tx: &RtpTxHandle,
    key: &str,
    recorder: &mut Recorder,
) -> Result<(), Error> {
    use tokio::time::sleep;

    let frames = load_wav_as_pcmu_frames(wav_path)?;
    if frames.is_empty() {
        anyhow::bail!("no frames");
    }

    log::info!(
        "[rtp tx] sending {} frames ({} samples) to {}",
        frames.len(),
        frames.len() * 160,
        dst
    );

    for frame in frames {
        // 送信音声も録音に含めて mixed を作る（キープアライブの無音は別扱い）
        recorder.push_tx_mulaw(&frame);
        tx.send_payload(key, frame);
        sleep(Duration::from_millis(20)).await;
    }
    Ok(())
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
