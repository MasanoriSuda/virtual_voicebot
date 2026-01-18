#![allow(dead_code)]
// session.rs
use std::net::SocketAddr;
use std::sync::Arc;

use chrono::{Local, Timelike};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::time::{Duration, Instant};

use crate::session::types::Sdp;
use crate::session::types::*;

use crate::app::AppEvent;
use crate::http::ingest::IngestPort;
use crate::media::Recorder;
use crate::recording;
use crate::recording::storage::StoragePort;
use crate::rtp::tx::RtpTxHandle;
use crate::session::capture::AudioCapture;
use crate::config;
use crate::session::timers::SessionTimers;
use anyhow::Error;
use log::{debug, info, warn};
use serde_json::json;

const KEEPALIVE_INTERVAL: Duration = Duration::from_millis(20);

const INTRO_MORNING_WAV_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/data/zundamon_intro_morning.wav");
const INTRO_AFTERNOON_WAV_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/data/zundamon_intro_afternoon.wav");
const INTRO_EVENING_WAV_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/data/zundamon_intro_evening.wav");

fn intro_wav_path_for_hour(hour: u32) -> &'static str {
    match hour {
        5..=11 => INTRO_MORNING_WAV_PATH,
        12..=16 => INTRO_AFTERNOON_WAV_PATH,
        _ => INTRO_EVENING_WAV_PATH,
    }
}

fn get_intro_wav_path() -> &'static str {
    let hour = Local::now().hour();
    intro_wav_path_for_hour(hour)
}

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
    ingest_port: Arc<dyn IngestPort>,
    storage_port: Arc<dyn StoragePort>,
    peer_sdp: Option<Sdp>,
    local_sdp: Option<Sdp>,
    session_out_tx: UnboundedSender<(CallId, SessionOut)>,
    tx_in: UnboundedSender<SessionIn>,
    app_tx: UnboundedSender<AppEvent>,
    media_cfg: MediaConfig,
    rtp_tx: RtpTxHandle,
    recorder: Recorder,
    started_at: Option<Instant>,
    started_wall: Option<std::time::SystemTime>,
    rtp_last_sent: Option<Instant>,
    timers: SessionTimers,
    sending_audio: bool,
    // バッファ/タイマ
    speaking: bool,
    capture: AudioCapture,
    intro_sent: bool,
    session_expires: Option<Duration>,
    session_refresher: Option<SessionRefresher>,
}

impl Session {
    pub fn spawn(
        call_id: CallId,
        from_uri: String,
        to_uri: String,
        session_out_tx: UnboundedSender<(CallId, SessionOut)>,
        app_tx: UnboundedSender<AppEvent>,
        media_cfg: MediaConfig,
        rtp_tx: RtpTxHandle,
        ingest_url: Option<String>,
        recording_base_url: Option<String>,
        ingest_port: Arc<dyn IngestPort>,
        storage_port: Arc<dyn StoragePort>,
    ) -> SessionHandle {
        let (tx_in, rx_in) = tokio::sync::mpsc::unbounded_channel();
        let call_id_clone = call_id.clone();
        let mut s = Self {
            state: SessState::Idle,
            call_id,
            from_uri,
            to_uri,
            ingest_url,
            recording_base_url,
            ingest_sent: false,
            ingest_port,
            storage_port,
            peer_sdp: None,
            local_sdp: None,
            session_out_tx,
            tx_in: tx_in.clone(),
            app_tx,
            media_cfg,
            rtp_tx,
            recorder: Recorder::new(call_id_clone),
            started_at: None,
            started_wall: None,
            rtp_last_sent: None,
            timers: SessionTimers::new(Duration::from_secs(0)),
            sending_audio: false,
            speaking: false,
            capture: AudioCapture::new(config::vad_config().clone()),
            intro_sent: false,
            session_expires: None,
            session_refresher: None,
        };
        tokio::spawn(async move {
            s.run(rx_in).await;
        });
        SessionHandle { tx_in }
    }

