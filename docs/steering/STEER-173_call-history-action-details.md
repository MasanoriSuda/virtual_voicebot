# STEER-173: 発着信履歴のアクション詳細表示とIVR経路追従

<!--
  ============================================================
  配置: docs/steering/ (Frontend-Backend 横断)
  ============================================================
-->

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-173 |
| タイトル | 発着信履歴のアクション詳細表示とIVR経路追従 |
| ステータス | Approved |
| 関連Issue | #173 |
| 優先度 | P1 |
| 作成日 | 2026-02-14 |

---

## 2. ストーリー（Why）

### 2.1 背景

現在の発着信履歴ページには、以下の情報が不足している：

1. **アクション詳細の不足**:
   - 着信許可/拒否の区別が不明確
   - 実際に何が起こったか（通常着信/IVR/ボイスボット/ビジー等）がわからない
   - 転送の成否が不明

2. **IVR経路追従の欠如**:
   - IVR 実行時に、どのキーが押されたか記録されていない
   - IVR フロー内での経路追従ができない
   - 転送試行/成立/終了のタイミングが不明
   - 通話終了理由（IVR選択中の離脱/転送試行中の離脱/完了）がわからない

3. **データ構造の問題**:
   - Backend `call_logs` テーブルには `action_code` のみがあり、実行結果の詳細が記録されていない
   - IVR セッションイベント（ノード訪問、DTMF入力、遷移）が記録されていない
   - 転送の詳細（試行/応答/終了）が記録されていない

**誰が困っているか**:
- システム管理者: IVR フローの動作確認・デバッグができない
- オペレーター: 顧客がどの経路で着信したかわからない
- 一般社員: 着信拒否されたのか、転送失敗なのか判断できない

**放置するとどうなるか**:
- IVR フローの問題を発見できない
- 顧客対応履歴が不完全で、トラブル時の原因究明が困難
- 転送失敗の原因分析ができない

### 2.2 目的

- 発着信履歴ページに **アクション詳細** を表示（着信許可/拒否、実行結果）
- **IVR経路追従** 機能を追加（別ページ遷移でタイムライン表示）
- Backend でIVRセッションイベントを記録し、Frontend で可視化

### 2.3 ユーザーストーリー

```
As a システム管理者
I want to IVR経路追従の詳細を確認したい
So that IVRフローの動作確認とデバッグができる

受入条件:
- [ ] 発着信履歴ページに「着信応答」カラムが追加されている（許可/拒否/無応答）
- [ ] 発着信履歴ページに「実行アクション」カラムが追加されている（実際の動作）
- [ ] 発着信履歴ページに「転送状況」カラムが追加されている（転送の成否）
- [ ] IVR実行時に「IVR詳細」リンクが表示され、別ページで経路追従が確認できる
- [ ] IVR詳細ページに、ノード訪問/DTMF入力/遷移のタイムラインが表示される
- [ ] IVR詳細ページに、転送試行/成立/終了のタイミングが表示される
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-14 |
| 起票理由 | 発着信履歴の詳細情報不足とIVR経路追従機能の要望 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-sonnet-4-5-20250929) |
| 作成日 | 2026-02-14 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "壁打ちお願いします" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | Codex | 2026-02-14 | NG | ALTER TABLE構文エラー、AR分類誤り、gen_ulid_uuid7()不在、enum制約不足、sequence一意制約欠如、announcement_reject表記誤り → 全て修正済み |
| 2 | Codex | 2026-02-14 | NG | call_log_id ライフサイクル問題（重大）→ A案（メモリバッファ）で対応、ファイルパス確認要、announcement_deny統一完了、本レビュー記録追加済み |
| 3 | Codex | 2026-02-14 | NG | 同期方式未確定（重大）→ sync_outbox パターンに統一、書き込み責務不一致（中）→ postgres.rs に統合、型/関数名ズレ（軽）→ SessionCoordinator/persist_call_ended に修正 |
| 4 | Codex | 2026-02-14 | OK | 重大・中・軽の指摘なし。残リスク（通話中クラッシュ時IVRイベント欠落、persist_call_ended回帰テスト、Frontend単体テスト）は実装フェーズで対応 |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-14 |
| 承認コメント | Codex レビュー4回を経て全指摘事項解消。sync_outbox パターンへの統一、既存実装との整合性確保を確認。実装フェーズへ移行可。 |

### 3.5 実装（該当する場合）

| 項目 | 値 |
|------|-----|
| 実装者 | Codex (GPT-5) |
| 実装日 | 2026-02-14 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "作業お願いします Refs #173" |
| コードレビュー | Pending |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | |
| マージ日 | |
| マージ先 | contract.md v2.3, RD-004 (FR-119), Backend/Frontend 各種仕様書 |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| docs/contract.md | 修正 | Call DTO 拡張、IvrSessionEvent DTO 追加 |
| docs/requirements/RD-004_call-routing.md | 追加 | FR-119: IVR経路追従記録 |
| virtual-voicebot-backend/docs/design/detail/DD-007_recording.md | 修正 | call_logs テーブル拡張 |
| virtual-voicebot-frontend/docs/design/detail/DD-xxx_call-history.md | 追加 | call-history ページ拡張、ivr-trace ページ新規 |
| virtual-voicebot-backend/docs/test/unit/UT-xxx.md | 追加 | IVR イベント記録テスト |
| virtual-voicebot-frontend/docs/test/unit/UT-xxx.md | 追加 | IVR 詳細ページテスト |

### 4.2 影響するコード

**Backend**:

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| virtual-voicebot-backend/migrations/xxx_call_logs_add_action_details.sql | 追加 | call_logs テーブル拡張（call_disposition, final_action, transfer_*） |
| virtual-voicebot-backend/migrations/xxx_create_ivr_session_events.sql | 追加 | ivr_session_events テーブル作成 |
| virtual-voicebot-backend/src/protocol/session/coordinator.rs | 修正 | SessionCoordinator に ivr_events フィールド追加、push_ivr_event() 実装 |
| virtual-voicebot-backend/src/interface/db/postgres.rs | 修正 | CallLogPort::persist_call_ended で ivr_session_events INSERT + sync_outbox enqueue |
| virtual-voicebot-backend/src/interface/sync/worker.rs | - | 変更不要（sync_outbox の既存処理で ivr_session_event も送信される） |

**Frontend**:

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| virtual-voicebot-frontend/lib/types.ts | 修正 | Call 型拡張、IvrSessionEvent 型追加 |
| virtual-voicebot-frontend/lib/db/sync.ts | 修正 | ivr_session_event エンティティ処理追加 |
| virtual-voicebot-frontend/app/api/ingest/sync/route.ts | 修正 | ivr_session_event の upsert 処理追加 |
| virtual-voicebot-frontend/components/call-history-content.tsx | 修正 | カラム追加（着信応答、実行アクション、転送状況、IVR詳細） |
| virtual-voicebot-frontend/app/calls/[callId]/ivr-trace/page.tsx | 追加 | IVR 経路詳細ページ（既存 /calls 系に統一） |
| virtual-voicebot-frontend/components/ivr-trace-timeline.tsx | 追加 | IVR タイムラインコンポーネント |
| virtual-voicebot-frontend/components/ivr-flow-chart.tsx | 追加 | IVR フローチャートコンポーネント |

---

## 5. 差分仕様（What / How）

### 5.1 Backend 変更

#### 5.1.1 call_logs テーブル拡張

**マイグレーション**: `xxx_call_logs_add_action_details.sql`

```sql
-- 着信応答区分の追加
ALTER TABLE call_logs
    ADD COLUMN call_disposition VARCHAR(20) NOT NULL DEFAULT 'allowed';

