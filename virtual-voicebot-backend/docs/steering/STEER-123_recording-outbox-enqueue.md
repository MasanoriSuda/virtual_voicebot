# STEER-123: 録音データ sync_outbox エンキュー実装（Serversync バグフィックス）

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-123 |
| タイトル | 録音データ sync_outbox エンキュー実装（Serversync バグフィックス） |
| ステータス | Draft |
| 関連Issue | #123 |
| 優先度 | P0 |
| 作成日 | 2026-02-07 |

---

## 2. ストーリー（Why）

### 2.1 背景

- STEER-096 で Serversync（Transactional Outbox Pattern）を設計・実装
- 仕様上は通話終了時に `call_log`, `recording`, `recording_file` を sync_outbox にエンキューする前提
- **実装ギャップ**: 現在の実装では `recording` と `recording_file` が sync_outbox にエンキューされていない
- 結果: 録音ファイルが storage に存在しても、sync_outbox に pending エントリがないため Serversync が送信できず、Frontend に録音が表示されない

### 2.2 目的

通話終了時のトランザクショナルライトを実装し、`call_log`, `recording`, `recording_file` の3つを sync_outbox に正しくエンキューする。

### 2.3 ユーザーストーリー

```
As a システム管理者
I want 通話終了後に録音データが自動的に Frontend に同期される
So that Frontend UI で録音を再生・確認できる

受入条件:
- [ ] 通話終了時に sync_outbox に3エントリ（call_log, recording, recording_file）が INSERT される
- [ ] トランザクション保証: すべての INSERT が成功した場合のみコミット
- [ ] Serversync 起動後、録音ファイルが Frontend に送信される
- [ ] Frontend UI で録音が再生できる
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-07 |
| 起票理由 | STEER-096 実装時に recording の outbox エンキューが漏れ、Frontend に録音が表示されないバグが発覚 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (Sonnet 4.5) |
| 作成日 | 2026-02-07 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Issue #123 のバグフィックス方針を整理し、STEER-123 を作成" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| - | - | - | - | |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | |
| 承認日 | |
| 承認コメント | |

### 3.5 実装

| 項目 | 値 |
|------|-----|
| 実装者 | Codex (Backend) |
| 実装日 | |
| 指示者 | @MasanoriSuda |
| 指示内容 | |
| コードレビュー | |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | |
| マージ日 | |
| マージ先 | - |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| docs/steering/STEER-096_serversync.md | 参照のみ | バグの根拠となる仕様を確認（修正不要） |
| docs/design/detail/DD-009_sync.md | 参照のみ | Outbox Worker 仕様を確認（修正不要） |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| **src/protocol/session/coordinator.rs** | 修正 | 通話終了時の DB 永続化 + outbox エンキューロジック追加 |
| **src/protocol/session/recording_manager.rs** | 参照のみ | 録音ファイルパス取得ロジック確認 |
| **src/interface/db/postgres.rs** | 参照のみ | 既存 SyncOutboxPort::enqueue() 実装確認（修正不要） |
| src/interface/sync/worker.rs | 参照のみ | Outbox Worker 動作確認（修正不要） |

---

## 5. 差分仕様（What / How）

### 5.1 通話終了時の DB 永続化 + Outbox エンキューロジック

**参照**: STEER-096 § 5.1, § 5.2

**現状の問題**:
- 通話終了時に `coordinator.rs` の `send_ingest()` メソッドが HTTP POST を試みる (line 274)
- しかし `INGEST_CALL_URL` は無効化され `None` 扱いのため（`mod.rs` line 103）、実際には何も送信されない
- Backend DB への永続化（`call_logs`, `recordings` テーブルへの INSERT）も行われていない
- `sync_outbox` への INSERT も行われていない
- 結果: 通話データが Backend DB にも Frontend DB にも保存されず、録音ファイルのみが storage に残る状態

**修正方針**: トランザクショナルライト（DB 永続化 + Outbox エンキュー）

通話終了イベント（`SessionCoordinator::on_terminated()` または新規メソッド）で以下を実行：

```rust
// 注: SessionCoordinator に CallLogPort を注入（self.call_log_port: Arc<dyn CallLogPort>）
async fn on_call_ended(
    &self,
    call_status: &str,        // ← 引数で受け取る（'ended', 'error', 'failed'。failed は error に正規化）
    recording_path: Option<PathBuf>,
) -> Result<(), SessionError> {
    // call_status の検証（DB 制約: 'ringing' | 'in_call' | 'ended' | 'error'）
    // 注: "failed" は内部で "error" に正規化される
    let (status, end_reason) = match call_status {
        "ended" => ("ended", "normal"),
        "failed" | "error" => ("error", "error"),
        _ => return Err(SessionError::InvalidStatus(call_status.to_string())),
    };

    // 0. 通話終了データの組み立て
    let call_log_id = Uuid::now_v7();  // 呼び出し側で UUID 生成
    let started_wall = self.started_wall.unwrap_or_else(SystemTime::now);  // panic 回避
    let started_at: DateTime<Utc> = started_wall.into();
    let ended_at = Utc::now();
    let duration_sec = self.started_at
        .map(|s| s.elapsed().as_secs().min(i32::MAX as u64) as i32);

    // 発信者番号を E.164 形式で抽出（DB 制約: ^\+[1-9][0-9]{1,14}$ に準拠）
    let caller_number = extract_e164_caller_number(&self.from_uri);

    // 録音データの組み立て（存在する場合）
    let recording = if let Some(rec_path) = recording_path {
        if let Ok(meta) = tokio::fs::metadata(&rec_path).await {
            if meta.is_file() {
                Some(EndedRecording {
                    id: Uuid::now_v7(),
                    file_path: rec_path.to_string_lossy().to_string(),
                    duration_sec,
                    format: "wav".to_string(),
                    file_size_bytes: Some(meta.len() as i64),
                    started_at,
                    ended_at: Some(ended_at),
                })
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    // 1. EndedCallLog 構造体を作成
    let ended_call = EndedCallLog {
        id: call_log_id,
        started_at,
        ended_at,
        duration_sec,
        external_call_id: call_log_id.to_string(),  // 一時的に UUID を使用
        sip_call_id: self.call_id.as_str().to_string(),
        caller_number,
        caller_category: "unknown".to_string(),
        action_code: "IV".to_string(),
        ivr_flow_id: None,
        answered_at: None,
        end_reason: end_reason.to_string(),
        status: status.to_string(),
        recording,
    };

    // 2. CallLogPort 経由で永続化（TX は PostgresAdapter 内部で完結）
    self.call_log_port.persist_call_ended(ended_call).await?;

    Ok(())
}
```

**重要事項**:
- **トランザクション保証**: call_log_index, call_logs, recordings, sync_outbox（×3）をすべて同一 Transaction 内で実行（PostgresAdapter 内部で完結）
- **Port 経由永続化**: `CallLogPort::persist_call_ended(EndedCallLog)` を呼び出し、TX 管理は Port 実装に委譲
- **Port 注入**: SessionCoordinator に `Arc<dyn CallLogPort>` を注入（境界侵食回避、AGENTS.md § 3 準拠）
- **配線場所**: `spawn_call()` または `Session::spawn()` 引数に CallLogPort を追加（writing.rs）
- **EndedCallLog 構造体**: 通話終了データを構造化して Port に渡す
- **実スキーマ準拠**: call_logs は external_call_id, sip_call_id, duration_sec, end_reason, status など実カラムを使用
- **caller_number 抽出**: `extract_user_from_to()` で SIP URI から抽出、`^\+[1-9][0-9]{1,14}$` チェック、不一致は NULL
- **status 引数**: `on_call_ended(call_status: &str, ...)` で受け取り、match で検証（'ended', 'error', 'failed'。failed は error に正規化）
- **end_reason 連動**: call_status に応じて end_reason を切り替え（'normal' / 'error'）
- **panic 回避**: `started_wall.unwrap()` → `unwrap_or_else(SystemTime::now)`、duration_sec も Option で安全に扱う
- **recordings スキーマ**: recording_type, sequence_number, upload_status, format など実カラムを使用
- **payload キー**: Frontend 正規化キーに準拠（durationSec, callerCategory, actionCode, endReason, status 等）
- **entity_id の型**: `Uuid` 型（`String` ではない）
- **entity_type の明示**: `"call_log"`, `"recording"`, `"recording_file"` を正確に設定
- **ファイル存在確認**: `recording_path` が存在する場合のみ `recording_file` をエンキュー

---

### 5.2 CallLogPort 定義と配線

**実装方針**: SessionCoordinator に `Arc<dyn CallLogPort>` を注入（境界侵食回避）、TX は PostgresAdapter 内部で完結

**EndedRecording 構造体** (shared/ports/call_log_port.rs):
```rust
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct EndedRecording {
    pub id: Uuid,
    pub file_path: String,
    pub duration_sec: Option<i32>,
    pub format: String,
    pub file_size_bytes: Option<i64>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}
```

**EndedCallLog 構造体** (shared/ports/call_log_port.rs):
```rust
#[derive(Clone, Debug)]
pub struct EndedCallLog {
    pub id: Uuid,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub duration_sec: Option<i32>,
    pub external_call_id: String,
    pub sip_call_id: String,
    pub caller_number: Option<String>,
    pub caller_category: String,
    pub action_code: String,
    pub ivr_flow_id: Option<Uuid>,
    pub answered_at: Option<DateTime<Utc>>,
    pub end_reason: String,
    pub status: String,
    pub recording: Option<EndedRecording>,
}
```

**Port 定義** (shared/ports/call_log_port.rs):
```rust
use thiserror::Error;
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Error)]
pub enum CallLogPortError {
    #[error("write failed: {0}")]
    WriteFailed(String),
}

pub type CallLogFuture<T> = Pin<Box<dyn Future<Output = Result<T, CallLogPortError>> + Send>>;

pub trait CallLogPort: Send + Sync {
    fn persist_call_ended(&self, call_log: EndedCallLog) -> CallLogFuture<()>;
}
```

**PostgresAdapter への実装**:
```rust
impl CallLogPort for PostgresAdapter {
    fn persist_call_ended(&self, call_log: EndedCallLog) -> CallLogFuture<()> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let mut tx = pool.begin().await.map_err(|e| CallLogPortError::WriteFailed(e.to_string()))?;

            // 1. call_log_index に INSERT（ID は呼び出し側で生成済み）
            sqlx::query!("INSERT INTO call_log_index (id, started_at) VALUES ($1, $2)", call_log.id, call_log.started_at)
                .execute(&mut *tx).await.map_err(|e| CallLogPortError::WriteFailed(e.to_string()))?;

            // 2. call_logs に INSERT
            sqlx::query!(
                "INSERT INTO call_logs (id, started_at, external_call_id, sip_call_id, caller_number,
                 caller_category, action_code, ivr_flow_id, answered_at, ended_at, duration_sec, end_reason, status)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
                call_log.id, call_log.started_at, call_log.external_call_id, call_log.sip_call_id,
                call_log.caller_number, call_log.caller_category, call_log.action_code,
                call_log.ivr_flow_id, call_log.answered_at, call_log.ended_at, call_log.duration_sec,
                call_log.end_reason, call_log.status
            )
            .execute(&mut *tx).await.map_err(|e| CallLogPortError::WriteFailed(e.to_string()))?;

            // 3. recordings に INSERT（存在する場合）
            if let Some(recording) = &call_log.recording {
                sqlx::query!(
                    "INSERT INTO recordings (id, call_log_id, recording_type, sequence_number, file_path,
                     upload_status, duration_sec, format, file_size_bytes, started_at, ended_at)
                     VALUES ($1, $2, 'full_call', 1, $3, 'local_only', $4, $5, $6, $7, $8)",
                    recording.id, call_log.id, recording.file_path, recording.duration_sec,
                    recording.format, recording.file_size_bytes, recording.started_at, recording.ended_at
                )
                .execute(&mut *tx).await.map_err(|e| CallLogPortError::WriteFailed(e.to_string()))?;

                // 4-6. sync_outbox × 3 に INSERT（call_log, recording, recording_file）
                // （省略、同様のパターン）
            }

            tx.commit().await.map_err(|e| CallLogPortError::WriteFailed(e.to_string()))?;
            Ok(())
        })
    }
}
```

**配線場所**: `spawn_call()` または `Session::spawn()` 引数に追加（writing.rs）

**配線例**:
```rust
// writing.rs
pub fn spawn_call(
    // ... 既存引数 ...
    call_log_port: Arc<dyn CallLogPort>,  // ← 追加
) -> SessionHandle {
    let handle = Session::spawn(
        // ... 既存引数 ...
        call_log_port,  // ← 追加
    );
    handle
}

