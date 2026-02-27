//! app モジュール（対話オーケストレーション層）
//! 現状は MVP 用のシンプル実装で、session からの音声バッファを受け取り
//! ai ポートを呼び出してボット音声(WAV)のパスを session に返す。
//! transport/sip/rtp には依存せず、SessionOut 経由のイベントのみを返す。

mod router;
mod sentence_accumulator;
mod wav_stream_chunker;

use std::future::pending;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout};
use tokio_stream::StreamExt;

use crate::protocol::session::types::CallId;
use crate::protocol::session::SessionOut;
use crate::service::call_control::router::{
    parse_intent_json, router_config, system_info_response, RouteAction, Router,
};
use crate::service::call_control::sentence_accumulator::SentenceAccumulator;
use crate::service::call_control::wav_stream_chunker::WavStreamChunker;
use crate::shared::config::{self, AppRuntimeConfig};
use crate::shared::error::ai::TtsError;
use crate::shared::ports::ai::{
    AiServices, AsrChunk, AsrStreamHandle, AsrStreamPort, ChatMessage, LlmStreamEvent,
    LlmStreamPort, Role, SerInputPcm, TtsStream, TtsStreamPort, WeatherQuery,
};
use crate::shared::ports::notification::{
    NotificationFuture, NotificationService as NotificationPort,
};
use crate::shared::ports::phone_lookup::PhoneLookupPort;
use crate::shared::utils::{mask_phone, mask_pii};

pub use crate::shared::ports::app::{
    app_event_channel, audio_chunk_channel, AppEvent, AppEventRx, AppEventTx, AudioChunkRx,
    AudioChunkTx, EndReason,
};
pub use crate::shared::ports::notification::NotificationService as AppNotificationPort;

const SORRY_WAV_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/data/zundamon_sorry.wav");
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

const APP_EVENT_CHANNEL_CAPACITY: usize = 16;
const APP_HISTORY_MAX_MESSAGES: usize = 20;

/// Starts and spawns an AppWorker task for the given call.
///
/// The spawned worker processes `AppEvent` messages for the specified `call_id` and performs
/// speech, NLU, routing, and notification handling.
///
/// # Parameters
///
/// - `call_id`: identifier for the call the worker will handle.
/// - `session_out_tx`: channel used by the worker to send `SessionOut` updates back to the session.
/// - `ai_port`: AI service port used for ASR, NLU, TTS, weather, and related operations.
/// - `phone_lookup`: optional phone lookup service used to resolve caller information.
/// - `notification_port`: notification service used to emit ringing/missed/ended notifications.
///
/// # Returns
///
/// An `AppEventTx` that can be used to send events to the spawned worker.
///
/// # Examples
///
/// ```no_run
/// use std::sync::Arc;
/// use tokio::sync::mpsc::channel;
/// use virtual_voicebot_backend::ai::DefaultAiPort;
/// use virtual_voicebot_backend::app::{spawn_app_worker, AppEvent};
/// use virtual_voicebot_backend::config::AppRuntimeConfig;
/// use virtual_voicebot_backend::entities::CallId;
/// use virtual_voicebot_backend::notification::NoopNotification;
/// use virtual_voicebot_backend::ports::ai::AiServices;
/// use virtual_voicebot_backend::ports::notification::NotificationService;
/// use virtual_voicebot_backend::ports::phone_lookup::{NoopPhoneLookup, PhoneLookupPort};
/// use virtual_voicebot_backend::session::SessionOut;
///
/// let (session_tx, _session_rx) = channel::<(CallId, SessionOut)>(128);
/// let ai_port: Arc<dyn AiServices> = Arc::new(DefaultAiPort::new());
/// let phone_lookup: Arc<dyn PhoneLookupPort> = Arc::new(NoopPhoneLookup::new());
/// let notification_port: Arc<dyn NotificationService> = Arc::new(NoopNotification::new());
/// let tx = spawn_app_worker(
///     CallId::new("call-123").unwrap(),
///     session_tx,
///     ai_port,
///     None,
///     None,
///     None,
///     None,
///     phone_lookup,
///     notification_port,
///     AppRuntimeConfig::from_env(),
/// );
/// let _ = tx.try_send(AppEvent::CallStarted {
///     call_id: CallId::new("call-123").unwrap(),
///     caller: None,
/// });
/// ```
#[allow(clippy::too_many_arguments)]
pub fn spawn_app_worker(
    call_id: CallId,
    session_out_tx: mpsc::Sender<(CallId, SessionOut)>,
    ai_port: Arc<dyn AiServices>,
    llm_stream_port: Option<Arc<dyn LlmStreamPort>>,
    asr_stream_port: Option<Arc<dyn AsrStreamPort>>,
    tts_stream_port: Option<Arc<dyn TtsStreamPort>>,
    audio_chunk_rx: Option<AudioChunkRx>,
    phone_lookup: Arc<dyn PhoneLookupPort>,
    notification_port: Arc<dyn NotificationPort>,
    app_cfg: AppRuntimeConfig,
) -> AppEventTx {
    let (tx, rx) = app_event_channel(APP_EVENT_CHANNEL_CAPACITY);
    let worker = AppWorker::new(
        call_id,
        session_out_tx,
        rx,
        ai_port,
        llm_stream_port,
        asr_stream_port,
        tts_stream_port,
        audio_chunk_rx,
        phone_lookup,
        notification_port,
        app_cfg,
    );
    tokio::spawn(async move { worker.run().await });
    tx
}

struct AppWorker {
    call_id: CallId,
    session_out_tx: mpsc::Sender<(CallId, SessionOut)>,
    rx: AppEventRx,
    active: bool,
    history: Vec<ChatMessage>,
    ai_port: Arc<dyn AiServices>,
    llm_stream_port: Option<Arc<dyn LlmStreamPort>>,
    asr_stream_port: Option<Arc<dyn AsrStreamPort>>,
    tts_stream_port: Option<Arc<dyn TtsStreamPort>>,
    audio_chunk_rx: Option<AudioChunkRx>,
    phone_lookup: Arc<dyn PhoneLookupPort>,
    router: Router,
    notification_port: Arc<dyn NotificationPort>,
    notification_state: NotificationState,
    app_cfg: AppRuntimeConfig,
    next_stream_generation_id: u64,
    asr_stream_handle: Option<AsrStreamHandle>,
    asr_stream_connect_failed_for_turn: bool,
}

#[derive(Debug, Default)]
struct NotificationState {
    ringing_notified: bool,
    missed_notified: bool,
    ended_notified: bool,
}

#[derive(Debug)]
struct TtsStreamEarlyStartError {
    error: TtsError,
    emitted_segments: usize,
}

