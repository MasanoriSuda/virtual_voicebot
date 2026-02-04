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

### 2.3 Phase 1 実装後レビュー（Codex 2026-02-03）

| # | 指摘 | 重要度 | 違反規約 | 根拠 |
|---|------|--------|----------|------|
| 11 | session が app に依存（AppEvent/EndReason 直接参照） | 重大 | BD-003 §2.3（依存方向） | coordinator.rs:17, mod.rs:10, sip_handler.rs:4 |
| 12 | session が http に依存（IngestPort が http 層） | 重大 | BD-003 §2.3（境界逆転） | coordinator.rs:19, ingest.rs:9 |
| 13 | HttpIngestPort::post が毎回 Client 生成 | 軽 | パフォーマンス | ingest.rs:23-28 |

> **備考**: #1（SessionRegistry 巨大Mutex）、#2（CallId 二重定義）、#6（RTP parser）は既存指摘として Phase 2/3 で対応予定

### 2.4 Phase 2 完了後レビュー（Codex 2026-02-03）

**指摘（重大）**

| # | 指摘 | 根拠 | 方向性 |
|---|------|------|--------|
| 14 | app（UseCase）内で reqwest を使った外部HTTP（LINE API）を実行 | notification.rs:1,148 | LINE通知アダプタをinfra層へ移し、appは ports::notification のtraitにのみ依存 |
| 15 | session層が config/recording/serde_json に依存しJSON組み立て | coordinator.rs:17,263 | ingest用DTOをportsに置き、JSON化は http adapter 側で行う |
| 16 | PII（ユーザー発話・意図JSON・電話番号）を info/warn でそのまま出力 | mod.rs:417,449,690 | デフォルトは伏字/長さのみ、詳細は明示的なデバッグフラグでのみ出力 |
| 17 | UseCaseが db モジュールに依存し、Port定義もdb内 | mod.rs:18, port.rs:1 | PhoneLookupPortは ports に移し、dbはadapter実装のみを持つ |

**指摘（中）**

| # | 指摘 | 根拠 | 方向性 |
|---|------|------|--------|
| 18 | Port境界が anyhow::Result や serde_json::Value を露出 | ingest.rs:1, port.rs:1, storage.rs:1 | PortはドメインDTOと専用エラー型にする |
| 19 | OnceLock によるグローバル状態が複数 | config.rs:163,320 | Configをコンストラクタ注入し、環境読込はcomposition rootに限定 |
| 20 | Arc<Mutex<HashMap>> の共有可変状態を理由説明なしで利用 | packet.rs:27, main.rs:67 | Actor化/所有権の明確化、または設計意図をドキュメント化 |
| 21 | AppEvent の call_id が String 固定で型安全性欠如 | app.rs:4 | Port/イベント境界で CallId を使い、外側で文字列化 |

**指摘（軽）**

| # | 指摘 | 根拠 | 方向性 |
|---|------|------|--------|
| 22 | CallId::new が空文字等を許容し不変条件が型で保証されない | identifiers.rs:7 | Result<CallId, ...> でバリデーション |
| 23 | AiServices が便利集約traitとして置かれ境界責務が曖昧 | ai.rs:27 | 用途別に明示的な依存を渡すか、用途特化のFacade型に |

**決定済み（PoC暫定処置として修正）**

- session → recording 直参照 → **撤廃**（StoragePort を ports/ に移動）
- db::port を db に置く → **ports/ へ移動**（PhoneLookupPort を ports/phone_lookup に）

### 2.5 Phase 3 準備レビュー（Codex 2026-02-03）

**指摘（重大）**

| # | 指摘 | 根拠 | 方向性 |
|---|------|------|--------|
| 24 | session が依然として巨大かつ責務集中、再肥大化リスク | coordinator.rs, mod.rs | state_machine 純化、handler 責務分離をもう一段進める |
| 19 | config のグローバル状態が多い（テスト困難・再現性低下） | config.rs | composition root で読み込み、Config を引数で渡す範囲を増やす |

