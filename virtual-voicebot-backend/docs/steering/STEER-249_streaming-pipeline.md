# STEER-249: LLM ストリーミング ＋ 文単位 TTS 先行再生（first-audio latency 改善）

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-249 |
| タイトル | LLM ストリーミング ＋ 文単位 TTS 先行再生（first-audio latency 改善） |
| ステータス | Draft |
| 関連Issue | #249 |
| 優先度 | P1 |
| 作成日 | 2026-02-25 |

---

## 2. ストーリー（Why）

### 2.1 背景

現状のボイスボットは ASR → LLM → TTS が完全直列であり、各ステージの最終結果を待ってから次へ進む。

```
（現状）
発話終了
  → ASR 全文確定（WAV 全読み → multipart POST → 転写完了）
  → LLM 全文確定（stream:false → body.text().await → parse）
  → TTS 全文合成（audio_query → synthesis → WAV 全受信 → 保存）
  → 再生開始
```

計測上の体感遅延は **3〜8 秒**（ローカル LLM + ローカル TTS 構成）になりやすい。

一方、再生側（[playback_service.rs](../../src/protocol/session/services/playback_service.rs)）は既に `step_playback()` によるフレーム逐次送出を行っており、「小さい WAV を早く渡す」だけで体感改善が即効する土台がある。

### 2.2 目的

**first-audio latency**（発話終了 → 最初の音声が相手に届くまでの時間）を短縮する。

第 1 段階として、コスト対効果が最も高い「LLM streaming 受信 ＋ 文単位 TTS 先行再生」を実装する。
ASR の真のストリーミング化・真の TTS ストリーミング化は **第 2 段階以降**（別 Issue）に分割する。

**目標レンジ（参考）**

| シナリオ | first-audio latency 目安 |
|---------|--------------------------|
| 現状（逐次） | 3〜8 秒 |
| 本 Issue（LLM streaming ＋ 文単位 TTS） | 1〜3 秒（体感差：大） |
| 第 2 段階（ASR partial/VAD 追加） | さらに 0.3〜1.5 秒改善余地 |

### 2.3 ユーザーストーリー

```
As a ボイスボット利用者（発信者）
I want to  質問後、応答の最初の一文をすばやく聞ける
So that    応答が早く感じられ、ボイスボットとの対話が自然になる

受入条件:
- [ ] LLM 全文完成前に最初の TTS ジョブが生成される
- [ ] LLM 全文完成前に最初の音声再生が開始される
- [ ] 既存の逐次動作が設定フラグで維持できる（既存経路を壊さない）
- [ ] フォールバック・タイムアウト挙動が既存と同等である
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-25 |
| 起票理由 | ボイスボット応答の体感遅延が大きいため、ストリーミング化を試行する |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Sonnet 4.6 |
| 作成日 | 2026-02-25 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "ストリーミング化ステアリング作成（#249）" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | @MasanoriSuda | - | - | |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | - |
| 承認日 | - |
| 承認コメント | |

### 3.5 実装

| 項目 | 値 |
|------|-----|
| 実装者 | Codex |
| 実装日 | - |
| 指示者 | @MasanoriSuda |
| 指示内容 | Approved 後に引き継ぎ |
| コードレビュー | - |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | - |
| マージ日 | - |
| マージ先 | - |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| docs/requirements/（AI 連携 RD） | 修正 | LLM ストリーミング要件、TTS キュー要件を追記 |
| docs/design/detail/（AI サービス DD） | 修正 | `LlmStreamPort` シグネチャ、`SentenceAccumulator` 設計、再生キュー設計 |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| `src/shared/error/ai.rs` | 修正 | `LlmError::Timeout(String)` バリアント追加（現行は `GenerationFailed / ContextTooLong / RateLimited` のみ） |
| `src/shared/ports/ai/llm.rs` | 追加 | `LlmStreamPort` トレイト（ストリーム戻り値版） |
| `src/shared/ports/ai.rs` | 修正 | `AppWorker` に streaming 専用 Port を接続する方針（§5.2 参照） |
| `src/service/ai/mod.rs` | 修正 | Ollama streaming 実装（`stream: true`、NDJSON パース） |
| `src/service/call_control/mod.rs` | 修正 | `handle_user_text()` でストリーム受信 → `mpsc` 経由で TTS worker に送出 |
| `src/protocol/session/types.rs` | 修正 | `SessionOut::AppEnqueueBotAudioFile`・`SessionControlIn::AppBotAudioFileEnqueue` 追加 |
| `src/main.rs` | 修正 | `AppEnqueueBotAudioFile` 受信 → `SessionControlIn::AppBotAudioFileEnqueue` 中継処理追加 |
| `src/protocol/session/handlers/mod.rs` | 修正 | `AppBotAudioFileEnqueue` ハンドラ追加（再生中ならキュー積み、なければ即 `start_playback`） |
| `src/protocol/session/services/playback_service.rs` | 修正 | 再生キュー（`VecDeque<Vec<Vec<u8>>>`）追加・`finish_playback()` でキュー先頭を自動消費 |
| `src/shared/config.rs`（または環境変数） | 追加 | `VOICEBOT_STREAMING_ENABLED` 他のフラグ |
| `Cargo.toml` | 修正 | `tokio-stream` 依存追加（`Stream` トレイト利用のため） |

---

## 5. 差分仕様（What / How）

### 5.1 前提：現状の確認

以下は既存コードの実測事実（調査時点）。

| 箇所 | 現状 |
|------|------|
| `call_control/mod.rs:384` | `AsrChunk { end: true }` を 1 回投げる（一括処理） |
| `ai/asr.rs:17` | μ-law をまとめて WAV 化し一時ファイル経由で ASR 呼び出し |
| `ai/mod.rs:339` | WAV 全読み後に multipart POST |
| `shared/ports/ai/llm.rs:5` | `LlmPort::generate_answer` 戻り値は `Result<String>` |
| `ai/mod.rs:1022` | Ollama リクエストに `stream: false` 固定 |
| `ai/mod.rs:387` | OpenAI は `resp.text().await` 後に parse（非ストリーム） |
| `shared/ports/ai/tts.rs:7` | `TtsPort::synth_to_wav` 戻り値は `Result<PathBuf>` |
| `ai/mod.rs:1390` | VOICEVOX は `/audio_query` → `/synthesis` で全 WAV 受信後に保存 |
| `playback_service.rs:53` | `step_playback()` はフレーム逐次 RTP 送信（すでに逐次）|

### 5.2 新規 Port（`LlmStreamPort`）

既存の `LlmPort`（戻り値 `Result<String>`）は **変更しない**。
ストリーム版は別トレイトを追加し、フラグ分岐で使い分ける。

```rust
// src/shared/ports/ai/llm.rs への追加

