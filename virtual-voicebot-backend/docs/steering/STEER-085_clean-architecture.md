# STEER-085: クリーンアーキテクチャ移行（ISP準拠 + ファイル分割）

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-085 |
| タイトル | クリーンアーキテクチャ移行（ISP準拠 + ファイル分割） |
| ステータス | Draft |
| 関連Issue | #52, #65, #85 |
| 優先度 | P0 |
| 作成日 | 2026-01-31 |

---

## 2. ストーリー（Why）

### 2.1 背景

Issue #52 および #65 のレビューにより、以下の課題が特定された：

**アーキテクチャ上の課題（Issue #52）**
- 現行の Hexagonal Architecture では、セッション管理がチャネル経由で行われており、スケーラビリティと可読性に限界がある
- 1プロセス = 1 SIP ポート + 1 RTP ポート の制約があり、水平スケールが困難

**コード品質の課題（Issue #65）**
- **大規模ファイル**: `sip/mod.rs`（2,041行）、`session/session.rs`（1,578行）、`session/b2bua.rs`（1,205行）
- **ISP違反**: `AiPort` トレイトが 5 つの無関係な責務を混合（ASR/Intent/LLM/Weather/TTS + SerPort）
- **エンティティ層の欠如**: ドメインモデルが明確に分離されていない
- **エラー型の不足**: `anyhow::Result` のみで、ドメイン固有エラーが未定義

### 2.2 目的

- **クリーンアーキテクチャ + tokio channel ベースモデル** への段階的移行（§8.1 決定事項参照）
- インターフェース分離原則（ISP）に準拠したトレイト設計
- ドメインモデル（エンティティ層）の明確化
- テスタビリティの向上

### 2.3 ユーザーストーリー

```
As a 開発者
I want to 責務が明確に分離されたアーキテクチャを持つ
So that 変更が局所化され、テストが容易になる

受入条件:
- [ ] AiPort が 5 つの個別トレイトに分割されている
- [ ] Session が状態マシン + I/O コーディネータに分離されている
- [ ] エンティティ層（entities/）が新設されている
- [ ] ドメイン固有エラー型が定義されている
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-01-31 |
| 起票理由 | Issue #52, #65 のアーキテクチャレビュー結果 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code |
| 作成日 | 2026-01-31 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "クリーンアーキテクチャとRustのあるべき姿にしたい" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | - | - | - | |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | - |
| 承認日 | - |
| 承認コメント | |

### 3.5 実装（該当する場合）

| 項目 | 値 |
|------|-----|
| 実装者 | Codex |
| 実装日 | 2026-02-01 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "本ステアリングに基づき実装" |
| 進捗 | Phase 1: AI/Notification Port 分割・ドメインエラー導入・compat 追加／Phase 2: entities 新設、SessionCoordinator+StateMachine 分離、Session 録音/ingest/rtp を manager へ移管、sip/core 分割（core.rs 抽出・types/codec/transport 参照へ移行）（進行中） |
| コードレビュー | - |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | - |
| マージ日 | - |
| マージ先 | DD-010, UT-010 |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| docs/design/detail/DD-010_clean-architecture.md | 新規 | クリーンアーキテクチャ詳細設計 |
| docs/test/unit/UT-010_ports.md | 新規 | ポートトレイト単体テスト仕様 |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| src/ports/ai.rs | 修正 | AiPort を 5 個別トレイトに分割 |
| src/ports/mod.rs | 修正 | 新ポートのエクスポート追加 |
| src/entities/ | 新規 | ドメインモデル（Call, Session, Recording） |
| src/error/ | 修正 | ドメイン固有エラー型追加 |
| src/session/session.rs | 修正 | SessionStateMachine + I/O 分離 |
| src/session/recording_manager.rs | 新規 | 録音ライフサイクル管理 |
| src/session/ingest_manager.rs | 新規 | イベント発行管理 |
| src/ai/mod.rs | 修正 | 個別トレイト実装に変更 |

---

## 5. アーキテクチャ原則（Architecture Principles）

> **本セクションの詳細は [BD-003_clean-architecture.md](../design/basic/BD-003_clean-architecture.md) に昇格しました。**
>
> 開発者向けサマリは [CONVENTIONS.md](../../CONVENTIONS.md) を参照してください。

### 5.1 概要（サマリ）

本プロジェクトは **Clean Architecture + DDD + EDA** を採用する。

#### レイヤー構造

```
Frameworks & Drivers  (infrastructure/)
        ↓