ALTER TABLE call_logs
    ADD CONSTRAINT chk_call_disposition
        CHECK (call_disposition IN ('allowed', 'denied', 'no_answer'));

-- 最終実行アクションの追加（enum化）
ALTER TABLE call_logs
    ADD COLUMN final_action VARCHAR(50);

ALTER TABLE call_logs
    ADD CONSTRAINT chk_final_action
        CHECK (final_action IN (
            -- 着信許可
            'normal_call', 'voicebot', 'ivr', 'voicemail', 'announcement',
            -- 着信拒否
            'busy', 'rejected', 'announcement_deny'
        ));

-- 転送ステータスの追加
ALTER TABLE call_logs
    ADD COLUMN transfer_status VARCHAR(20) NOT NULL DEFAULT 'no_transfer';

ALTER TABLE call_logs
    ADD CONSTRAINT chk_transfer_status
        CHECK (transfer_status IN ('none', 'trying', 'answered', 'failed', 'no_transfer'));

-- 転送日時の追加
ALTER TABLE call_logs
    ADD COLUMN transfer_started_at TIMESTAMPTZ;

ALTER TABLE call_logs
    ADD COLUMN transfer_answered_at TIMESTAMPTZ;

ALTER TABLE call_logs
    ADD COLUMN transfer_ended_at TIMESTAMPTZ;

-- インデックス追加
CREATE INDEX idx_call_logs_disposition ON call_logs(call_disposition);
CREATE INDEX idx_call_logs_final_action ON call_logs(final_action);
CREATE INDEX idx_call_logs_transfer_status ON call_logs(transfer_status);
```

**フィールド定義**:

| フィールド | 型 | 説明 | 値の例 |
|-----------|-----|------|--------|
| call_disposition | VARCHAR(20) | 着信応答区分 | 'allowed' / 'denied' / 'no_answer' |
| final_action | VARCHAR(50) | 最終実行アクション | 'normal_call' / 'voicebot' / 'ivr' / 'busy' / 'announcement_deny' 等 |
| transfer_status | VARCHAR(20) | 転送ステータス | 'none' / 'trying' / 'answered' / 'failed' / 'no_transfer' |
| transfer_started_at | TIMESTAMPTZ | 転送開始日時 | |
| transfer_answered_at | TIMESTAMPTZ | 転送応答日時（B-leg 応答） | |
| transfer_ended_at | TIMESTAMPTZ | 転送終了日時 | |

**call_disposition の値**:
- `'allowed'`: 着信許可（VR/VB/IV/VM/AN/AR）
  - AR = Announce+Record（アナウンス再生後に録音）も許可に含む
- `'denied'`: 着信拒否（BZ/RJ）
- `'no_answer'`: 無応答（NR）

**final_action の値（enum 化）**:
- 着信許可: `'normal_call'` / `'voicemail'` / `'voicebot'` / `'ivr'` / `'announcement'`
- 着信拒否: `'busy'` / `'rejected'` / `'announcement_deny'`

**transfer_status の値**:
- `'no_transfer'`: 転送なし
- `'none'`: 転送未試行
- `'trying'`: 転送試行中（B2BUA セッション確立中）
- `'answered'`: 転送成立（B-leg 応答）
- `'failed'`: 転送失敗

#### 5.1.2 ivr_session_events テーブル作成

**マイグレーション**: `xxx_create_ivr_session_events.sql`

```sql
-- IVR セッションイベント記録テーブル
CREATE TABLE ivr_session_events (
    -- id はアプリ側で UUID v7 を生成して INSERT
    id UUID PRIMARY KEY,
    call_log_id UUID NOT NULL,
    sequence INT NOT NULL CHECK (sequence >= 0),
    event_type VARCHAR(20) NOT NULL CHECK (event_type IN (
        'node_enter',     -- ノード訪問
        'dtmf_input',     -- DTMF 入力
        'transition',     -- 遷移
        'timeout',        -- タイムアウト
        'invalid_input',  -- 無効入力
        'exit'            -- IVR 終了
    )),
    occurred_at TIMESTAMPTZ NOT NULL,

    -- イベント詳細（event_type に応じて使用）
    node_id UUID,           -- ノード訪問時
    dtmf_key VARCHAR(1),    -- DTMF 入力時
    transition_id UUID,     -- 遷移時
    exit_action VARCHAR(2), -- IVR 終了時のアクションコード
    exit_reason VARCHAR(50),-- IVR 終了理由

    metadata JSONB,         -- その他の情報
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_ivr_event_call_log
        FOREIGN KEY (call_log_id) REFERENCES call_log_index(id)
        ON DELETE CASCADE,

    -- 同一 call_log_id 内で sequence が一意であることを保証
    CONSTRAINT uq_ivr_event_sequence
        UNIQUE (call_log_id, sequence)
);

