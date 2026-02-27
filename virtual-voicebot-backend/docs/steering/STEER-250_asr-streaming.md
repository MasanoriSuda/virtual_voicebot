# STEER-250: 真の ASR ストリーミング化（第2段階）

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-250 |
| タイトル | 真の ASR ストリーミング化（第2段階） |
| ステータス | Approved |
| 関連Issue | #250 |
| 前提 Issue | #249（LLM ストリーミング ＋ 文単位 TTS 先行再生）|
| 優先度 | P1 |
| 作成日 | 2026-02-26 |

---

## 2. ストーリー（Why）

### 2.1 背景

現行の ASR パイプラインは **「発話終了まで全音声をバッファし、完成した WAV を HTTP POST する」** 一括処理モデルである。

```
[RTP フレーム着信]
      │
      ▼
AudioCapture.ingest()   ← 20ms フレームを蓄積、RMS VAD でエンド検出
      │（end_silence または max_speech_ms に達したときだけ Some(full_utterance) を返す）
      ▼
AppEvent::AudioBuffered  ← 完全発話バッファ（pcm_mulaw + pcm_linear16）
      │
      ▼
transcribe_asr()         ← WAV ファイル書き出し → HTTP multipart POST → 完全テキスト返却
      │（ASR 待ち: ASR_LOCAL_TIMEOUT_MS デフォルト 3000ms）
      ▼
handle_user_text()       ← LLM → TTS → 再生
```

結果として：

- 発話終了から ASR 完了まで **最低 300～3000ms** の空白がある
- #249（LLM/TTS ストリーミング）で LLM 以降の first-audio latency は改善されるが、**ASR 待ち時間は温存される**
- 「話し終えた瞬間から応答が返るまで」のユーザー体感遅延は依然として大きい

### 2.2 目的

発話と並行して ASR を実行する（= 話しながら文字起こしを進める）ことで、
**発話終了 → テキスト確定 → LLM 開始 のギャップをほぼゼロにする**。

#249 と組み合わせることで first-audio latency の主要ボトルネック（ASR + LLM）を両方解消する。

### 2.3 ユーザーストーリー

