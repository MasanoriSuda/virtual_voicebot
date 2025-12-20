//! app モジュール（対話オーケストレーション層）
//! 現状は MVP 用のシンプル実装で、session からの音声バッファを受け取り
//! ai::{asr,llm,tts} を呼び出してボット音声(WAV)のパスを session に返す。
//! transport/sip/rtp には依存せず、SessionOut 経由のイベントのみを返す。

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::ai::{asr, llm, tts};
use crate::session::SessionOut;

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
    session_out_tx: UnboundedSender<SessionOut>,
) {
    let worker = AppWorker::new(call_id, session_out_tx, rx);
    tokio::spawn(async move { worker.run().await });
}

struct AppWorker {
    call_id: String,
    session_out_tx: UnboundedSender<SessionOut>,
    rx: UnboundedReceiver<AppEvent>,
    active: bool,
    history: Vec<(String, String)>, // (user, bot)
}

impl AppWorker {
    fn new(
        call_id: String,
        session_out_tx: UnboundedSender<SessionOut>,
        rx: UnboundedReceiver<AppEvent>,
    ) -> Self {
        Self {
            call_id,
            session_out_tx,
            rx,
            active: false,
            history: Vec::new(),
        }
    }

    async fn run(mut self) {
        while let Some(ev) = self.rx.recv().await {
            match ev {
                AppEvent::CallStarted { call_id } => {
                    if call_id != self.call_id {
                        log::warn!(
                            "[app {}] CallStarted received for mismatched call_id={}",
                            self.call_id,
                            call_id
                        );
                    }
                    self.active = true;
                }
                AppEvent::AudioBuffered { call_id, pcm_mulaw } => {
                    if call_id != self.call_id {
                        log::warn!(
                            "[app {}] AudioBuffered received for mismatched call_id={}",
                            self.call_id,
                            call_id
                        );
                    }
                    if !self.active {
                        log::debug!(
                            "[app {}] dropped audio because call not active",
                            self.call_id
                        );
                        continue;
                    }
                    let call_id = self.call_id.clone();
                    if let Err(e) = self.handle_audio_buffer(&call_id, pcm_mulaw).await {
                        log::warn!("[app {}] audio handling failed: {:?}", self.call_id, e);
                    }
                }
                AppEvent::CallEnded { call_id } => {
                    if call_id != self.call_id {
                        log::warn!(
                            "[app {}] CallEnded received for mismatched call_id={}",
                            self.call_id,
                            call_id
                        );
                    }
                    break;
                }
            }
        }
    }
}

impl AppWorker {
    async fn handle_audio_buffer(
        &mut self,
        call_id: &str,
        pcm_mulaw: Vec<u8>,
    ) -> anyhow::Result<()> {
        // ASR: チャンクI/F（1チャンクのみだが将来拡張用）
        let asr_chunks = vec![asr::AsrChunk {
            pcm_mulaw,
            end: true,
        }];
        let user_text = match asr::transcribe_chunks(call_id, &asr_chunks).await {
            Ok(t) => t,
            Err(e) => {
                log::warn!("[app {call_id}] ASR failed: {e:?}");
                "すみません、聞き取れませんでした。".to_string()
            }
        };

        // LLM（簡易履歴を踏まえてプロンプトを構築）
        let prompt = self.build_prompt(&user_text);
        let answer_text = match llm::generate_answer(&prompt).await {
            Ok(ans) => ans,
            Err(e) => {
                log::warn!("[app {call_id}] LLM failed: {e:?}");
                "すみません、うまく答えを用意できませんでした。".to_string()
            }
        };

        // 履歴に追加
        self.history.push((user_text.clone(), answer_text.clone()));

        // TTS
        match tts::synth_to_wav(&answer_text, None).await {
            Ok(bot_wav) => {
                let _ = self
                    .session_out_tx
                    .send(SessionOut::AppSendBotAudioFile { path: bot_wav });
            }
            Err(e) => {
                log::warn!("[app {call_id}] TTS failed: {e:?}");
            }
        }
        Ok(())
    }

    fn build_prompt(&self, latest_user: &str) -> String {
        let mut prompt = String::new();
        for (u, b) in &self.history {
            prompt.push_str("User: ");
            prompt.push_str(u);
            prompt.push_str("\nBot: ");
            prompt.push_str(b);
            prompt.push('\n');
        }
        prompt.push_str("User: ");
        prompt.push_str(latest_user);
        prompt
    }
}