-- インデックス
CREATE INDEX idx_ivr_events_call_log ON ivr_session_events(call_log_id, sequence);
CREATE INDEX idx_ivr_events_occurred_at ON ivr_session_events(occurred_at);
CREATE INDEX idx_ivr_events_type ON ivr_session_events(event_type);
```

**注記**: ivr_session_events テーブル自体には `synced_at` カラムは不要。Frontend への同期は既存の `sync_outbox` テーブルを使用。

**event_type の説明**:

| event_type | 説明 | 使用するフィールド |
|-----------|------|------------------|
| node_enter | ノード訪問 | node_id, occurred_at |
| dtmf_input | DTMF 入力 | dtmf_key, occurred_at |
| transition | 遷移 | transition_id, occurred_at |
| timeout | タイムアウト | node_id, occurred_at |
| invalid_input | 無効入力 | node_id, occurred_at |
| exit | IVR 終了 | exit_action, exit_reason, occurred_at |

**イベント記録例**:

```sql
-- 例1: ノード訪問
INSERT INTO ivr_session_events (call_log_id, sequence, event_type, occurred_at, node_id)
VALUES ('019503a0-...', 0, 'node_enter', '2026-02-14T10:00:05Z', '019503a0-node1...');

-- 例2: DTMF 入力
INSERT INTO ivr_session_events (call_log_id, sequence, event_type, occurred_at, dtmf_key)
VALUES ('019503a0-...', 1, 'dtmf_input', '2026-02-14T10:00:10Z', '1');

-- 例3: 遷移
INSERT INTO ivr_session_events (call_log_id, sequence, event_type, occurred_at, transition_id)
VALUES ('019503a0-...', 2, 'transition', '2026-02-14T10:00:11Z', '019503a0-trans1...');

-- 例4: IVR 終了
INSERT INTO ivr_session_events (call_log_id, sequence, event_type, occurred_at, exit_action, exit_reason)
VALUES ('019503a0-...', 3, 'exit', '2026-02-14T10:00:15Z', 'VR', 'transfer_initiated');
```

#### 5.1.3 IVR イベント記録処理

**修正箇所**:
- `virtual-voicebot-backend/src/protocol/session/coordinator.rs` (SessionCoordinator にフィールド追加)
- `virtual-voicebot-backend/src/interface/db/postgres.rs` (CallLogPort::persist_call_ended 拡張)

**⚠️ 重要: call_log_id ライフサイクルと同期方式**

現行実装では `call_log_id` は通話終了時（coordinator.rs:447）に生成されるため、通話中の IVR イベント記録時に FK 制約違反が発生します。また、Frontend への同期は既存の `sync_outbox` テーブルを使用します。

**採用方式**: **メモリ保持 + 通話終了時一括 INSERT + Outbox enqueue**

- SessionCoordinator に `Vec<IvrEventRecord>` でイベントをメモリ保持
- 通話終了時の `CallLogPort::persist_call_ended()` 内で：
  1. call_log_id 生成（既存）
  2. call_logs / recordings INSERT（既存）
  3. **ivr_session_events へ bulk INSERT**（新規）
  4. **sync_outbox へ各 ivr_session_event を enqueue**（新規）
  5. トランザクションコミット（既存）

**メリット**:
- 既存の call_log_id ライフサイクルを変更不要
- 既存の sync_outbox パターンを踏襲（call_log / recording と同じ）
- トランザクション内で完結、整合性保証

**デメリット**:
- 通話中クラッシュ時にイベントが失われる（MVP では許容可能）

**実装イメージ**:

```rust
// ============================================================
// src/protocol/session/coordinator.rs
// ============================================================

// 🆕 IVR イベント記録用構造体
#[derive(Debug, Clone)]
pub struct IvrEventRecord {
    pub event_type: String,
    pub occurred_at: DateTime<Utc>,
    pub node_id: Option<Uuid>,
    pub dtmf_key: Option<String>,
    pub transition_id: Option<Uuid>,
    pub exit_action: Option<String>,
    pub exit_reason: Option<String>,
}

pub struct SessionCoordinator {
    // ... 既存フィールド ...

    // 🆕 IVR イベント記録用（メモリ保持）
    pub ivr_events: Vec<IvrEventRecord>,
}

impl SessionCoordinator {
    // 🆕 IVR イベント追加メソッド
    pub fn push_ivr_event(&mut self, event: IvrEventRecord) {
        self.ivr_events.push(event);
    }

    // IVR フロー開始時の例
    pub async fn enter_ivr_flow(&mut self, ivr_flow_id: Uuid) {
        // ... 既存の IVR 開始処理 ...

        // 🆕 ノード訪問イベント記録
        self.push_ivr_event(IvrEventRecord {
            event_type: "node_enter".to_string(),
            occurred_at: Utc::now(),
            node_id: Some(first_node_id),
            dtmf_key: None,
            transition_id: None,
            exit_action: None,
            exit_reason: None,
        });
    }

    // DTMF 入力時の例
    pub async fn handle_dtmf(&mut self, key: char) {
        // 🆕 DTMF 入力イベント記録
        self.push_ivr_event(IvrEventRecord {
            event_type: "dtmf_input".to_string(),
            occurred_at: Utc::now(),
            node_id: None,
            dtmf_key: Some(key.to_string()),
            transition_id: None,
            exit_action: None,
            exit_reason: None,
        });

        // ... 既存の DTMF 処理 ...
    }

    // 通話終了時（既存関数を拡張）
    pub async fn finalize_and_persist(self) -> Result<()> {
        // ... 既存の call_log_id 生成・EndedCallLog 構築 ...

        let ended_call = EndedCallLog {
            id: call_log_id,  // Uuid::now_v7() で生成（既存）
            // ... 既存フィールド ...
            ivr_events: self.ivr_events,  // 🆕 IVR イベントを渡す
            // ...
        };

        // CallLogPort::persist_call_ended に渡す
        self.call_log_port.persist_call_ended(ended_call).await?;
        Ok(())
    }
}