impl AppWorker {
    /// Creates a new AppWorker initialized for the given call.
    ///
    /// The returned worker is inactive, has an empty chat history, a fresh Router,
    /// and a default NotificationState ready to track ringing/missed/ended notifications.
    ///
    /// # Parameters
    ///
    /// - `call_id`: identifier for the call this worker will manage.
    /// - `session_out_tx`: channel to send SessionOut updates back to the session.
    /// - `rx`: receiver for AppEvent messages destined for this worker.
    /// - `ai_port`: AI service port (transcription, NLU, TTS, etc.).
    /// - `phone_lookup`: phone lookup service port.
    /// - `notification_port`: notification service used to emit ringing/missed/ended notifications.
    ///
    /// # Returns
    ///
    /// A configured `AppWorker` instance ready to be spawned.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use std::sync::Arc;
    ///
    /// use crate::shared::ports::app::app_event_channel;
    /// use tokio::sync::mpsc::channel;
    ///
    /// // Create channels
    /// let (session_tx, _session_rx) =
    ///     channel::<(crate::protocol::session::types::CallId, crate::protocol::session::SessionOut)>(128);
    /// let (_event_tx, event_rx) = app_event_channel(16);
    ///
    /// // Placeholder implementations for required ports would be provided in real code.
    /// // Here we show the shape of the call; replace `ai_impl` and `phone_impl` with real Arcs.
    /// let ai_impl: Arc<dyn crate::shared::ports::ai::AiServices> = Arc::new(crate::service::ai::DefaultAiPort::new());
    /// let phone_impl: Arc<dyn crate::shared::ports::phone_lookup::PhoneLookupPort> =
    ///     Arc::new(crate::shared::ports::phone_lookup::NoopPhoneLookup::new());
    /// let notif_impl: Arc<dyn crate::shared::ports::notification::NotificationService> =
    ///     Arc::new(crate::interface::notification::NoopNotification::new());
    ///
    /// let worker = crate::service::call_control::AppWorker::new(
    ///     crate::protocol::session::types::CallId::new("call-123").unwrap(),
    ///     session_tx,
    ///     event_rx,
    ///     ai_impl,
    ///     phone_impl,
    ///     notif_impl,
    ///     crate::shared::config::AppRuntimeConfig::from_env(),
    /// );
    /// ```
    #[allow(clippy::too_many_arguments)]
    fn new(
        call_id: CallId,
        session_out_tx: mpsc::Sender<(CallId, SessionOut)>,
        rx: AppEventRx,
        ai_port: Arc<dyn AiServices>,
        llm_stream_port: Option<Arc<dyn LlmStreamPort>>,
        asr_stream_port: Option<Arc<dyn AsrStreamPort>>,
        tts_stream_port: Option<Arc<dyn TtsStreamPort>>,
        audio_chunk_rx: Option<AudioChunkRx>,
        phone_lookup: Arc<dyn PhoneLookupPort>,
        notification_port: Arc<dyn NotificationPort>,
        app_cfg: AppRuntimeConfig,
    ) -> Self {
        Self {
            call_id,
            session_out_tx,
            rx,
            active: false,
            history: Vec::new(),
            ai_port,
            llm_stream_port,
            asr_stream_port,
            tts_stream_port,
            audio_chunk_rx,
            phone_lookup,
            router: Router::new(),
            notification_port,
            notification_state: NotificationState::default(),
            app_cfg,
            next_stream_generation_id: 1,
            asr_stream_handle: None,
            asr_stream_connect_failed_for_turn: false,
        }
    }

    /// Process incoming `AppEvent` messages for this worker until the call finishes.
    ///
    /// This method continuously receives events from the internal channel and handles them:
    /// - `CallRinging`: logs mismatched call IDs and triggers ringing notification.
    /// - `CallStarted`: logs caller presence, marks the worker active, and performs phone lookup.
    /// - `AudioBuffered`: if the call is active, processes the audio buffer; otherwise drops it.
    /// - `CallEnded`: logs mismatched call IDs, triggers end/missed notifications as appropriate, and stops the loop.
    ///
    /// Mismatched `call_id` values are logged as warnings but do not stop processing other events.
    ///
    /// # Examples
    ///
    /// ```
    /// // Spawn the worker's event loop (types omitted for brevity).
    /// // let worker = AppWorker::new(...);
    /// // tokio::spawn(async move { worker.run().await });
    /// ```
    async fn run(mut self) {
        loop {
            tokio::select! {
                ev = self.rx.recv() => {
                    let Some(ev) = ev else {
                        break;
                    };
                    if !self.handle_app_event(ev).await {
                        break;
                    }
                }
                chunk = async {
                    match &self.audio_chunk_rx {
                        Some(rx) => rx.recv().await,
                        None => pending::<Option<crate::shared::ports::app::RtpAudioChunk>>().await,
                    }
                } => {
                    match chunk {
                        Some(chunk) => {
                            if chunk.call_id != self.call_id {
                                log::warn!(
                                    "[app {}] AudioChunk received for mismatched call_id={}",
                                    self.call_id,
                                    chunk.call_id
                                );
                            } else if self.active {
                                self.handle_audio_chunk(&chunk.call_id, chunk.pcm_mulaw).await;
                            }
                        }
                        None => {
                            self.audio_chunk_rx = None;
                            self.close_asr_stream_handle_best_effort();
                        }
                    }
                }
            }
        }
    }
}

impl AppWorker {
    fn close_asr_stream_handle_best_effort(&mut self) {
        if let Some(handle) = self.asr_stream_handle.take() {
            // Do not block the app loop on stream shutdown; best-effort EOS is enough here.
            let _ = handle.audio_tx.try_send_end();
        }
    }