**指摘（中）**

| # | 指摘 | 根拠 | 方向性 |
|---|------|------|--------|
| 22 | CallId の型安全性が不十分（空文字許容） | identifiers.rs | CallId::new を Result にしてバリデーション追加 |
| 25 | recording 周りの非同期処理でエラーが握り潰されやすい | recording_manager.rs | merge/copy/stop の失敗をメトリクス or エラー集約に寄せる |
| 26 | anyhow がドメイン層に残存している箇所がある | recording_manager.rs, message.rs | 「ドメインに近い層」から専用エラー型に置換 |
| 27 | App層のAI処理が一箇所に集中しすぎてテストしにくい | mod.rs | 意図判定・ルーティング・応答生成をユースケース関数に分離 |

**指摘（軽）**

| # | 指摘 | 根拠 | 方向性 |
|---|------|------|--------|
| 28 | ログメッセージが大量でノイズになりやすい | mod.rs | info/debug の整理、重要イベントのみ info へ |
| 6 | RTP parser の仕様逸脱が残っている | parser.rs | CSRC/extension 対応を追加 |
| 29 | SessionRegistry でActor化は済んだがアクセスパターン設計意図が薄い | types.rs | register/unregister 規約をコメントで明確化 |

### 2.6 Phase 3 実装後レビュー（Codex 2026-02-03）

**指摘（重大/中）**

| # | 指摘 | 根拠 | 方向性 |
|---|------|------|--------|
| 30 | unbounded_channel でバックログが無制限に積み上がる | mod.rs:97, coordinator.rs:130, main.rs:77 | mpsc::channel に変更、ASR/PCM は最新優先で try_send + 旧データ破棄ポリシー明文化 |
| 31 | rtp_port_map が単一の受信ポート→call_id を上書きし同時通話で問題 | main.rs:73,219, rx.rs:109 | 通話ごとにRTPポート割当、または5-tuple/SSRCで紐付け。単一通話前提なら制約明示 |
| 32 | OnceLock 設定を下位層が直接参照（依存性注入崩れ） | config.rs:152, packet.rs:95 | Timeouts/RtpConfig をコンストラクタ引数で渡す |
| 33 | std::sync::Mutex を async ホットパスでロック（tokio ブロック） | packet.rs:27, rx.rs:34,111 | actor 化、または tokio::sync::Mutex/DashMap に置換 |
| 34 | AppWorker 履歴が無制限に増加、LLM 呼び出しで全履歴クローン | mod.rs:111,482 | 履歴の上限設定とリングバッファ化、または直近 N 件のみ送信 |

**指摘（軽）**

| # | 指摘 | 根拠 | 方向性 |
|---|------|------|--------|
| 35 | RtpPacket が CSRC/拡張ヘッダ非対応宣言だがパーサは黙ってスキップ | packet.rs:4, parser.rs:44 | extension/csrc_count>0 を明示的エラー化、または構造体に保持 |
| 36 | SessionIn::Abort(anyhow::Error) が境界を越えドメインエラー喪失 | types.rs:148 | SessionError などのドメインエラー型に寄せる |

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

### 3.7 session → app 依存方向違反（指摘 #11）

**現状:**
```rust
// coordinator.rs:17
use crate::app::AppEvent;

// mod.rs:10
use crate::app::EndReason;

// sip_handler.rs:4
use crate::app::AppEvent;
```

**違反:**
- BD-003 §2.3「依存方向: app → session が正」
- session が app に依存すると循環依存のリスク

**あるべき姿:**
```rust
// ports/session_events.rs または session/events.rs に定義
pub enum SessionEvent {
    CallEnded { call_id: CallId, reason: EndReason },
    // ...
}

// app 側で From/adapter で受け取る
impl From<SessionEvent> for AppEvent {
    fn from(event: SessionEvent) -> Self { ... }
}
```

