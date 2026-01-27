//! app モジュール（対話オーケストレーション層）
//! 現状は MVP 用のシンプル実装で、session からの音声バッファを受け取り
//! ai ポートを呼び出してボット音声(WAV)のパスを session に返す。
//! transport/sip/rtp には依存せず、SessionOut 経由のイベントのみを返す。

mod router;

use std::sync::Arc;

use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use crate::app::router::{
    parse_intent_json, router_config, system_info_response, RouteAction, Router,
};
use crate::config;
use crate::db::port::PhoneLookupPort;
use crate::ports::ai::{AiSerPort, AsrChunk, ChatMessage, Role, SerInputPcm, WeatherQuery};
use crate::session::SessionOut;

const SORRY_WAV_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/data/zundamon_sorry.wav");
const SPEC_FILTER_KEYWORDS: [&str; 21] = [
    "仕様",
    "内部",
    "システム",
    "システムプロンプト",
    "プロンプト",
    "設定",
    "制限",
    "ポリシー",
    "モデル",
    "llm",
    "gpt",
    "claude",
    "スペック",
    "version",
    "バージョン",
    "構成",
    "アーキテクチャ",
    "ログ",
    "運用",
    "api",
    "トークン",
];

#[derive(Debug)]
pub enum AppEvent {
    CallStarted {
        call_id: String,
        caller: Option<String>,
    },
    AudioBuffered {
        call_id: String,
        pcm_mulaw: Vec<u8>,
        pcm_linear16: Vec<i16>,
    },
    CallEnded { call_id: String },
}

/// シンプルな app ワーカーを起動する。現行の挙動を維持するため、
/// ai 呼び出しの順序/回数/エラー時のフォールバックは従来と同じにしている。
pub fn spawn_app_worker(
    call_id: String,
    session_out_tx: UnboundedSender<(String, SessionOut)>,
    ai_port: Arc<dyn AiSerPort>,
    phone_lookup: Arc<dyn PhoneLookupPort>,
) -> UnboundedSender<AppEvent> {
    let (tx, rx) = unbounded_channel();
    let worker = AppWorker::new(call_id, session_out_tx, rx, ai_port, phone_lookup);
    tokio::spawn(async move { worker.run().await });
    tx
}

struct AppWorker {
    call_id: String,
    session_out_tx: UnboundedSender<(String, SessionOut)>,
    rx: UnboundedReceiver<AppEvent>,
    active: bool,
    history: Vec<ChatMessage>,
    ai_port: Arc<dyn AiSerPort>,
    phone_lookup: Arc<dyn PhoneLookupPort>,
    router: Router,
}

impl AppWorker {
    fn new(
        call_id: String,
        session_out_tx: UnboundedSender<(String, SessionOut)>,
        rx: UnboundedReceiver<AppEvent>,
        ai_port: Arc<dyn AiSerPort>,
        phone_lookup: Arc<dyn PhoneLookupPort>,
    ) -> Self {
        Self {
            call_id,
            session_out_tx,
            rx,
            active: false,
            history: Vec::new(),
            ai_port,
            phone_lookup,
            router: Router::new(),
        }
    }