    async fn handle_app_event(&mut self, ev: AppEvent) -> bool {
        match ev {
            AppEvent::CallRinging {
                call_id,
                from,
                timestamp,
            } => {
                if call_id != self.call_id {
                    log::warn!(
                        "[app {}] CallRinging received for mismatched call_id={}",
                        self.call_id,
                        call_id
                    );
                    return true;
                }
                self.notify_ringing(call_id.clone(), from, timestamp);
                true
            }
            AppEvent::CallStarted { call_id, caller } => {
                if call_id != self.call_id {
                    log::warn!(
                        "[app {}] CallStarted received for mismatched call_id={}",
                        self.call_id,
                        call_id
                    );
                    return true;
                }
                self.active = true;
                let caller_display = caller.as_deref().filter(|value| !value.trim().is_empty());
                if let Some(value) = caller_display {
                    log::debug!(
                        "[app {}] caller extracted={}",
                        self.call_id,
                        mask_phone(value)
                    );
                } else {
                    log::debug!("[app {}] caller missing", self.call_id);
                }
                self.handle_phone_lookup(caller).await;
                true
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
                    return true;
                }
                if !self.active {
                    log::debug!(
                        "[app {}] dropped audio because call not active",
                        self.call_id
                    );
                    return true;
                }
                if let Err(e) = self
                    .handle_audio_buffer(&call_id, pcm_mulaw, pcm_linear16)
                    .await
                {
                    log::warn!("[app {}] audio handling failed: {:?}", self.call_id, e);
                }
                true
            }
            AppEvent::CallEnded {
                call_id,
                from,
                reason,
                duration_sec,
                timestamp,
            } => {
                if call_id != self.call_id {
                    log::warn!(
                        "[app {}] CallEnded received for mismatched call_id={}",
                        self.call_id,
                        call_id
                    );
                    return true;
                }
                self.close_asr_stream_handle_best_effort();
                self.notify_ended(call_id.as_str(), from, reason, duration_sec, timestamp);
                false
            }
        }
    }

    /// Processes an incoming audio buffer for a call: performs ASR and SER, classifies intent,
    /// routes to the appropriate action (fixed response, system info, chat, weather, or transfer),
    /// updates conversation history, synthesizes a reply to WAV, and sends session outputs
    /// (bot audio or transfer requests) back to the session.
    ///
    /// The function handles failures at each stage by logging and falling back to safe responses:
    /// - On ASR failure it uses a Japanese apology text.
    /// - On empty ASR result it plays a fixed sorry WAV file.
    /// - On intent classification or LLM failures it uses sensible defaults or router-configured messages.
    /// - Transfer actions send a transfer request or a not-found prompt and return early.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use std::sync::Arc;
    /// # use virtual_voicebot_backend::entities::CallId;
    /// # async fn example() {
    /// // `worker` is an instance of the surrounding type that provides `handle_audio_buffer`.
    /// // This example demonstrates the call pattern; constructing a full `AppWorker` requires
    /// // multiple dependencies not shown here.
    /// let call_id = CallId::new("call-123").unwrap();
    /// let pcm_mulaw: Vec<u8> = vec![]; // mu-law encoded bytes from the session
    /// let pcm_linear16: Vec<i16> = vec![]; // linear16 PCM samples for SER
    /// // await the handler
    /// // worker.handle_audio_buffer(&call_id, pcm_mulaw, pcm_linear16).await.unwrap();
    /// # }
    /// ```
    async fn handle_audio_chunk(&mut self, call_id: &CallId, pcm_mulaw: Vec<u8>) {
        let Some(port) = &self.asr_stream_port else {
            return;
        };
        if self.asr_stream_connect_failed_for_turn {
            return;
        }

        if self.asr_stream_handle.is_none() {
            let url = config::asr_streaming_server_url();
            match port.transcribe_stream(call_id.to_string(), url).await {
                Ok(handle) => {
                    self.asr_stream_handle = Some(handle);
                    self.asr_stream_connect_failed_for_turn = false;
                }
                Err(e) => {
                    self.asr_stream_connect_failed_for_turn = true;
                    log::warn!(
                        "[asr stream {call_id}] connection failed: {e}; fallback to sequential"
                    );
                    return;
                }
            }
        }

        if let Some(handle) = &self.asr_stream_handle {
            let _ = handle.audio_tx.try_send_chunk_latest(pcm_mulaw);
        }
    }

    async fn take_streaming_asr_result_or_fallback(
        &mut self,
        call_id: &CallId,
        pcm_mulaw: Vec<u8>,
    ) -> String {
        self.asr_stream_connect_failed_for_turn = false;
        let Some(handle) = self.asr_stream_handle.take() else {
            return self.transcribe_asr(call_id, pcm_mulaw).await;
        };

        if handle.audio_tx.send_end().await.is_err() {
            log::warn!("[asr stream {call_id}] failed to send EOS; fallback to sequential");
            return self.transcribe_asr(call_id, pcm_mulaw).await;
        }

        match handle.final_rx.await {
            Ok(Ok(text)) => text,
            Ok(Err(e)) => {
                log::warn!(
                    "[asr stream {call_id}] consumer task error: {e}; fallback to sequential"
                );
                self.transcribe_asr(call_id, pcm_mulaw).await
            }
            Err(_) => {
                log::warn!("[asr stream {call_id}] consumer task dropped; fallback to sequential");
                self.transcribe_asr(call_id, pcm_mulaw).await
            }
        }
    }

    async fn handle_audio_buffer(
        &mut self,
        call_id: &CallId,
        pcm_mulaw: Vec<u8>,
        pcm_linear16: Vec<i16>,
    ) -> anyhow::Result<()> {
        self.analyze_ser(call_id, pcm_linear16).await;
        let user_text = self
            .take_streaming_asr_result_or_fallback(call_id, pcm_mulaw)
            .await;

        let trimmed = user_text.trim();
        if trimmed.is_empty() {
            log::debug!("[app {call_id}] empty ASR text after filtering, playing sorry audio");
            let _ = self
                .session_out_tx
                .send((
                    self.call_id.clone(),
                    SessionOut::AppSendBotAudioFile {
                        path: SORRY_WAV_PATH.to_string(),
                    },
                ))
                .await;
            return Ok(());
        }

        self.handle_user_text(call_id, trimmed).await
    }

    async fn transcribe_asr(&self, call_id: &CallId, pcm_mulaw: Vec<u8>) -> String {
        let asr_chunks = vec![AsrChunk {
            pcm_mulaw,
            end: true,
        }];
        let call_id_str = call_id.to_string();
        match self
            .ai_port
            .transcribe_chunks(call_id_str, asr_chunks)
            .await
        {
            Ok(t) => t,
            Err(e) => {
                log::warn!("[app {call_id}] ASR failed: {e:?}");
                "すみません、聞き取れませんでした。".to_string()
            }
        }
    }

    async fn analyze_ser(&self, call_id: &CallId, pcm_linear16: Vec<i16>) {
        let ser_input = SerInputPcm {
            session_id: call_id.to_string(),
            stream_id: "main".to_string(),
            pcm: pcm_linear16,
            sample_rate: 8000,
            channels: 1,
        };
        match self.ai_port.analyze(ser_input).await {
            Ok(result) => {
                log::debug!(
                    "[app {call_id}] ser emotion={:?} confidence={:.2}",
                    result.emotion,
                    result.confidence
                );
            }
            Err(err) => {
                log::warn!("[app {call_id}] SER failed: {err}");
            }
        }
    }

    async fn handle_user_text(&mut self, call_id: &CallId, trimmed: &str) -> anyhow::Result<()> {
        if is_spec_question(trimmed) {
            log::warn!(
                "[security] spec question blocked: call_id={} input={}",
                call_id,
                mask_pii(trimmed)
            );
            let answer_text = system_info_response();
            match self
                .ai_port
                .synth_to_wav(call_id.to_string(), answer_text, None)
                .await
            {
                Ok(bot_wav) => {
                    let _ = self
                        .session_out_tx
                        .send((
                            self.call_id.clone(),
                            SessionOut::AppSendBotAudioFile {
                                path: bot_wav.to_string_lossy().to_string(),
                            },
                        ))
                        .await;
                }
                Err(e) => {
                    log::warn!("[app {call_id}] TTS failed: {e:?}");
                }
            }
            return Ok(());
        }

        let intent_json = match self
            .ai_port
            .classify_intent(call_id.to_string(), trimmed.to_string())
            .await
        {
            Ok(raw) => raw,
            Err(err) => {
                log::warn!("[app {call_id}] intent classify failed: {err:?}");
                "{\"intent\":\"general_chat\",\"query\":\"\"}".to_string()
            }
        };
        let intent_result = parse_intent_json(&intent_json, trimmed);
        let intent_json_len = intent_json.chars().count();
        log::debug!(
            "[app {call_id}] intent classified={} raw_len={}",
            intent_result.raw_intent,
            intent_json_len
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
                if config::voicebot_streaming_enabled() && self.llm_stream_port.is_some() {
                    return self
                        .handle_user_text_streaming(call_id, query, messages)
                        .await;
                }
                return self
                    .handle_user_text_sequential(call_id, query, messages)
                    .await;
            }
            RouteAction::Weather {
                query,
                location,
                date,
            } => {
                let req = WeatherQuery { location, date };
                let answer_text = match self.ai_port.handle_weather(call_id.to_string(), req).await
                {
                    Ok(text) => text,
                    Err(err) => {
                        log::warn!("[app {call_id}] weather failed: {err:?}");
                        router_config().weather_error_response.clone()
                    }
                };
                (answer_text, query)
            }
            RouteAction::Transfer { person } => {
                let target = self.router.resolve_transfer_person(person.as_str());
                if let Some(resolved) = target {
                    let confirm_message = self.router.transfer_confirm_message();
                    match self
                        .ai_port
                        .synth_to_wav(call_id.to_string(), confirm_message.clone(), None)
                        .await
                    {
                        Ok(bot_wav) => {
                            let _ = self
                                .session_out_tx
                                .send((
                                    self.call_id.clone(),
                                    SessionOut::AppSendBotAudioFile {
                                        path: bot_wav.to_string_lossy().to_string(),
                                    },
                                ))
                                .await;
                        }
                        Err(e) => {
                            log::warn!("[app {call_id}] transfer TTS failed: {e:?}");
                        }
                    }
                    let _ = self
                        .session_out_tx
                        .send((
                            self.call_id.clone(),
                            SessionOut::AppRequestTransfer { person: resolved },
                        ))
                        .await;
                } else {
                    let not_found = self.router.transfer_not_found_message();
                    match self
                        .ai_port
                        .synth_to_wav(call_id.to_string(), not_found.clone(), None)
                        .await
                    {
                        Ok(bot_wav) => {
                            let _ = self
                                .session_out_tx
                                .send((
                                    self.call_id.clone(),
                                    SessionOut::AppSendBotAudioFile {
                                        path: bot_wav.to_string_lossy().to_string(),
                                    },
                                ))
                                .await;
                        }
                        Err(e) => {
                            log::warn!("[app {call_id}] transfer not-found TTS failed: {e:?}");
                        }
                    }
                }
                return Ok(());
            }
        };

        self.push_history(user_query, answer_text.clone());

        // TTS
        match self
            .ai_port
            .synth_to_wav(call_id.to_string(), answer_text, None)
            .await
        {
            Ok(bot_wav) => {
                let _ = self
                    .session_out_tx
                    .send((
                        self.call_id.clone(),
                        SessionOut::AppSendBotAudioFile {
                            path: bot_wav.to_string_lossy().to_string(),
                        },
                    ))
                    .await;
            }
            Err(e) => {
                log::warn!("[app {call_id}] TTS failed: {e:?}");
            }
        }
        Ok(())
    }

    fn push_history(&mut self, user_query: String, answer_text: String) {
        self.history.push(ChatMessage {
            role: Role::User,
            content: user_query,
        });
        self.history.push(ChatMessage {
            role: Role::Assistant,
            content: answer_text,
        });
        if self.history.len() > APP_HISTORY_MAX_MESSAGES {
            let excess = self.history.len() - APP_HISTORY_MAX_MESSAGES;
            self.history.drain(0..excess);
        }
    }

    fn next_stream_generation_id(&mut self) -> u64 {
        let id = self.next_stream_generation_id;
        self.next_stream_generation_id = self.next_stream_generation_id.wrapping_add(1).max(1);
        id
    }

    async fn handle_user_text_sequential(
        &mut self,
        call_id: &CallId,
        user_query: String,
        messages: Vec<ChatMessage>,
    ) -> anyhow::Result<()> {
        let answer_text = match self
            .ai_port
            .generate_answer(call_id.to_string(), messages)
            .await
        {
            Ok(ans) => ans,
            Err(e) => {
                log::warn!("[app {call_id}] LLM failed: {e:?}");
                "すみません、うまく答えを用意できませんでした。".to_string()
            }
        };

        self.push_history(user_query, answer_text.clone());

        match self
            .ai_port
            .synth_to_wav(call_id.to_string(), answer_text, None)
            .await
        {
            Ok(bot_wav) => {
                let _ = self
                    .session_out_tx
                    .send((
                        self.call_id.clone(),
                        SessionOut::AppSendBotAudioFile {
                            path: bot_wav.to_string_lossy().to_string(),
                        },
                    ))
                    .await;
            }
            Err(e) => {
                log::warn!("[app {call_id}] TTS failed: {e:?}");
            }
        }
        Ok(())
    }

    async fn handle_user_text_streaming(
        &mut self,
        call_id: &CallId,
        user_query: String,
        messages: Vec<ChatMessage>,
    ) -> anyhow::Result<()> {
        let Some(llm_stream_port) = self.llm_stream_port.clone() else {
            return self
                .handle_user_text_sequential(call_id, user_query, messages)
                .await;
        };

        let stream = match llm_stream_port
            .generate_answer_stream(call_id.to_string(), messages.clone())
            .await
        {
            Ok(stream) => stream,
            Err(e) => {
                log::warn!("[app {call_id}] LLM stream start failed: {e}, fallback to sequential");
                return self
                    .handle_user_text_sequential(call_id, user_query, messages)
                    .await;
            }
        };

        let (sentence_tx, mut sentence_rx) =
            mpsc::channel::<String>(config::sentence_channel_capacity());
        let generation_id = self.next_stream_generation_id();
        let first_token_timeout = config::llm_streaming_first_token_timeout();
        let idle_timeout = config::sentence_max_wait();
        let total_timeout = config::llm_streaming_total_timeout();
        let max_chars = config::sentence_max_chars();
        let call_id_for_consumer = call_id.to_string();

        let consumer = tokio::spawn(async move {
            tokio::pin!(stream);
            let mut acc = SentenceAccumulator::new(max_chars);
            let mut first_token_received = false;
            let mut full_answer = String::new();
            let mut sentences_sent = 0usize;
            let mut had_error = false;

            let timed = timeout(total_timeout, async {
                loop {
                    let wait_duration = if first_token_received {
                        idle_timeout
                    } else {
                        first_token_timeout
                    };
                    tokio::select! {
                        item = stream.next() => {
                            match item {
                                Some(Ok(event)) => {
                                    match event {
                                        LlmStreamEvent::Token(token) => {
                                            first_token_received = true;
                                            full_answer.push_str(&token);
                                            if let Some(sentence) = acc.push(&token) {
                                                sentences_sent += 1;
                                                if sentence_tx.send(sentence).await.is_err() {
                                                    break;
                                                }
                                            }
                                        }
                                        LlmStreamEvent::End => {
                                            if let Some(tail) = acc.flush() {
                                                sentences_sent += 1;
                                                let _ = sentence_tx.send(tail).await;
                                            }
                                            break;
                                        }
                                    }
                                }
                                Some(Err(e)) => {
                                    log::warn!(
                                        "[app {}] LLM stream error: {e}",
                                        call_id_for_consumer
                                    );
                                    had_error = true;
                                    if let Some(tail) = acc.flush() {
                                        sentences_sent += 1;
                                        let _ = sentence_tx.send(tail).await;
                                    }
                                    break;
                                }
                                None => {
                                    // Expected normal end is explicit `LlmStreamEvent::End`.
                                    // `None` here means producer ended without terminal event.
                                    had_error = true;
                                    if let Some(tail) = acc.flush() {
                                        sentences_sent += 1;
                                        let _ = sentence_tx.send(tail).await;
                                    }
                                    break;
                                }
                            }
                        }
                        _ = sleep(wait_duration) => {
                            log::warn!(
                                "[app {}] LLM stream timeout: phase={}",
                                call_id_for_consumer,
                                if first_token_received { "chunk-idle" } else { "first-token" }
                            );
                            had_error = true;
                            if let Some(flushed) = acc.flush() {
                                sentences_sent += 1;
                                let _ = sentence_tx.send(flushed).await;
                            }
                            break;
                        }
                    }
                }
            })
            .await;

            if timed.is_err() {
                log::warn!("[app {}] LLM stream total timeout", call_id_for_consumer);
                had_error = true;
                if let Some(flushed) = acc.flush() {
                    sentences_sent += 1;
                    let _ = sentence_tx.send(flushed).await;
                }
            }

            (full_answer, sentences_sent, had_error)
        });

        while let Some(sentence) = sentence_rx.recv().await {
            self.enqueue_tts_sentence(call_id, sentence, generation_id)
                .await;
        }

        let (full_answer, sentences_sent, had_error) = match consumer.await {
            Ok(result) => result,
            Err(e) => {
                log::warn!("[app {call_id}] LLM stream consumer join failed: {e}");
                (String::new(), 0, true)
            }
        };

        if had_error && sentences_sent == 0 {
            log::warn!("[app {call_id}] streaming failed with 0 sentences, fallback to sequential");
            return self
                .handle_user_text_sequential(call_id, user_query, messages)
                .await;
        }

        if !full_answer.trim().is_empty() {
            self.push_history(user_query, full_answer);
        }
        Ok(())
    }

    async fn enqueue_tts_sentence(
        &mut self,
        call_id: &CallId,
        sentence: String,
        generation_id: u64,
    ) {
        if config::voicebot_streaming_enabled()
            && config::voicebot_tts_streaming_enabled()
            && self
                .try_enqueue_tts_sentence_streaming(call_id, &sentence, generation_id)
                .await
        {
            return;
        }

        if let Err(e) = self
            .enqueue_tts_sentence_sequential(call_id, sentence, generation_id)
            .await
        {
            log::warn!("[app {call_id}] TTS failed: {e:?}");
        }
    }

    async fn enqueue_tts_sentence_sequential(
        &mut self,
        call_id: &CallId,
        sentence: String,
        generation_id: u64,
    ) -> Result<(), TtsError> {
        let wav_path = self
            .ai_port
            .synth_to_wav(call_id.to_string(), sentence, None)
            .await?;
        self.enqueue_streaming_bot_audio_file(wav_path, generation_id)
            .await;
        Ok(())
    }

    async fn try_enqueue_tts_sentence_streaming(
        &mut self,
        call_id: &CallId,
        sentence: &str,
        generation_id: u64,
    ) -> bool {
        let Some(port) = self.tts_stream_port.clone() else {
            return false;
        };

        let stream = match port
            .synth_stream(call_id.to_string(), sentence.to_string())
            .await
        {
            Ok(stream) => stream,
            Err(e) => {
                log::warn!("[tts stream {call_id}] synth_stream start failed: {e}; fallback");
                return false;
            }
        };

        if config::tts_streaming_early_start_enabled() {
            match self
                .stream_tts_sentence_early_start(call_id, stream, generation_id)
                .await
            {
                Ok(()) => return true,
                Err(err) => {
                    if err.emitted_segments > 0 {
                        log::warn!(
                            "[tts stream {call_id}] early-start failed after {} segment(s): {}; keep partial playback",
                            err.emitted_segments,
                            err.error
                        );
                        return true;
                    }
                    log::warn!(
                        "[tts stream {call_id}] early-start failed before first segment: {}; fallback",
                        err.error
                    );
                    return false;
                }
            }
        }

        match self
            .collect_tts_stream_to_tmp(call_id, stream, generation_id)
            .await
        {
            Ok(path) => {
                self.enqueue_streaming_bot_audio_file(path, generation_id)
                    .await;
                true
            }
            Err(e) => {
                log::warn!("[tts stream {call_id}] collect/write failed: {e}; fallback");
                false
            }
        }
    }

    async fn stream_tts_sentence_early_start(
        &mut self,
        call_id: &CallId,
        stream: TtsStream,
        generation_id: u64,
    ) -> Result<(), TtsStreamEarlyStartError> {
        let first_chunk_timeout = config::tts_streaming_first_chunk_timeout();
        let total_timeout = config::tts_streaming_total_timeout();
        let mut emitted_segments = 0usize;
        let mut next_segment_index = 0usize;
        let mut chunker = WavStreamChunker::new(config::tts_streaming_early_start_bytes());

        let streamed = timeout(total_timeout, async {
            tokio::pin!(stream);

            let first = tokio::select! {
                item = stream.next() => item,
                _ = sleep(first_chunk_timeout) => {
                    return Err(TtsError::SynthesisFailed(format!(
                        "first chunk timeout ({} ms)",
                        first_chunk_timeout.as_millis()
                    )));
                }
            };

            match first {
                Some(Ok(bytes)) => {
                    let wav_segments = chunker.push(&bytes).map_err(|e| {
                        TtsError::SynthesisFailed(format!("wav stream parse failed: {e}"))
                    })?;
                    for wav in wav_segments {
                        self.write_and_enqueue_tts_stream_segment(
                            call_id,
                            generation_id,
                            next_segment_index,
                            wav,
                        )
                        .await?;
                        emitted_segments += 1;
                        next_segment_index += 1;
                    }
                }
                Some(Err(e)) => return Err(e),
                None => return Err(TtsError::SynthesisFailed("empty TTS stream".to_string())),
            }

            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        let wav_segments = chunker.push(&bytes).map_err(|e| {
                            TtsError::SynthesisFailed(format!("wav stream parse failed: {e}"))
                        })?;
                        for wav in wav_segments {
                            self.write_and_enqueue_tts_stream_segment(
                                call_id,
                                generation_id,
                                next_segment_index,
                                wav,
                            )
                            .await?;
                            emitted_segments += 1;
                            next_segment_index += 1;
                        }
                    }
                    Err(e) => return Err(e),
                }
            }

            if let Some(wav) = chunker.finish().map_err(|e| {
                TtsError::SynthesisFailed(format!("wav stream finalize failed: {e}"))
            })? {
                self.write_and_enqueue_tts_stream_segment(
                    call_id,
                    generation_id,
                    next_segment_index,
                    wav,
                )
                .await?;
                emitted_segments += 1;
            }

            if emitted_segments == 0 {
                return Err(TtsError::SynthesisFailed("empty TTS stream".to_string()));
            }
            Ok(())
        })
        .await
        .map_err(|_| {
            TtsError::SynthesisFailed(format!("total timeout ({} ms)", total_timeout.as_millis()))
        });

        match streamed {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(TtsStreamEarlyStartError {
                error,
                emitted_segments,
            }),
            Err(error) => Err(TtsStreamEarlyStartError {
                error,
                emitted_segments,
            }),
        }
    }

    async fn collect_tts_stream_to_tmp(
        &self,
        call_id: &CallId,
        stream: TtsStream,
        generation_id: u64,
    ) -> Result<PathBuf, TtsError> {
        let first_chunk_timeout = config::tts_streaming_first_chunk_timeout();
        let total_timeout = config::tts_streaming_total_timeout();
        let tmp_path = self.tts_stream_tmp_path(call_id, generation_id);

        let wav_bytes = timeout(total_timeout, async {
            tokio::pin!(stream);
            let mut buf = Vec::<u8>::new();

            let first = tokio::select! {
                item = stream.next() => item,
                _ = sleep(first_chunk_timeout) => {
                    return Err(TtsError::SynthesisFailed(format!(
                        "first chunk timeout ({} ms)",
                        first_chunk_timeout.as_millis()
                    )));
                }
            };

            match first {
                Some(Ok(bytes)) => buf.extend_from_slice(&bytes),
                Some(Err(e)) => return Err(e),
                None => return Err(TtsError::SynthesisFailed("empty TTS stream".to_string())),
            }

            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(bytes) => buf.extend_from_slice(&bytes),
                    Err(e) => return Err(e),
                }
            }

            if buf.is_empty() {
                return Err(TtsError::SynthesisFailed("empty TTS stream".to_string()));
            }

            Ok(buf)
        })
        .await
        .map_err(|_| {
            TtsError::SynthesisFailed(format!("total timeout ({} ms)", total_timeout.as_millis()))
        })??;

        tokio::fs::write(&tmp_path, &wav_bytes)
            .await
            .map_err(|e| TtsError::SynthesisFailed(format!("tmp wav write failed: {e}")))?;
        Ok(tmp_path)
    }

    fn tts_stream_tmp_path(&self, call_id: &CallId, generation_id: u64) -> PathBuf {
        self.tts_stream_segment_tmp_path(call_id, generation_id, 0)
    }

    fn tts_stream_segment_tmp_path(
        &self,
        call_id: &CallId,
        generation_id: u64,
        segment_index: usize,
    ) -> PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let safe_call_id: String = call_id
            .to_string()
            .chars()
            .map(|c| match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' | '.' | '@' => c,
                _ => '_',
            })
            .collect();
        PathBuf::from(format!(
            "/tmp/tts_stream_output_{}_{}_{}_{}.wav",
            safe_call_id, generation_id, segment_index, ts
        ))
    }

    async fn write_and_enqueue_tts_stream_segment(
        &self,
        call_id: &CallId,
        generation_id: u64,
        segment_index: usize,
        wav_bytes: Vec<u8>,
    ) -> Result<(), TtsError> {
        let tmp_path = self.tts_stream_segment_tmp_path(call_id, generation_id, segment_index);
        tokio::fs::write(&tmp_path, &wav_bytes)
            .await
            .map_err(|e| TtsError::SynthesisFailed(format!("tmp wav write failed: {e}")))?;
        self.enqueue_streaming_bot_audio_file(tmp_path, generation_id)
            .await;
        Ok(())
    }

    async fn enqueue_streaming_bot_audio_file(&self, wav_path: PathBuf, generation_id: u64) {
        let _ = self
            .session_out_tx
            .send((
                self.call_id.clone(),
                SessionOut::AppEnqueueBotAudioFile {
                    path: wav_path.to_string_lossy().to_string(),
                    generation_id,
                },
            ))
            .await;
    }

    /// Triggers a single ringing notification for the call if one has not already been sent.
    ///
    /// This marks ringing as notified in the worker's notification state and schedules the
    /// notification future on the worker's notifier helper. Subsequent calls have no effect.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Assume `worker` is a mutable AppWorker instance.
    /// // The first call schedules a ringing notification; the second call is ignored.
    /// let ts = chrono::FixedOffset::east_opt(9 * 3600).unwrap().now();
    /// worker.notify_ringing(CallId::new("call-123").unwrap(), "+819012345678".to_string(), ts);
    /// worker.notify_ringing(CallId::new("call-123").unwrap(), " +819012345678".to_string(), ts);
    /// ```
    fn notify_ringing(
        &mut self,
        call_id: CallId,
        from: String,
        timestamp: chrono::DateTime<chrono::FixedOffset>,
    ) {
        if self.notification_state.ringing_notified {
            return;
        }
        self.notification_state.ringing_notified = true;
        let fut = self
            .notification_port
            .notify_ringing(call_id, from, timestamp);
        self.spawn_notify("ringing", fut);
    }

    /// Trigger end-of-call notifications according to the provided end reason and throttle duplicate notifications.
    ///
    /// For `EndReason::Cancel` this sends a "missed" notification with the given `timestamp`. For `EndReason::Bye` this sends an "ended" notification using `duration_sec` if present. Each notification type is sent at most once per worker; subsequent calls for the same notification type are ignored. Other end reasons do not produce notifications.
    ///
    /// # Parameters
    ///
    /// - `from`: identifier or display name of the caller used in the notification.
    /// - `reason`: reason the call ended; determines which notification (if any) is sent.
    /// - `duration_sec`: duration of the call in seconds; required for `EndReason::Bye`.
    /// - `timestamp`: timestamp to attach to missed notifications.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Assuming `worker` is a mutable AppWorker instance:
    /// use chrono::FixedOffset;
    /// let timestamp = chrono::Utc::now().with_timezone(&FixedOffset::east(0));
    /// worker.notify_ended("call-123", "alice".to_string(), EndReason::Bye, Some(42), timestamp);
    /// worker.notify_ended("call-123", "alice".to_string(), EndReason::Cancel, None, timestamp);
    /// ```
    fn notify_ended(
        &mut self,
        call_id: &str,
        from: String,
        reason: EndReason,
        duration_sec: Option<u64>,
        timestamp: chrono::DateTime<chrono::FixedOffset>,
    ) {
        match reason {
            EndReason::Cancel => {
                if self.notification_state.missed_notified {
                    return;
                }
                self.notification_state.missed_notified = true;
                let fut = self.notification_port.notify_missed(from, timestamp);
                self.spawn_notify("missed", fut);
            }
            EndReason::Bye => {
                if self.notification_state.ended_notified {
                    return;
                }
                let Some(duration_sec) = duration_sec else {
                    return;
                };
                self.notification_state.ended_notified = true;
                let fut = self
                    .notification_port
                    .notify_ended(call_id, from, duration_sec);
                self.spawn_notify("ended", fut);
            }
            _ => {}
        }
    }

    /// Spawn a background task to run a notification future and log any failure.
    ///
    /// The spawned task awaits the provided `fut`. If the future resolves to an `Err`,
    /// a warning is logged that includes the worker's call id and the provided `label`.
    ///
    /// # Parameters
    ///
    /// - `label`: short static label included in the warning log to identify the notification.
    /// - `fut`: a future that yields a `Result` indicating notification success or failure.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Run a notification in background and log if it fails.
    /// self.spawn_notify(
    ///     "ringing",
    ///     notification_port.notify_ringing(call_id.clone(), from, timestamp),
    /// );
    /// ```
    fn spawn_notify(&self, label: &'static str, fut: NotificationFuture) {
        let call_id = self.call_id.clone();
        tokio::spawn(async move {
            if let Err(err) = fut.await {
                log::warn!("[app {call_id}] notification {label} failed: {err}");
            }
        });
    }

    /// Performs an optional phone-number lookup for the current call.
    ///
    /// If phone lookup is disabled in configuration or the provided `caller` is
    /// missing or empty, the function returns immediately. When a caller string is
    /// present, this triggers the phone lookup port and logs whether the lookup
    /// succeeded, returned no match (treated as default IVR enabled), or failed.
    ///
    /// # Parameters
    ///
    /// - `caller`: optional caller identifier (phone number or caller ID). Empty or
    ///   whitespace-only strings are treated as absent and skipped.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // given a mutable `worker: AppWorker` in scope:
    /// worker.handle_phone_lookup(Some("03-1234-5678".into())).await;
    /// worker.handle_phone_lookup(None).await;
    /// ```
    async fn handle_phone_lookup(&mut self, caller: Option<String>) {
        if !self.app_cfg.phone_lookup_enabled {
            log::debug!("[app {}] phone lookup disabled", self.call_id);
            return;
        }
        let Some(caller) = caller.filter(|v| !v.trim().is_empty()) else {
            log::debug!("[app {}] caller missing, skip lookup", self.call_id);
            return;
        };
        let caller_masked = mask_phone(&caller);
        match self.phone_lookup.lookup_phone(caller.clone()).await {
            Ok(Some(result)) => {
                log::debug!(
                    "[app {}] phone lookup found caller={} category={:?} action_code={} ivr_flow_id={:?}",
                    self.call_id,
                    caller_masked,
                    result.caller_category,
                    result.action_code,
                    result.ivr_flow_id
                );
            }
            Ok(None) => {
                log::debug!(
                    "[app {}] phone lookup not found caller={} (no routing result)",
                    self.call_id,
                    caller_masked
                );
            }
            Err(err) => {
                log::warn!(
                    "[app {}] phone lookup failed caller={}: {}",
                    self.call_id,
                    caller_masked,
                    err
                );
            }
        }
    }

    // build_prompt はロール分離に伴い廃止
}