```
As a 電話ユーザー
I want to ボイスボットが話し終えた直後から応答し始めてほしい
So that 違和感のない自然な会話テンポで操作できる

受入条件:
- [ ] 発話中に ASR がリアルタイムで音声を処理し始める
- [ ] 発話終了（VAD end_silence 検出）から LLM 開始までのギャップが 200ms 以内（目安）
- [ ] 既存逐次経路（VOICEBOT_ASR_STREAMING_ENABLED=false）で全既存テスト PASS
- [ ] ASR ストリーミングが利用不可の場合（フラグ OFF / port=None）は逐次経路へ自動フォールバック
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-26 |
| 起票理由 | STEER-249 §9.1「スコープ外（別 Issue 推奨）」で分割確定 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Sonnet 4.6 |
| 作成日 | 2026-02-26 |
| 指示者 | @MasanoriSuda |
| 指示内容 | Approved 後に引き継ぎ |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | @MasanoriSuda | 2026-02-26 | OK | Round 1〜3（重大6件・中4件・軽微2件）の指摘・対応を経て LGTM。OQ-1〜7 全決定。 |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-26 |
| 承認コメント | LGTM。OQ-1〜7 決定済み。Codex へ引き継ぎ可。 |

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
| docs/requirements/（AI 連携 RD） | 修正 | ASR ストリーミング要件追加 |
| docs/design/detail/（AI サービス DD） | 修正 | `AsrStreamPort` シグネチャ、WebSocket クライアント設計 |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| `src/shared/ports/ai/asr.rs` | 追加 | `AsrStreamPort` トレイト、`AsrAudioMsg`（`Chunk/End`）、`AsrStreamHandle`（`audio_tx + final_rx`）|
| `src/protocol/session/handlers/mod.rs` | 修正 | `InSpeech` 中に専用チャネル（`audio_chunk_tx`）へフレームを送出する分岐を追加 |
| `src/shared/ports/app.rs` | 追加 | `RtpAudioChunk` 型・`AudioChunkTx` ラッパー（latest-wins `try_send_latest` 付き）・`AudioChunkRx`（`AppEventRx` と同型、`async recv()` 提供）・`audio_chunk_channel()` コンストラクタ |
| `src/protocol/session/coordinator.rs` | 修正 | `audio_chunk_tx: AudioChunkTx`（or `Option`）フィールド追加、`audio_chunk_rx` を `spawn_app_worker()` に渡す |
| `src/protocol/session/capture.rs` | 修正 | `is_in_speech()` メソッド追加（`state` フィールドが非公開のため公開 accessor が必要） |
| `src/service/ai/mod.rs` | 追加 | Whisper WebSocket クライアント実装（`call_whisper_stream()`）|
| `src/service/call_control/mod.rs` | 修正 | `handle_audio_chunk()` 追加（専用チャネルから呼ばれる）、`handle_audio_buffer()` の ASR/SER 分離 |
| `script/whisper_server.py` または `script/whisper_server2.py` | 修正 | WebSocket エンドポイント追加（OQ-5 決定: 選択肢 A。対象ファイルは実装担当者が現行稼働中のものを確認して決定する） |
| `src/shared/config/mod.rs` | 追加 | `VOICEBOT_ASR_STREAMING_ENABLED`、`ASR_STREAMING_CONNECT_TIMEOUT_MS` 等の環境変数 |
| `Cargo.toml` | 修正 | `tokio-tungstenite`（WebSocket クライアント）依存追加（**OQ-1 参照**） |

---

## 5. 差分仕様（What / How）

### 5.1 前提：現状の確認

| 箇所 | 現状 |
|------|------|
| `src/protocol/session/capture.rs:89` | `AudioCapture::ingest()` は `CaptureState::InSpeech` 中は常に `None` を返し、`end_silence_ms` 達成または `max_speech_ms` 達成時のみ `Some(full_utterance)` を返す |
| `src/protocol/session/handlers/mod.rs:747` | `if let Some(buffer) = self.capture.ingest(&payload)` の場合のみ `AppEvent::AudioBuffered` を `try_send_latest()` する。中間フレームを app へ通知する経路は存在しない |
| `src/shared/ports/app.rs:9` | `AppEvent` に `AudioChunk` バリアントなし（`AudioBuffered` のみ） |
| `src/shared/ports/ai/asr.rs:5` | `AsrPort::transcribe_chunks()` は `Vec<AsrChunk>` 一括渡し、`AiFuture<Result<String, AsrError>>` 返却（ストリームなし） |
| `src/service/ai/mod.rs:503` | `transcribe_with_http_asr()` は WAV ファイルを `/tmp` に書き、HTTP multipart POST で送信。WebSocket 経路なし |
| `src/shared/error/ai.rs:24` | `AsrError::Timeout` は `#[error("Timeout")]` でフィールドなし（引数なし） |

### 5.2 `AsrStreamPort` 独立トレイト（ISP 準拠）

既存 `AsrPort`（`transcribe_chunks` → `Result<String>`）は変更しない。
LLM ストリーミングの `LlmStreamPort`（STEER-249 §5.2）と同じ設計方針で、**独立トレイト** として追加する。

```rust
// src/shared/ports/ai/asr.rs（追加）

/// 音声送信メッセージ（EOS を明示するための enum）
pub enum AsrAudioMsg {
    Chunk(Vec<u8>),  // μ-law フレーム（20ms 相当）
    End,             // 発話終端通知 → WebSocket クローズのトリガー
}

/// 部分転写イベント（OQ-4 の設計に従い定義する）
pub enum AsrStreamEvent {
    Partial(String),   // 途中の仮テキスト
    Final(String),     // 発話確定テキスト（ストリーム終端）
}

pub type AsrStream = Pin<Box<dyn Stream<Item = Result<AsrStreamEvent, AsrError>> + Send>>;

pub trait AsrStreamPort: Send + Sync {
    /// 音声フレーム（μ-law 8kHz）を WebSocket 経由で送信しながら部分転写を受信する。
    /// `AsrStreamHandle::audio_tx` にフレームを送出すると、内部 consumer task が
    /// WebSocket へ中継し、確定テキストを `final_rx` 経由で返す。
    fn transcribe_stream(
        &self,
        call_id: String,
        endpoint_url: String,
    ) -> AiFuture<Result<AsrStreamHandle, AsrError>>;
}

/// 音声送信 + 確定テキスト受信ハンドル
///
/// - `audio_tx`: `AsrAudioMsg::Chunk` でフレームを送出、`AsrAudioMsg::End` で EOS を明示する。
/// - `final_rx`: consumer task が `AsrStream` を消費して確定テキストを `oneshot` で通知する。
///   **全タイムアウト（first_partial / final）は consumer task が内部で処理するため、
///   呼び出し元は `final_rx.await` のみ行えばよい（外側の timeout 不要）。**
pub struct AsrStreamHandle {
    pub audio_tx: tokio::sync::mpsc::Sender<AsrAudioMsg>,                   // フレーム送出 + EOS
    pub final_rx: tokio::sync::oneshot::Receiver<Result<String, AsrError>>, // 確定テキスト（1回のみ）
}
```

