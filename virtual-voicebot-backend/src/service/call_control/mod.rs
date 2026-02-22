//! app モジュール（対話オーケストレーション層）
//! 現状は MVP 用のシンプル実装で、session からの音声バッファを受け取り
//! ai ポートを呼び出してボット音声(WAV)のパスを session に返す。
//! transport/sip/rtp には依存せず、SessionOut 経由のイベントのみを返す。

mod router;

use std::sync::Arc;

use tokio::sync::mpsc;

use crate::protocol::session::types::CallId;
use crate::protocol::session::SessionOut;
use crate::service::call_control::router::{
    parse_intent_json, router_config, system_info_response, RouteAction, Router,
};
use crate::shared::config::AppRuntimeConfig;
use crate::shared::ports::ai::{
    AiServices, AsrChunk, ChatMessage, Role, SerInputPcm, WeatherQuery,
};
use crate::shared::ports::notification::{
    NotificationFuture, NotificationService as NotificationPort,
};
use crate::shared::ports::phone_lookup::PhoneLookupPort;
use crate::shared::utils::{mask_phone, mask_pii};

pub use crate::shared::ports::app::{
    app_event_channel, AppEvent, AppEventRx, AppEventTx, EndReason,
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
///     phone_lookup,
///     notification_port,
///     AppRuntimeConfig::from_env(),
/// );
/// let _ = tx.try_send(AppEvent::CallStarted {
///     call_id: CallId::new("call-123").unwrap(),
///     caller: None,
/// });
/// ```
pub fn spawn_app_worker(
    call_id: CallId,
    session_out_tx: mpsc::Sender<(CallId, SessionOut)>,
    ai_port: Arc<dyn AiServices>,
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
    phone_lookup: Arc<dyn PhoneLookupPort>,
    router: Router,
    notification_port: Arc<dyn NotificationPort>,
    notification_state: NotificationState,
    app_cfg: AppRuntimeConfig,
}

#[derive(Debug, Default)]
struct NotificationState {
    ringing_notified: bool,
    missed_notified: bool,
    ended_notified: bool,
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
    fn new(
        call_id: CallId,
        session_out_tx: mpsc::Sender<(CallId, SessionOut)>,
        rx: AppEventRx,
        ai_port: Arc<dyn AiServices>,
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
            phone_lookup,
            router: Router::new(),
            notification_port,
            notification_state: NotificationState::default(),
            app_cfg,
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
        while let Some(ev) = self.rx.recv().await {
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
                    }
                    self.notify_ringing(call_id.clone(), from, timestamp);
                }
                AppEvent::CallStarted { call_id, caller } => {
                    if call_id != self.call_id {
                        log::warn!(
                            "[app {}] CallStarted received for mismatched call_id={}",
                            self.call_id,
                            call_id
                        );
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
                    if let Err(e) = self
                        .handle_audio_buffer(&call_id, pcm_mulaw, pcm_linear16)
                        .await
                    {
                        log::warn!("[app {}] audio handling failed: {:?}", self.call_id, e);
                    }
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
                    }
                    self.notify_ended(call_id.as_str(), from, reason, duration_sec, timestamp);
                    break;
                }
            }
        }
    }
}

impl AppWorker {
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
    async fn handle_audio_buffer(
        &mut self,
        call_id: &CallId,
        pcm_mulaw: Vec<u8>,
        pcm_linear16: Vec<i16>,
    ) -> anyhow::Result<()> {
        let user_text = self.transcribe_asr(call_id, pcm_mulaw).await;
        self.analyze_ser(call_id, pcm_linear16).await;

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

        let intent_json = match self.ai_port.classify_intent(trimmed.to_string()).await {
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

        // 履歴に追加
        self.history.push(ChatMessage {
            role: Role::User,
            content: user_query,
        });
        self.history.push(ChatMessage {
            role: Role::Assistant,
            content: answer_text.clone(),
        });
        if self.history.len() > APP_HISTORY_MAX_MESSAGES {
            let excess = self.history.len() - APP_HISTORY_MAX_MESSAGES;
            self.history.drain(0..excess);
        }

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