**最小差分案:**
- AppEvent を `ports/` または `session::events` へ移動
- app 側が From/adapter で受け取る設計に変更

---

### 3.8 session → http 境界逆転（指摘 #12）

**現状:**
```rust
// coordinator.rs:19
use crate::http::IngestPort;

// ingest.rs:9
// IngestPort が http 層に定義されている
```

**違反:**
- BD-003 §2.3「Port は内側（domain/session）、Adapter は外側（http）」
- session が http に依存すると境界が逆転

**あるべき姿:**
```rust
// ports/ingest.rs に Port（trait）を定義
pub trait IngestPort: Send + Sync {
    async fn post(&self, data: IngestData) -> Result<(), IngestError>;
}

// http/ingest_adapter.rs に実装
pub struct HttpIngestAdapter { ... }
impl IngestPort for HttpIngestAdapter { ... }
```

**最小差分案:**
- IngestPort を `ports/` に移動
- http 側は実装（adapter）として依存

---

### 3.9 HttpIngestPort Client 再利用（指摘 #13）

**現状:**
```rust
// ingest.rs:23-28
pub async fn post(&self, data: IngestData) -> Result<()> {
    let client = reqwest::Client::new();  // 毎回生成
    // ...
}
```

**問題:**
- 接続プール再利用が効かない
- パフォーマンス低下

**あるべき姿:**
```rust
pub struct HttpIngestAdapter {
    client: reqwest::Client,  // 保持して再利用
    base_url: String,
}

impl HttpIngestAdapter {
    pub fn new(base_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
        }
    }
}
```

---

## 4. 対応優先度

### Phase 1: BD-004 実装前に必須（AC-1..4 完了 / AC-5 未確認）

| # | 対応 | 理由 | 工数目安 | 状態 |
|---|------|------|---------|------|
| 4 | SessionCoordinator 分割 | IVR ロジック追加先を確保 | 大 | ✅ |
| 3 | SessionStateMachine 改修 | IVR 状態管理に必要 | 中 | ✅ |

### Phase 1.5: 依存方向修正（重大指摘対応）

| # | 対応 | 理由 | 工数目安 |
|---|------|------|---------|
| 11 | session → app 依存解消 | 依存方向ルール違反（重大） | 中 |
| 12 | session → http 依存解消 | 境界逆転（重大） | 中 |

### Phase 2: 品質向上

| # | 対応 | 理由 | 工数目安 |
|---|------|------|---------|
| 1 | SessionRegistry Actor化 | パフォーマンス・デッドロック防止 | 中 |
| 2 | CallId 統一 | DDD 境界明確化 | 小 |
| 5 | AsrError 導入 | エラーハンドリング改善 | 小 |
| 13 | HttpIngestAdapter Client 再利用 | パフォーマンス改善 | 小 |

### Phase 2.5: PoC暫定処置解消（依存方向完全適合）

| # | 対応 | 理由 | 工数目安 |
|---|------|------|---------|
| 14 | LINE通知アダプタをinfra層へ移動 | UseCase内のIO直接実行（重大） | 中 |
| 15 | session→recording依存解消、StoragePort移動 | 依存方向違反（重大） | 中 |
| 16 | PIIログ出力のマスキング | AGENTS違反（重大） | 小 |
| 17 | PhoneLookupPort を ports へ移動 | 依存方向違反（重大） | 小 |
| 18 | Port境界からanyhow/serde_json排除 | infra型漏れ（中） | 中 |
| 21 | AppEvent.call_id を CallId 型に | 型安全性（中） | 小 |

### Phase 3: アーキテクチャ深化（規模拡大前に対応推奨）

**重大**

| # | 対応 | 理由 | 工数目安 |
|---|------|------|---------|
| 24 | session 責務の更なる分離（state_machine 純化、handler 責務分離） | 再肥大化防止（重大） | 大 |
| 19 | config グローバル状態解消（Config を composition root で注入） | テスト再現性（重大） | 中 |

**中**