**consumer task の責務（`AsrStreamPort` 実装内部で `tokio::spawn`）:**

```
consumer task の処理フロー:
  ┌─ tokio::select! ────────────────────────────────────────────┐
  │  audio_rx.recv() → Chunk → WebSocket へ転送                 │
  │                  → End   → WebSocket クローズ、final 待ちへ  │
  │  transcript_stream.next() → Partial → last_partial を更新   │
  │                           → Final  → oneshot に Ok(text) 送信│
  │  first_partial_timeout 超過 → oneshot に Err(Timeout) 送信   │
  └──────────────────────────────────────────────────────────────┘
  End 受信後の final_timeout 超過 → oneshot に Ok(last_partial) 送信（OQ-6: 劣化許容）
```

タイムアウトは consumer task が全て担うため、呼び出し元（`handle_audio_buffer`）は
`handle.final_rx.await` のみで確定テキストを取得できる。

> **設計論点 A**（OQ-1）: WebSocket vs chunked HTTP（HTTP/2 streaming）どちらを採用するか。
> 設計例は WebSocket で記述しているが、Whisper サーバー側の対応方式に依存する。

### 5.3 `AppWorker` への接続方針

STEER-249 の `llm_stream_port: Option<Arc<dyn LlmStreamPort>>` と同じく、
`AppWorker` に **別フィールド** として追加する（`AiServices` 集約トレイトは変更しない）。

```rust
// src/service/call_control/mod.rs（修正イメージ）

pub struct AppWorker {
    // ...（既存フィールド）
    asr_stream_port: Option<Arc<dyn AsrStreamPort>>,  // 追加（None = フラグ OFF or 未対応）
}
```

フラグ `VOICEBOT_ASR_STREAMING_ENABLED=true` かつ `asr_stream_port.is_some()` の場合のみ
ストリーミング経路へ進む。どちらかが欠けると逐次経路（`transcribe_asr()` 既存実装）へフォールバック。

### 5.4 中間音声フレームの配送（`AudioCapture` → `AppWorker`）

現行: `InSpeech` 中は `ingest()` が `None` を返すため app 側には何も届かない。

**SER 経路の保護（重要制約）**

現行 `handle_audio_buffer()` は `pcm_linear16` を引数として受け取り `analyze_ser()` を呼ぶ
（`src/service/call_control/mod.rs:370–377`）。**`AudioBuffered` を廃止すると SER 入力が失われる**ため、
ストリーミング有効時も `AudioBuffered` は引き続き発火させる。

```
ストリーミング有効時の役割分担:
  AudioChunk  → ASR WebSocket へ中継（専用チャネル経由）
  AudioBuffered → SER のみ実行（ASR は skip し、final_rx から transcript を取得）
```

**専用チャネルの分離（バックプレッシャ対応）**

現行 `AppEvent` チャネルは容量 16（`APP_EVENT_CHANNEL_CAPACITY = 16`、`mod.rs:61`）の共有チャネルであり、
20ms 毎に RTP フレームを送信すると制御イベント（`CallRinging` / `CallEnded` 等）を圧迫する可能性がある。

**→ `RtpAudioChunk` 専用の bounded channel を追加し、制御チャネルと分離する。**
**→ 「満杯なら最古を捨てる（latest-wins）」挙動は `AppEventTx::try_send_latest()` と同様のラッパーで実装する。**

