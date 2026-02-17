<!-- SOURCE_OF_TRUTH: クリーンアーキテクチャ設計原則 -->
# クリーンアーキテクチャ設計原則（BD-003）

> Virtual Voicebot Backend のアーキテクチャ原則を定義する

| 項目 | 値 |
|------|-----|
| ID | BD-003 |
| ステータス | Approved |
| 作成日 | 2026-01-31 |
| 改訂日 | 2026-02-06 |
| バージョン | 3.0 |
| 関連Issue | #52, #65, #95, #108 |
| 関連RD | RD-001 |
| 付録 | [依存関係図](BD-003_dependency-diagram.md) |

---

## 1. 概要

本ドキュメントは、Virtual Voicebot Backend における**アーキテクチャ原則**を定義する。
すべての設計・実装はこの原則に準拠すること。

---

## 2. クリーンアーキテクチャの適用

本プロジェクトでは **Clean Architecture**（Robert C. Martin）を適用する。

### 2.1 レイヤー構造

```
┌─────────────────────────────────────────────────────────────────┐
│                         Interface                                │
│  (HTTP Server, Health Check, Monitoring, External Sync)         │
│  - src/interface/ (http, health, monitoring, sync)             │
└─────────────────────────────────────────────────────────────────┘
                              ↓ depends on
┌─────────────────────────────────────────────────────────────────┐
│                          Service                                 │
│  (Application Business Rules / Use Cases)                       │
│  - src/service/ (ai, call_control, recording)                  │
└─────────────────────────────────────────────────────────────────┘
                              ↓ depends on
┌─────────────────────────────────────────────────────────────────┐
│                         Protocol                                 │
│  (SIP, RTP, Session, Transport)                                 │
│  - src/protocol/ (sip, rtp, session, transport)                │
└─────────────────────────────────────────────────────────────────┘
                              ↓ depends on
┌─────────────────────────────────────────────────────────────────┐
│                          Shared                                  │
│  (Entities, Ports, Config, Error, Codec, Media)                 │
│  - src/shared/ (entities, ports, config, error, codec, media)  │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 依存性の方向（Dependency Rule）

**原則**: 依存性は常に内側（中心）に向かう。外側のレイヤーは内側を知るが、内側は外側を知らない。

| 許可される依存 | 禁止される依存 |
|---------------|---------------|
| Adapter → Port | Entity → Adapter |
| Use Case → Entity | Entity → Use Case |
| Infrastructure → Adapter | Use Case → Infrastructure |

```rust
// ✅ 正しい依存方向
use crate::shared::entities::Call;        // Service → Entity (Shared)
use crate::shared::ports::ai::AsrPort;    // Service → Port (Shared)
use crate::service::call_control::CallControlService;  // Interface → Service

// ❌ 禁止: 内側から外側への依存
// use crate::service::ai::AiService;  // Protocol 内でこれは禁止
// use crate::protocol::sip::SipEngine;  // Shared 内でこれは禁止
```

### 2.3 依存性逆転の実現（Dependency Inversion）

外側のレイヤーへの依存は **Port（トレイト）** を介して逆転させる。

```rust
// src/shared/ports/ai.rs - インターフェース定義（Shared層）
pub trait AsrPort: Send + Sync {
    fn transcribe(&self, audio: AudioChunk) -> Result<String, AsrError>;
}

// src/service/ai/openai_asr.rs - 実装（Service層）
pub struct OpenAiAsr { /* ... */ }
impl AsrPort for OpenAiAsr {
    fn transcribe(&self, audio: AudioChunk) -> Result<String, AsrError> { /* ... */ }
}