use tokio_stream::Stream;
use std::pin::Pin;

pub type LlmStream = Pin<Box<dyn Stream<Item = Result<String, LlmError>> + Send>>;

pub trait LlmStreamPort: Send + Sync {
    /// LLM の出力をトークン単位のストリームとして受信する。
    /// 各 Item は "トークン断片文字列" または LlmError。
    fn generate_answer_stream(
        &self,
        call_id: String,
        messages: Vec<ChatMessage>,
    ) -> AiFuture<Result<LlmStream, LlmError>>;
}
```

> **設計論点 A**（**決定 OQ-1**）: `LlmPort` に `generate_answer_stream` を追加してデフォルト実装を提供するか、それとも独立トレイトにするか。
> → **独立トレイト**（既存 `LlmPort` を汚さない、ISP 準拠）。

**`AiServices` との接続方針**

現状 `AppWorker` は `ai_port: Arc<dyn AiServices>` のみを持つ（`call_control/mod.rs:112`）。
`AiServices` は `AsrPort + IntentPort + LlmPort + ...` の集約トレイトで、`LlmStreamPort` を含まない（`shared/ports/ai.rs:30`）。

`LlmStreamPort` を独立トレイトにするため、接続には以下の 2 案がある。

| 案 | 内容 | 長所 | 短所 |
|----|------|------|------|
| **A（推奨）** | `AppWorker` に `llm_stream_port: Option<Arc<dyn LlmStreamPort>>` を別フィールドで追加 | 既存 `AiServices` に影響なし。`None` = streaming 無効として設定フラグと直接連携できる | 引数が 1 つ増える |
| B | `AiServices` に `LlmStreamPort` を追加 | 呼び出し側がシンプル | 全実装クラスへの変更が必要 |

**→ 案 A を採用する。** `spawn_app_worker()` の引数として `llm_stream_port: Option<Arc<dyn LlmStreamPort>>` を追加し、`VOICEBOT_STREAMING_ENABLED=true` の場合のみ `Some(...)` を渡す。

### 5.3 Ollama ストリーミング実装

Ollama は `POST /api/chat` に `stream: true` を送ると NDJSON で応答する。

```
{"model":"...","message":{"role":"assistant","content":"こん"},"done":false}
{"model":"...","message":{"role":"assistant","content":"にちは"},"done":false}
{"model":"...","message":{"role":"assistant","content":""},"done":true}
```

**HTTP client の timeout 方針（重要）**

既存の `http_client(timeout)` は `Client::builder().timeout(timeout).build()` で構築されており、
`timeout()` は **総リクエスト timeout**（レスポンス body の受信を含む）を設定する（`ai/mod.rs:123`）。

streaming レスポンスは body が長時間流れ続けるため、この client をそのまま使うと
`LLM_LOCAL_TIMEOUT_MS` でストリームが強制終了し、§5.5 の `LLM_STREAMING_TOTAL_TIMEOUT_MS` と責務が競合する。

**→ streaming 専用 HTTP client を別途構築する。`http_client()` は流用しない。**

```rust
// streaming 用: connect のみ timeout を設定し、body timeout は設けない
// total timeout は §5.5 の tokio::time::timeout で管理する
fn http_client_for_stream(connect_timeout: Duration) -> Result<Client> {
    Ok(Client::builder()
        .connect_timeout(connect_timeout)   // TCP ハンドシェイクのみ
        // .timeout() は設定しない（streaming body を途中で切らないため）
        .build()?)
}
```

> **`LlmError::Timeout` について**: 現行 `LlmError`（`src/shared/error/ai.rs:24`）には `GenerationFailed / ContextTooLong / RateLimited` しか存在しない。
> streaming timeout 用に `Timeout(String)` バリアントを追加する（§4.2 参照）。
> ```rust
> // src/shared/error/ai.rs への追加（差分）
> #[error("Timeout: {0}")]
> Timeout(String),
> ```

```rust
// ai/mod.rs への追加（Ollama streaming 呼び出し）