```rust
// src/shared/ports/app.rs（追加）

pub struct RtpAudioChunk {
    pub call_id: CallId,
    pub pcm_mulaw: Vec<u8>,  // 20ms μ-law フレーム（160 bytes）
}

/// latest-wins セマンティクスを持つ専用チャネル送信側
/// （AppEventTx::try_send_latest と同じ方式: 満杯なら最古を drop して再 try_send）
pub struct AudioChunkTx {
    tx: mpsc::Sender<RtpAudioChunk>,
    rx: Arc<Mutex<mpsc::Receiver<RtpAudioChunk>>>,
}

impl AudioChunkTx {
    /// AppEventTx::try_send_latest() と同方式:
    /// TrySendError::Full から chunk を回収して最古を drop 後に再 try_send する。
    pub fn try_send_latest(&self, chunk: RtpAudioChunk) {
        match self.tx.try_send(chunk) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(chunk)) => {
                if let Ok(mut rx) = self.rx.try_lock() {
                    let _ = rx.try_recv();  // 最古を破棄
                }
                let _ = self.tx.try_send(chunk); // 最新を再送
            }
            Err(_) => {}  // Disconnected は無視
        }
    }
}

/// AppEventRx と同構造: Arc<Mutex<Receiver>> を保持し非同期 recv を提供する
pub struct AudioChunkRx {
    rx: Arc<Mutex<mpsc::Receiver<RtpAudioChunk>>>,
}

impl AudioChunkRx {
    pub async fn recv(&self) -> Option<RtpAudioChunk> {
        let mut rx = self.rx.lock().await;
        rx.recv().await
    }
}

/// AppEventTx/AppEventRx と同型パターン: Arc<Mutex<Receiver>> を Tx/Rx で共有する
pub fn audio_chunk_channel(capacity: usize) -> (AudioChunkTx, AudioChunkRx) {
    let (tx, rx) = mpsc::channel(capacity);
    let shared = Arc::new(Mutex::new(rx));
    (
        AudioChunkTx { tx, rx: Arc::clone(&shared) },
        AudioChunkRx { rx: shared },
    )
}
```

**`is_in_speech()` 判定の順序（重要）**

`AudioCapture::ingest()` は内部で `Idle → InSpeech` 遷移を行うため、
**`ingest()` の前に `is_in_speech()` を呼ぶと発話開始直後の第1フレームが `false` となり、
最初の 20ms フレームを ASR ストリームへ送れない**。

→ **`ingest()` を呼んだ後に `is_in_speech()` をチェックする**。
→ `ingest()` が `Some(buffer)` を返した場合（発話終了）は `reset_state()` 済みで `InSpeech=false` となるため、
　 終端フレームを二重送信することはない。

```rust
// src/protocol/session/handlers/mod.rs:747 付近（修正イメージ）

let capture_result = self.capture.ingest(&payload);

// ingest() 後にチェック → 発話開始直後の第1フレームも捕捉できる
// ingest() が Some を返した場合は reset_state() 済みで is_in_speech()=false になるため二重送信なし
if self.capture.is_in_speech() && voicebot_asr_streaming_enabled() {
    if let Some(tx) = &self.audio_chunk_tx {
        tx.try_send_latest(RtpAudioChunk {
            call_id: self.call_id.clone(),
            pcm_mulaw: payload.clone(),
        });
    }
}

if let Some(buffer) = capture_result {
    // 既存: AudioBuffered（発話終了通知）は SER 用に維持
    let pcm_linear16 = buffer.iter().map(|&b| mulaw_to_linear16(b)).collect();
    let _ = self.app_tx.try_send_latest(AppEvent::AudioBuffered {
        call_id: self.call_id.clone(),
        pcm_mulaw: buffer,
        pcm_linear16,
    });
    self.capture.start();
}
```

> **注**: `AudioCapture::is_in_speech()` の追加が必要（`state: CaptureState` フィールドが `pub(super)` のため公開 accessor が必要）。
> **注**: `AppEvent::AudioChunk` バリアントは追加しない（専用チャネル `AudioChunkTx` で代替するため）。

### 5.5 `call_control` ストリーミング ASR 分岐

**`audio_chunk_rx` の所有者と起動（重大2対応）**

`audio_chunk_tx / audio_chunk_rx` ペアは `SessionCoordinator`（または `spawn_app_worker()`）で生成する。
`audio_chunk_rx` は `AppWorker` が所有し、`AppEventRx`（制御イベント）と並列に `tokio::select!` で監視する。

```
SessionCoordinator                         AppWorker
  audio_chunk_tx ──channel──▶ audio_chunk_rx ─▶ handle_audio_chunk()
  app_tx         ──channel──▶ AppEventRx     ─▶ handle_audio_buffer() / handle_call_xxx()
```

`AppWorker` のメインループ（修正イメージ）:

```rust
// AppWorker の run() 内
loop {
    tokio::select! {
        Some(chunk) = audio_chunk_rx.recv() => {
            self.handle_audio_chunk(&chunk.call_id, chunk.pcm_mulaw).await;
        }
        Some(event) = app_event_rx.recv() => {
            match event {
                AppEvent::AudioBuffered { call_id, pcm_mulaw, pcm_linear16 } =>
                    self.handle_audio_buffer(&call_id, pcm_mulaw, pcm_linear16).await?,
                // ... 他のイベント
            }
        }
    }
}
```