// ============================================================
// src/interface/db/postgres.rs (CallLogPort::persist_call_ended 拡張)
// ============================================================

impl CallLogPort for PostgresAdapter {
    fn persist_call_ended(&self, call_log: EndedCallLog) -> CallLogFuture<()> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let mut tx = pool.begin().await.map_err(map_call_log_write_err)?;

            // 1. call_log_index INSERT（既存）
            sqlx::query("INSERT INTO call_log_index (id, started_at) VALUES ($1, $2)")
                .bind(call_log.id)
                .bind(call_log.started_at)
                .execute(&mut *tx)
                .await
                .map_err(map_call_log_write_err)?;

            // 2. call_logs INSERT（既存）
            sqlx::query("INSERT INTO call_logs (...) VALUES (...)")
                // ... 既存の bind ...
                .execute(&mut *tx)
                .await
                .map_err(map_call_log_write_err)?;

            // 3. sync_outbox へ call_log enqueue（既存）
            sqlx::query("INSERT INTO sync_outbox (entity_type, entity_id, payload) VALUES ($1, $2, $3)")
                .bind("call_log")
                .bind(call_log.id)
                .bind(json!({ /* call_log DTO */ }))
                .execute(&mut *tx)
                .await
                .map_err(map_call_log_write_err)?;

            // 4. 🆕 ivr_session_events bulk INSERT + sync_outbox enqueue
            if !call_log.ivr_events.is_empty() {
                for (seq, event) in call_log.ivr_events.iter().enumerate() {
                    let event_id = Uuid::now_v7();

                    // ivr_session_events INSERT
                    sqlx::query(
                        "INSERT INTO ivr_session_events (
                            id, call_log_id, sequence, event_type, occurred_at,
                            node_id, dtmf_key, transition_id, exit_action, exit_reason
                         ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
                    )
                    .bind(event_id)
                    .bind(call_log.id)
                    .bind(seq as i32)
                    .bind(&event.event_type)
                    .bind(event.occurred_at)
                    .bind(event.node_id)
                    .bind(event.dtmf_key.as_deref())
                    .bind(event.transition_id)
                    .bind(event.exit_action.as_deref())
                    .bind(event.exit_reason.as_deref())
                    .execute(&mut *tx)
                    .await
                    .map_err(map_call_log_write_err)?;

                    // sync_outbox enqueue（call_log / recording と同じパターン）
                    sqlx::query(
                        "INSERT INTO sync_outbox (entity_type, entity_id, payload)
                         VALUES ($1, $2, $3)"
                    )
                    .bind("ivr_session_event")
                    .bind(event_id)
                    .bind(json!({
                        "id": event_id.to_string(),
                        "callLogId": call_log.id.to_string(),
                        "sequence": seq,
                        "eventType": &event.event_type,
                        "occurredAt": event.occurred_at.to_rfc3339(),
                        "nodeId": event.node_id.as_ref().map(Uuid::to_string),
                        "dtmfKey": &event.dtmf_key,
                        "transitionId": event.transition_id.as_ref().map(Uuid::to_string),
                        "exitAction": &event.exit_action,
                        "exitReason": &event.exit_reason,
                        "metadata": serde_json::Value::Null,
                    }))
                    .execute(&mut *tx)
                    .await
                    .map_err(map_call_log_write_err)?;
                }
            }

            // 5. recording 処理（既存）
            // ...

            tx.commit().await.map_err(map_call_log_write_err)?;
            Ok(())
        })
    }
}
```

**注記**:
- SessionCoordinator の実際のフィールド名・メソッド名は実装時に現行コードに合わせて調整
- EndedCallLog 構造体に `ivr_events: Vec<IvrEventRecord>` フィールドを追加- ivr_events.rs という新規ファイルは不要（postgres.rs に統合）

#### 5.1.4 Serversync 拡張

**Backend 側の変更**: **不要**

- §5.1.3 で `postgres.rs::persist_call_ended()` 内で `sync_outbox` へ ivr_session_event を enqueue 済み
- `worker.rs` は既存のまま動作（`sync_outbox` から entity_type="ivr_session_event" を取得して送信）

**Frontend 修正箇所**:
- `virtual-voicebot-frontend/lib/db/sync.ts`
- `virtual-voicebot-frontend/app/api/ingest/sync/route.ts`

**Frontend 追加処理**:
1. `entityType = "ivr_session_event"` の処理を追加（現在は unsupported でスキップされる）
2. `ivr_session_events` テーブルへの upsert 処理を実装
3. エラーハンドリング（FK 制約違反時の対応）

**実装イメージ（Frontend sync.ts）**:
```typescript
// lib/db/sync.ts

export async function processSyncEntry(entry: SyncEntry) {
  switch (entry.entityType) {
    case 'call_log':
      await upsertCallLog(entry.payload as Call)
      break
    case 'recording':
      await upsertRecording(entry.payload as Recording)
      break
    case 'ivr_session_event':  // 🆕 追加
      await upsertIvrSessionEvent(entry.payload as IvrSessionEvent)
      break
    default:
      console.warn(`[sync] Unsupported entityType: ${entry.entityType}`)
  }
}

