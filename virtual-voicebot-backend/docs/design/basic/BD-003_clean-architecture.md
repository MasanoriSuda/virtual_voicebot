<!-- SOURCE_OF_TRUTH: クリーンアーキテクチャ設計原則 -->
# クリーンアーキテクチャ設計原則（BD-003）

> Virtual Voicebot Backend のアーキテクチャ原則を定義する

| 項目 | 値 |
|------|-----|
| ID | BD-003 |
| ステータス | Approved |
| 作成日 | 2026-01-31 |
| 改訂日 | 2026-02-03 |
| バージョン | 2.0 |
| 関連Issue | #52, #65, #95 |
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

### 6.1 現行構造（v2.0）

```
src/
├── main.rs                      # エントリポイント（Composition Root）
│
├── entities/                    # [L0] Enterprise Business Rules (Entity層)
│   ├── call.rs                  # Call エンティティ（Aggregate Root）
│   ├── recording.rs             # Recording エンティティ
│   ├── participant.rs           # Participant 値オブジェクト
│   └── identifiers.rs           # CallId, SessionId 等
│
├── ports/                       # [L1] Port 定義（インターフェース）
│   ├── ai.rs, ai/               # AI ポート（AsrPort, LlmPort, TtsPort, IntentPort）
│   ├── app.rs                   # App イベントポート（AppEvent, EndReason）
│   ├── ingest.rs                # Ingest ポート
│   ├── storage.rs               # Storage ポート
│   ├── phone_lookup.rs          # 電話番号検索ポート
│   └── notification.rs          # 通知ポート
│
├── app/                         # [L5] Application Business Rules (Use Case層)
│   ├── mod.rs                   # AppService（オーケストレーション）
│   └── router.rs                # イベントルーティング
│
├── session/                     # [L4] Session Orchestration
│   ├── coordinator.rs           # I/O コーディネータ（≦500行）
│   ├── state_machine.rs         # 純粋な状態遷移（SessionEvent→SessionCommand）
│   ├── handlers/                # イベントハンドラ
│   │   ├── sip_handler.rs
│   │   ├── rtp_handler.rs
│   │   └── timer_handler.rs
│   ├── services/                # ドメインサービス
│   │   ├── ivr_service.rs
│   │   ├── b2bua_service.rs
│   │   └── playback_service.rs
│   ├── types.rs                 # SessionHandle, Sdp 等
│   └── registry.rs              # SessionRegistry（Actor パターン）
│
├── ai/                          # [L2] AI Adapter 実装
│   ├── asr.rs, llm.rs, tts.rs
│   └── intent.rs, weather.rs
│
├── db/                          # [L2] DB Adapter 実装
│   └── tsurugi.rs               # TsurugiAdapter
│
├── http/                        # [L2] HTTP Adapter 実装
│   ├── mod.rs                   # axum サーバー
│   └── ingest.rs                # HttpIngestAdapter
│
├── notification/                # [L2] 通知 Adapter 実装
│   └── mod.rs                   # LINE 通知
│
├── recording/                   # [L2] 録音 Adapter 実装
│   └── storage.rs               # LocalStorageAdapter
│
├── media/                       # [L2] メディア処理
│   └── mod.rs                   # Recorder, merge
│
├── sip/                         # [L3] SIP プロトコルスタック
│   ├── core.rs, builder.rs
│   └── transaction.rs
│
├── rtp/                         # [L3] RTP プロトコルスタック
│   ├── tx.rs, rx.rs
│   └── codec.rs, dtmf.rs
│
├── transport/                   # [L3] ネットワーク I/O
│   ├── packet.rs
│   └── tls.rs
│
├── config/                      # [L0] 設定（横断的関心事）
│   └── mod.rs
│
├── error/                       # [L0] エラー型定義
│   └── ai.rs
│
└── logging/                     # [L0] ログ（横断的関心事）
    └── mod.rs
```

### 6.2 モジュール層対応表

| レイヤー | モジュール | 許可される依存先 | 禁止される依存先 |
|----------|-----------|-----------------|-----------------|
| **L0: Foundation** | entities, config, error, logging | なし | すべて |
| **L1: Ports** | ports | entities, error | adapters, infrastructure |
| **L2: Adapters** | ai, db, http, notification, recording, media | ports, config, error | session, app, entities直接 |
| **L3: Infrastructure** | sip, rtp, transport | config, session（コールバック） | app, entities直接 |
| **L4: Session** | session | ports, entities, config, L3 | app, http, db直接 |
| **L5: Application** | app | ports, session, config | adapters直接, infrastructure直接 |
| **L6: Entry** | main | すべて | - |

### 6.3 禁止依存の具体例

```rust
// ❌ 禁止: session → app
// src/session/coordinator.rs で以下は禁止
use crate::app::AppService;  // NG

// ✅ 正しい: session → ports::app
use crate::ports::app::AppEvent;  // OK

// ❌ 禁止: session → http
use crate::http::IngestPort;  // NG

// ✅ 正しい: session → ports::ingest
use crate::ports::ingest::IngestPort;  // OK

// ❌ 禁止: app → db 直接
use crate::db::TsurugiAdapter;  // NG

// ✅ 正しい: app → ports::phone_lookup
use crate::ports::phone_lookup::PhoneLookupPort;  // OK
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
| 2026-02-03 | 2.0 | §6 ディレクトリ構造を現行実装に合わせて改訂、モジュール層対応表追加、禁止依存の具体例追加、依存関係図付録追加（#95 対応） | Claude Code |