> **停止時の扱い**: `audio_chunk_tx` を `SessionCoordinator` が drop すると
> `audio_chunk_rx.recv()` が `None` を返す → `select!` 側でそのブランチが無効化される。

```rust
// src/service/call_control/mod.rs（修正イメージ）

// AppWorker に追加するフィールド
asr_stream_handle: Option<AsrStreamHandle>,  // 発話中のみ Some

/// 専用チャネルから RTP チャンクを受け取り ASR ストリームへ中継する
async fn handle_audio_chunk(&mut self, call_id: &CallId, pcm_mulaw: Vec<u8>) {
    let Some(port) = &self.asr_stream_port else { return; };

    if self.asr_stream_handle.is_none() {
        // 発話開始 → WebSocket 接続 + consumer task 起動（consumer task がタイムアウトも担う）
        let url = config::asr_streaming_url();
        match port.transcribe_stream(call_id.to_string(), url).await {
            Ok(handle) => { self.asr_stream_handle = Some(handle); }
            Err(e) => {
                log::warn!("[asr stream {call_id}] connection failed: {e}; fallback to sequential");
                return;
            }
        }
    }

    // フレームを consumer task 経由で WebSocket へ送出（EOS ではない）
    if let Some(handle) = &self.asr_stream_handle {
        let _ = handle.audio_tx.try_send(AsrAudioMsg::Chunk(pcm_mulaw));
    }
}

/// AudioBuffered（発話終了）を受け取る — SER は必ず実行、ASR はストリーミング/逐次を切り替える
async fn handle_audio_buffer(&mut self, call_id: &CallId, pcm_mulaw: Vec<u8>, pcm_linear16: Vec<i16>) {
    // SER は常に実行（pcm_linear16 を使用するため AudioBuffered を廃止しない）
    self.analyze_ser(call_id, pcm_linear16).await;

    let user_text = if let Some(handle) = self.asr_stream_handle.take() {
        // EOS 送出 → consumer task が first_partial/final タイムアウトを内部処理 → oneshot で結果返却
        let _ = handle.audio_tx.send(AsrAudioMsg::End).await;
        // consumer task がタイムアウトを全て担うため、呼び出し元は final_rx.await のみ行う
        match handle.final_rx.await {
            Ok(Ok(text)) => text,
            Ok(Err(e)) => {
                // first_partial タイムアウト or WebSocket エラー → 逐次フォールバック
                log::warn!("[asr stream {call_id}] consumer task error: {e}; fallback to sequential");
                self.transcribe_asr(call_id, pcm_mulaw).await
            }
            Err(_) => {
                // consumer task が予期せず drop → 逐次フォールバック
                log::warn!("[asr stream {call_id}] consumer task dropped; fallback to sequential");
                self.transcribe_asr(call_id, pcm_mulaw).await
            }
        }
    } else {
        // 逐次経路（ストリーミング OFF / handle=None）
        self.transcribe_asr(call_id, pcm_mulaw).await
    };

    let trimmed = user_text.trim();
    if trimmed.is_empty() { /* ... 既存: sorry audio */ }
    self.handle_user_text(call_id, trimmed).await
}
```

**タイムアウト責務（consumer task に集約）**

| タイムアウト種別 | 担当箇所 | 設定キー |
|----------------|---------|---------|
| WebSocket 接続（TCP + handshake） | `AsrStreamPort` 実装の connect_timeout | `ASR_STREAMING_CONNECT_TIMEOUT_MS`（新規） |
| 発話開始 → 最初の Partial 受信 | consumer task 内 `tokio::select!` | `ASR_STREAMING_FIRST_PARTIAL_TIMEOUT_MS`（新規）|
| EOS 後の Final 受信待ち | consumer task 内 `tokio::time::timeout` | `ASR_STREAMING_FINAL_TIMEOUT_MS`（新規） |

consumer task が全タイムアウトを処理し `final_rx` へ結果を送信するため、
`handle_audio_buffer()` 側での `tokio::time::timeout` ラップは不要。

- `first_partial` タイムアウト → `final_rx` に `Err(AsrError::Timeout)` → 呼び出し元が逐次フォールバック
  （現行 `AsrError::Timeout` はフィールドなし。ログ用コンテキストが必要な場合は実装時に `AsrError::Timeout(String)` へ拡張可。§5.1 参照）