async function upsertIvrSessionEvent(event: IvrSessionEvent) {
  // ivr_session_events テーブルへ upsert
  await db.execute(
    sql`
      INSERT INTO ivr_session_events (
        id, call_log_id, sequence, event_type, occurred_at,
        node_id, dtmf_key, transition_id, exit_action, exit_reason, metadata
      ) VALUES (
        ${event.id}, ${event.callLogId}, ${event.sequence}, ${event.eventType}, ${event.occurredAt},
        ${event.nodeId}, ${event.dtmfKey}, ${event.transitionId}, ${event.exitAction}, ${event.exitReason}, ${event.metadata}
      )
      ON CONFLICT (id) DO UPDATE SET
        sequence = EXCLUDED.sequence,
        event_type = EXCLUDED.event_type,
        occurred_at = EXCLUDED.occurred_at,
        node_id = EXCLUDED.node_id,
        dtmf_key = EXCLUDED.dtmf_key,
        transition_id = EXCLUDED.transition_id,
        exit_action = EXCLUDED.exit_action,
        exit_reason = EXCLUDED.exit_reason,
        metadata = EXCLUDED.metadata
    `
  )
}
```

---

### 5.2 contract.md 更新

#### 5.2.1 Call DTO 拡張

**ファイル**: `docs/contract.md`

**変更箇所**: §3.1 Call

```markdown
### 3.1 Call

| フィールド | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| id | string (UUID) | Yes | Backend DB call_logs.id |
| externalCallId | string | Yes | アプリ層で生成する通話識別子 |
| callerNumber | string \| null | Yes | E.164 形式。null = 非通知 |
| callerCategory | CallerCategory | Yes | 発信者分類 |
| actionCode | string | Yes | 2 文字アクションコード |
| status | CallStatus | Yes | 通話ステータス |
| startedAt | string (ISO8601) | Yes | 通話開始日時 |
| answeredAt | string (ISO8601) \| null | No | 応答日時。null = 未応答 |
| endedAt | string (ISO8601) \| null | No | 終了日時 |
| durationSec | number \| null | No | 通話時間（秒） |
| endReason | EndReason | Yes | 終了理由 |
| **callDisposition** | **CallDisposition** | **Yes** | **着信応答区分** |
| **finalAction** | **FinalAction \| null** | **No** | **最終実行アクション** |
| **transferStatus** | **TransferStatus** | **Yes** | **転送ステータス** |
| **transferStartedAt** | **string (ISO8601) \| null** | **No** | **転送開始日時** |
| **transferAnsweredAt** | **string (ISO8601) \| null** | **No** | **転送応答日時** |
| **transferEndedAt** | **string (ISO8601) \| null** | **No** | **転送終了日時** |
```

#### 5.2.2 新規 Enum 定義

**追加箇所**: §2. 正規 Enum 定義

```markdown
### CallDisposition
`"allowed" | "denied" | "no_answer"`

### FinalAction
`"normal_call" | "voicebot" | "ivr" | "voicemail" | "announcement" | "busy" | "rejected" | "announcement_deny"`

### TransferStatus
`"no_transfer" | "none" | "trying" | "answered" | "failed"`

### IvrEventType
`"node_enter" | "dtmf_input" | "transition" | "timeout" | "invalid_input" | "exit"`
```

#### 5.2.3 新規 DTO: IvrSessionEvent

**追加箇所**: §3. Public DTO（Read Model）

```markdown
### 3.14 IvrSessionEvent

| フィールド | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| id | string (UUID) | Yes | ivr_session_events.id |
| callLogId | string (UUID) | Yes | 紐付く Call の id |
| sequence | number | Yes | イベント順序（0始まり） |
| eventType | IvrEventType | Yes | イベント種別 |
| occurredAt | string (ISO8601) | Yes | イベント発生日時 |
| nodeId | string (UUID) \| null | No | 訪問したノード |
| dtmfKey | string \| null | No | 押下されたキー |
| transitionId | string (UUID) \| null | No | 遷移 |
| exitAction | string \| null | No | IVR 終了時のアクションコード |
| exitReason | string \| null | No | IVR 終了理由 |
| metadata | object \| null | No | その他の情報（JSONB） |

**イベント種別の説明**:
- `node_enter`: ノード訪問（nodeId を使用）
- `dtmf_input`: DTMF 入力（dtmfKey を使用）
- `transition`: 遷移（transitionId を使用）
- `timeout`: タイムアウト（nodeId を使用）
- `invalid_input`: 無効入力（nodeId を使用）
- `exit`: IVR 終了（exitAction, exitReason を使用）

**JSON 例**:
```json
{
  "id": "019503a0-1234-7000-8000-000000000010",
  "callLogId": "019503a0-1234-7000-8000-000000000001",
  "sequence": 2,
  "eventType": "dtmf_input",
  "occurredAt": "2026-02-14T10:00:10.000Z",
  "nodeId": null,
  "dtmfKey": "1",
  "transitionId": null,
  "exitAction": null,
  "exitReason": null,
  "metadata": null
}
```
```

#### 5.2.4 API エンドポイント更新

**修正箇所**: §5.1 Backend → Frontend（Sync / Ingest）

```markdown
#### POST /api/ingest/sync

**リクエスト**:
```json
{
  "entries": [
    {
      "entityType": "call_log" | "recording" | "ivr_session_event" | ...,
      "entityId": "019503a0-...",
      "payload": { /* 該当エンティティの DTO */ },
      "createdAt": "2026-02-14T10:00:00Z"
    }
  ]
}
```

**処理内容**:
- `entityType = "ivr_session_event"` の場合、Frontend DB の `ivr_session_events` テーブルに upsert
```

---

### 5.3 Frontend 変更

#### 5.3.1 型定義更新（lib/types.ts）

**ファイル**: `virtual-voicebot-frontend/lib/types.ts`

**追加**:
```typescript
// --- Enums ---

export type CallDisposition = "allowed" | "denied" | "no_answer"

export type FinalAction =
  // 着信許可
  | "normal_call"
  | "voicebot"
  | "ivr"
  | "voicemail"
  | "announcement"
  // 着信拒否
  | "busy"
  | "rejected"
  | "announcement_deny"

export type TransferStatus = "no_transfer" | "none" | "trying" | "answered" | "failed"

export type IvrEventType = "node_enter" | "dtmf_input" | "transition" | "timeout" | "invalid_input" | "exit"

// --- Core DTOs ---

export interface Call {
  // ... 既存フィールド ...

  // 🆕 新規フィールド
  callDisposition: CallDisposition
  finalAction: FinalAction | null
  transferStatus: TransferStatus
  transferStartedAt: string | null  // ISO8601
  transferAnsweredAt: string | null // ISO8601
  transferEndedAt: string | null    // ISO8601
}