| # | 対応 | 理由 | 工数目安 |
|---|------|------|---------|
| 22 | CallId バリデーション追加（CallId::new を Result に） | 型安全性（中） | 小 |
| 25 | recording 非同期エラーの集約（merge/copy/stop 失敗をメトリクス化） | エラー可視化（中） | 小 |
| 26 | anyhow ドメイン層残存箇所の専用エラー型置換 | エラー型一貫性（中） | 中 |
| 27 | App層AI処理の分離（意図判定/ルーティング/応答生成をユースケース関数に） | テスタビリティ（中） | 中 |

**軽**

| # | 対応 | 理由 | 工数目安 |
|---|------|------|---------|
| 6 | RTP parser 改修（CSRC/extension 対応） | 仕様準拠（軽） | 小 |
| 28 | ログ info/debug 整理（重要イベントのみ info） | ノイズ削減（軽） | 小 |
| 29 | SessionRegistry アクセスパターン規約コメント追加 | 設計意図明確化（軽） | 小 |
| 20 | Arc<Mutex<HashMap>> 設計明確化 | 並行設計意図（軽） | 小 |
| 23 | AiServices 分割 | 境界責務（軽） | 小 |

### Phase 4: 並行処理・スケーラビリティ改善

**重大/中**

| # | 対応 | 理由 | 工数目安 |
|---|------|------|---------|
| 30 | unbounded_channel → mpsc::channel（バックプレッシャ対応） | AGENTS バックプレッシャ方針違反 | 中 |
| 31 | rtp_port_map 同時通話対応（通話ごとにRTPポート割当 or 5-tuple/SSRC紐付け） | 同時通話不可（重大） | 大 |
| 32 | OnceLock 下位層直接参照解消（Timeouts/RtpConfig をコンストラクタ注入） | 依存性注入崩れ | 中 |
| 33 | std::sync::Mutex → tokio::sync::Mutex/DashMap（async ホットパス対応） | tokio ブロック | 中 |
| 34 | AppWorker 履歴上限設定（リングバッファ化 or 直近 N 件のみ送信） | メモリ/CPU 線形悪化 | 小 |

**軽**

| # | 対応 | 理由 | 工数目安 |
|---|------|------|---------|
| 35 | RtpPacket CSRC/extension 仕様明確化（エラー化 or 構造体保持） | 期待仕様曖昧 | 小 |
| 36 | SessionIn::Abort → SessionError（ドメインエラー型） | 境界越えエラー喪失 | 小 |

---

## 5. 受入条件（Acceptance Criteria）

### Phase 1（AC-1..4 完了 / AC-5 未確認）

- [x] AC-1: SessionCoordinator が 500 行以下になっている
- [x] AC-2: IVR ロジックが `session/services/ivr_service.rs` に分離されている
- [x] AC-3: B2BUA ロジックが `session/services/b2bua_service.rs` に分離されている
- [x] AC-4: SessionStateMachine が SessionEvent を受け取り SessionCommand を返す設計になっている
- [ ] AC-5: 既存のテストが全て pass する
  - 備考: `recording_http_e2e` がローカルで PermissionDenied のため未確認

### Phase 1.5（依存方向修正）

- [x] AC-10: session 層が app 層に直接依存していない（AppEvent/EndReason が ports/ または session::events に移動）
- [x] AC-11: session 層が http 層に直接依存していない（IngestPort が ports/ に移動）

### Phase 2 ✅ 完了

- [x] AC-6: SessionRegistry が Actor パターンで実装されている
- [x] AC-7: CallId が entities/identifiers.rs のみに定義されている
- [x] AC-8: AsrPort が AsrError を返す設計になっている
- [x] AC-9: AiPort が AsrPort にリネームされている
- [x] AC-12: HttpIngestAdapter が reqwest::Client を保持して再利用している

### Phase 2.5（PoC暫定処置解消）