- `final` タイムアウト → `final_rx` に `Ok(last_partial)` → OQ-6 の劣化許容（最後の Partial を使う）

### 5.6 Whisper サーバーサイド変更（OQ-5）

現行のローカル Whisper サーバースクリプト（`script/whisper_server.py` / `script/whisper_server2.py`）は
HTTP multipart エンドポイントのみ実装しており、WebSocket 経路は存在しない。
（採用対象のファイルは OQ-5 決定後に確定する。）

真の ASR ストリーミングには以下のどちらかが必要：

| 選択肢 | 概要 | 備考 |
|--------|------|------|
| A: WebSocket エンドポイント追加 | `ws://host/transcribe_stream` で音声チャンクを受け付け、Partial/Final を返す | Faster-Whisper の `transcribe()` をチャンク対応させる必要あり |
| B: whisper.cpp サーバーへ移行 | whisper.cpp の `--stream` モードを使い WebSocket をそのまま利用 | Python サーバーを廃止・移行コスト大 |

> 選択は OQ-5 で決定する。

### 5.7 機能フラグ

```toml
# .env または EnvironmentFile

# ASR ストリーミングモード有効化（デフォルト false = 既存逐次動作）
# NOTE: ON でも asr_stream_port が None の場合は逐次経路へフォールバック
VOICEBOT_ASR_STREAMING_ENABLED=false

# ASR ストリーミング用 WebSocket URL
# ※ ASR_LOCAL_SERVER_URL（HTTP）とは別設定
ASR_STREAMING_SERVER_URL=ws://localhost:9001/transcribe_stream

# WebSocket 接続タイムアウト ms
ASR_STREAMING_CONNECT_TIMEOUT_MS=3000

# 最初の Partial 受信タイムアウト ms
ASR_STREAMING_FIRST_PARTIAL_TIMEOUT_MS=3000

# 発話終了後の Final 受信タイムアウト ms
ASR_STREAMING_FINAL_TIMEOUT_MS=2000
```

**`Cargo.toml` への依存追加**

```toml
# [dependencies] に追加（OQ-1 で確定後）
tokio-tungstenite = "0.26"  # WebSocket クライアント（OQ-1 決定: WebSocket 採用）
```

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #250 | STEER-250 | 起票 |
| STEER-249 §9.1 | STEER-250 | スコープ分割（前提 Issue） |
| STEER-250 | `AsrStreamPort` | 新規 Port 追加 |
| STEER-250 | `RtpAudioChunk`（専用チャネル型）| 新規型追加（`AppEvent` 共有チャネルとは別） |
| STEER-250 | `AudioCapture::is_in_speech()` | 既存コンポーネント拡張 |
| STEER-250 | `AsrAudioMsg`（`Chunk/End`） | EOS 明示のための enum 追加 |
| STEER-250 | Whisper サーバー WebSocket 対応 | サーバーサイド変更 |
| `AsrStreamPort` | Whisper WebSocket 実装 | 実装 |
| `AsrStreamPort` | UT（streaming Port mock） | テスト |

---

## 7. オープンクエスチョン

### 7.1 決定済み（OQ-1〜7）

> すべて @MasanoriSuda により 2026-02-26 承認。