// 🆕 新規 DTO
export interface IvrSessionEvent {
  id: string
  callLogId: string
  sequence: number
  eventType: IvrEventType
  occurredAt: string  // ISO8601
  nodeId: string | null
  dtmfKey: string | null
  transitionId: string | null
  exitAction: string | null
  exitReason: string | null
  metadata: Record<string, unknown> | null
}
```

#### 5.3.2 call-history ページカラム追加

**ファイル**: `virtual-voicebot-frontend/components/call-history-content.tsx`

**追加カラム**:

| カラム名 | 表示内容 | データソース |
|---------|---------|------------|
| **着信応答** | 「許可」「拒否」「無応答」 | `call.callDisposition` |
| **実行アクション** | 「通常着信」「IVR」「ボイスボット」「留守番電話」「ビジー」等 | `call.finalAction` |
| **転送状況** | 「転送成立」「転送失敗」「転送なし」 | `call.transferStatus` |
| **IVR詳細** | 「詳細を見る」リンク（IVR実行時のみ） | `call.actionCode === 'IV'` |

**実装イメージ**:
```tsx
// call-history-content.tsx

const CallsTable = ({ calls }: { calls: CallRecord[] }) => {
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>日時</TableHead>
          <TableHead>発信者</TableHead>
          <TableHead>着信応答</TableHead>  {/* 🆕 */}
          <TableHead>実行アクション</TableHead>  {/* 🆕 */}
          <TableHead>転送状況</TableHead>  {/* 🆕 */}
          <TableHead>通話時間</TableHead>
          <TableHead>IVR詳細</TableHead>  {/* 🆕 */}
        </TableRow>
      </TableHeader>
      <TableBody>
        {calls.map((call) => (
          <TableRow key={call.id}>
            <TableCell>{formatDateTime(call.startedAt)}</TableCell>
            <TableCell>{call.fromName} {call.from}</TableCell>
            <TableCell>{dispositionLabel(call.callDisposition)}</TableCell>  {/* 🆕 */}
            <TableCell>{finalActionLabel(call.finalAction)}</TableCell>  {/* 🆕 */}
            <TableCell>{transferStatusLabel(call.transferStatus)}</TableCell>  {/* 🆕 */}
            <TableCell>{formatDuration(call.durationSec)}</TableCell>
            <TableCell>
              {call.actionCode === 'IV' && (
                <Link href={`/calls/${call.id}/ivr-trace`}>詳細を見る</Link>
              )}
            </TableCell>  {/* 🆕 */}
          </TableRow>
        ))}
      </TableBody>
    </Table>
  )
}

// ラベル変換関数
function dispositionLabel(disposition: CallDisposition): string {
  switch (disposition) {
    case 'allowed': return '許可'
    case 'denied': return '拒否'
    case 'no_answer': return '無応答'
  }
}

function finalActionLabel(action: FinalAction | null): string {
  if (!action) return '-'

  const labels: Record<FinalAction, string> = {
    'normal_call': '通常着信',
    'voicebot': 'ボイスボット',
    'ivr': 'IVR',
    'voicemail': '留守番電話',
    'announcement': 'アナウンス',
    'busy': 'ビジー',
    'rejected': '着信拒否',
    'announcement_deny': 'アナウンス拒否',
  }
  return labels[action]
}

function transferStatusLabel(status: TransferStatus): string {
  switch (status) {
    case 'no_transfer': return '転送なし'
    case 'none': return '-'
    case 'trying': return '転送試行中'
    case 'answered': return '転送成立'
    case 'failed': return '転送失敗'
  }
}
```

#### 5.3.3 IVR 詳細ページ作成

**ファイル**: `virtual-voicebot-frontend/app/calls/[callId]/ivr-trace/page.tsx`

**注記**: 既存の `/calls` 系ルートに統一（§8.3 確認事項で決定）

**レイアウト**:
```
┌─────────────────────────────────────────────┐
│ IVR 経路詳細 - 通話ID: c_20260214_001      │
├─────────────────────────────────────────────┤
│ 発信者: +819012345678 (山田太郎)            │
│ 開始: 2026-02-14 10:00:00                   │
│ 終了: 2026-02-14 10:05:30                   │
│ IVRフロー: メインメニュー                   │
├─────────────────────────────────────────────┤
│ タブ: [タイムライン] [フローチャート]       │
├─────────────────────────────────────────────┤
│ タイムライン:                               │
│                                             │
│ 10:00:05 📢 ノード訪問: ウェルカムメッセージ │
│ 10:00:10 🔢 DTMF 入力: 1                     │
│ 10:00:11 ➡️ 遷移: 営業部                     │
│ 10:00:12 📢 ノード訪問: 営業部メニュー       │
│ 10:00:15 🔢 DTMF 入力: 2                     │
│ 10:00:16 ➡️ 遷移: VR転送                     │
│ 10:00:17 🚪 IVR 終了: 転送開始               │
│                                             │
│ 10:00:18 📞 転送試行開始                     │
│ 10:00:25 ✅ 転送成立                         │
│ 10:05:30 📴 転送終了                         │
└─────────────────────────────────────────────┘
```

**実装イメージ**:
```tsx
// app/calls/[callId]/ivr-trace/page.tsx

export default async function IvrTracePage({ params }: { params: { callId: string } }) {
  const call = await getCall(params.callId)
  const events = await getIvrSessionEvents(params.callId)

  return (
    <div className="p-6">
      <h1 className="text-2xl font-bold">IVR 経路詳細</h1>

      {/* 通話情報 */}
      <CallInfoCard call={call} />

      {/* タブ */}
      <Tabs defaultValue="timeline">
        <TabsList>
          <TabsTrigger value="timeline">タイムライン</TabsTrigger>
          <TabsTrigger value="flowchart">フローチャート</TabsTrigger>
        </TabsList>

        <TabsContent value="timeline">
          <IvrTraceTimeline events={events} call={call} />
        </TabsContent>

        <TabsContent value="flowchart">
          <IvrFlowChart events={events} call={call} />
        </TabsContent>
      </Tabs>
    </div>
  )
}
```

#### 5.3.4 IVR タイムラインコンポーネント

**ファイル**: `virtual-voicebot-frontend/components/ivr-trace-timeline.tsx`

**実装イメージ**:
```tsx
// components/ivr-trace-timeline.tsx

