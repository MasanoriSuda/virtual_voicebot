//! app モジュール（対話オーケストレーション層）
//! 現状は MVP 用のシンプル実装で、session からの音声バッファを受け取り
//! ai ポートを呼び出してボット音声(WAV)のパスを session に返す。
//! transport/sip/rtp には依存せず、SessionOut 経由のイベントのみを返す。

use std::sync::Arc;

use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use crate::ports::ai::{AiPort, AsrChunk, ChatMessage, Role};
use crate::session::SessionOut;

const SORRY_WAV_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/data/zundamon_sorry.wav");

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
    session_out_tx: UnboundedSender<(String, SessionOut)>,
    ai_port: Arc<dyn AiPort>,
) -> UnboundedSender<AppEvent> {
    let (tx, rx) = unbounded_channel();
    let worker = AppWorker::new(call_id, session_out_tx, rx, ai_port);
    tokio::spawn(async move { worker.run().await });
    tx
}

struct AppWorker {
    call_id: String,
    session_out_tx: UnboundedSender<(String, SessionOut)>,
    rx: UnboundedReceiver<AppEvent>,
    active: bool,
    history: Vec<ChatMessage>,
    ai_port: Arc<dyn AiPort>,
}

impl AppWorker {
    fn new(
        call_id: String,
        session_out_tx: UnboundedSender<(String, SessionOut)>,
        rx: UnboundedReceiver<AppEvent>,
        ai_port: Arc<dyn AiPort>,
    ) -> Self {
        Self {
            call_id,
            session_out_tx,
            rx,
            active: false,
            history: Vec::new(),
            ai_port,
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
        let asr_chunks = vec![AsrChunk {
            pcm_mulaw,
            end: true,
        }];
        let user_text = match self
            .ai_port
            .transcribe_chunks(call_id.to_string(), asr_chunks)
            .await
        {
            Ok(t) => t,
            Err(e) => {
                log::warn!("[app {call_id}] ASR failed: {e:?}");
                "すみません、聞き取れませんでした。".to_string()
            }
        };

        let trimmed = user_text.trim();
        if trimmed.is_empty() {
            log::debug!(
                "[app {call_id}] empty ASR text after filtering, playing sorry audio"
            );
            let _ = self.session_out_tx.send((
                self.call_id.clone(),
                SessionOut::AppSendBotAudioFile {
                    path: SORRY_WAV_PATH.to_string(),
                },
            ));
            return Ok(());
        }

        let mut messages = Vec::with_capacity(self.history.len() + 1);
        messages.extend(self.history.iter().cloned());
        messages.push(ChatMessage {
            role: Role::User,
            content: trimmed.to_string(),
        });

        let answer_text = match self.ai_port.generate_answer(messages).await {
            Ok(ans) => ans,
            Err(e) => {
                log::warn!("[app {call_id}] LLM failed: {e:?}");
                "すみません、うまく答えを用意できませんでした。".to_string()
            }
        };

        // 履歴に追加
        self.history.push(ChatMessage {
            role: Role::User,
            content: trimmed.to_string(),
        });
        self.history.push(ChatMessage {
            role: Role::Assistant,
            content: answer_text.clone(),
        });

        // TTS
        match self.ai_port.synth_to_wav(answer_text, None).await {
            Ok(bot_wav) => {
                let _ = self
                    .session_out_tx
                    .send((self.call_id.clone(), SessionOut::AppSendBotAudioFile { path: bot_wav }));
            }
            Err(e) => {
                log::warn!("[app {call_id}] TTS failed: {e:?}");
            }
        }
        Ok(())
    }

    // build_prompt はロール分離に伴い廃止
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorry_wav_path_points_to_data_dir() {
        assert!(SORRY_WAV_PATH.ends_with("/data/zundamon_sorry.wav"));
    }
}