pub(super) async fn call_ollama_for_chat_stream(
    messages: &[ChatMessage],
    system_prompt: &str,
    model: &str,
    endpoint_url: &str,
    connect_timeout: Duration,           // TCP ハンドシェイク（AGENTS.md:116）
    response_header_timeout: Duration,   // send() 後のレスポンスヘッダ受信まで（AGENTS.md:116）
) -> Result<LlmStream> {
    let client = http_client_for_stream(connect_timeout)?;   // 専用 client
    // ...
    let req = OllamaChatRequest { model: ..., messages: ..., stream: true };

    // send().await 自体を response_header_timeout で包む
    // TCP 接続後のヘッダ待ち（サーバー側処理時間）もタイムアウト対象とする（AGENTS.md:116）
    let resp = tokio::time::timeout(
        response_header_timeout,
        client.post(endpoint_url).json(&req).send(),
    )
    .await
    .map_err(|_| LlmError::Timeout("response header timeout".into()))??;

    // resp.bytes_stream() を NDJSON 行単位に split → content フィールドを yield
    // done:true の行で Stream 終了（first-token 以降のタイムアウトは §5.5 で管理）
    // ...
}
```

> `response_header_timeout` には `LLM_STREAMING_FIRST_TOKEN_TIMEOUT_MS` と同じ値を渡す（呼び出し元で統一）。
> これにより `first_token_timeout` は「レスポンスヘッダ受信 → 最初の NDJSON トークン到着」を対象とし、
> `response_header_timeout` は「HTTP POST 送信 → レスポンスヘッダ到着」を対象とする二段構えとなる。

> **設計論点 B**（**決定 OQ-2**）: OpenAI クラウド LLM のストリーミング（SSE `data:` 形式）を今回スコープに含めるか。
> → **含まない**。初回は Ollama（ローカル）のみ。OpenAI streaming は別 Issue へ分割。

### 5.4 文単位アキュムレーター（`SentenceAccumulator`）

LLM トークンを蓄積し、文の区切りを検出したら `完成文字列` を返す。

```rust
// src/service/call_control/sentence_accumulator.rs（新規）

/// 文字列分割専用。タイマー判定は呼び出し側（§5.5 のストリームループ）で行う。
pub struct SentenceAccumulator {
    buf: String,
    max_chars: usize, // デフォルト 50 文字（VOICEBOT_STREAMING_SENTENCE_MAX_CHARS）
}

impl SentenceAccumulator {
    /// トークン断片を追記し、文が完成したら Some(sentence) を返す。
    /// 完成条件: 句点 OR max_chars 超過（文字列ルールのみ）
    pub fn push(&mut self, token: &str) -> Option<String> { ... }

    /// バッファに残った未送信テキストを強制フラッシュ（LLM done 時 / idle timeout 時）。
    pub fn flush(&mut self) -> Option<String> { ... }
}
```

**文区切りルール（決定 OQ-3）**

`SentenceAccumulator` は文字列ロジックのみを担う。`max_wait_ms` による強制フラッシュは
`stream.next().await` が止まった時にタイマーが走らないため、`SentenceAccumulator` 内では扱えない。
**idle timeout 判定は §5.5 の `tokio::select!` で行い、`acc.flush()` を呼ぶ**（責務分離）。

| 優先順位 | 条件 | 担当 |
|---------|------|------|
| 1 | 句点（`。！？`）を含んだ時点 | `acc.push()` → `Some(sentence)` |
| 2 | `max_chars`（50 文字）に達した時点 | `acc.push()` → `Some(sentence)` |
| 3 | `CHUNK_IDLE_TIMEOUT`（`max_wait_ms`）経過 | §5.5 の `tokio::select!` → `acc.flush()` |
| — | LLM done 時 | §5.5 の stream 終端 → `acc.flush()` |

> **設計論点 C**（**決定 OQ-3**）: `max_chars` 初期値は **50 文字**（`VOICEBOT_STREAMING_SENTENCE_MAX_CHARS`）。`max_wait_ms` は `VOICEBOT_STREAMING_SENTENCE_MAX_WAIT_MS`（環境変数）で設定し、判定は §5.5 の `tokio::select!` で行う。

### 5.5 `handle_user_text()` ストリーミング分岐

**パイプライン構成**

```
LLM stream consumer  ──(bounded mpsc)──▶  TTS worker（直列）
     │                                          │
  acc.push() で文を切り出し                 synth_to_wav()
  idle timeout は tokio::select! で          │
  acc.flush() を呼ぶ                    session_out_tx.send(
                                          AppEnqueueBotAudioFile)