export function IvrTraceTimeline({
  events,
  call
}: {
  events: IvrSessionEvent[],
  call: Call
}) {
  return (
    <div className="space-y-2">
      {events.map((event) => (
        <div key={event.id} className="flex items-start gap-4 border-l-2 border-gray-300 pl-4 py-2">
          <div className="text-sm text-gray-500">
            {formatTime(event.occurredAt)}
          </div>
          <div className="flex-1">
            {renderEventIcon(event.eventType)}
            <span className="ml-2">{renderEventDescription(event)}</span>
          </div>
        </div>
      ))}

      {/* 転送情報 */}
      {call.transferStatus !== 'no_transfer' && (
        <>
          {call.transferStartedAt && (
            <div className="flex items-start gap-4 border-l-2 border-blue-500 pl-4 py-2">
              <div className="text-sm text-gray-500">{formatTime(call.transferStartedAt)}</div>
              <div>📞 転送試行開始</div>
            </div>
          )}

          {call.transferAnsweredAt && (
            <div className="flex items-start gap-4 border-l-2 border-green-500 pl-4 py-2">
              <div className="text-sm text-gray-500">{formatTime(call.transferAnsweredAt)}</div>
              <div>✅ 転送成立</div>
            </div>
          )}

          {call.transferEndedAt && (
            <div className="flex items-start gap-4 border-l-2 border-gray-500 pl-4 py-2">
              <div className="text-sm text-gray-500">{formatTime(call.transferEndedAt)}</div>
              <div>📴 転送終了</div>
            </div>
          )}
        </>
      )}
    </div>
  )
}

function renderEventIcon(eventType: IvrEventType): string {
  switch (eventType) {
    case 'node_enter': return '📢'
    case 'dtmf_input': return '🔢'
    case 'transition': return '➡️'
    case 'timeout': return '⏱️'
    case 'invalid_input': return '❌'
    case 'exit': return '🚪'
  }
}

function renderEventDescription(event: IvrSessionEvent): string {
  switch (event.eventType) {
    case 'node_enter':
      return `ノード訪問: ${getNodeName(event.nodeId)}`
    case 'dtmf_input':
      return `DTMF 入力: ${event.dtmfKey}`
    case 'transition':
      return `遷移: ${getTransitionName(event.transitionId)}`
    case 'timeout':
      return `タイムアウト`
    case 'invalid_input':
      return `無効入力`
    case 'exit':
      return `IVR 終了: ${event.exitReason}`
  }
}
```

#### 5.3.5 IVR フローチャートコンポーネント

**ファイル**: `virtual-voicebot-frontend/components/ivr-flow-chart.tsx`

**実装イメージ**:
```tsx
// components/ivr-flow-chart.tsx