| # | 質問 | 選択肢 | 決定 |
|---|------|--------|------|
| OQ-1 | WebSocket か chunked HTTP か？ | WebSocket / chunked HTTP（HTTP/2） | **WebSocket**（Whisper.cpp の既存実装が WebSocket ベース、ブラウザ・ライブラリとも広くサポート） |
| OQ-2 | `RtpAudioChunk` の送信粒度は？ | RTP フレーム毎（20ms）/ N フレームまとめ（例 100ms） | **RTP フレーム毎（20ms）**（最小遅延のため）。精度劣化が問題になる場合は将来 `ASR_STREAMING_CHUNK_MS` 等の config を追加して N フレームまとめに変更できる余地を残す |
| OQ-3 | ストリーミング有効時の `AudioBuffered` の役割は？ | AudioBuffered を廃止し AudioChunk に一本化 / AudioBuffered を SER 専用として維持 | **`AudioBuffered` を SER 専用として維持する**（`handle_audio_buffer()` は `pcm_linear16` を `analyze_ser()` へ渡す必要があり、廃止すると SER 入力が失われる） |
| OQ-4 | `AsrStreamEvent` の終端設計は？ | `Stream::None` のみ / 明示的な `Final(String)` | **明示的な `Final(String)`**（#249 OQ-9 と同じ方針。再生・状態遷移の曖昧さを排除） |
| OQ-5 | Whisper サーバーの変更方針は？ | 既存スクリプトに WebSocket 追加 / whisper.cpp サーバーへ移行 | **既存スクリプトに WebSocket エンドポイント追加（選択肢 A）**（移行コストを最小化。精度要件が上がれば B を再検討）。対象ファイル（`whisper_server.py` / `whisper_server2.py`）は実装担当者が現行稼働中のものを確認して決定する |
| OQ-6 | `ASR_STREAMING_FINAL_TIMEOUT_MS` 超過時の扱いは？ | 最後の Partial をそのまま使って LLM へ / 逐次経路（HTTP ASR）へフォールバック | **最後の Partial を使って LLM へ（劣化許容）**（フォールバック時に再度 HTTP ASR を呼ぶとレイテンシが倍増するため） |
| OQ-7 | OpenAI ASR streaming（Realtime API）を今回スコープに含めるか？ | 含む / 含まない | **含まない**（Realtime API は別プロトコル・課金体系。まずローカル Whisper のみ） |

---

## 8. レビューチェックリスト

### 8.1 仕様レビュー（Review → Approved）

- [ ] ストーリー（Why）と目的が合意されているか
- [ ] `AsrStreamPort` のシグネチャが実装可能か（tokio-tungstenite 依存 OK か）
- [ ] `AudioBuffered` が SER 専用として維持され、`analyze_ser()` 呼び出しが保たれるか（SER 入力が失われないか）
- [ ] `AudioChunk` を共有 `AppEvent` チャネルに混ぜず、専用チャネルで分離しているか（バックプレッシャ方針）
- [ ] `AsrAudioMsg::End` で EOS が明示され、`final_rx` から確定テキストを取得できるか
- [ ] `transcript_rx`（`AsrStream`）が consumer task 1箇所のみで消費され、`final_rx` 経由で結果を受け取るか（二重消費なし）
- [ ] `AudioCapture::is_in_speech()` 追加が既存テストと競合しないか
- [ ] 機能フラグ（`VOICEBOT_ASR_STREAMING_ENABLED=false`）で既存動作が保護されるか
- [ ] タイムアウト方針が `AGENTS.md:116` の外部 I/O timeout 必須要件を満たすか（connect/first-partial/final の 3 層）
- [ ] `asr_stream_port=None` 時に逐次経路へフォールバックするか
- [ ] Final タイムアウト時の挙動（最後の Partial を使う）が受入条件に落ちているか
- [ ] Whisper サーバー側変更のスコープ（OQ-5）が決定されているか
- [ ] OQ-1〜7 が全て決定されているか
- [ ] `audio_chunk_rx` が `AppWorker` に所有され `tokio::select!` で `AppEventRx` と並列監視されているか（`SessionCoordinator` → `spawn_app_worker()` 経由で受け渡す設計が §5.5 に明記されているか）
- [ ] consumer task が `first_partial_timeout` / `final_timeout` を内部処理し、`final_rx` に `Err(AsrError::Timeout)` / `Ok(last_partial)` を送信するか（`handle_audio_buffer()` 側で外側 `timeout` ラップ不要なことが明確か）
- [ ] `is_in_speech()` チェックが `ingest()` の **後** に置かれており、発話開始直後の第1フレーム（`Idle→InSpeech` 遷移直後）も ASR ストリームへ送出できるか
- [ ] `AudioChunkTx::try_send_latest()` が `AppEventTx::try_send_latest()` と同方式（`TrySendError::Full` から値を回収して再 `try_send`）で実装されているか（`is_err_and` で値を消費する pseudocode 誤りを踏まず、正しく回収する）

### 8.2 マージ前チェック（Approved → Merged）

- [ ] 実装完了（Codex）
- [ ] `VOICEBOT_ASR_STREAMING_ENABLED=false` で既存テスト全 PASS
- [ ] `VOICEBOT_ASR_STREAMING_ENABLED=true` で手動通話テスト実施（Partial → Final フロー確認）
- [ ] CodeRabbit レビュー対応済み
- [ ] 本体仕様書への反映方針確認

---

## 9. 備考

### 9.1 スコープ外（別 Issue 推奨）