/// Detects whether a text appears to be a specification or policy question.
///
/// # Returns
///
/// `true` if the input contains any specification-related keywords, `false` otherwise.
///
/// # Examples
///
/// ```ignore
/// assert!(is_spec_question("使ってるモデルは？"));
/// assert!(is_spec_question("LLMは何？"));
/// assert!(!is_spec_question("今日の天気はどうですか？"));
/// ```
fn is_spec_question(input: &str) -> bool {
    let lowered = input.to_ascii_lowercase();
    SPEC_FILTER_KEYWORDS.iter().any(|kw| lowered.contains(kw))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex, OnceLock};

    use futures_util::stream;
    use tokio::sync::{mpsc as tokio_mpsc, Mutex as AsyncMutex};
    use tokio::time::Duration;

    use crate::interface::notification::NoopNotification;
    use crate::shared::error::ai::{
        AsrError, IntentError, LlmError, SerError, TtsError, WeatherError,
    };
    use crate::shared::ports::ai::{
        AiFuture, AsrChunk, AsrPort, ChatMessage, Intent, IntentPort, LlmPort, SerInputPcm,
        SerOutcome, SerPort, TtsPort, TtsStream, TtsStreamPort, WeatherPort, WeatherQuery,
        WeatherResponse,
    };
    use crate::shared::ports::notification::NotificationService;
    use crate::shared::ports::phone_lookup::{NoopPhoneLookup, PhoneLookupPort};

    #[derive(Clone)]
    struct FakeAiPort {
        state: Arc<Mutex<FakeAiState>>,
    }

    #[derive(Debug)]
    struct FakeAiState {
        synth_to_wav_calls: usize,
        synth_to_wav_paths: VecDeque<PathBuf>,
    }

    impl FakeAiPort {
        fn new(paths: Vec<PathBuf>) -> (Self, Arc<Mutex<FakeAiState>>) {
            let state = Arc::new(Mutex::new(FakeAiState {
                synth_to_wav_calls: 0,
                synth_to_wav_paths: paths.into(),
            }));
            (
                Self {
                    state: Arc::clone(&state),
                },
                state,
            )
        }
    }

    impl AsrPort for FakeAiPort {
        fn transcribe_chunks(
            &self,
            _call_id: String,
            _chunks: Vec<AsrChunk>,
        ) -> AiFuture<Result<String, AsrError>> {
            Box::pin(async { Err(AsrError::ServiceUnavailable) })
        }
    }

    impl IntentPort for FakeAiPort {
        fn classify_intent(
            &self,
            _call_id: String,
            _text: String,
        ) -> AiFuture<Result<Intent, IntentError>> {
            Box::pin(async { Err(IntentError::UnknownIntent) })
        }
    }

    impl LlmPort for FakeAiPort {
        fn generate_answer(
            &self,
            _call_id: String,
            _messages: Vec<ChatMessage>,
        ) -> AiFuture<Result<String, LlmError>> {
            Box::pin(async { Err(LlmError::GenerationFailed("unused".to_string())) })
        }
    }

    impl WeatherPort for FakeAiPort {
        fn handle_weather(
            &self,
            _call_id: String,
            _query: WeatherQuery,
        ) -> AiFuture<Result<WeatherResponse, WeatherError>> {
            Box::pin(async { Err(WeatherError::ServiceUnavailable) })
        }
    }

    impl TtsPort for FakeAiPort {
        fn synth_to_wav(
            &self,
            _call_id: String,
            _text: String,
            _path: Option<String>,
        ) -> AiFuture<Result<PathBuf, TtsError>> {
            let state = Arc::clone(&self.state);
            Box::pin(async move {
                let mut state = state.lock().expect("fake ai mutex poisoned");
                state.synth_to_wav_calls += 1;
                let path = state
                    .synth_to_wav_paths
                    .pop_front()
                    .unwrap_or_else(|| PathBuf::from("/tmp/fake_sequential_tts.wav"));
                Ok(path)
            })
        }
    }

    impl SerPort for FakeAiPort {
        fn analyze(&self, _input: SerInputPcm) -> AiFuture<Result<SerOutcome, SerError>> {
            Box::pin(async {
                Ok(SerOutcome {
                    session_id: "test".to_string(),
                    stream_id: "test".to_string(),
                    emotion: crate::shared::ports::ai::Emotion::Neutral,
                    confidence: 0.0,
                    arousal: None,
                    valence: None,
                })
            })
        }
    }

    #[derive(Clone)]
    struct FakeTtsStreamPort {
        state: Arc<Mutex<FakeTtsStreamState>>,
    }

    struct FakeTtsStreamState {
        scripts: VecDeque<TtsStreamScript>,
        synth_stream_calls: usize,
    }

    enum TtsStreamScript {
        Pending,
        Items(Vec<Result<Vec<u8>, TtsError>>),
    }

    impl FakeTtsStreamPort {
        fn new(scripts: Vec<TtsStreamScript>) -> (Self, Arc<Mutex<FakeTtsStreamState>>) {
            let state = Arc::new(Mutex::new(FakeTtsStreamState {
                scripts: scripts.into(),
                synth_stream_calls: 0,
            }));
            (
                Self {
                    state: Arc::clone(&state),
                },
                state,
            )
        }
    }

    impl TtsStreamPort for FakeTtsStreamPort {
        fn synth_stream(
            &self,
            _call_id: String,
            _text: String,
        ) -> AiFuture<Result<TtsStream, TtsError>> {
            let script = {
                let mut state = self.state.lock().expect("fake tts stream mutex poisoned");
                state.synth_stream_calls += 1;
                state.scripts.pop_front()
            };
            Box::pin(async move {
                let stream: TtsStream = match script.unwrap_or(TtsStreamScript::Pending) {
                    TtsStreamScript::Pending => {
                        Box::pin(stream::pending::<Result<Vec<u8>, TtsError>>())
                    }
                    TtsStreamScript::Items(items) => Box::pin(stream::iter(items)),
                };
                Ok(stream)
            })
        }
    }

    struct ScopedTestEnv {
        previous: Vec<(&'static str, Option<String>)>,
    }

    impl ScopedTestEnv {
        fn set(entries: &[(&'static str, &'static str)]) -> Self {
            let mut previous = Vec::with_capacity(entries.len());
            for (key, value) in entries {
                previous.push((*key, std::env::var(key).ok()));
                std::env::set_var(key, value);
            }
            Self { previous }
        }
    }

    impl Drop for ScopedTestEnv {
        fn drop(&mut self) {
            for (key, prev) in self.previous.drain(..).rev() {
                if let Some(value) = prev {
                    std::env::set_var(key, value);
                } else {
                    std::env::remove_var(key);
                }
            }
        }
    }

    fn init_tts_streaming_test_env() -> ScopedTestEnv {
        ScopedTestEnv::set(&[
            ("VOICEBOT_STREAMING_ENABLED", "true"),
            ("VOICEBOT_TTS_STREAMING_ENABLED", "true"),
            ("VOICEBOT_TTS_STREAMING_EARLY_START_ENABLED", "true"),
            ("TTS_STREAMING_FIRST_CHUNK_TIMEOUT_MS", "20"),
            ("TTS_STREAMING_TOTAL_TIMEOUT_MS", "500"),
            ("TTS_STREAMING_EARLY_START_BYTES", "8"),
        ])
    }

    fn test_lock() -> &'static AsyncMutex<()> {
        static LOCK: OnceLock<AsyncMutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| AsyncMutex::new(()))
    }

    fn build_tts_test_worker(
        ai_port: Arc<dyn AiServices>,
        tts_stream_port: Option<Arc<dyn TtsStreamPort>>,
    ) -> (
        AppWorker,
        CallId,
        tokio_mpsc::Receiver<(CallId, SessionOut)>,
    ) {
        let call_id = CallId::new("call-tts-stream-test").expect("valid call id");
        let (session_out_tx, session_out_rx) = tokio_mpsc::channel(16);
        let (_app_tx, app_rx) = app_event_channel(APP_EVENT_CHANNEL_CAPACITY);
        let phone_lookup: Arc<dyn PhoneLookupPort> = Arc::new(NoopPhoneLookup::new());
        let notification_port: Arc<dyn NotificationService> = Arc::new(NoopNotification::new());
        let worker = AppWorker::new(
            call_id.clone(),
            session_out_tx,
            app_rx,
            ai_port,
            None,
            None,
            tts_stream_port,
            None,
            phone_lookup,
            notification_port,
            AppRuntimeConfig {
                phone_lookup_enabled: false,
            },
        );
        (worker, call_id, session_out_rx)
    }

    async fn recv_session_out(
        rx: &mut tokio_mpsc::Receiver<(CallId, SessionOut)>,
    ) -> (CallId, SessionOut) {
        timeout(Duration::from_millis(200), rx.recv())
            .await
            .expect("session_out recv timeout")
            .expect("session_out channel closed")
    }

    fn collect_enqueue_events(items: Vec<(CallId, SessionOut)>) -> Vec<(CallId, String, u64)> {
        items
            .into_iter()
            .map(|(call_id, out)| match out {
                SessionOut::AppEnqueueBotAudioFile {
                    path,
                    generation_id,
                } => (call_id, path, generation_id),
                other => panic!("unexpected SessionOut: {other:?}"),
            })
            .collect()
    }

    fn build_pcm16_mono_wav(samples: &[i16], sample_rate: u32) -> Vec<u8> {
        let mut pcm = Vec::with_capacity(samples.len() * 2);
        for s in samples {
            pcm.extend_from_slice(&s.to_le_bytes());
        }
        let data_len = pcm.len() as u32;
        let byte_rate = sample_rate * 2;
        let mut out = Vec::with_capacity(44 + pcm.len());
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data_len).to_le_bytes());
        out.extend_from_slice(b"WAVE");
        out.extend_from_slice(b"fmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&sample_rate.to_le_bytes());
        out.extend_from_slice(&byte_rate.to_le_bytes());
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&16u16.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&data_len.to_le_bytes());
        out.extend_from_slice(&pcm);
        out
    }

    fn split_bytes(bytes: Vec<u8>, sizes: &[usize]) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        let mut offset = 0usize;
        for &size in sizes {
            if offset >= bytes.len() {
                break;
            }
            let end = (offset + size).min(bytes.len());
            out.push(bytes[offset..end].to_vec());
            offset = end;
        }
        if offset < bytes.len() {
            out.push(bytes[offset..].to_vec());
        }
        out
    }

    fn parse_segment_index_from_tts_stream_tmp(path: &str) -> Option<usize> {
        let file = path.rsplit('/').next()?;
        let stem = file.strip_suffix(".wav")?;
        let mut parts = stem.rsplitn(3, '_');
        let _ts = parts.next()?;
        let seg = parts.next()?;
        seg.parse::<usize>().ok()
    }

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

    #[tokio::test(flavor = "current_thread")]
    async fn enqueue_tts_sentence_falls_back_to_sequential_on_first_chunk_timeout() {
        let _guard = test_lock().lock().await;
        let _env_guard = init_tts_streaming_test_env();

        let (fake_ai, fake_ai_state) =
            FakeAiPort::new(vec![PathBuf::from("/tmp/fallback_seq.wav")]);
        let (fake_tts_stream_port, fake_tts_stream_state) =
            FakeTtsStreamPort::new(vec![TtsStreamScript::Pending]);
        let tts_stream_port: Arc<dyn TtsStreamPort> = Arc::new(fake_tts_stream_port);
        let (mut worker, call_id, mut session_out_rx) =
            build_tts_test_worker(Arc::new(fake_ai), Some(tts_stream_port));

        worker
            .enqueue_tts_sentence(&call_id, "テスト".to_string(), 101)
            .await;

        let synth_to_wav_calls = {
            let state = fake_ai_state.lock().expect("fake ai mutex poisoned");
            state.synth_to_wav_calls
        };
        assert_eq!(
            synth_to_wav_calls, 1,
            "sequential fallback should call synth_to_wav"
        );
        let synth_stream_calls = {
            let tts_state = fake_tts_stream_state
                .lock()
                .expect("fake tts stream mutex poisoned");
            tts_state.synth_stream_calls
        };
        assert_eq!(synth_stream_calls, 1, "streaming path should be attempted");

        let (event_call_id, out) = recv_session_out(&mut session_out_rx).await;
        assert_eq!(event_call_id, call_id);
        match out {
            SessionOut::AppEnqueueBotAudioFile {
                path,
                generation_id,
            } => {
                assert_eq!(path, "/tmp/fallback_seq.wav");
                assert_eq!(generation_id, 101);
            }
            other => panic!("unexpected SessionOut: {other:?}"),
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn try_enqueue_tts_sentence_streaming_early_start_keeps_partial_on_error_without_fallback(
    ) {
        let _guard = test_lock().lock().await;
        let _env_guard = init_tts_streaming_test_env();

        let wav = build_pcm16_mono_wav(&[1, 2, 3, 4, 5, 6], 24_000);
        let mut chunks = split_bytes(wav, &[56]); // header + enough PCM for >=1 segment
        chunks.push(Vec::new()); // no-op chunk is fine
        let mut items: Vec<Result<Vec<u8>, TtsError>> = chunks
            .into_iter()
            .filter(|c| !c.is_empty())
            .map(Ok)
            .collect();
        items.push(Err(TtsError::SynthesisFailed("stream broke".to_string())));

        let (fake_ai, fake_ai_state) =
            FakeAiPort::new(vec![PathBuf::from("/tmp/should_not_be_used.wav")]);
        let (fake_tts_stream_port, _fake_tts_stream_state) =
            FakeTtsStreamPort::new(vec![TtsStreamScript::Items(items)]);
        let tts_stream_port: Arc<dyn TtsStreamPort> = Arc::new(fake_tts_stream_port);
        let (mut worker, call_id, mut session_out_rx) =
            build_tts_test_worker(Arc::new(fake_ai), Some(tts_stream_port));

        let ok = worker
            .try_enqueue_tts_sentence_streaming(&call_id, "途中で落ちる", 202)
            .await;
        assert!(ok, "partial playback emitted => no sequential fallback");

        let synth_to_wav_calls = {
            let state = fake_ai_state.lock().expect("fake ai mutex poisoned");
            state.synth_to_wav_calls
        };
        assert_eq!(
            synth_to_wav_calls, 0,
            "sequential fallback must not run after partial early-start output"
        );

        let first = recv_session_out(&mut session_out_rx).await;
        let events = collect_enqueue_events(vec![first]);
        assert_eq!(events[0].0, call_id);
        assert_eq!(events[0].2, 202);
        assert!(parse_segment_index_from_tts_stream_tmp(&events[0].1).is_some());
        let _ = tokio::fs::remove_file(&events[0].1).await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn try_enqueue_tts_sentence_streaming_early_start_emits_multiple_enqueues_in_order() {
        let _guard = test_lock().lock().await;
        let _env_guard = init_tts_streaming_test_env();

        let wav = build_pcm16_mono_wav(&[10, 20, 30, 40, 50, 60, 70, 80, 90, 100], 24_000);
        let items = split_bytes(wav, &[50, 8, 8, 8, 8])
            .into_iter()
            .map(Ok)
            .collect::<Vec<_>>();

        let (fake_ai, fake_ai_state) =
            FakeAiPort::new(vec![PathBuf::from("/tmp/should_not_be_used.wav")]);
        let (fake_tts_stream_port, _fake_tts_stream_state) =
            FakeTtsStreamPort::new(vec![TtsStreamScript::Items(items)]);
        let tts_stream_port: Arc<dyn TtsStreamPort> = Arc::new(fake_tts_stream_port);
        let (mut worker, call_id, mut session_out_rx) =
            build_tts_test_worker(Arc::new(fake_ai), Some(tts_stream_port));

        let ok = worker
            .try_enqueue_tts_sentence_streaming(&call_id, "複数セグメント", 303)
            .await;
        assert!(ok);

        let synth_to_wav_calls = {
            let state = fake_ai_state.lock().expect("fake ai mutex poisoned");
            state.synth_to_wav_calls
        };
        assert_eq!(
            synth_to_wav_calls, 0,
            "successful streaming should not fallback"
        );

        let mut raw_events = Vec::new();
        for _ in 0..3 {
            raw_events.push(recv_session_out(&mut session_out_rx).await);
        }
        let events = collect_enqueue_events(raw_events);
        assert!(events.len() >= 3);
        for (event_call_id, _path, generation_id) in &events {
            assert_eq!(event_call_id, &call_id);
            assert_eq!(*generation_id, 303);
        }

        let indices: Vec<usize> = events
            .iter()
            .map(|(_, path, _)| {
                parse_segment_index_from_tts_stream_tmp(path)
                    .expect("segment index must be encoded in tmp file name")
            })
            .collect();
        assert_eq!(
            indices,
            vec![0, 1, 2],
            "segments should be enqueued in order"
        );

        for (_, path, _) in events {
            let _ = tokio::fs::remove_file(path).await;
        }
    }
}