export function IvrFlowChart({
  events,
  call
}: {
  events: IvrSessionEvent[],
  call: Call
}) {
  // IVR フロー構造を取得
  const flow = useIvrFlow(call.ivrFlowId)

  // 訪問したノードを抽出
  const visitedNodes = events
    .filter(e => e.eventType === 'node_enter')
    .map(e => e.nodeId)

  return (
    <div className="p-4">
      <svg width="800" height="600">
        {/* フローチャート描画 */}
        {flow.nodes.map((node) => (
          <NodeBox
            key={node.id}
            node={node}
            visited={visitedNodes.includes(node.id)}
          />
        ))}

        {/* 遷移の矢印 */}
        {flow.transitions.map((transition) => (
          <TransitionArrow key={transition.id} transition={transition} />
        ))}
      </svg>
    </div>
  )
}
```

---

### 5.4 未確定点（Open Questions）

| ID | 質問 | 決定 | 理由 | 決定日 | 決定者 |
|----|------|------|------|--------|--------|
| Q1 | `call_disposition` の値は 'allowed' / 'denied' / 'no_answer' でよいか？ | **A: よい** | 着信応答の区別として十分 | 2026-02-14 | @MasanoriSuda |
| Q2 | `final_action` は文字列でよいか、それとも enum にするか？ | **B: enum（固定値）** | 型安全性とバリデーションのため enum 化 | 2026-02-14 | @MasanoriSuda |
| Q3 | IVR セッションイベントは全て記録するか？ | **A: 全て記録** | デバッグとトラブルシューティングのため全記録 | 2026-02-14 | @MasanoriSuda |
| Q4 | 転送の詳細（B2BUA セッション情報）はどこまで記録するか？ | **A: 基本情報のみ（開始/応答/終了）** | MVP では基本情報のみ。SIP 詳細は将来拡張 | 2026-02-14 | @MasanoriSuda |
| Q5 | Frontend の IVR 詳細ページの UI はタイムライン形式でよいか？ | **C: 両方（タイムライン + フローチャート）** | タイムラインで時系列確認、フローチャートで経路可視化 | 2026-02-14 | @MasanoriSuda |
| Q6 | `final_action` の enum 値は？ | **A: 上記提案（'normal_call' / 'voicebot' / 'ivr' / 'busy' 等）** | 主要アクションをカバー | 2026-02-14 | @MasanoriSuda |

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #173 | STEER-173 | 起票 |
| STEER-173 | contract.md v2.3 | 契約更新 |
| STEER-173 | RD-004-FR-119 | 要件追加 |
| STEER-173 | Backend DD-xxx | 設計追加 |
| STEER-173 | Frontend DD-xxx | 設計追加 |
| STEER-173 | Backend/Frontend UT-xxx | テスト追加 |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] **Backend**:
  - [ ] call_logs テーブル拡張の設計は適切か
  - [ ] ivr_session_events テーブルの設計は適切か
  - [ ] IVR イベント記録処理の設計は明確か
  - [ ] Serversync 拡張の設計は明確か

- [ ] **Frontend**:
  - [ ] Call 型拡張は contract.md と一致しているか
  - [ ] IvrSessionEvent 型は contract.md と一致しているか
  - [ ] call-history ページのカラム追加は要件を満たすか
  - [ ] IVR 詳細ページの UI 設計は明確か

- [ ] **契約**:
  - [ ] contract.md の更新は完全か
  - [ ] 新規 Enum 定義は適切か
  - [ ] 新規 DTO 定義は適切か

- [ ] **整合性**:
  - [ ] Backend-Frontend 間のデータ契約は一致しているか
  - [ ] 既存仕様との整合性があるか
  - [ ] トレーサビリティが維持されているか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] Backend 実装が完了している
- [ ] Frontend 実装が完了している
- [ ] マイグレーションがテスト済み
- [ ] Serversync のテストが完了している
- [ ] Frontend の表示確認が完了している
- [ ] IVR 詳細ページの動作確認が完了している
- [ ] contract.md への反映が完了している
- [ ] RD-004 への反映が完了している

---

## 8. 備考

### 8.1 パフォーマンス考慮事項

| 項目 | リスク | 対策 |
|------|--------|------|
| IVR イベント記録のオーバーヘッド | 中 | 非同期記録、バッチ書き込み検討 |
| ivr_session_events テーブルのサイズ増加 | 低 | パーティショニング検討（将来） |
| Frontend IVR 詳細ページの読み込み速度 | 低 | イベント数が多い場合のページング検討 |
| Serversync のペイロード増加 | 中 | バッチサイズ調整、圧縮検討 |

### 8.2 マイグレーション注意事項

- 既存の `call_logs` レコードの `call_disposition` / `final_action` をどう埋めるか：
  - デフォルト値: `call_disposition = 'allowed'`, `final_action = NULL`
  - 既存レコードは後から手動/スクリプトで補完可能

### 8.5 レビュー指摘への対応（2026-02-14 Codex レビュー）

**第1回レビュー指摘**:

| 指摘 | 対応 |
|------|------|
| ALTER TABLE の SQL 構文エラー | ✅ 修正: 各カラムごとに ALTER TABLE を分離 |
| AR の分類が間違っている | ✅ 修正: AR は 'allowed' に変更（BD-004 に準拠） |
| gen_ulid_uuid7() 関数が存在しない | ✅ 修正: DEFAULT 削除、アプリ側で UUID 生成 |
| Frontend が ivr_session_event をスキップ | ✅ 対応: sync.ts/route.ts に処理追加を明記 |
| final_action の enum 化が不完全 | ✅ 修正: CHECK 制約追加、FinalAction 型定義 |
| ivr_session_events に sequence 一意制約なし | ✅ 修正: UNIQUE (call_log_id, sequence) 制約追加 |
| announcement_reject の表示ラベルが逆 | ✅ 修正: announcement_deny に変更 |

**第2回レビュー指摘**:

| 指摘 | 対応 |
|------|------|
| **重大**: call_log_id ライフサイクル問題 | ✅ 修正: A案（メモリバッファ）採用、§5.1.3 実装詳細記載 |
| **中**: ファイルパス不一致 | ✅ 修正: §4.2, §5.1.4 のパス訂正（sync/worker.rs, db/sync.ts 等） |
| **中**: announcement_deny 不一致 | ✅ 修正: Line 231 フィールド定義テーブルを統一 |
| **軽**: レビュー記録未反映 | ✅ 対応: §3.3 に両レビュー記録追加 |

**第3回レビュー指摘**:

| 指摘 | 対応 |
|------|------|
| **重大**: ivr_session_events の同期方式未確定 | ✅ 修正: §5.1.2 注記追加、§5.1.3 で sync_outbox enqueue 明記、既存パターン踏襲 |
| **中**: 書き込み責務の境界不一致 | ✅ 修正: §4.2 で ivr_events.rs 削除、§5.1.3 で postgres.rs::persist_call_ended に統合 |
| **軽**: 型/関数名が現行コードとズレ | ✅ 修正: §5.1.3 で SessionHandler→SessionCoordinator、finalize_call_log→persist_call_ended に変更 |

### 8.3 確認事項（Codex レビュー）

| 項目 | 現仕様 | 既存実装 | 推奨 | 決定 | 決定日 | 決定者 |
|------|--------|---------|------|------|--------|--------|
| IVR 詳細ページのルートパス | `/call-history/[callId]/ivr-trace` | `/calls` 系（page.tsx） | 既存に合わせて `/calls/[callId]/ivr-trace` | **推奨に決定** | 2026-02-14 | @MasanoriSuda |

**決定**: 既存の `/calls` 系に統一し、`/calls/[callId]/ivr-trace` を使用する。

### 8.4 将来拡張の可能性

- IVR イベントに音声ファイル URL を含める（再生確認用）
- IVR フローチャートの自動生成（Graphviz 等）
- IVR 実行時の統計情報（平均遷移時間、離脱率等）
- 転送先の詳細情報（B2BUA B-leg の SIP URI 等）

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-14 | 初版作成（Draft） | Claude Code (claude-sonnet-4-5-20250929) |
| 2026-02-14 | Codex レビュー指摘対応（SQL構文、enum化、制約追加等） | Claude Code (claude-sonnet-4-5-20250929) |
| 2026-02-14 | ルートパス統一（/calls 系に変更）§8.3 確認事項決定 | Claude Code (claude-sonnet-4-5-20250929) |
| 2026-02-14 | Codex 第2回レビュー指摘対応（call_log_id ライフサイクル、ファイルパス訂正、announcement_deny統一、レビュー記録追加） | Claude Code (claude-sonnet-4-5-20250929) |
| 2026-02-14 | Codex 第3回レビュー指摘対応（同期方式を sync_outbox 統一、postgres.rs に統合、型/関数名修正） | Claude Code (claude-sonnet-4-5-20250929) |
| 2026-02-14 | Codex 第4回レビュー OK判定受領、Status: Approved へ更新、§3.3/§3.4 記入完了 | Claude Code (claude-sonnet-4-5-20250929) |