// coordinator.rs
pub struct SessionCoordinator {
    // ... 既存フィールド ...
    call_log_port: Arc<dyn CallLogPort>,  // ← 追加
}

// SessionCoordinator で利用
let ended_call = EndedCallLog { /* ... */ };
self.call_log_port.persist_call_ended(ended_call).await?;
```

**実装箇所**: `PostgresAdapter::persist_call_ended()` 内部（TX は内部で完結）

**SQL テンプレート**（PostgresAdapter 内部で実行）:
```sql
-- 1. call_log_index（FK 先を先に INSERT）
INSERT INTO call_log_index (id, started_at)
VALUES ($1, $2);

-- 2. call_logs（パーティションテーブル、実スキーマ + status 列準拠）
INSERT INTO call_logs (
    id, started_at, external_call_id, sip_call_id, caller_number, caller_category,
    action_code, ivr_flow_id, answered_at, ended_at, duration_sec, end_reason, status
) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13);

-- 3. recordings（録音がある場合、実スキーマ準拠）
INSERT INTO recordings (
    id, call_log_id, recording_type, sequence_number, file_path, s3_url,
    upload_status, duration_sec, format, file_size_bytes, started_at, ended_at
) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12);

-- 4-6. sync_outbox × 3（call_log, recording, recording_file）
INSERT INTO sync_outbox (entity_type, entity_id, payload)
VALUES ('call_log', $1, $2);