- [x] AC-13: LINE通知がports::notificationのtraitを経由している（notification.rs内のreqwest直接利用が解消）
- [x] AC-14: session層がrecording層に直接依存していない（StoragePortがports/に移動）
- [x] AC-15: PIIがログにマスクされて出力される（ユーザー発話・電話番号が伏字）
- [x] AC-16: PhoneLookupPortがports/phone_lookupに定義されている（db層はadapter実装のみ）
- [x] AC-17: Port境界がanyhow::ResultやSerdeJsonを露出していない（ドメインDTO/専用エラー型のみ）
- [x] AC-18: AppEvent.call_idがCallId型になっている（String固定が解消）

### Phase 3 ✅ 完了（アーキテクチャ深化）

**重大**
- [x] AC-19: session/state_machine が純粋な状態遷移のみを担当（IO非依存）
  - 根拠: coordinator.rs, mod.rs
  - 方向性: state_machine 純化、handler 責務分離をもう一段進める
- [x] AC-20: Config がグローバル状態でなく、必要箇所に引数で注入されている
  - 根拠: config.rs:163,320（OnceLock 複数）
  - 方向性: composition root で読み込み、Config を引数で渡す範囲を増やす

**中**
- [x] AC-21: CallId::new が Result を返しバリデーションされている
  - 根拠: identifiers.rs:7（空文字許容）
  - 方向性: Result<CallId, CallIdError> でバリデーション追加
- [x] AC-22: recording 非同期エラーがメトリクスまたはエラー集約に記録される
  - 根拠: recording_manager.rs（merge/copy/stop の失敗が握り潰される）
  - 方向性: 失敗をメトリクス or エラー集約に寄せる
- [x] AC-23: recording_manager, message 等のドメイン近傍から anyhow が排除されている
  - 根拠: recording_manager.rs, message.rs
  - 方向性: 専用エラー型に置換（段階的でOK）
- [x] AC-24: App層のAI処理（意図判定/ルーティング/応答生成）がユースケース関数に分離されている
  - 根拠: app/mod.rs（一箇所に集中しすぎ）
  - 方向性: 意図判定・ルーティング・応答生成をユースケース関数に分離

**軽**
- [x] AC-25: ログ出力が info/debug で整理され、重要イベントのみ info
  - 根拠: mod.rs（ログメッセージが大量でノイズ）
  - 方向性: info/debug の整理
- [x] AC-26: RTP parser が CSRC/extension に対応している
  - 根拠: parser.rs（仕様逸脱）
  - 方向性: CSRC/extension 対応を追加
- [x] AC-27: SessionRegistry の register/unregister 規約がコメントで明確化されている
  - 根拠: types.rs（アクセスパターン設計意図が薄い）
  - 方向性: どこで register/unregister するかの規約をコメントで明確化

### Phase 4（並行処理・スケーラビリティ改善）

**重大/中**
- [x] AC-28: イベントチャネルが mpsc::channel で bounded になり、バックプレッシャポリシーが明文化されている
  - 根拠: mod.rs:97, coordinator.rs:130, main.rs:77
  - 方向性: mpsc::channel、ASR/PCM は最新優先で try_send + 旧データ破棄
- [x] AC-29: 同時通話に対応している（通話ごとにRTPポート割当、または5-tuple/SSRC紐付け、または単一通話制約が明示）
  - 根拠: main.rs:73,219, rx.rs:109
  - 方向性: 通話ごとにRTPポート割当、または制約明示
- [x] AC-30: Timeouts/RtpConfig がコンストラクタ引数で渡されている（OnceLock 直接参照解消）
  - 根拠: config.rs:152, packet.rs:95
  - 方向性: コンストラクタ注入
- [x] AC-31: async ホットパスで tokio::sync::Mutex または DashMap を使用している
  - 根拠: packet.rs:27, rx.rs:34,111
  - 方向性: std::sync::Mutex → tokio::sync::Mutex/DashMap
- [x] AC-32: AppWorker 履歴に上限が設定されている（リングバッファ or 直近 N 件）
  - 根拠: mod.rs:111,482
  - 方向性: 履歴上限設定

