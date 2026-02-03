# STEER-095: Backend 磨き上げ（クリーンアーキテクチャ適合）

> 現行実装と設計書の乖離を解消し、クリーンアーキテクチャに適合させる

| 項目 | 値 |
|------|-----|
| ステータス | Draft |
| 作成日 | 2026-02-02 |
| 関連Issue | #95 |
| 対応BD | BD-003（クリーンアーキテクチャ設計原則） |
| 対応規約 | AGENTS.md |
| 実装担当 | Codex |

---

## 1. 概要

### 1.1 目的

現行の Backend 実装と設計書（BD-003, AGENTS.md）との乖離を解消し、クリーンアーキテクチャ原則に適合させる。

### 1.2 背景

- BD-003（クリーンアーキテクチャ設計原則）が Approved 済み
- AGENTS.md に並行処理モデル・依存方向・イベント駆動の規約が定義済み
- 現行実装に複数の違反・乖離が存在
- BD-004（IVR/コールルーティング）追加前に基盤を整備する必要がある

---

## 2. 乖離分析

### 2.1 Codex 指摘事項（Issue #95）

| # | 指摘 | 重要度 | 違反規約 |
|---|------|--------|----------|
| 1 | SessionRegistry の巨大Mutex | 高 | AGENTS.md §4 |
| 2 | CallId 二重定義 | 中 | BD-003 §3.1（DDD境界） |
| 3 | STEER-085 設計と実装の乖離 | 高 | BD-003 §3.2（EDA） |
| 4 | SessionCoordinator 責務過多 | 高 | BD-003 §2（SRP） |
| 5 | AiPort ドメインエラー喪失 | 中 | BD-003 §2.3（依存性逆転） |
| 6 | RTP parser 規格外パケット | 低 | RFC 3550 準拠 |

### 2.2 Claude Code 観点の追加分析

| # | 観点 | 現状 | あるべき姿 | 根拠 |
|---|------|------|-----------|------|
| 7 | Port/Adapter 境界 | 一部の Adapter が Entity を直接参照 | Port（trait）経由で依存性逆転 | BD-003 §2.3 |
| 8 | Domain Event の不在 | イベントが session 層に閉じている | domain/events/ に DomainEvent 定義 | BD-003 §3.1 |
| 9 | UseCase 層の不明確 | app 層と session 層の責務が曖昧 | UseCase を明確に分離 | BD-003 §2.1 |
| 10 | エラー型の設計 | anyhow::Error が多用 | ドメイン固有エラー型を定義 | AGENTS.md §5 |

---

## 3. 現状と理想の対比

### 3.1 SessionRegistry（指摘 #1）

**現状:**
```rust
// types.rs:261-291
pub type SessionMap = Arc<Mutex<HashMap<CallId, SessionHandle>>>;
```

**違反:**
- AGENTS.md §4「共有ロック（巨大Mutex）で全体の整合性を取らない」

**あるべき姿:**
```rust
// Actor パターンでチャネル経由でアクセス
pub struct SessionRegistry {
    tx: mpsc::Sender<RegistryCommand>,
}

enum RegistryCommand {
    Register { call_id: CallId, handle: SessionHandle, reply: oneshot::Sender<()> },
    Unregister { call_id: CallId },
    Get { call_id: CallId, reply: oneshot::Sender<Option<SessionHandle>> },
}
```

**対応方針:**
- **Actor 化を採用**: 専用タスク + チャネルで非同期アクセス

> ※ DashMap 案は不採用（より明確な非同期境界を確保するため Actor 化を選択）

---

### 3.2 CallId 二重定義（指摘 #2）

**現状:**
```rust
// entities/identifiers.rs:3-14
pub struct CallId(String);

// session/types.rs:15-16
pub type CallId = String;  // 別定義
```

**違反:**
- BD-003 §3.1「Value Object は src/entities/ に配置」

**あるべき姿:**
```rust
// entities/identifiers.rs のみに定義
pub struct CallId(pub String);

// session/types.rs では re-export
pub use crate::entities::CallId;
```

---

### 3.3 SessionStateMachine 設計乖離（指摘 #3）

**現状:**
```rust
// state_machine.rs:1-31
// next_session_state の薄いラッパのみ
// 実際の分岐は session_coordinator.rs:334-420 に集中
```

**違反:**
- BD-003 §3.2「イベント駆動アーキテクチャ」
- STEER-085 の SessionEvent/Command 設計

**あるべき姿（BD-003 §3.2 準拠）:**
```rust
// state_machine.rs
pub struct SessionStateMachine {
    state: SessionState,
}

impl SessionStateMachine {
    pub fn process_event(&mut self, event: SessionEvent) -> Vec<SessionCommand> {
        match (&self.state, event) {
            (SessionState::Idle, SessionEvent::IncomingCall { .. }) => {
                self.state = SessionState::Ringing;
                vec![SessionCommand::SendRinging, SessionCommand::StartTimer(30)]
            }
            // ...
        }
    }
}

pub enum SessionEvent {
    IncomingCall { call_id: CallId, from: Participant },
    DtmfReceived { digit: char },
    Timeout,
    // ...
}

pub enum SessionCommand {
    SendRinging,
    SendOk,
    PlayAudio(AudioSource),
    StartTimer(u64),
    // ...
}
```

---

### 3.4 SessionCoordinator 責務過多（指摘 #4）

**現状:**
- 1490 行の God Object
- 責務: SIP 制御、RTP 管理、IVR、B2BUA、Playback、状態管理

**違反:**
- BD-003 §2「単一責任の原則（SRP）」