    async fn run(mut self) {
        while let Some(ev) = self.rx.recv().await {
            match ev {
                AppEvent::CallStarted { call_id, caller } => {
                    if call_id != self.call_id {
                        log::warn!(
                            "[app {}] CallStarted received for mismatched call_id={}",
                            self.call_id,
                            call_id
                        );
                    }
                    self.active = true;
                    let caller_display = caller
                        .as_deref()
                        .filter(|value| !value.trim().is_empty());
                    if let Some(value) = caller_display {
                        log::info!("[app {}] caller extracted={}", self.call_id, value);
                    } else {
                        log::info!("[app {}] caller missing", self.call_id);
                    }
                    self.handle_phone_lookup(caller).await;
                }
                AppEvent::AudioBuffered {
                    call_id,
                    pcm_mulaw,
                    pcm_linear16,
                } => {
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
                    if let Err(e) =
                        self.handle_audio_buffer(&call_id, pcm_mulaw, pcm_linear16).await
                    {
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
        pcm_linear16: Vec<i16>,
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

        let ser_input = SerInputPcm {
            session_id: call_id.to_string(),
            stream_id: "main".to_string(),
            pcm: pcm_linear16,
            sample_rate: 8000,
            channels: 1,
        };
        match self.ai_port.analyze(ser_input).await {
            Ok(result) => {
                log::info!(
                    "[app {call_id}] ser emotion={:?} confidence={:.2}",
                    result.emotion,
                    result.confidence
                );
            }
            Err(err) => {
                log::warn!("[app {call_id}] SER failed: {}", err.reason);
            }
        }

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

        if is_spec_question(trimmed) {
            log::warn!(
                "[security] spec question blocked: call_id={} input={}",
                call_id,
                trimmed
            );
            let answer_text = system_info_response();
            match self.ai_port.synth_to_wav(answer_text, None).await {
                Ok(bot_wav) => {
                    let _ = self.session_out_tx.send((
                        self.call_id.clone(),
                        SessionOut::AppSendBotAudioFile { path: bot_wav },
                    ));
                }
                Err(e) => {
                    log::warn!("[app {call_id}] TTS failed: {e:?}");
                }
            }
            return Ok(());
        }

        let intent_json = match self.ai_port.classify_intent(trimmed.to_string()).await {
            Ok(raw) => raw,
            Err(err) => {
                log::warn!("[app {call_id}] intent classify failed: {err:?}");
                "{\"intent\":\"general_chat\",\"query\":\"\"}".to_string()
            }
        };
        let intent_result = parse_intent_json(&intent_json, trimmed);
        log::info!(
            "[app {call_id}] intent classified={} raw={}",
            intent_result.raw_intent,
            intent_json
        );

        let (answer_text, user_query) = match self.router.route(intent_result) {
            RouteAction::FixedResponse(text) => (text, trimmed.to_string()),
            RouteAction::SystemInfo => (system_info_response(), trimmed.to_string()),
            RouteAction::GeneralChat { query } => {
                let mut messages = Vec::with_capacity(self.history.len() + 1);
                messages.extend(self.history.iter().cloned());
                messages.push(ChatMessage {
                    role: Role::User,
                    content: query.clone(),
                });

                let answer_text = match self.ai_port.generate_answer(messages).await {
                    Ok(ans) => ans,
                    Err(e) => {
                        log::warn!("[app {call_id}] LLM failed: {e:?}");
                        "すみません、うまく答えを用意できませんでした。".to_string()
                    }
                };
                (answer_text, query)
            }
            RouteAction::Weather {
                query,
                location,
                date,
            } => {
                let req = WeatherQuery { location, date };
                let answer_text = match self.ai_port.handle_weather(req).await {
                    Ok(text) => text,
                    Err(err) => {
                        log::warn!("[app {call_id}] weather failed: {err:?}");
                        router_config().weather_error_response.clone()
                    }
                };
                (answer_text, query)
            }
        };

        // 履歴に追加
        self.history.push(ChatMessage {
            role: Role::User,
            content: user_query,
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

    async fn handle_phone_lookup(&mut self, caller: Option<String>) {
        if !config::phone_lookup_enabled() {
            log::info!("[app {}] phone lookup disabled", self.call_id);
            return;
        }
        let Some(caller) = caller.filter(|v| !v.trim().is_empty()) else {
            log::info!("[app {}] caller missing, skip lookup", self.call_id);
            return;
        };
        match self.phone_lookup.lookup_phone(caller.clone()).await {
            Ok(Some(result)) => {
                log::info!(
                    "[app {}] phone lookup found caller={} ivr_enabled={}",
                    self.call_id,
                    caller,
                    result.ivr_enabled
                );
            }
            Ok(None) => {
                log::info!(
                    "[app {}] phone lookup not found caller={} (default ivr enabled)",
                    self.call_id,
                    caller
                );
            }
            Err(err) => {
                log::warn!(
                    "[app {}] phone lookup failed caller={}: {}",
                    self.call_id,
                    caller,
                    err
                );
            }
        }
    }

    // build_prompt はロール分離に伴い廃止
}

fn is_spec_question(input: &str) -> bool {
    let lowered = input.to_ascii_lowercase();
    SPEC_FILTER_KEYWORDS
        .iter()
        .any(|kw| lowered.contains(kw))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorry_wav_path_points_to_data_dir() {
        assert!(SORRY_WAV_PATH.ends_with("/data/zundamon_sorry.wav"));
    }

    #[test]
    fn spec_filter_detects_keywords() {
        assert!(is_spec_question("使ってるモデルは？"));
        assert!(is_spec_question("LLMは何？"));
    }

    #[test]
    fn spec_filter_allows_normal_text() {
        assert!(!is_spec_question("今日の天気は？"));
    }
}