Interface Adapters    (adapters/)
        ↓
Application Rules     (app/, session/)
        ↓
Enterprise Rules      (entities/, domain/)
```

> **Note**: `adapters/` は Interface Adapters 層に属する。`infrastructure/` は Frameworks & Drivers 層（SIP/RTP スタック、外部ライブラリ直接利用）に属する。

#### 5.1.2 主要原則（詳細は BD-003 参照）

| 原則 | 内容 |
|------|------|
| **Dependency Rule** | 依存は常に内側へ。外側は内側を知るが、内側は外側を知らない |
| **Dependency Inversion** | Port（トレイト）を介して外部依存を逆転 |
| **ISP** | 1 トレイト = 1 責務 |

### 5.2 駆動形

| 駆動形 | 適用内容 |
|--------|---------|
| **DDD** | Entity, Value Object, Aggregate, Repository, Domain Event |
| **EDA** | Protocol Event → Session Event → Session Command → Domain Event |

### 5.3 必須デザインパターン

| パターン | 用途 |
|----------|------|
| Repository | データアクセス抽象化 |
| Factory | オブジェクト生成 |
| Strategy | AI プロバイダ選択 |
| State | 状態遷移管理 |
| Adapter | 外部サービス接続 |

### 5.4 ディレクトリ構造（詳細は BD-003 §6 参照）

```
src/
├── entities/       # Enterprise Business Rules
├── domain/         # Domain Services & Events
├── ports/          # Port 定義
├── app/            # Application Business Rules
├── session/        # Session 管理
├── adapters/       # Interface Adapters
├── infrastructure/ # Frameworks & Drivers
├── error/          # エラー型定義
└── compat/         # 後方互換
```

---

## 6. 差分仕様（What / How）

### 6.1 フェーズ概要

本リファクタリングは段階的に実施する（§8 の決定事項に基づき修正）：

| Phase | 名称 | 概要 | 状態 |
|-------|------|------|------|
| 1 | ポートトレイト分離 | ISP 準拠のトレイト設計、tokio channel ベース | **本ステアリング対象** |
| 2 | Domain 分離 + ファイル分割 | entities/ 層新設、sip/mod.rs 分割 | **本ステアリング対象** |
| 3 | Event Sourcing | イベントソーシング基盤 | **保留**（要件刺さるまで） |
| 4 | CQRS | 読み書き分離 | **保留**（Phase 3 依存） |

**本ステアリングのスコープ: Phase 1 + Phase 2**

> **Note**: Actix Actor は当面導入せず、tokio の channel ベースで実装（§8.1）

---

### 6.2 Phase 1: ポートトレイト分離

#### 6.2.1 現状の問題点

```rust
// 現行: 5 つの無関係な責務を混合
pub trait AiPort: Send + Sync {
    fn transcribe_chunks(...) -> AiFuture<Result<String>>;  // ASR
    fn classify_intent(...) -> AiFuture<Result<String>>;     // Intent
    fn generate_answer(...) -> AiFuture<Result<String>>;     // LLM
    fn handle_weather(...) -> AiFuture<Result<String>>;      // Weather
    fn synth_to_wav(...) -> AiFuture<Result<String>>;        // TTS
}
```

**ISP 違反**:
- クライアントが ASR のみ必要な場合でも、5 メソッド全ての実装が必要
- テスト時のモック作成が困難
- 各サービスの異なるエラー型・タイムアウト・リトライポリシーを表現できない

#### 6.2.2 改善後のトレイト設計

```rust
// src/ports/asr.rs
pub trait AsrPort: Send + Sync {
    /// Phase 1: call_id は String で受け取る（後方互換）
    /// Phase 2 以降: CallId 値オブジェクトへ移行予定（§6.3.4 参照）
    fn transcribe_chunks(
        &self,
        call_id: String,  // TODO(Phase2): CallId に変更
        chunks: Vec<AsrChunk>,
    ) -> AiFuture<Result<String, AsrError>>;
}

// src/ports/intent.rs
pub trait IntentPort: Send + Sync {
    fn classify_intent(
        &self,
        text: String,
    ) -> AiFuture<Result<Intent, IntentError>>;
}