```

OQ-7 の `bounded mpsc` を採用することで、LLM stream 消費と TTS 合成を分離しつつ、
TTS が詰まったときに `mpsc` の `send().await` がバックプレッシャとして LLM 消費を自然に抑制する。

**タイムアウト責務分担（OQ-8 の実装位置）**

OQ-8「first-token + total の二段構え」は §5.5 の stream consumer 側（`tokio::select!`）に集約する。
§5.3 の `call_ollama_for_chat_stream` は TCP 接続（`connect_timeout`）とレスポンスヘッダ受信（`response_header_timeout`）を担う。body timeout は設けない（body は §5.5 の `tokio::select!` が制御する）。

| タイムアウト種別 | 担当箇所 | 設定キー |
|----------------|---------|---------|
| TCP 接続（connect） | §5.3 streaming 専用 client の `connect_timeout`（**既存 `http_client()` は流用しない**） | `LLM_STREAMING_CONNECT_TIMEOUT_MS`（新規。または既存 `LLM_LOCAL_TIMEOUT_MS` 流用可） |
| レスポンスヘッダ受信（response-header） | §5.3 `tokio::time::timeout(response_header_timeout, send().await)` | `LLM_STREAMING_FIRST_TOKEN_TIMEOUT_MS` と同値を呼び出し元で渡す（別設定不要） |
| first-token（最初のトークン受信） | §5.5 `tokio::select!`（`first_received` フラグで切替） | `LLM_STREAMING_FIRST_TOKEN_TIMEOUT_MS`（新規） |
| chunk-idle（トークン間隔） | §5.5 `tokio::select!` | `VOICEBOT_STREAMING_SENTENCE_MAX_WAIT_MS`（OQ-11 兼用） |
| total（ストリーム全体） | §5.5 `tokio::time::timeout` で consumer task 全体を包む | `LLM_STREAMING_TOTAL_TIMEOUT_MS`（新規） |

> 補足: 既存 `LLM_LOCAL_TIMEOUT_MS` は非 streaming リクエスト（`call_ollama_for_chat`）の総 timeout として引き続き使用。streaming 経路には影響しない。

```rust
// src/service/call_control/mod.rs（修正イメージ）

async fn handle_user_text(&mut self, call_id: &CallId, trimmed: &str) -> anyhow::Result<()> {
    // ...（intent 分類は変更なし）

    // フラグ ON かつ streaming port が利用可能な場合のみ streaming 経路へ（中指摘対応）
    // port が None の場合はフラグに関わらず逐次へフォールバック
    if config::voicebot_streaming_enabled() && self.llm_stream_port.is_some() {
        self.handle_user_text_streaming(call_id, messages).await
    } else {
        self.handle_user_text_sequential(call_id, messages).await // 既存経路
    }
}

