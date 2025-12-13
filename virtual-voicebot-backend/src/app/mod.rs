//! app モジュール（対話オーケストレーション層）
//! 現状は MVP 用のシンプル実装で、session からの音声バッファを受け取り
//! ai::{asr,llm,tts} を呼び出してボット音声(WAV)のパスを session に返す。
//! transport/sip/rtp には依存せず、SessionIn 経由のイベントのみを返す。

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::ai;
use crate::session::SessionIn;

#[derive(Debug)]
pub enum AppEvent {
    CallStarted { call_id: String },
    AudioBuffered { call_id: String, pcm_mulaw: Vec<u8> },
    CallEnded { call_id: String },
}

/// シンプルな app ワーカーを起動する。現行の挙動を維持するため、
/// ai 呼び出しの順序/回数/エラー時のフォールバックは従来と同じにしている。
pub fn spawn_app_worker(
    call_id: String,
    rx: UnboundedReceiver<AppEvent>,
    session_tx: UnboundedSender<SessionIn>,
) {
    let worker = AppWorker::new(call_id, session_tx, rx);
    tokio::spawn(async move { worker.run().await });
}

struct AppWorker {
    call_id: String,
    session_tx: UnboundedSender<SessionIn>,
    rx: UnboundedReceiver<AppEvent>,
    active: bool,
}

impl AppWorker {
    fn new(
        call_id: String,
        session_tx: UnboundedSender<SessionIn>,
        rx: UnboundedReceiver<AppEvent>,
    ) -> Self {
        Self {
            call_id,
            session_tx,
            rx,
            active: false,
        }
    }

    async fn run(mut self) {
        while let Some(ev) = self.rx.recv().await {
            match ev {
                AppEvent::CallStarted { .. } => {
                    self.active = true;
                }
                AppEvent::AudioBuffered { pcm_mulaw, .. } => {
                    if !self.active {
                        log::debug!(
                            "[app {}] dropped audio because call not active",
                            self.call_id
                        );
                        continue;
                    }
                    if let Err(e) =
                        handle_audio_buffer(&self.call_id, pcm_mulaw, self.session_tx.clone()).await
                    {
                        log::warn!("[app {}] audio handling failed: {:?}", self.call_id, e);
                    }
                }
                AppEvent::CallEnded { .. } => break,
            }
        }
    }
}

async fn handle_audio_buffer(
    call_id: &str,
    pcm_mulaw: Vec<u8>,
    session_tx: UnboundedSender<SessionIn>,
) -> anyhow::Result<()> {
    let input_wav = format!("/tmp/input_from_peer_{call_id}.wav");
    write_mulaw_to_wav(&pcm_mulaw, &input_wav)?;

    let user_text = match ai::transcribe_and_log(&input_wav).await {
        Ok(t) => t,
        Err(e) => {
            log::warn!("[app {call_id}] ASR failed: {e:?}");
            "すみません、聞き取れませんでした。".to_string()
        }
    };

    let bot_wav = match ai::handle_user_question_from_whisper(&user_text).await {
        Ok(p) => p,
        Err(e) => {
            log::warn!("[app {call_id}] LLM/TTS failed: {e:?}");
            return Ok(());
        }
    };

    let _ = session_tx.send(SessionIn::AppBotAudioFile { path: bot_wav });
    Ok(())
}

fn write_mulaw_to_wav(payloads: &[u8], path: &str) -> anyhow::Result<()> {
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