// src/ports/llm.rs
pub trait LlmPort: Send + Sync {
    fn generate_answer(
        &self,
        messages: Vec<ChatMessage>,
    ) -> AiFuture<Result<String, LlmError>>;
}

// src/ports/weather.rs
pub trait WeatherPort: Send + Sync {
    fn handle_weather(
        &self,
        query: WeatherQuery,
    ) -> AiFuture<Result<WeatherResponse, WeatherError>>;
}

// src/ports/tts.rs
pub trait TtsPort: Send + Sync {
    fn synth_to_wav(
        &self,
        text: String,
        path: Option<String>,
    ) -> AiFuture<Result<PathBuf, TtsError>>;
}

// src/ports/ser.rs（感情認識は別トレイト維持）
pub trait SerPort: Send + Sync {
    fn analyze(
        &self,
        input: SerInputPcm,
    ) -> AiFuture<Result<SerOutcome, SerError>>;
}

// 後方互換用：全サービスを束ねる場合
pub trait AiServices: AsrPort + IntentPort + LlmPort + WeatherPort + TtsPort + SerPort {}
```

#### 6.2.3 ドメイン固有エラー型

```rust
// src/error/ai.rs
#[derive(Debug, thiserror::Error)]
pub enum AsrError {
    #[error("Transcription failed: {0}")]
    TranscriptionFailed(String),
    #[error("Audio too short")]
    AudioTooShort,
    #[error("Service unavailable")]
    ServiceUnavailable,
    #[error("Timeout")]
    Timeout,
}

#[derive(Debug, thiserror::Error)]
pub enum IntentError {
    #[error("Classification failed: {0}")]
    ClassificationFailed(String),
    #[error("Unknown intent")]
    UnknownIntent,
}

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("Generation failed: {0}")]
    GenerationFailed(String),
    #[error("Context too long")]
    ContextTooLong,
    #[error("Rate limited")]
    RateLimited,
}

#[derive(Debug, thiserror::Error)]
pub enum TtsError {
    #[error("Synthesis failed: {0}")]
    SynthesisFailed(String),
    #[error("Text too long")]
    TextTooLong,
    #[error("Voice not found")]
    VoiceNotFound,
}

#[derive(Debug, thiserror::Error)]
pub enum WeatherError {
    #[error("Weather query failed: {0}")]
    QueryFailed(String),
    #[error("Location not found")]
    LocationNotFound,
    #[error("Service unavailable")]
    ServiceUnavailable,
}