// src/service/call_control/dialog.rs - 使用（Service層）
pub struct DialogService<A: AsrPort> {
    asr: A,  // 具体型ではなくトレイトに依存
}
```

---

## 3. 駆動形（Driving Principles）

### 3.1 ドメイン駆動設計（DDD: Domain-Driven Design）

| 概念 | 適用 | 配置先 |
|------|------|--------|
| **Entity** | Call, Recording | `src/shared/entities/` |
| **Value Object** | CallId, SessionId, Participant | `src/shared/entities/` |
| **Aggregate** | Call（ルート）+ Recording | `src/shared/entities/` |
| **Domain Service** | CallStateTransition | `src/service/call_control/` |
| **Repository** | CallRepository, RecordingRepository | `src/shared/ports/` (trait) + `src/service/*/` (impl) |
| **Domain Event** | CallStarted, CallEnded, RecordingCompleted | `src/shared/events/` |

**Aggregate Root の原則**:
- Aggregate Root を通じてのみ内部 Entity を操作する
- トランザクション境界は Aggregate 単位
- 不変条件は Aggregate Root で強制する

```rust
// Call は Aggregate Root
// Recording は Call に属する Entity
pub struct Call {
    id: CallId,
    recordings: Vec<Recording>,  // 内部の Entity
}

impl Call {
    // Aggregate Root を通じてのみ Recording を操作
    pub fn add_recording(&mut self, recording: Recording) -> Result<(), CallError> {
        if self.state != CallState::Active {
            return Err(CallError::InvalidState);
        }
        self.recordings.push(recording);
        Ok(())
    }
}
```

### 3.2 イベント駆動アーキテクチャ（EDA: Event-Driven Architecture）

SIP/RTP のリアルタイム処理に適したイベント駆動を採用。

```
┌─────────┐    Event    ┌─────────────┐    Command    ┌─────────────┐
│   SIP   │ ─────────→ │   Session   │ ───────────→ │     App     │
│ Adapter │  SipEvent  │ Coordinator │  AppCommand  │   Service   │
└─────────┘            └─────────────┘               └─────────────┘
     ↑                        │                            │
     │                        │ SessionCommand             │ DomainEvent
     │                        ↓                            ↓
┌─────────┐            ┌─────────────┐               ┌─────────────┐
│Transport│ ←───────── │    RTP      │ ←──────────── │  Recording  │
│         │  SendSip   │   Manager   │   AudioFrame │   Service   │
└─────────┘            └─────────────┘               └─────────────┘
```

**イベント型の分類**:

| 種別 | 例 | 用途 |
|------|-----|------|
| **Protocol Event** | `SipEvent::Invite`, `RtpFrame` | プロトコル層からの入力 |
| **Session Event** | `SessionEvent::SipInvite`, `SessionEvent::Timeout` | セッション層への入力 |
| **Session Command** | `SessionCommand::StartRtp`, `SessionCommand::StopRecording` | セッション層からの出力 |
| **Domain Event** | `CallStarted`, `CallEnded`, `RecordingCompleted` | ドメイン層のイベント（監査ログ用） |

---

## 4. 必須デザインパターン

### 4.1 パターン一覧

| パターン | 用途 | 適用箇所 |
|----------|------|---------|
| **Repository** | データアクセスの抽象化 | `CallRepository`, `RecordingRepository` |
| **Factory** | 複雑なオブジェクト生成 | `SessionFactory`, `CallFactory` |
| **Strategy** | アルゴリズムの切り替え | ASR/LLM/TTS プロバイダ選択 |
| **State** | 状態遷移の管理 | `SessionStateMachine`, `CallState` |
| **Observer** | イベント通知 | `DomainEventPublisher` |
| **Adapter** | 外部サービスとの接続 | `OpenAiAdapter`, `GoogleAsrAdapter` |

### 4.2 State パターン

状態遷移は State パターンで実装する。

```rust
pub trait CallStateHandler {
    fn on_invite(&self, call: &mut Call) -> Result<CallState, CallError>;
    fn on_ack(&self, call: &mut Call) -> Result<CallState, CallError>;
    fn on_bye(&self, call: &mut Call) -> Result<CallState, CallError>;
    fn on_cancel(&self, call: &mut Call) -> Result<CallState, CallError>;
}

pub struct SetupState;
impl CallStateHandler for SetupState {
    fn on_invite(&self, call: &mut Call) -> Result<CallState, CallError> {
        Ok(CallState::Ringing(RingingState))
    }
    fn on_ack(&self, _: &mut Call) -> Result<CallState, CallError> {
        Err(CallError::InvalidTransition)
    }
}
```

### 4.3 Repository パターン

データアクセスは Repository パターンで抽象化する。

```rust
// src/shared/ports/repository.rs - インターフェース
pub trait CallRepository: Send + Sync {
    fn save(&self, call: &Call) -> Result<(), RepositoryError>;
    fn find_by_id(&self, id: &CallId) -> Result<Option<Call>, RepositoryError>;
    fn find_active(&self) -> Result<Vec<Call>, RepositoryError>;
}

// src/service/call_control/postgres_call_repository.rs - 実装
pub struct PostgresCallRepository { pool: PgPool }
impl CallRepository for PostgresCallRepository { /* ... */ }