**あるべき姿:**
```
session/
├── coordinator.rs       # オーケストレーションのみ（≦ 500行）
├── state_machine.rs     # 状態遷移ロジック
├── handlers/
│   ├── sip_handler.rs   # SIP イベント処理
│   ├── rtp_handler.rs   # RTP イベント処理
│   └── timer_handler.rs # タイマー処理
├── services/
│   ├── ivr_service.rs   # IVR ロジック（BD-004 で追加予定）
│   ├── b2bua_service.rs # B2BUA ロジック
│   └── playback_service.rs # 音声再生ロジック
└── types.rs
```

---

### 3.5 AiPort ドメインエラー喪失（指摘 #5）

> **用語**: 現行実装の `AiPort` を、責務を明確化した `AsrPort`（ASR = Automatic Speech Recognition）にリネーム予定

**現状:**
```rust
// ai_port.rs:46-93
pub async fn transcribe(&self, audio: &[u8]) -> anyhow::Result<String>
```

**違反:**
- BD-003 §2.3「依存性逆転」: Port はドメイン固有エラーを返すべき

**あるべき姿:**
```rust
// ports/asr.rs
#[derive(Debug, thiserror::Error)]
pub enum AsrError {
    #[error("Audio too short")]
    AudioTooShort,
    #[error("Transcription failed: {0}")]
    TranscriptionFailed(String),
    #[error("Timeout")]
    Timeout,
    #[error("Rate limited")]
    RateLimited,
}

pub trait AsrPort: Send + Sync {
    async fn transcribe(&self, audio: &[u8]) -> Result<String, AsrError>;
}
```

---

### 3.6 RTP parser 規格外パケット（指摘 #6）

**現状:**
```rust
// parser.rs:21-40
// CSRC / extension を無視して payload を切り出し
```

**リスク:**
- CSRC count > 0 のパケットで payload 位置を誤認
- extension = true のパケットで拡張ヘッダを payload として解釈

**あるべき姿:**
```rust
pub fn parse_rtp(data: &[u8]) -> Result<RtpPacket, RtpParseError> {
    let header = parse_header(data)?;

    // CSRC をスキップ
    let csrc_len = header.cc as usize * 4;
    let mut offset = 12 + csrc_len;

    // Extension をスキップ
    if header.extension {
        let ext_len = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize * 4;
        offset += 4 + ext_len;
    }

    let payload = &data[offset..];
    Ok(RtpPacket { header, payload })
}
```

---

## 4. 対応優先度

### Phase 1: BD-004 実装前に必須

| # | 対応 | 理由 | 工数目安 |
|---|------|------|---------|
| 4 | SessionCoordinator 分割 | IVR ロジック追加先を確保 | 大 |
| 3 | SessionStateMachine 改修 | IVR 状態管理に必要 | 中 |

### Phase 2: 品質向上

| # | 対応 | 理由 | 工数目安 |
|---|------|------|---------|
| 1 | SessionRegistry Actor化 | パフォーマンス・デッドロック防止 | 中 |
| 2 | CallId 統一 | DDD 境界明確化 | 小 |
| 5 | AsrError 導入 | エラーハンドリング改善 | 小 |

### Phase 3: 将来対応可

| # | 対応 | 理由 | 工数目安 |
|---|------|------|---------|
| 6 | RTP parser 改修 | 現時点で動作に影響なし | 小 |
| 7-10 | 追加改善 | 長期的な保守性向上 | 中 |

---

## 5. 受入条件（Acceptance Criteria）

### Phase 1

- [x] AC-1: SessionCoordinator が 500 行以下になっている
- [x] AC-2: IVR ロジックが `session/services/ivr_service.rs` に分離されている
- [x] AC-3: B2BUA ロジックが `session/services/b2bua_service.rs` に分離されている
- [x] AC-4: SessionStateMachine が SessionEvent を受け取り SessionCommand を返す設計になっている
- [ ] AC-5: 既存のテストが全て pass する
  - 備考: `recording_http_e2e` がローカルで PermissionDenied のため未確認

### Phase 2

- [ ] AC-6: SessionRegistry が Actor パターンで実装されている
- [ ] AC-7: CallId が entities/identifiers.rs のみに定義されている
- [ ] AC-8: AsrPort が AsrError を返す設計になっている
- [ ] AC-9: AiPort が AsrPort にリネームされている

---

## 6. 決定事項

- [x] Q1: SessionRegistry の Actor 化と DashMap 化、どちらを採用するか？
  - **決定: Actor 化**（より明確な非同期境界）
- [x] Q2: Phase 1 と Phase 2 を同時に進めるか、Phase 1 完了後に Phase 2 に進むか？
  - **決定: Phase 1 → BD-004 → Phase 2**（段階的に進める）

---

## 7. 実装メモ（Phase 1）

- SessionCoordinator 分割: `src/session/session_coordinator.rs` -> `src/session/coordinator.rs` + handlers/services へ移管（379行）
- SessionStateMachine: `SessionEvent -> SessionCommand` 化 + `apply_commands` 追加
- IVR/B2BUA/Playback: `src/session/services/` に分離（IVR のテストも移動）

---

## 変更履歴

| 日付 | バージョン | 変更内容 | 作成者 |
|------|-----------|---------|--------|
| 2026-02-02 | 1.0 | 初版作成（#95 対応） | Claude Code |
| 2026-02-03 | 1.1 | Q1/Q2 決定（Actor化、Phase順次進行） | Claude Code |
| 2026-02-03 | 1.2 | レビュー指摘対応（閾値統一、A/B併記解消、用語注記追加） | Claude Code |
| 2026-02-03 | 1.3 | AC-9 追加（AiPort→AsrPort リネーム）、閾値500行は意図的緩和と確認 | Claude Code |
| 2026-02-03 | 1.4 | Phase 1 実装（AC-1..AC-4 達成: coordinator分割/state_machine更新/services分離） | Codex |