#[derive(Debug, thiserror::Error)]
pub enum SerError {
    #[error("SER analysis failed: {0}")]
    AnalysisFailed(String),
    #[error("Audio format invalid")]
    InvalidFormat,
    #[error("Model not loaded")]
    ModelNotLoaded,
}
```

---

### 6.3 Phase 2: エンティティ層の新設

#### 6.3.1 現状の問題点

- ドメインモデルが `session/types.rs` に散在
- 値オブジェクトと識別子の区別が曖昧
- ドメイン不変条件がコード全体に分散

#### 6.3.2 エンティティ層の構造

```
src/entities/
├── mod.rs           # エクスポート
├── call.rs          # Call エンティティ
├── session.rs       # Session 値オブジェクト
├── recording.rs     # Recording エンティティ
├── participant.rs   # Participant 値オブジェクト
└── identifiers.rs   # 識別子（CallId, SessionId, etc.）
```

#### 6.3.3 Call エンティティ

```rust
// src/entities/call.rs
use crate::entities::identifiers::{CallId, SessionId};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Call {
    id: CallId,
    session_id: SessionId,
    from: Participant,
    to: Participant,
    state: CallState,
    started_at: DateTime<Utc>,
    ended_at: Option<DateTime<Utc>>,
    recordings: Vec<RecordingRef>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CallState {
    Setup,
    Ringing,
    Active,
    Releasing,
    Ended(EndReason),
}

#[derive(Debug, Clone, PartialEq)]
pub enum EndReason {
    Normal,
    Cancelled,
    Rejected,
    Timeout,
    Error(String),
}

impl Call {
    pub fn new(id: CallId, from: Participant, to: Participant) -> Self { ... }

    /// 現在の状態を取得
    pub fn state(&self) -> &CallState {
        &self.state
    }

    /// 状態遷移（不変条件を強制）
    pub fn transition(&mut self, to_state: CallState) -> Result<(), CallError> {
        match (&self.state, &to_state) {
            (CallState::Setup, CallState::Ringing) => Ok(()),
            (CallState::Setup, CallState::Active) => Ok(()),
            (CallState::Ringing, CallState::Active) => Ok(()),
            (CallState::Active, CallState::Releasing) => Ok(()),
            (CallState::Releasing, CallState::Ended(_)) => Ok(()),
            _ => Err(CallError::InvalidTransition {
                from: self.state.clone(),
                to: to_state,
            }),
        }?;
        self.state = to_state;
        Ok(())
    }

    /// 通話時間（Ended の場合のみ計算可能）
    pub fn duration(&self) -> Option<Duration> {
        self.ended_at.map(|e| e - self.started_at)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CallError {
    #[error("Invalid state transition from {from:?} to {to:?}")]
    InvalidTransition { from: CallState, to: CallState },
}
```

#### 6.3.4 識別子

```rust
// src/entities/identifiers.rs
use std::fmt;

/// SIP Call-ID に対応
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CallId(String);

impl CallId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CallId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 内部セッション識別子
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(uuid::Uuid);

impl SessionId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}
```

---

### 6.4 Phase 2: Session 分離

#### 6.4.1 現状の問題点

`session/session.rs`（1,578行）が以下の責務を全て担当：
- 状態マシン管理
- 録音ファイル I/O
- イベント Ingest（HTTP POST）
- RTP ストリーム管理
- 音声キャプチャ & VAD

**God Object アンチパターン**

#### 6.4.2 責務分離後の構造

```
session/
├── mod.rs                   # エクスポート
├── state_machine.rs         # SessionStateMachine（純粋な状態遷移）
├── session_coordinator.rs   # I/O コーディネータ
├── recording_manager.rs     # 録音ライフサイクル管理
├── ingest_manager.rs        # イベント発行管理
├── rtp_stream_manager.rs    # RTP ライフサイクル管理
└── types.rs                 # 型定義（既存）
```

#### 6.4.3 SessionStateMachine（純粋な状態遷移）

```rust
// src/session/state_machine.rs
use crate::entities::call::{Call, CallState, CallError};

/// 純粋な状態マシン（I/O なし）
pub struct SessionStateMachine {
    call: Call,
}

impl SessionStateMachine {
    pub fn new(call: Call) -> Self {
        Self { call }
    }

    /// イベントを受け取り、状態遷移を行う
    pub fn handle_event(&mut self, event: SessionEvent) -> Result<Vec<SessionCommand>, CallError> {
        let mut commands = Vec::new();

        match event {
            SessionEvent::SipInvite { offer } => {
                self.call.transition(CallState::Ringing)?;
                commands.push(SessionCommand::SendRinging);
                commands.push(SessionCommand::StartRecording);
            }
            SessionEvent::SipAck => {
                self.call.transition(CallState::Active)?;
                commands.push(SessionCommand::StartRtp);
                commands.push(SessionCommand::StartApp);
            }
            SessionEvent::SipBye => {
                self.call.transition(CallState::Releasing)?;
                commands.push(SessionCommand::StopRtp);
                commands.push(SessionCommand::StopRecording);
                commands.push(SessionCommand::SendOk);
            }
            // ...
        }

        Ok(commands)
    }

    pub fn state(&self) -> &CallState {
        self.call.state()  // Call::state() getter を使用
    }

    pub fn call(&self) -> &Call {
        &self.call
    }
}

pub enum SessionEvent {
    SipInvite { offer: Sdp },
    SipAck,
    SipBye,
    SipCancel,
    RtpFrame { data: Vec<u8> },
    AppOutput { response: String },
    SessionTimeout,
}

pub enum SessionCommand {
    SendRinging,
    SendOk,
    StartRtp,
    StopRtp,
    StartRecording,
    StopRecording,
    StartApp,
    StopApp,
    IngestEvent { payload: serde_json::Value },
}
```

#### 6.4.4 SessionCoordinator（I/O コーディネータ）

```rust
// src/session/session_coordinator.rs
use crate::session::state_machine::{SessionStateMachine, SessionEvent, SessionCommand};
use crate::session::recording_manager::RecordingManager;
use crate::session::ingest_manager::IngestManager;

/// I/O コーディネータ（状態マシンとアダプタを接続）
pub struct SessionCoordinator {
    state_machine: SessionStateMachine,
    recording: RecordingManager,
    ingest: IngestManager,
    rtp_tx: RtpTxHandle,
}

impl SessionCoordinator {
    pub async fn handle(&mut self, event: SessionEvent) -> Result<()> {
        let commands = self.state_machine.handle_event(event)?;

        for cmd in commands {
            self.execute_command(cmd).await?;
        }

        Ok(())
    }

    async fn execute_command(&mut self, cmd: SessionCommand) -> Result<()> {
        match cmd {
            SessionCommand::StartRecording => {
                self.recording.start().await?;
            }
            SessionCommand::StopRecording => {
                let path = self.recording.stop().await?;
                // 録音完了後の処理
            }
            SessionCommand::IngestEvent { payload } => {
                self.ingest.post(payload).await?;
            }
            SessionCommand::StartRtp => {
                self.rtp_tx.start()?;
            }
            SessionCommand::StopRtp => {
                self.rtp_tx.stop()?;
            }
            // ...
        }
        Ok(())
    }
}
```

---

### 6.5 NotificationPort の分離（Issue #65 指摘）

#### 6.5.1 現状

```rust
// 現行: 3 イベントを 1 トレイトに混合
pub trait NotificationPort: Send + Sync {
    fn notify_ringing(...) -> NotificationFuture;
    fn notify_missed(...) -> NotificationFuture;
    fn notify_ended(...) -> NotificationFuture;
}
```

#### 6.5.2 改善後

```rust
// src/ports/notification/ringing.rs
pub trait RingingNotifier: Send + Sync {
    fn notify_ringing(&self, from: String, timestamp: DateTime<FixedOffset>) -> NotificationFuture;
}

// src/ports/notification/missed.rs
pub trait MissedCallNotifier: Send + Sync {
    fn notify_missed(&self, from: String, timestamp: DateTime<FixedOffset>) -> NotificationFuture;
}

// src/ports/notification/ended.rs
pub trait CallEndedNotifier: Send + Sync {
    fn notify_ended(&self, from: String, duration_sec: u64) -> NotificationFuture;
}

// 後方互換
pub trait NotificationService: RingingNotifier + MissedCallNotifier + CallEndedNotifier {}
```

---

## 7. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #52 | STEER-085 | 起票 |
| Issue #65 | STEER-085 | 起票 |
| STEER-085 | DD-010 | 詳細設計 |
| DD-010 | UT-010 | 単体テスト |

---

## 8. 決定事項（Decisions）

### 8.1 Q1: Actix Actor vs tokio のみ → **tokio のみ**

| 項目 | 内容 |
|------|------|
| 決定 | **まず tokio のみで実装** |
| 理由 | 学習コスト最小化、既存コードとの整合性 |
| 条件 | 必要が証明されたら actor 化を検討（ただし移行計画込み） |
| 決定日 | 2026-01-31 |

### 8.2 Q2: Event Sourcing → **要件が刺さるまで入れない**

| 項目 | 内容 |
|------|------|
| 決定 | **Phase 3（Event Sourcing）は保留** |
| 理由 | 現時点で監査要件・リプレイ要件が明確でない |
| 代替 | イベント定義と監査ログ程度で布石を打つ |
| 再検討条件 | 監査要件が具体化した場合 |
| 決定日 | 2026-01-31 |

### 8.3 Q3: 大規模ファイル分割方針 → **早期に分割**

| 項目 | 内容 |
|------|------|
| 決定 | **早期に分割を実施** |
| 分割順序 | 1. types/error 独立 → 2. codec → 3. transport → 4. transaction/services |
| 実施方法 | 安全な移動PRで段階的に実施 |
| 決定日 | 2026-01-31 |

**分割詳細（sip/mod.rs: 2,041行）**:

```
src/sip/
├── mod.rs              # エクスポート + メインロジック（500行以下目標）
├── types.rs            # SipEvent, SipRequest, SipResponse 等の型定義
├── error.rs            # SipError 定義
├── codec.rs            # SIP メッセージパース/ビルド
├── transport.rs        # トランスポート層抽象
├── transaction.rs      # RFC 3261 §17 トランザクション FSM
└── services/
    ├── mod.rs
    ├── invite_handler.rs
    ├── bye_handler.rs
    └── register_handler.rs
```

### 8.4 Q4: 後方互換性維持期間 → **N-1（必要なら N-2）**

| 項目 | 内容 |
|------|------|
| 決定 | **リリース N-1 まで互換維持**（必要に応じて N-2） |
| 管理方法 | リリース数で廃止を管理 |
| 実装ルール | 互換コードは隔離＋テスト必須 |
| 決定日 | 2026-01-31 |

**deprecated 運用ルール**:

```rust
// 例: 旧 AiPort の deprecated 化
#[deprecated(since = "0.3.0", note = "Use individual ports (AsrPort, LlmPort, etc.) instead")]
pub trait AiPort: AsrPort + IntentPort + LlmPort + WeatherPort + TtsPort + SerPort {}

// 互換コードは compat/ に隔離
src/compat/
├── mod.rs
└── ai_port.rs  # 旧 AiPort → 新ポート変換アダプタ
```

---

## 9. 未確定事項（Open Questions）

> **すべての質問が決定済み**（§8 参照）

---

## 10. 受入条件（Acceptance Criteria）

### 10.1 Phase 1 完了条件

- [ ] AC-1: `AiPort` が `AsrPort`, `IntentPort`, `LlmPort`, `WeatherPort`, `TtsPort` に分割されている
- [ ] AC-2: 各ポートに対応するドメイン固有エラー型（`AsrError`, `LlmError` 等）が定義されている
- [ ] AC-3: `DefaultAiPort` が各個別ポートを実装している
- [ ] AC-4: 既存のテストがすべてパスする

### 10.2 Phase 2 完了条件

- [ ] AC-5: `src/entities/` ディレクトリが新設され、`Call`, `Recording`, 識別子が定義されている
- [ ] AC-6: `SessionStateMachine` が純粋な状態遷移のみを担当している（I/O なし）
- [ ] AC-7: `SessionCoordinator` が I/O を担当している
- [ ] AC-8: `session/session.rs` が 500 行以下に削減されている
- [ ] AC-9: `NotificationPort` が 3 つの個別トレイトに分割されている

---

## 11. リスク / ロールバック観点

### 11.1 リスク

| リスク | 影響 | 対策 |
|--------|------|------|
| 大規模リファクタリングによるリグレッション | 高 | Phase 1 完了ごとにテスト実行、CI/CD 強化 |
| sip/mod.rs 分割時のバグ混入 | 中 | 安全な移動PR（types → codec → transport → transaction の順） |
| 後方互換性の破壊 | 中 | deprecated 警告 + compat/ 隔離 + N-1 維持（§8.4） |

### 11.2 ロールバック戦略

- 各 Phase を独立した PR で実施
- sip/mod.rs 分割は更に小さな PR に分割（1ファイル移動 = 1 PR）
- Phase 1 完了時点で main にマージ
- 問題発生時は該当 PR のみ revert

---

## 12. 備考

**重要**: 本ステアリングは仕様のみを定義する。

**実装は Codex 担当へ引き継いでください。**

Codex への引き継ぎ事項：

**Phase 1（ポートトレイト分離）**:
1. `AiPort` を 5 個別トレイトに分割（BD-003 §5 参照）
2. ドメイン固有エラー型を定義（BD-003 §5.3 参照）
3. `compat/ai_port.rs` で後方互換アダプタを提供
4. 既存テストがパスすることを確認

**Phase 2（Domain 分離 + ファイル分割）**:
1. `src/entities/` を新設（BD-003 §6 参照）
2. sip/mod.rs を分割（§8.3 の順序に従う）
   - PR1: types.rs, error.rs 分離
   - PR2: codec.rs 分離
   - PR3: transport.rs 分離
   - PR4: transaction.rs, services/ 分離
3. Session を状態マシン + コーディネータに分離（§6.4）

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-01-31 | 初版作成 | Claude Code |
| 2026-01-31 | Q1〜Q4 決定事項反映 | Claude Code |
| 2026-01-31 | §5 アーキテクチャ原則追加（レイヤー構造、DDD、EDA、デザインパターン） | Claude Code |
| 2026-01-31 | §5 を BD-003 へ昇格、CONVENTIONS.md 新設、本セクションは参照に簡略化 | Claude Code |
| 2026-02-01 | Codex レビュー指摘対応（Refs #85）: Actor Model 矛盾解消、adapters/ 二重所属修正、call_id 移行方針追記、WeatherError/SerError 追加、Call::state() getter 追記 | Claude Code |