// src/shared/test_support/in_memory_call_repository.rs - テスト用
pub struct InMemoryCallRepository { calls: RwLock<HashMap<CallId, Call>> }
impl CallRepository for InMemoryCallRepository { /* ... */ }
```

### 4.4 Factory パターン

複雑なオブジェクト生成は Factory パターンで隠蔽する。

```rust
pub struct SessionFactory {
    config: SessionConfig,
    asr: Arc<dyn AsrPort>,
    llm: Arc<dyn LlmPort>,
    tts: Arc<dyn TtsPort>,
}

impl SessionFactory {
    pub fn create_session(&self, call: Call) -> SessionCoordinator {
        let state_machine = SessionStateMachine::new(call);
        let recording_manager = RecordingManager::new(&self.config);
        let ingest_manager = IngestManager::new(&self.config);

        SessionCoordinator::new(
            state_machine,
            recording_manager,
            ingest_manager,
            Arc::clone(&self.asr),
            Arc::clone(&self.llm),
            Arc::clone(&self.tts),
        )
    }
}
```

---

## 5. インターフェース分離原則（ISP）

### 5.1 原則

クライアントは、使用しないメソッドに依存してはならない。

### 5.2 禁止例

```rust
// ❌ 禁止: 5 つの無関係な責務を 1 つのトレイトに混合
pub trait AiPort: Send + Sync {
    fn transcribe_chunks(...) -> ...;  // ASR
    fn classify_intent(...) -> ...;     // Intent
    fn generate_answer(...) -> ...;     // LLM
    fn handle_weather(...) -> ...;      // Weather
    fn synth_to_wav(...) -> ...;        // TTS
}
```

### 5.3 正しい設計

```rust
// ✅ 正しい: 責務ごとに分離されたトレイト
pub trait AsrPort: Send + Sync {
    fn transcribe_chunks(...) -> Result<String, AsrError>;
}

pub trait IntentPort: Send + Sync {
    fn classify_intent(...) -> Result<Intent, IntentError>;
}

pub trait LlmPort: Send + Sync {
    fn generate_answer(...) -> Result<String, LlmError>;
}