    async fn run(&mut self, mut rx: UnboundedReceiver<SessionIn>) {
        while let Some(ev) = rx.recv().await {
            let next_state = next_session_state(self.state, &ev);
            let mut advance_state = true;
            match (self.state, ev) {
                (SessState::Idle, SessionIn::SipInvite { offer, session_timer, .. }) => {
                    self.peer_sdp = Some(offer);
                    if let Some(timer) = session_timer {
                        self.update_session_expires(timer);
                    }
                    let answer = self.build_answer_pcmu8k();
                    self.local_sdp = Some(answer.clone());
                    let _ = self
                        .session_out_tx
                        .send((self.call_id.clone(), SessionOut::SipSend100));
                    let _ = self
                        .session_out_tx
                        .send((self.call_id.clone(), SessionOut::SipSend180));
                    let _ = self
                        .session_out_tx
                        .send((self.call_id.clone(), SessionOut::SipSend200 { answer }));
                }
                (SessState::Early, SessionIn::SipAck) => {
                    // 相手SDPからRTP宛先を確定して送信開始
                    if self.intro_sent {
                        advance_state = false;
                    }
                    if !advance_state {
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
                    let dst_addr = match format!("{ip}:{port}").parse() {
                        Ok(a) => Some(a),
                        Err(e) => {
                            warn!(
                                "[session {}] invalid RTP destination {}:{} ({:?})",
                                self.call_id, ip, port, e
                            );
                            advance_state = false;
                            None
                        }
                    };
                    let Some(dst_addr) = dst_addr else {
                        continue;
                    };

                    // rtp側でソケットを持つように変更
                    // RTP送信開始（rtp 側で Seq/TS/SSRC を管理）
                    self.rtp_tx
                        .start(self.call_id.clone(), dst_addr, 0, 0x12345678, 0, 0);

                    let _ = self.session_out_tx.send((
                        self.call_id.clone(),
                        SessionOut::RtpStartTx {
                            dst_ip: ip.clone(),
                            dst_port: port,
                            pt: 0,
                        },
                    )); // PCMU
                    self.capture.reset();
                    self.intro_sent = true;

                    self.align_rtp_clock();

                    let _ = self.app_tx.send(AppEvent::CallStarted {
                        call_id: self.call_id.clone(),
                    });

                    self.sending_audio = true;
                    let intro_wav_path = get_intro_wav_path();
                    match send_wav_as_rtp_pcmu(
                        intro_wav_path,
                        dst_addr,
                        &self.rtp_tx,
                        &self.call_id,
                        &mut self.recorder,
                        self.storage_port.as_ref(),
                    )
                    .await
                    {
                        Ok(()) => {
                            self.rtp_last_sent = Some(Instant::now());
                            info!(
                                "[session {}] sent intro wav {}",
                                self.call_id, intro_wav_path
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

                    self.capture.start();
                    info!(
                        "[session {}] capture window started after intro playback",
                        self.call_id
                    );

                    self.start_keepalive_timer();
                    self.start_session_timer_if_needed();
                }
                (SessState::Established, SessionIn::MediaRtpIn { payload, .. }) => {
                    debug!(
                        "[session {}] RTP payload received len={}",
                        self.call_id,
                        payload.len()
                    );
                    self.recorder.push_rx_mulaw(&payload);
                    if let Some(buffer) = self.capture.ingest(&payload) {
                        info!(
                            "[session {}] buffered audio ready for app ({} bytes)",
                            self.call_id,
                            buffer.len()
                        );
                        let _ = self.app_tx.send(AppEvent::AudioBuffered {
                            call_id: self.call_id.clone(),
                            pcm_mulaw: buffer,
                        });
                        self.capture.start();
                    }
                    let _ = self.session_out_tx.send((
                        self.call_id.clone(),
                        SessionOut::Metrics {
                            name: "rtp_in",
                            value: payload.len() as i64,
                        },
                    ));
                }
                (SessState::Established, SessionIn::SipReInvite { session_timer, .. }) => {
                    if let Some(timer) = session_timer {
                        self.update_session_expires(timer);
                    }
                    let answer = match self.local_sdp.clone() {
                        Some(answer) => answer,
                        None => {
                            let answer = self.build_answer_pcmu8k();
                            self.local_sdp = Some(answer.clone());
                            answer
                        }
                    };
                    let _ = self
                        .session_out_tx
                        .send((self.call_id.clone(), SessionOut::SipSend200 { answer }));
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
                    let _ = self
                        .session_out_tx
                        .send((self.call_id.clone(), SessionOut::RtpStopTx));
                    let _ = self
                        .session_out_tx
                        .send((self.call_id.clone(), SessionOut::SipSendBye200));
                    let _ = self.app_tx.send(AppEvent::CallEnded {
                        call_id: self.call_id.clone(),
                    });
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
                            self.storage_port.as_ref(),
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
                    let _ = self
                        .session_out_tx
                        .send((self.call_id.clone(), SessionOut::RtpStopTx));
                    let _ = self
                        .session_out_tx
                        .send((self.call_id.clone(), SessionOut::SipSendBye200));
                    let _ = self.app_tx.send(AppEvent::CallEnded {
                        call_id: self.call_id.clone(),
                    });
                }
                (_, SessionIn::SipSessionExpires { timer }) => {
                    self.update_session_expires(timer);
                }
                (_, SessionIn::SessionRefreshDue) => {
                    if let (Some(expires), Some(SessionRefresher::Uas)) =
                        (self.session_expires, self.session_refresher)
                    {
                        let _ = self.session_out_tx.send((
                            self.call_id.clone(),
                            SessionOut::SipSendUpdate { expires },
                        ));
                        self.update_session_expires(SessionTimerInfo {
                            expires,
                            refresher: SessionRefresher::Uas,
                        });
                    }
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
                    let _ = self
                        .session_out_tx
                        .send((self.call_id.clone(), SessionOut::RtpStopTx));
                    let _ = self
                        .session_out_tx
                        .send((self.call_id.clone(), SessionOut::AppSessionTimeout));
                    let _ = self.app_tx.send(AppEvent::CallEnded {
                        call_id: self.call_id.clone(),
                    });
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
                    let _ = self
                        .session_out_tx
                        .send((self.call_id.clone(), SessionOut::RtpStopTx));
                    let _ = self.app_tx.send(AppEvent::CallEnded {
                        call_id: self.call_id.clone(),
                    });
                }
                _ => { /* それ以外は無視 or ログ */ }
            }
            if advance_state {
                self.state = next_state;
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
        self.timers
            .start_keepalive(self.tx_in.clone(), KEEPALIVE_INTERVAL);
    }

    /// keepalive タイマを停止する（存在する場合のみ）
    fn stop_keepalive_timer(&mut self) {
        self.timers.stop_keepalive();
    }

    /// Session Timer（RFC 4028 準拠）。設定がある場合のみ開始する。
    fn start_session_timer_if_needed(&mut self) {
        let Some(expires) = self.session_expires else {
            return;
        };
        if expires.is_zero() {
            return;
        }
        let refresh_after = self.refresh_after(expires);
        self.timers
            .start_session_timer(self.tx_in.clone(), expires, refresh_after);
    }

    fn stop_session_timer(&mut self) {
        self.timers.stop_session_timer();
    }

    fn update_session_expires(&mut self, timer: SessionTimerInfo) {
        self.session_expires = Some(timer.expires);
        self.session_refresher = Some(timer.refresher);
        let refresh_after = self.refresh_after(timer.expires);
        self.timers
            .update_session_expires(timer.expires, self.tx_in.clone(), refresh_after);
    }

    fn refresh_after(&self, expires: Duration) -> Option<Duration> {
        if self.session_refresher != Some(SessionRefresher::Uas) {
            return None;
        }
        let total_ms = expires.as_millis();
        if total_ms == 0 {
            return None;
        }
        let refresh_ms = total_ms.saturating_mul(8) / 10;
        if refresh_ms == 0 {
            return None;
        }
        let refresh_ms = std::cmp::min(refresh_ms, u64::MAX as u128) as u64;
        Some(Duration::from_millis(refresh_ms))
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

        let url = ingest_url.clone();
        let call_id = self.call_id.clone();
        let ingest_port = self.ingest_port.clone();
        self.ingest_sent = true;
        tokio::spawn(async move {
            if let Err(e) = ingest_port.post(url, payload).await {
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
    storage_port: &dyn StoragePort,
) -> Result<(), Error> {
    use tokio::time::sleep;

    let frames = storage_port.load_wav_as_pcmu_frames(wav_path)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intro_path_matches_time_window() {
        assert_eq!(intro_wav_path_for_hour(5), INTRO_MORNING_WAV_PATH);
        assert_eq!(intro_wav_path_for_hour(11), INTRO_MORNING_WAV_PATH);
        assert_eq!(intro_wav_path_for_hour(12), INTRO_AFTERNOON_WAV_PATH);
        assert_eq!(intro_wav_path_for_hour(16), INTRO_AFTERNOON_WAV_PATH);
        assert_eq!(intro_wav_path_for_hour(17), INTRO_EVENING_WAV_PATH);
        assert_eq!(intro_wav_path_for_hour(4), INTRO_EVENING_WAV_PATH);
    }
}
