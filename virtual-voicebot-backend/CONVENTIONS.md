# CONVENTIONS.md

> Virtual Voicebot Backend の開発規約・原則

---

## 1. アーキテクチャ原則（必読）

本プロジェクトは **Clean Architecture** を採用する。
詳細は [BD-003_clean-architecture.md](docs/design/basic/BD-003_clean-architecture.md) を参照。

### 1.1 レイヤー構造

```
Frameworks & Drivers  (infrastructure/, adapters/)
        ↓
Interface Adapters    (adapters/)
        ↓
Application Rules     (app/, session/)
        ↓
Enterprise Rules      (entities/, domain/)
```

### 1.2 依存性の方向（Dependency Rule）

**依存は常に内側へ。外側のレイヤーは内側を知るが、内側は外側を知らない。**

```rust
// ✅ OK: Adapter → Port
use crate::ports::AsrPort;

// ✅ OK: Use Case → Entity
use crate::entities::Call;

// ❌ NG: Entity → Adapter
// use crate::adapters::ai::OpenAiClient;  // 禁止
```

### 1.3 依存性逆転（Dependency Inversion）

外部サービスへの依存は **Port（トレイト）** で抽象化する。

```rust
// Port（トレイト）を定義
pub trait AsrPort: Send + Sync {
    fn transcribe(&self, audio: AudioChunk) -> Result<String, AsrError>;
}

// Use Case はトレイトに依存
pub struct DialogService<A: AsrPort> {
    asr: A,
}
```

---

## 2. インターフェース分離原則（ISP）

### 2.1 原則

**1 トレイト = 1 責務**

クライアントは使用しないメソッドに依存してはならない。

### 2.2 禁止例

```rust
// ❌ 禁止: 複数の責務を 1 つのトレイトに混合
pub trait AiPort {
    fn transcribe(...);    // ASR
    fn generate(...);      // LLM
    fn synthesize(...);    // TTS
}
```

### 2.3 正しい設計

```rust
// ✅ 正しい: 責務ごとに分離
pub trait AsrPort { fn transcribe(...); }
pub trait LlmPort { fn generate(...); }
pub trait TtsPort { fn synthesize(...); }
```

---

## 3. ドメイン駆動設計（DDD）

### 3.1 構成要素

| 概念 | 配置先 | 例 |
|------|--------|-----|
| Entity | `entities/` | Call, Recording |
| Value Object | `entities/` | CallId, Participant |
| Aggregate Root | `entities/` | Call |
| Domain Service | `domain/services/` | CallStateTransition |
| Repository | `ports/` (trait) | CallRepository |
| Domain Event | `domain/events/` | CallStarted, CallEnded |

### 3.2 Aggregate Root の原則

- Aggregate Root を通じてのみ内部 Entity を操作する
- 不変条件は Aggregate Root で強制する

```rust
impl Call {
    pub fn add_recording(&mut self, rec: Recording) -> Result<(), CallError> {
        if self.state != CallState::Active {
            return Err(CallError::InvalidState);  // 不変条件
        }
        self.recordings.push(rec);
        Ok(())
    }
}
```

---

## 4. デザインパターン

### 4.1 必須パターン

| パターン | 用途 | 適用箇所 |
|----------|------|---------|
| **Repository** | データアクセス抽象化 | CallRepository |
| **Factory** | オブジェクト生成 | SessionFactory |
| **Strategy** | アルゴリズム切替 | AI プロバイダ選択 |
| **State** | 状態遷移管理 | SessionStateMachine |
| **Adapter** | 外部サービス接続 | OpenAiAdapter |

### 4.2 State パターン

状態遷移は State パターンで実装し、純粋な状態マシンと I/O を分離する。

```rust
// 純粋な状態マシン（I/O なし）
pub struct SessionStateMachine { call: Call }

// I/O コーディネータ
pub struct SessionCoordinator {
    state_machine: SessionStateMachine,
    recording: RecordingManager,
    ingest: IngestManager,
}
```

---

## 5. イベント駆動

### 5.1 イベント型の分類

| 種別 | 例 | 用途 |
|------|-----|------|
| Protocol Event | SipEvent::Invite | プロトコル層入力 |
| Session Event | SessionEvent::SipInvite | セッション層入力 |
| Session Command | SessionCommand::StartRtp | セッション層出力 |
| Domain Event | CallStarted | 監査ログ用 |

---

## 6. エラー処理

### 6.1 ドメイン固有エラー型

`anyhow::Result` のみでなく、ドメイン固有のエラー型を定義する。

```rust
// src/error/ai.rs
#[derive(Debug, thiserror::Error)]
pub enum AsrError {
    #[error("Transcription failed: {0}")]
    TranscriptionFailed(String),
    #[error("Service unavailable")]
    ServiceUnavailable,
    #[error("Timeout")]
    Timeout,
}
```

### 6.2 エラー型の配置

| エラー種別 | 配置先 |
|-----------|--------|
| ドメインエラー | `error/domain.rs` |
| AI エラー | `error/ai.rs` |
| リポジトリエラー | `error/repository.rs` |
| インフラエラー | `error/infrastructure.rs` |

---

## 7. ファイル構成ルール

### 7.1 ファイルサイズ

- **500 行以下**を目標とする
- 超える場合は責務分割を検討

### 7.2 モジュール分割

大規模モジュールは以下の順序で分割：

1. `types.rs` - 型定義
2. `error.rs` - エラー型
3. `codec.rs` / `parser.rs` - パース/ビルド
4. `services/` - ハンドラ群

---

## 8. コードレビューチェックリスト

### 8.1 依存方向

- [ ] Entity は Adapter に依存していないか
- [ ] Use Case は Infrastructure に直接依存していないか
- [ ] Port（トレイト）を介して外部依存を逆転しているか

### 8.2 インターフェース分離

- [ ] トレイトは単一責任か
- [ ] 不要なメソッドを含んでいないか

### 8.3 ドメインモデル

- [ ] Aggregate Root を通じて内部 Entity を操作しているか
- [ ] 不変条件は Entity 内で強制されているか

### 8.4 テスタビリティ

- [ ] 外部依存はモック可能か
- [ ] 純粋な状態遷移ロジックは分離されているか

---

## 9. 参照

| ドキュメント | 内容 |
|-------------|------|
| [BD-003](docs/design/basic/BD-003_clean-architecture.md) | クリーンアーキテクチャ設計原則（正式版） |
| [STEER-085](docs/steering/STEER-085_clean-architecture.md) | クリーンアーキテクチャ移行ステアリング |
| [CLAUDE.md](../CLAUDE.md) | Claude Code の役割定義 |
| [AGENTS.md](AGENTS.md) | Codex の役割定義 |

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-01-31 | 初版作成 | @MasanoriSuda + Claude Code |