pub trait TtsPort: Send + Sync {
    fn synth_to_wav(...) -> Result<PathBuf, TtsError>;
}
```

---

## 6. ディレクトリ構造

### 6.1 現行構造（v3.0: 3層アーキテクチャ）

```text
src/
├── main.rs                      # エントリポイント（Composition Root）
│
├── interface/                   # Interface層（外部インターフェース）
│   ├── http/                    # HTTP API サーバー
│   │   ├── mod.rs
│   │   └── recordings.rs        # 録音配信エンドポイント
│   ├── health/                  # ヘルスチェック
│   │   └── mod.rs
│   ├── monitoring/              # メトリクス・トレース
│   │   └── mod.rs
│   └── sync/                    # 外部システム同期
│       └── mod.rs
│
├── service/                     # Service層（ビジネスロジック）
│   ├── ai/                      # AI サービス
│   │   ├── mod.rs
│   │   ├── asr.rs               # AsrPort 実装
│   │   ├── llm.rs               # LlmPort 実装
│   │   ├── tts.rs               # TtsPort 実装
│   │   └── intent.rs            # IntentPort 実装
│   ├── call_control/            # 通話制御サービス（旧 app/）
│   │   ├── mod.rs
│   │   ├── dialog.rs            # 対話制御
│   │   └── router.rs            # イベントルーティング
│   └── recording/               # 録音サービス
│       ├── mod.rs
│       └── storage.rs           # StoragePort 実装
│
├── protocol/                    # Protocol層（プロトコル処理）
│   ├── sip/                     # SIP プロトコルスタック
│   │   ├── mod.rs
│   │   ├── core.rs
│   │   ├── builder.rs
│   │   └── transaction.rs
│   ├── rtp/                     # RTP プロトコルスタック
│   │   ├── mod.rs
│   │   ├── tx.rs
│   │   ├── rx.rs
│   │   ├── codec.rs
│   │   └── dtmf.rs
│   ├── session/                 # セッション管理
│   │   ├── mod.rs
│   │   ├── coordinator.rs       # I/O コーディネータ
│   │   ├── state_machine.rs     # 状態マシン
│   │   ├── handlers/            # イベントハンドラ
│   │   │   ├── sip_handler.rs
│   │   │   ├── rtp_handler.rs
│   │   │   └── timer_handler.rs
│   │   ├── services/            # セッションサービス
│   │   │   ├── ivr_service.rs
│   │   │   ├── b2bua_service.rs
│   │   │   └── playback_service.rs
│   │   ├── types.rs
│   │   └── registry.rs          # SessionRegistry
│   └── transport/               # ネットワーク I/O
│       ├── mod.rs
│       ├── packet.rs
│       └── tls.rs
│
└── shared/                      # Shared層（横断的関心事）
    ├── entities/                # ドメインエンティティ
    │   ├── mod.rs
    │   ├── call.rs              # Call（Aggregate Root）
    │   ├── recording.rs         # Recording
    │   ├── participant.rs       # Participant
    │   └── identifiers.rs       # CallId, SessionId
    ├── ports/                   # ポート定義（トレイト）
    │   ├── mod.rs
    │   ├── ai.rs                # AsrPort, LlmPort, TtsPort, IntentPort
    │   ├── ingest.rs            # IngestPort
    │   ├── storage.rs           # StoragePort
    │   └── notification.rs      # NotificationPort
    ├── config/                  # 設定
    │   └── mod.rs
    ├── error/                   # エラー型定義
    │   ├── mod.rs
    │   ├── ai.rs
    │   ├── domain.rs
    │   └── protocol.rs
    ├── codec/                   # コーデック（PCMU等）
    │   └── mod.rs
    ├── media/                   # メディア処理（録音生成等）
    │   └── mod.rs
    └── test_support/            # テスト支援
        └── mod.rs
```

### 6.2 モジュール層対応表

| レイヤー | モジュール | 許可される依存先 | 禁止される依存先 |
|----------|-----------|-----------------|-----------------|
| **Shared** | shared/* (entities, ports, config, error, codec, media, test_support) | 同一 Shared 内のみ | Protocol, Service, Interface |
| **Protocol** | protocol/* (sip, rtp, session, transport) | Shared | Service, Interface |
| **Service** | service/* (ai, call_control, recording) | Protocol, Shared | Interface |
| **Interface** | interface/* (http, health, monitoring, sync) | Service, Protocol, Shared | なし（最上位） |
| **Entry** | main.rs | すべて | - |

### 6.3 禁止依存の具体例

```rust
// ❌ 禁止: Protocol → Service
// src/protocol/session/coordinator.rs で以下は禁止
use crate::service::call_control::CallControlService;  // NG