INSERT INTO sync_outbox (entity_type, entity_id, payload)
VALUES ('recording', $1, $2);

INSERT INTO sync_outbox (entity_type, entity_id, payload)
VALUES ('recording_file', $1, $2);
```

**トレース**:
- → UT: UT-XXX-TC-01（通話終了時の一括永続化テスト）

---

### 5.3 テストケース

#### UT-XXX-TC-01: 通話終了時の outbox エンキューテスト

**対象**: `SessionCoordinator::on_call_ended()` または新規メソッド

**目的**: 通話終了時に sync_outbox に3エントリが正しく INSERT されることを検証

**入力**:
- SessionCoordinator（通話終了状態）
- call_status: `"ended"` / `"error"` / `"failed"`（failed は error に正規化）
- recording_path: `Some(PathBuf::from("storage/recordings/{callId}/mixed.wav"))`
- CallLogPort: `Arc<dyn CallLogPort>`

**期待結果**:
1. `call_log_index` テーブルに1レコード INSERT
2. `call_logs` テーブルに1レコード INSERT
3. `recordings` テーブルに1レコード INSERT
4. `sync_outbox` テーブルに3レコード INSERT:
   - entity_type='call_log', entity_id={call_log_id}
   - entity_type='recording', entity_id={recording_id}
   - entity_type='recording_file', entity_id={recording_id}
5. すべての INSERT が同一トランザクション内で完了

**実装例**（Rust + sqlx）:

```rust
#[tokio::test]
async fn test_on_call_ended_enqueues_all_entities() {
    // Setup
    let db_adapter = Arc::new(setup_test_db().await);
    let call_log_port: Arc<dyn CallLogPort> = db_adapter.clone();  // PostgresAdapter は CallLogPort を実装

    let coordinator = SessionCoordinator::new_for_test(
        CallId::new("test-call-id").unwrap(),
        "sip:+81312345678@example.com".to_string(),
        "sip:+81987654321@example.com".to_string(),
        call_log_port.clone(),
        // ... 他の引数
    );

    let recording_path = Some(PathBuf::from("storage/recordings/test-call/mixed.wav"));

    // Execute（persist_call_ended() が内部で TX 完結）
    coordinator.on_call_ended("ended", recording_path).await.unwrap();

    // Verify（Port 経由で永続化済み、pool 経由で検証）
    let pool = db_adapter.pool();  // テスト用 accessor

    // Verify call_log_index
    let call_log_index = sqlx::query!("SELECT * FROM call_log_index")
        .fetch_all(pool).await.unwrap();
    assert_eq!(call_log_index.len(), 1);

    // Verify call_logs
    let call_logs = sqlx::query!("SELECT * FROM call_logs")
        .fetch_all(pool).await.unwrap();
    assert_eq!(call_logs.len(), 1);
    assert_eq!(call_logs[0].status, "ended");
    assert_eq!(call_logs[0].end_reason, "normal");

    // Verify recordings
    let recordings = sqlx::query!("SELECT * FROM recordings")
        .fetch_all(pool).await.unwrap();
    assert_eq!(recordings.len(), 1);

    // Verify sync_outbox (3 entries)
    let outbox_entries = sqlx::query!("SELECT * FROM sync_outbox ORDER BY id")
        .fetch_all(pool).await.unwrap();
    assert_eq!(outbox_entries.len(), 3);
    assert_eq!(outbox_entries[0].entity_type, "call_log");
    assert_eq!(outbox_entries[1].entity_type, "recording");
    assert_eq!(outbox_entries[2].entity_type, "recording_file");

    // Cleanup: テスト終了後に TRUNCATE または fixture でクリーンアップ
    sqlx::query!("TRUNCATE call_logs, call_log_index, recordings, sync_outbox CASCADE")
        .execute(pool).await.unwrap();
}
```

**トレース**:
- ← DD: DD-009-FN-XX（on_call_ended 実装）

---

#### ST-XXX-TC-01: E2E 録音同期テスト

**対象**: Backend → Serversync → Frontend 全体フロー

**目的**: 通話終了後、録音が Frontend に同期されることを検証

**手順**:
1. Backend で通話を終了（録音あり）
2. sync_outbox に3エントリが INSERT されることを確認
3. Serversync を起動（または手動トリガー）
4. Serversync が sync_outbox から3エントリを取得
5. `POST /api/ingest/sync` (call_log, recording) が送信される
6. `POST /api/ingest/recording-file` (multipart) が送信される
7. Frontend DB に call_log, recording が保存される
8. Frontend storage に mixed.wav が保存される
9. Frontend UI で録音が再生できる

**期待結果**: すべての手順が正常に完了し、Frontend UI で録音が表示・再生される

**トレース**:
- ← RD: RD-004（Backend 要件）
- ← RD: RD-005（Frontend 要件）
- ← STEER-096, STEER-116（Serversync, Frontend Ingest API）

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #123 | STEER-123 | 起票 |
| STEER-096 | STEER-123 | 仕様参照（実装ギャップを埋める） |
| STEER-123 | DD-009-FN-XX | 詳細設計追加 |
| DD-009-FN-XX | UT-XXX-TC-01 | 単体テスト |
| STEER-123 | ST-XXX-TC-01 | E2E システムテスト |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] トランザクショナルライトの実装方針が明確か
- [ ] entity_type の設定が STEER-096 § 5.2 と一致しているか
- [ ] payload の構造が Serversync Worker と互換性があるか
- [ ] テストケースが網羅的か（通話終了、録音あり/なし）
- [ ] 既存の STEER-096 仕様と整合性があるか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] `SessionCoordinator::on_call_ended()` または新規メソッド実装完了
- [ ] DB Port を SessionCoordinator に注入、または新規 repository メソッドへ委譲
- [ ] トランザクション内で call_log_index, call_logs, recordings, sync_outbox × 3 を一括 INSERT する実装完了
- [ ] 実スキーマに準拠（external_call_id, sip_call_id, duration_sec, recording_type, sequence_number 等）
- [ ] payload キーが Frontend 正規化キーに準拠（durationSec, callerCategory, actionCode 等）
- [ ] UT-XXX-TC-01（単体テスト: 一括永続化の検証）PASS
- [ ] ST-XXX-TC-01（E2E テスト: Backend → Serversync → Frontend）PASS
- [ ] Serversync が sync_outbox から3エントリを取得・送信できる
- [ ] Frontend UI で録音が再生できる

---

## 8. 備考

### 8.1 STEER-096 との関係

- STEER-096 は Serversync の **設計仕様**
- STEER-123 は STEER-096 の **実装ギャップを埋める**バグフィックス
- STEER-096 § 5.1, § 5.2 に録音の outbox エンキューが明示されているため、実装漏れが原因

### 8.2 一時的な回避策（Issue #123 記載）

バグフィックス完了まで、手動で録音を注入可能：

```bash
# call_log を送信
curl -X POST http://localhost:3000/api/ingest/sync \
  -H "Content-Type: application/json" \
  -d '{
    "entries": [{
      "entityType": "call_log",
      "entityId": "01942bca-1234-7890-abcd-ef1234567890",
      "payload": {
        "id": "01942bca-1234-7890-abcd-ef1234567890",
        "externalCallId": "01942bca-1234-7890-abcd-ef1234567890",
        "sipCallId": "sip-call-id-12345@example.com",
        "callerNumber": "+81312345678",
        "callerCategory": "unknown",
        "actionCode": "IV",
        "startedAt": "2026-02-07T10:00:00Z",
        "endedAt": "2026-02-07T10:05:30Z",
        "durationSec": 330,
        "endReason": "normal"
      },
      "createdAt": "2026-02-07T10:05:30Z"
    }]
  }'