- **OpenAI ASR Realtime API 対応**: WebRTC/WebSocket ベースの別プロトコル。別 Issue で検討
- **真の TTS ストリーミング**: VOICEVOX `/synthesis` streaming endpoint 対応（第 3 段階）
- **barge-in（ASR ストリーミング中の割り込み）**: ASR WebSocket をキャンセルする設計が別途必要

### 9.2 前提 Issue との関係

STEER-249（#249）の実装が完了している前提で本 Issue を実装する。
具体的には `AppEnqueueBotAudioFile` の再生キュー機能が実装済みであることが必要
（ASR 確定後すぐに LLM → TTS → 再生キューへ投入するフローを使うため）。

### 9.3 参照コード（調査時点のスナップショット）

| 参照 | 内容 |
|------|------|
| `src/protocol/session/capture.rs:89` | `AudioCapture::ingest()` — VAD ステートマシン本体 |
| `src/protocol/session/capture.rs:6` | `CaptureState::Idle / InSpeech` — `InSpeech` 中はフレームを `payloads` に蓄積するのみ |
| `src/protocol/session/handlers/mod.rs:747` | `capture.ingest()` 呼び出しと `AudioBuffered` 送信 |
| `src/shared/ports/app.rs:9` | `AppEvent` 定義（`AudioChunk` バリアントなし） |
| `src/shared/ports/ai/asr.rs:5` | `AsrPort::transcribe_chunks()` — `Vec<AsrChunk>` 一括渡し |
| `src/service/ai/mod.rs:503` | `transcribe_with_http_asr()` — WAV ファイル経由 HTTP POST |
| `src/service/ai/mod.rs:916` | `transcribe_and_log()` — cloud → local → raspi フォールバック |
| `src/shared/error/ai.rs:4` | `AsrError::Timeout`（引数なし、`AsrError::TranscriptionFailed(String)` が引数あり） |
| `virtual-voicebot-backend/AGENTS.md:116` | 外部 I/O は timeout 必須 |

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-26 | 初版作成（Draft） | Claude Sonnet 4.6 |
| 2026-02-26 | レビュー指摘対応 Round 1: §5.2 AsrStreamHandle に AsrAudioMsg（EOS 明示）・final_rx（oneshot）追加、§5.4 AudioChunk を専用チャネル分離・AudioBuffered を SER 専用維持、§5.5 handle_audio_buffer の ASR/SER 分離・consumer task の責務明記、§5.6 Whisper スクリプトファイル名修正、OQ-2・3 推奨案修正、§4.2 影響範囲・§8.1 チェックリスト更新 | Claude Sonnet 4.6 |
| 2026-02-26 | レビュー指摘対応 Round 2: §5.2 consumer task 責務フロー図追加・タイムアウト（first_partial/final）を consumer task に完全集約、§5.4 AudioChunkTx ラッパー定義（latest-wins `try_send_latest`）・`is_in_speech()` チェック順序を `ingest()` 後に修正（第1フレーム捕捉）、§5.5 `audio_chunk_rx` 所有者と `tokio::select!` ループ明記・`handle_audio_buffer` を `final_rx.await` のみに簡素化・タイムアウト責務表更新、§4.2 `RtpAudioChunk/AudioChunkTx` エントリ追加・`coordinator.rs` への受け渡し追記、§8.1 Round 2 対応チェック項目4件追加 | Claude Sonnet 4.6 |
| 2026-02-26 | レビュー指摘対応 Round 3（軽微2件）: §5.5 `AsrError::Timeout(...)` → `AsrError::Timeout`（引数なしに統一・`AsrError` 拡張は実装時判断の注記追加）、§5.4 `AudioChunkRx` 型を追加（`AppEventRx` と同型パターン・`try_send_latest` pseudocode を `TrySendError::Full` 値回収方式に修正）、§4.2 `AudioChunkRx` エントリ追記 | Claude Sonnet 4.6 |
| 2026-02-26 | OQ-1〜7 全決定（@MasanoriSuda 承認）: §7.1「未決定」→「決定済み」・推奨案→決定に列名変更・承認者/日付注記追加。OQ-2 に 100ms まとめへの将来 config 余地注記。OQ-5 に対象ファイルは実装時確認の注記。§5.7 Cargo.toml コメント・§4.2 whisper 行を決定内容に更新 | Claude Sonnet 4.6 |
| 2026-02-26 | ステータス Draft → Approved（@MasanoriSuda LGTM）: §1 ステータス更新、§3.3 レビュー結果・§3.4 承認欄記入 | Claude Sonnet 4.6 |