// ✅ 正しい: Protocol → Shared (ports)
use crate::shared::ports::ingest::IngestPort;  // OK

// ❌ 禁止: Protocol → Interface
// src/protocol/sip/core.rs で以下は禁止
use crate::interface::http::HttpServer;  // NG

// ✅ 正しい: Protocol → Shared
use crate::shared::entities::CallId;  // OK

// ❌ 禁止: Shared → Protocol
// src/shared/entities/call.rs で以下は禁止
use crate::protocol::sip::SipEngine;  // NG

// ✅ 正しい: Shared は Shared 内のみ参照
use crate::shared::entities::Recording;  // OK

// ❌ 禁止: Shared → Service
// src/shared/ports/ai.rs で以下は禁止
use crate::service::ai::OpenAiAsr;  // NG（Port 定義が実装に依存してはならない）

// ✅ 正しい: Service → Shared (ports)
// src/service/ai/openai_asr.rs
use crate::shared::ports::ai::AsrPort;  // OK（Service が Port を実装）

// ❌ 禁止: Service → Interface
// src/service/call_control/dialog.rs で以下は禁止
use crate::interface::http::HttpServer;  // NG

// ✅ 正しい: Interface → Service
// src/interface/http/mod.rs
use crate::service::call_control::CallControlService;  // OK
```

---

## 7. コンプライアンスチェックリスト

新規コード追加時は以下を確認すること：

### 7.1 依存方向

- [ ] Entity は Adapter に依存していないか
- [ ] Use Case は Infrastructure に直接依存していないか
- [ ] Port（トレイト）を介して外部依存を逆転しているか

### 7.2 インターフェース分離

- [ ] トレイトは単一責任か（1 トレイト = 1 責務）
- [ ] クライアントが使用しないメソッドを含んでいないか

### 7.3 ドメインモデル

- [ ] Aggregate Root を通じてのみ内部 Entity を操作しているか
- [ ] 不変条件は Entity/Aggregate 内で強制されているか

### 7.4 テスタビリティ

- [ ] 外部依存はモック可能か（トレイトで抽象化されているか）
- [ ] 純粋な状態遷移ロジックは分離されているか

---

## 8. 参照

| ドキュメント | 内容 |
|-------------|------|
| [BD-001](BD-001_architecture.md) | システムアーキテクチャ（モジュール構成） |
| [BD-002](BD-002_app-layer.md) | App層設計 |
| [STEER-085](../steering/STEER-085_clean-architecture.md) | クリーンアーキテクチャ移行ステアリング |
| [STEER-108](../steering/STEER-108_sip-core-engine-refactor.md) | 3層アーキテクチャへのリファクタリング |
| [CONVENTIONS.md](../../../CONVENTIONS.md) | 開発規約（本原則のサマリ） |

---

## 変更履歴

| 日付 | バージョン | 変更内容 | 作成者 |
|------|-----------|---------|--------|
| 2026-01-31 | 1.0 | 初版作成（STEER-085 §5 より昇格） | @MasanoriSuda + Claude Code |
| 2026-02-03 | 2.0 | §6 ディレクトリ構造を現行実装に合わせて改訂、モジュール層対応表追加、禁止依存の具体例追加、依存関係図付録追加（#95 対応） | Claude Code |
| 2026-02-05 | 2.1 | §6.2 L3→session（コールバック）許可を撤廃、L3→ports のみに変更。§6.3 に rtp/sip→session 禁止例を追加（#95 Phase 5 対応） | Claude Code |
| 2026-02-06 | 3.0 | 3層アーキテクチャに全面改訂（STEER-108）：Interface/Service/Protocol/Shared 構造に再編、全セクションのimport例とディレクトリ構造を更新 | Claude Code |