# recording を送信
curl -X POST http://localhost:3000/api/ingest/sync \
  -H "Content-Type: application/json" \
  -d '{
    "entries": [{
      "entityType": "recording",
      "entityId": "01942bca-5678-7890-abcd-ef1234567890",
      "payload": {
        "id": "01942bca-5678-7890-abcd-ef1234567890",
        "callLogId": "01942bca-1234-7890-abcd-ef1234567890",
        "recordingType": "full_call",
        "sequenceNumber": 1,
        "uploadStatus": "local_only",
        "durationSec": 330,
        "format": "wav"
      },
      "createdAt": "2026-02-07T10:05:30Z"
    }]
  }'

# recording ファイルを送信
curl -X POST http://localhost:3000/api/ingest/recording-file \
  -F "callLogId=01942bca-1234-7890-abcd-ef1234567890" \
  -F "recordingId=01942bca-5678-7890-abcd-ef1234567890" \
  -F "audio=@storage/recordings/01942bca-1234-7890-abcd-ef1234567890/mixed.wav" \
  -F "meta=@storage/recordings/01942bca-1234-7890-abcd-ef1234567890/meta.json"
```

### 8.3 将来の拡張

- **録音分割対応**: 現在は1通話1録音ファイル想定。将来は複数録音（inbound/outbound 分離）も考慮
- **S3 直接アップロード**: 現在は Frontend ローカル保存。将来は S3/MinIO に直接アップロード
- **リトライ戦略**: Serversync の自動リトライに依存。失敗回数が一定を超えたら DLQ（Dead Letter Queue）に移動

### 8.4 関連イシュー（本イシューでは対応しない残課題）

#### Issue #124: ivrFlowId 欠落の問題
- **問題**: ivrFlowId が欠落する可能性がある（データ欠損系の不具合）
- **優先度**: 中～高
- **影響範囲**: IVR フロー情報を使う画面/ロジックで値が落ちる（通話同期機能自体は停止しない）
- **対応方針**: 本イシュー（#123）では対応せず、#124 で個別対応
- **備考**: 致命的ではないが、早期対処が必要

#### Issue #125: callId の名前不一致
- **問題**: ディレクトリ名（UUID 形式の `callLogId`）と、メタデータ（SIP Call-ID 形式の `callId`）が異なり、可読性上の混乱がある
- **優先度**: 低
- **影響範囲**: 実害はないが、可読性・保守性に影響
- **対応方針**: 本イシュー（#123）では対応せず、#125 で個別対応
- **検討案**:
  - `callId` を `sipCallId` に改名
  - または `callLogId` も併記
  - またはドキュメントに「meta.callId は SIP Call-ID、保存パスは callLogId」と明記

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-07 | 初版作成 | Claude Code (Sonnet 4.5) |
| 2026-02-07 | Codex レビュー指摘対応（1回目）: § 4.2 対象モジュール修正（coordinator.rs等）、§ 5.1 現状分析追加（DB永続化未実装）、§ 5.2/5.3 削除（既存 SyncOutboxPort 利用）、entity_id を Uuid に修正、§ 8.2 curl 例に createdAt 追加 | Claude Code (Sonnet 4.5) |
| 2026-02-07 | Codex レビュー指摘対応（2回目）: § 5.1 トランザクション境界明確化（pool.begin → tx.commit）、§ 5.2 を「既存実装利用」から「TX内SQL直接実行」に変更、§ 7.2 チェックリスト修正（SessionCoordinator + TX一括永続化）、call_logs.status を 'ended' に修正 | Claude Code (Sonnet 4.5) |
| 2026-02-07 | Codex レビュー指摘対応（3回目）: § 5.1 実スキーマ準拠（call_log_index 追加、external_call_id/sip_call_id/duration_sec/recording_type/sequence_number 等）、DB Port 注入方針明記、payload キーを Frontend 正規化キー準拠（durationSec, callerCategory 等）、§ 8.2 curl 例修正 | Claude Code (Sonnet 4.5) |
| 2026-02-07 | Codex レビュー指摘対応（4回目）: § 5.1 caller_number に extract_user_from_to() 使用（E.164 チェック）、status 列 INSERT 追加、unwrap() panic 回避（unwrap_or_else）、§ 5.2 DB Port 配線詳細追加（Arc<PostgresAdapter> + spawn_call 引数）、§ 5.3 UT 例を SessionCoordinator ベースに修正 | Claude Code (Sonnet 4.5) |
| 2026-02-07 | Codex レビュー指摘対応（5回目）: § 5.1 is_terminated() → status 引数で受け取る方針、caller_number を ^\+[1-9][0-9]{1,14}$ チェック、§ 5.2 Arc<PostgresAdapter> → Arc<dyn CallLogPort> に変更（境界侵食回避）、SQL テンプレートに status 列追加、§ 5.3 UT 例を Port 経由検証に修正 | Claude Code (Sonnet 4.5) |
| 2026-02-07 | Codex レビュー指摘対応（6回目）: § 5.1 call_status を match で検証（"ended" / "error" のみ許可）、end_reason を status に応じて切り替え（'normal' / 'error'）、§ 5.3 UT 例の tx.rollback() コメント削除（cleanup は TRUNCATE/fixture で行う） | Claude Code (Sonnet 4.5) |
| 2026-02-07 | 実装準拠修正（Refs #123）: § 5.1 begin_transaction() 削除、EndedCallLog 構造体作成 + persist_call_ended() 呼び出しに変更、§ 5.2 CallLogPort 定義を begin_transaction → persist_call_ended に変更（TX は PostgresAdapter 内部で完結）、§ 5.3 UT 例を pool 経由検証 + TRUNCATE cleanup に修正 | Claude Code (Sonnet 4.5) |
| 2026-02-07 | Codex レビュー対応（実装整合）: § 5.1 EndedCallLog/EndedRecording を実装準拠フィールドに更新（id, external_call_id, caller_category, action_code, ivr_flow_id, answered_at, DateTime<Utc> 等）、"failed" → "error" 正規化を明記、UUID 採番を呼び出し側で実施、§ 5.2 CallLogPortError::WriteFailed に修正 | Claude Code (Sonnet 4.5) |
| 2026-02-07 | Codex 再確認対応: call_status 説明文を統一（'ended', 'error', 'failed'。failed は error に正規化）、テストケース入力説明も統一 | Claude Code (Sonnet 4.5) |
| 2026-02-07 | § 8.4 関連イシュー追加: #124（ivrFlowId 欠落）、#125（callId 名前不一致）を残課題として記録 | Claude Code (Sonnet 4.5) |