**軽**
- [x] AC-33: RtpPacket の CSRC/extension 仕様が明確化されている（エラー化 or 構造体保持）
  - 根拠: packet.rs:4, parser.rs:44
  - 方向性: extension/csrc_count>0 を明示的エラー化、または構造体に保持
- [x] AC-34: SessionIn::Abort がドメインエラー型（SessionError）になっている
  - 根拠: types.rs:148
  - 方向性: SessionError に寄せる

---

## 6. 決定事項・未確定事項

### 決定済み

- [x] Q1: SessionRegistry の Actor 化と DashMap 化、どちらを採用するか？
  - **決定: Actor 化**（より明確な非同期境界）
- [x] Q2: Phase 1 と Phase 2 を同時に進めるか、Phase 1 完了後に Phase 2 に進むか？
  - **決定: Phase 1 → BD-004 → Phase 2**（段階的に進める）

- [x] Q3: AppEvent を session から直接送る設計を維持するか？
  - **決定: B) 修正する** - AppEvent を ports/ または session::events へ移動
- [x] Q4: IngestPort を ports/ に寄せる方針で問題ないか？
  - **決定: A) Yes** - IngestPort を ports/ に移動し、http 側は adapter として実装
- [x] Q5: session → recording 直参照の設計意図は？
  - **決定: PoC暫定処置のため撤廃** - StoragePort を ports/ に移動
- [x] Q6: db::port を db に置く理由は？
  - **決定: PoC暫定処置のため ports/ へ移動** - PhoneLookupPort を ports/phone_lookup に

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
| 2026-02-03 | 1.5 | Phase 1 実装後レビュー追記（#11-13: 依存方向違反、境界逆転、Client再利用）、Phase 1.5 追加、Q3/Q4 追加 | Claude Code |
| 2026-02-03 | 1.6 | Q3/Q4 決定（AppEvent移動、IngestPort移動） | Claude Code |
| 2026-02-03 | 1.7 | Phase 1 見出し修正（AC-5 未確認を明記） | Claude Code |
| 2026-02-03 | 1.8 | Phase 1.5 実装（AC-10/11/12 達成） | Codex |
| 2026-02-03 | 1.9 | Phase 2 実装（AC-6: SessionRegistry Actor化） | Codex |
| 2026-02-03 | 2.0 | Phase 2 実装（AC-7: CallId 統一） | Codex |
| 2026-02-03 | 2.1 | AC-8/9 対応（AsrPort の型設計確認、AiPort 互換削除） | Codex |
| 2026-02-03 | 2.2 | Phase 2 完了後レビュー追記（#14-23）、Phase 2.5 追加、Q5/Q6 決定 | Claude Code |
| 2026-02-03 | 2.3 | BD-003 v2.0 改訂（依存関係図作成、モジュール層対応表追加）、Phase 2.5 実装準備完了 | Claude Code |
| 2026-02-03 | 2.4 | Phase 3 準備レビュー追記（#24-29: session再分離、config注入、CallId検証、recording/anyhow/AI分離、ログ整理、Registry規約）、AC-19〜27 追加 | Claude Code |
| 2026-02-03 | 2.5 | Phase 3 AC-19〜27 に根拠・方向性詳細を追加 | Claude Code |
| 2026-02-03 | 2.6 | Phase 3 実装後レビュー追記（#30-36: unbounded_channel、rtp_port_map同時通話、OnceLock、sync::Mutex、AppWorker履歴、RtpPacket仕様、SessionIn::Abort）、Phase 4 追加、AC-28〜34 追加 | Claude Code |
| 2026-02-05 | 2.7 | Phase 3 完了マーク追加（AC-19〜27 全達成） | Claude Code |
| 2026-02-04 | 2.8 | Phase 4 AC-28〜34 実装完了（bounded channel、RTP同時通話、注入、tokio::Mutex、履歴上限、RTP仕様、SessionError） | Codex |
