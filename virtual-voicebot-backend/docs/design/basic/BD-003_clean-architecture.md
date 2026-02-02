<!-- SOURCE_OF_TRUTH: クリーンアーキテクチャ設計原則 -->
# クリーンアーキテクチャ設計原則（BD-003）

> Virtual Voicebot Backend のアーキテクチャ原則を定義する

| 項目 | 値 |
|------|-----|
| ID | BD-003 |
| ステータス | Approved |
| 作成日 | 2026-01-31 |
| 関連Issue | #52, #65 |
| 関連RD | RD-001 |

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
│                    Frameworks & Drivers                         │
│  (HTTP Server, SIP Stack, RTP Stack, Database, External APIs)   │
│  - axum, tokio, PostgreSQL, OpenAI/Google/AWS APIs             │
└─────────────────────────────────────────────────────────────────┘
                              ↓ depends on
┌─────────────────────────────────────────────────────────────────┐
│                    Interface Adapters                           │
│  (Controllers, Gateways, Presenters)                            │
│  - src/adapters/ (ai/, db/, http/, notification/)              │
└─────────────────────────────────────────────────────────────────┘
                              ↓ depends on
┌─────────────────────────────────────────────────────────────────┐
│                    Application Business Rules                   │
│  (Use Cases / Application Services)                             │
│  - src/app/, src/session/ (Coordinator層)                       │
└─────────────────────────────────────────────────────────────────┘
                              ↓ depends on
┌─────────────────────────────────────────────────────────────────┐
│                    Enterprise Business Rules                    │
│  (Entities / Domain Models)                                     │
│  - src/entities/ (Call, Recording, Participant, etc.)          │
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
use crate::entities::Call;        // Use Case → Entity
use crate::ports::AsrPort;        // Adapter → Port

// ❌ 禁止: 内側から外側への依存
// use crate::adapters::ai::OpenAiClient;  // Entity 内でこれは禁止
```

### 2.3 依存性逆転の実現（Dependency Inversion）

外側のレイヤーへの依存は **Port（トレイト）** を介して逆転させる。

```rust
// src/ports/asr.rs - インターフェース定義（内側）
pub trait AsrPort: Send + Sync {
    fn transcribe(&self, audio: AudioChunk) -> Result<String, AsrError>;
}

// src/adapters/ai/openai_asr.rs - 実装（外側）
pub struct OpenAiAsr { /* ... */ }
impl AsrPort for OpenAiAsr {
    fn transcribe(&self, audio: AudioChunk) -> Result<String, AsrError> { /* ... */ }
}

// src/app/dialog.rs - 使用（中間）
pub struct DialogService<A: AsrPort> {
    asr: A,  // 具体型ではなくトレイトに依存
}
```

---

## 3. 駆動形（Driving Principles）

### 3.1 ドメイン駆動設計（DDD: Domain-Driven Design）

| 概念 | 適用 | 配置先 |
|------|------|--------|
| **Entity** | Call, Recording | `src/entities/` |
| **Value Object** | CallId, SessionId, Participant | `src/entities/` |
| **Aggregate** | Call（ルート）+ Recording | `src/entities/` |
| **Domain Service** | CallStateTransition | `src/domain/services/` |
| **Repository** | CallRepository, RecordingRepository | `src/ports/` (trait) + `src/adapters/db/` (impl) |
| **Domain Event** | CallStarted, CallEnded, RecordingCompleted | `src/domain/events/` |

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
// src/ports/repository.rs - インターフェース
pub trait CallRepository: Send + Sync {
    fn save(&self, call: &Call) -> Result<(), RepositoryError>;
    fn find_by_id(&self, id: &CallId) -> Result<Option<Call>, RepositoryError>;
    fn find_active(&self) -> Result<Vec<Call>, RepositoryError>;
}

// src/adapters/db/postgres_call_repository.rs - 実装
pub struct PostgresCallRepository { pool: PgPool }
impl CallRepository for PostgresCallRepository { /* ... */ }

// src/adapters/db/in_memory_call_repository.rs - テスト用
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

```
src/
├── main.rs                      # エントリポイント（DI コンテナ構築）
│
├── entities/                    # Enterprise Business Rules (Entity層)
│   ├── mod.rs
│   ├── call.rs                  # Call エンティティ（Aggregate Root）
│   ├── recording.rs             # Recording エンティティ
│   ├── participant.rs           # Participant 値オブジェクト
│   └── identifiers.rs           # CallId, SessionId 等
│
├── domain/                      # Domain Services & Events
│   ├── services/
│   ├── events/
│   └── factories/
│
├── ports/                       # Port 定義（インターフェース）
│   ├── asr.rs, llm.rs, tts.rs   # AI ポート
│   ├── repository.rs            # リポジトリポート
│   └── notification.rs          # 通知ポート
│
├── app/                         # Application Business Rules (Use Case層)
│   ├── dialog_service.rs
│   ├── recording_service.rs
│   └── notification_service.rs
│
├── session/                     # Session 管理（Application層の一部）
│   ├── state_machine.rs         # 純粋な状態遷移
│   ├── session_coordinator.rs   # I/O コーディネータ
│   └── types.rs
│
├── adapters/                    # Interface Adapters
│   ├── ai/                      # AI サービス実装
│   ├── db/                      # DB 実装
│   ├── http/                    # HTTP 実装
│   └── notification/            # 通知実装
│
├── infrastructure/              # Frameworks & Drivers
│   ├── sip/                     # SIP プロトコルスタック
│   ├── rtp/                     # RTP プロトコルスタック
│   ├── transport/               # ネットワーク I/O
│   └── config/                  # 設定管理
│
├── error/                       # エラー型定義
│   ├── domain.rs
│   ├── ai.rs
│   └── repository.rs
│
└── compat/                      # 後方互換（N-1 維持）
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
| [CONVENTIONS.md](../../../CONVENTIONS.md) | 開発規約（本原則のサマリ） |

---

## 変更履歴

| 日付 | バージョン | 変更内容 | 作成者 |
|------|-----------|---------|--------|
| 2026-01-31 | 1.0 | 初版作成（STEER-085 §5 より昇格） | @MasanoriSuda + Claude Code |