async fn handle_user_text_streaming(
    &mut self,
    call_id: &CallId,
    messages: Vec<ChatMessage>,
) -> anyhow::Result<()> {
    // llm_stream_port は呼び出し元で is_some() 確認済み
    let stream_result = self.llm_stream_port
        .as_ref()
        .expect("checked by caller")
        .generate_answer_stream(call_id.to_string(), messages.clone())
        .await;

    let stream = match stream_result {
        Ok(s) => s,
        Err(e) => {
            // ストリーム開始失敗 → 逐次経路へフォールバック（LLM Err 時の既存挙動を踏襲）
            log::warn!("[app {call_id}] LLM stream start failed: {e}, fallback to sequential");
            return self.handle_user_text_sequential(call_id, messages).await;
        }
    };

    // bounded channel: TTS 詰まり時に LLM 消費を自然に抑制（OQ-7）
    let (sentence_tx, sentence_rx) =
        tokio::sync::mpsc::channel::<String>(config::sentence_channel_capacity());

    // ─── LLM stream consumer タスク ─────────────────────────────────────
    let first_token_timeout = config::llm_streaming_first_token_timeout();  // OQ-8
    let idle_timeout        = config::sentence_max_wait();                   // OQ-11
    let total_timeout       = config::llm_streaming_total_timeout();         // OQ-8
    let max_chars           = config::sentence_max_chars();

    let full_answer_cell    = Arc::new(tokio::sync::Mutex::new(String::new()));
    let full_answer_ref     = full_answer_cell.clone();
    // 1文でも TTS 送信できたかを追跡する（フォールバック判定用）
    let sentences_sent      = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let sentences_sent_ref  = sentences_sent.clone();
    let had_error           = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let had_error_ref       = had_error.clone();

    let consumer = tokio::spawn(async move {
        tokio::pin!(stream);
        let mut acc = SentenceAccumulator::new(max_chars);
        let mut first_token_received = false;

        // total タイムアウトで consumer 全体を包む（OQ-8）
        let timed_out = tokio::time::timeout(total_timeout, async {
            loop {
                // first-token / chunk-idle の切替（OQ-8）
                let wait_duration = if first_token_received { idle_timeout } else { first_token_timeout };
                tokio::select! {
                    item = stream.next() => {
                        match item {
                            Some(Ok(token)) => {
                                first_token_received = true;
                                full_answer_ref.lock().await.push_str(&token);
                                if let Some(sentence) = acc.push(&token) {
                                    sentences_sent_ref.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                    let _ = sentence_tx.send(sentence).await;
                                }
                            }
                            Some(Err(e)) => {
                                log::warn!("[stream] LLM error: {e}");
                                had_error_ref.store(true, std::sync::atomic::Ordering::Relaxed);
                                // バッファ残余があれば送出してから終了
                                if let Some(tail) = acc.flush() {
                                    sentences_sent_ref.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                    let _ = sentence_tx.send(tail).await;
                                }
                                break;
                            }
                            None => {
                                // LLM done: バッファ残余をフラッシュ（OQ-3）
                                if let Some(tail) = acc.flush() {
                                    sentences_sent_ref.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                    let _ = sentence_tx.send(tail).await;
                                }
                                break;
                            }
                        }
                    }
                    // first-token / chunk-idle timeout（OQ-8 / OQ-11）
                    _ = tokio::time::sleep(wait_duration) => {
                        log::warn!("[stream] timeout (first_token_received={first_token_received})");
                        had_error_ref.store(true, std::sync::atomic::Ordering::Relaxed);
                        if let Some(flushed) = acc.flush() {
                            sentences_sent_ref.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            let _ = sentence_tx.send(flushed).await;
                        }
                        break;
                    }
                }
            }
        }).await;

        if timed_out.is_err() {
            // total タイムアウト到達
            log::warn!("[stream] LLM total timeout exceeded");
            had_error_ref.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        // sentence_tx drop → TTS worker の while let ループが終了
    });

    // ─── TTS worker（直列 / OQ-4）────────────────────────────────────────
    let mut rx = sentence_rx;
    while let Some(sentence) = rx.recv().await {
        match self.ai_port.synth_to_wav(call_id.to_string(), sentence, None).await {
            Ok(wav_path) => {
                let _ = self.session_out_tx
                    .send((
                        self.call_id.clone(),
                        SessionOut::AppEnqueueBotAudioFile {   // 新規イベント（§5.6）
                            path: wav_path.to_string_lossy().to_string(),
                        },
                    ))
                    .await;
            }
            Err(e) => log::warn!("[app {call_id}] TTS failed: {e:?}"),
        }
    }

    // consumer 完了を待機（重大指摘対応）
    let _ = consumer.await;

    let sent  = sentences_sent.load(std::sync::atomic::Ordering::Relaxed);
    let error = had_error.load(std::sync::atomic::Ordering::Relaxed);

    if error && sent == 0 {
        // 1文も音声を出せなかった → 逐次経路へフォールバック
        log::warn!("[app {call_id}] streaming failed with 0 sentences, fallback to sequential");
        return self.handle_user_text_sequential(call_id, messages).await;
    }

    // 履歴は全文確定後に追加（既存と同じタイミング）
    let full_answer = full_answer_cell.lock().await.clone();
    if !full_answer.is_empty() {
        self.push_history(user_query, full_answer);
    }
    Ok(())
}
```

> **設計論点 D**（**決定 OQ-4**）: TTS ジョブは直列（`while let Some(sentence) = rx.recv().await` ループ）。順序保証・VOICEVOX 負荷の観点から初回は安全側。`mpsc` があるので LLM 消費は TTS 待ちで止まらない。

### 5.6 再生キュー対応とメッセージ中継経路

現状の `start_playback()` は呼ぶたびに前の再生をキャンセルする（`cancel_playback()` を冒頭で呼ぶ）。
連続送信に対応するために **再生キュー** を追加する。

**新規イベント型定義（`session/types.rs` 修正）**

```rust
// SessionOut（call_control → main）に追加
AppEnqueueBotAudioFile { path: String },

// SessionControlIn（main → session handler）に追加
AppBotAudioFileEnqueue { path: String },
```

**メッセージ中継経路（既存 `AppSendBotAudioFile` の経路に倣う）**

```
call_control/mod.rs
  └── session_out_tx.send(SessionOut::AppEnqueueBotAudioFile { path })
        ↓
main.rs（:426 付近の SessionOut マッチに追加）
  SessionOut::AppEnqueueBotAudioFile { path } => {
      sess_tx.control_tx.send(SessionControlIn::AppBotAudioFileEnqueue { path })
  }
        ↓
session/handlers/mod.rs（:580 付近の SessionControlIn マッチに追加）
  (SessState::Established, SessionControlIn::AppBotAudioFileEnqueue { path }) => {
      // enqueue_playback は async fn（start_playback と同様に spawn_blocking + timeout を内包）
      if let Err(e) = self.enqueue_playback(&path).await {
          warn!("[session {}] enqueue_playback failed: {:?}", self.call_id, e);
      }
  }
        ↓
playback_service.rs
  // async fn: 内部で start_playback（spawn_blocking + timeout）を呼ぶ可能性があるため
  pub(crate) async fn enqueue_playback(&mut self, path: &str) -> Result<(), Error> {
      if self.playback.is_some() {
          // 再生中 → WAV をフレーム化してキューに積む
          // spawn_blocking + io_timeout は start_playback と同じパターンを踏襲（AGENTS.md:116）
          let frames = self.load_frames_with_timeout(path).await?;
          self.playback_queue.push_back(frames);
      } else {
          // 再生なし → 即 start_playback（既存メソッドを流用）
          self.start_playback(&[path]).await?;
      }
      Ok(())
  }

  pub(crate) fn finish_playback(&mut self, restart_ivr_timeout: bool) {
      // 既存処理の前に: キューに次のフレームがあれば自動消費（同期でフレームリスト設定）
      if let Some(next_frames) = self.playback_queue.pop_front() {
          // フレームは既にロード済みなので同期で設定できる
          self.align_rtp_clock();
          self.playback = Some(PlaybackState { frames: next_frames, index: 0 });
          self.sending_audio = true;
          return;
      }
      // ... 既存の announce_mode / ivr_state 処理
  }
```

**`SessionCoordinator` フィールド追加**

```rust
playback_queue: VecDeque<Vec<Vec<u8>>>,  // 追加キュー（WAV フレーム列）
```

既存の `AppSendBotAudioFile`（即再生・上書き）は維持し、ストリーミング経路専用に `AppEnqueueBotAudioFile` を追加する。

> **設計論点 E**（**決定 OQ-5**）: 既存の `AppSendBotAudioFile` に「enqueue フラグ」を追加するか、新規イベント型にするか。
> → **新規イベント型** `AppEnqueueBotAudioFile`（既存シグナルの意味・挙動を変えない。責務が明確）。

### 5.7 機能フラグ

```toml
# .env または EnvironmentFile

# ストリーミングモード有効化（デフォルト false = 既存逐次動作）
# NOTE: ON でも llm_stream_port が None の場合は逐次経路へフォールバックする
VOICEBOT_STREAMING_ENABLED=false

# 文単位 TTS の最大文字数（デフォルト 50）
VOICEBOT_STREAMING_SENTENCE_MAX_CHARS=50

# chunk-idle timeout ms（first-token 受信後のトークン間隔上限 / OQ-11 兼用）
VOICEBOT_STREAMING_SENTENCE_MAX_WAIT_MS=2000

# bounded mpsc の容量（OQ-7）
VOICEBOT_STREAMING_SENTENCE_CHANNEL_CAPACITY=4

# TCP 接続 timeout ms（streaming 専用 client の connect_timeout / OQ-8）
# ※ LLM_LOCAL_TIMEOUT_MS（非 streaming 総 timeout）とは別設定
LLM_STREAMING_CONNECT_TIMEOUT_MS=5000

# first-token timeout ms（最初のトークンが届くまでの上限 / OQ-8）
LLM_STREAMING_FIRST_TOKEN_TIMEOUT_MS=5000

# total timeout ms（ストリーム全体の最大許容時間 / OQ-8）
# ※ streaming client には .timeout() を設定しないため、この値が実質の上限となる
LLM_STREAMING_TOTAL_TIMEOUT_MS=60000
```

**`Cargo.toml` への依存追加（軽指摘対応）**

現状 `futures` / `tokio-stream` の直接依存がない（`Cargo.toml:6-31` 確認済み）。
`LlmStream` の `Stream` トレイトに `tokio-stream` を使う。

```toml
# Cargo.toml [dependencies] に追加
tokio-stream = "0.1"
```

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #249 | STEER-249 | 起票 |
| STEER-249 | `LlmStreamPort` | 新規 Port 追加 |
| STEER-249 | `SentenceAccumulator` | 新規コンポーネント |
| STEER-249 | `AppEnqueueBotAudioFile` | 新規セッションイベント |
| STEER-249 | `PlaybackQueue` | 既存 playback 拡張 |
| `LlmStreamPort` | Ollama streaming 実装 | 実装 |
| `LlmStreamPort` | UT（streaming Port mock） | テスト |
| `SentenceAccumulator` | UT（境界値テスト） | テスト |

---

## 7. オープンクエスチョン

### 7.1 決定済み（OQ-1〜11）

| # | 質問 | **決定** | 根拠 |
|---|------|----------|------|
| OQ-1 | `LlmStreamPort` は独立トレイトか `LlmPort` 拡張か？ | **独立トレイト** | 既存 `LlmPort`（`Result<String>`）を汚さない。ISP 準拠。拡張すると非 streaming 実装まで影響が広がる |
| OQ-2 | OpenAI cloud LLM streaming を今回スコープに含めるか？ | **含まない** | まずローカル Ollama のみで十分。SSE パースは別 Issue |
| OQ-3 | 文フラッシュ条件は？ | **句点 OR 50文字 OR `max_wait_ms`（3条件 OR）** | 2条件では長文や途中でストリームが詰まった場合に TTS が止まる可能性あり |
| OQ-4 | TTS ジョブは直列か並列か？ | **直列** | 順序保証・VOICEVOX 負荷。再生キュー整備後に並列化を検討 |
| OQ-5 | `AppEnqueueBotAudioFile` は新規型か既存フラグ追加か？ | **新規型** | 既存シグナルの意味・挙動を変えない。責務が明確 |
| OQ-6 | barge-in を今回スコープに含めるか？ | **含まない** | キャンセルトークン設計・再生キューの停止が別途必要。別 Issue へ |
| OQ-7 | `mpsc` チャネルを bounded にするか？ | **bounded** | AGENTS.md 既存のバックプレッシャ方針に従う（`:127–128`） |
| OQ-8 | LLM タイムアウト設計は？ | **first-token + total の二段構え** | first-token タイムアウト単独では長大な応答で total が無制限になる |
| OQ-9 | LLM/TTS の chunk 終端・応答終端をどう表現するか？ | **明示的な終端イベント型** | `Stream` の `None` のみでは再生・状態遷移の判断が曖昧になる。終端 variant を持つことで責務を明確化 |
| OQ-10 | `turn_id` / `generation_id` をイベントに載せるか？ | **載せる** | 将来の barge-in 実装時に古い chunk を破棄しやすくする布石。早期に乗せておくコストは小さい |
| OQ-11 | streaming 中の idle timeout をどう検知するか？ | **別途 chunk 間 timeout（`VOICEBOT_STREAMING_SENTENCE_MAX_WAIT_MS` 兼用）** | first-token/total のみでは途中で止まった stream を検知できない。§5.5 の `tokio::select!` で chunk-idle として実装済み |

### 7.2 未決定

なし。OQ-1〜11 は全て決定済み。

---

## 8. レビューチェックリスト

### 8.1 仕様レビュー（Review → Approved）

- [ ] ストーリー（Why）と目的が合意されているか
- [ ] `LlmStreamPort` のシグネチャが実装可能か（tokio-stream 依存 OK か）
- [ ] 文区切りルールが実運用に適しているか（日本語 TTS との相性）
- [ ] 再生キュー設計が既存 `cancel_playback()` / barge-in と矛盾しないか
- [ ] 機能フラグのデフォルト値（`false`）で既存動作が保証されるか
- [ ] タイムアウト方針が `AGENTS.md:116` の外部 I/O timeout 必須要件を満たすか（connect/response-header/first-token/chunk-idle/total の 5 層が全て設定されているか）
- [ ] `VOICEBOT_STREAMING_ENABLED=true` かつ `llm_stream_port=None` 時に逐次経路へフォールバックするか
- [ ] LLM stream エラー時・0文送信時に逐次経路へフォールバックするか（`handle_user_text_sequential` を呼ぶ）
- [ ] `enqueue_playback` が `async fn` で `spawn_blocking + io_timeout` を踏襲しているか（AGENTS.md:116）
- [x] OQ-9（終端イベント型）の設計が `LlmStream` シグネチャに反映されているか（**決定: 明示的な終端イベント型**）
- [x] OQ-10（`generation_id`）の要否が決定し、イベント定義に反映されているか（**決定: 載せる**）
- [x] OQ-11（idle timeout）の実装方針が決定されているか（**決定: 別途 chunk 間 timeout**）
- [x] OQ-9〜11 が全て決定されているか

### 8.2 マージ前チェック（Approved → Merged）

- [ ] 実装完了（Codex）
- [ ] `VOICEBOT_STREAMING_ENABLED=false` で既存テスト全 PASS
- [ ] `VOICEBOT_STREAMING_ENABLED=true` で手動通話テスト実施
- [ ] CodeRabbit レビュー対応済み
- [ ] 本体仕様書への反映方針確認

---

## 9. 備考

### 9.1 スコープ外（別 Issue 推奨）

- **真の ASR ストリーミング**: `AsrPort` の Port 変更・ローカル whisper サーバーの WebSocket/chunk 対応まで影響が広い（第 2 段階）
- **真の TTS ストリーミング**: VOICEVOX の `/synthesis` streaming endpoint 対応（第 3 段階）
- **OpenAI LLM streaming**: SSE 形式パース実装（第 2 段階候補）
- **barge-in（割り込み再生キャンセル）**: TTS ジョブ・再生キューの停止 + キャンセルトークン設計が必要。既存 `AGENTS.md:127–128` の割り込み優先方針を踏まえた別設計が必要

### 9.2 未追跡ファイルの注意

`git status` にて `virtual-voicebot-backend/script/whisper_server_9010.py` が未追跡状態。
調査上は実験的ファイルと見られる。SoT として扱うかはオーナー判断が必要。

### 9.3 参照コード（調査時点のスナップショット）

| 参照 | 内容 |
|------|------|
| `src/service/call_control/mod.rs:384` | `AsrChunk { end: true }` 一括投入 |
| `src/service/call_control/mod.rs:425` | `handle_user_text()` 主フロー |
| `src/service/call_control/mod.rs:588` | TTS 呼び出し → `AppSendBotAudioFile` 送信 |
| `src/shared/ports/ai/llm.rs:5` | `LlmPort::generate_answer` 戻り値 `Result<String>` |
| `src/shared/ports/ai/tts.rs:7` | `TtsPort::synth_to_wav` 戻り値 `Result<PathBuf>` |
| `src/service/ai/mod.rs:1022` | Ollama `stream: false` 固定 |
| `src/protocol/session/services/playback_service.rs:18` | `start_playback()` 先頭で既存再生をキャンセル |
| `src/protocol/session/services/playback_service.rs:53` | `step_playback()` フレーム逐次 RTP 送信 |
| `virtual-voicebot-backend/AGENTS.md:116` | 外部 I/O は timeout 必須 |
| `virtual-voicebot-backend/AGENTS.md:127–128` | TTS→RTP 割り込み優先方針 |

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-25 | 初版作成（Draft） | Claude Sonnet 4.6 |
| 2026-02-25 | OQ-1〜8 決定済みに更新、OQ-3 に `max_wait_ms` 第 3 条件追加、OQ-9〜11 追加 | Claude Sonnet 4.6 |
| 2026-02-25 | レビュー指摘対応: §4.2 影響範囲追加、§5.2 AiServices 接続方針明記（別フィールド案）、§5.4 SentenceAccumulator を文字列分割専用に限定、§5.5 mpsc パイプライン設計に刷新（tokio::select! idle timeout）、§5.6 メッセージ中継経路を全段記述、§5.7 tokio-stream 依存追加 | Claude Sonnet 4.6 |
| 2026-02-26 | レビュー指摘対応 Round 2: §5.5 エラー時フォールバック（0文判定→逐次経路）追加、フラグ分岐を `streaming_enabled && port.is_some()` に修正、OQ-8 タイムアウト 4 層を §5.5 に明記・設定キー追加（§5.7）、§5.6 enqueue_playback を async fn に統一 | Claude Sonnet 4.6 |
| 2026-02-26 | レビュー指摘対応 Round 3: §5.3 streaming 専用 http client（connect_timeout のみ・既存 http_client() 不流用）追加・タイムアウト責務表修正、§5.7 に LLM_STREAMING_CONNECT_TIMEOUT_MS 追加、フォールバック文言を「逐次経路へフォールバック」に統一 | Claude Sonnet 4.6 |
| 2026-02-26 | レビュー指摘対応 Round 4: §5.3 `call_ollama_for_chat_stream` に `response_header_timeout` 引数追加・`send().await` を `tokio::time::timeout` で包む、§5.5 タイムアウト説明文を `connect_timeout + response_header_timeout` 前提に修正、§8.1 チェックリスト `futures::Stream` → `tokio-stream` に修正 | Claude Sonnet 4.6 |
| 2026-02-26 | レビュー指摘対応 Round 5: §4.2 に `src/shared/error/ai.rs`（`LlmError::Timeout(String)` 追加）を追記、§5.3 擬似コード前に `LlmError::Timeout` バリアント追加仕様を明記 | Claude Sonnet 4.6 |
| 2026-02-26 | レビュー指摘対応 Round 6: §5.5 タイムアウト責務表に `response-header` 行を追加（5層に統一）、§8.1 チェックリスト文言を「4層」→「5層（connect/response-header/first-token/chunk-idle/total）」に修正 | Claude Sonnet 4.6 |
| 2026-02-26 | OQ-9〜11 全て決定（推奨案採用）: §7.1「決定済み OQ-1〜11」に統合、§7.2 を「未決定なし」に更新、§8.1 チェックリストの OQ-9〜11 項目を完了マーク | @MasanoriSuda 決定 / Claude Sonnet 4.6 反映 |
