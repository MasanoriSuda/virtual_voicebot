# STEER-096: Serversync実装（Backend-Frontend 同期機構）

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-096 |
| タイトル | Serversync実装（Backend-Frontend 同期機構） |
| ステータス | Approved |
| 関連Issue | #96 |
| 優先度 | P0 |
| 作成日 | 2026-02-07 |

---

## 2. ストーリー（Why）

### 2.1 背景

現在、Backend（Raspberry Pi）と Frontend（別 PostgreSQL）は独立した DB を持つが、同期メカニズムが未実装である：

- **通話データ**: Backend で発生した call_logs / recordings が Frontend に届かない
- **設定データ**: Frontend で変更した registered_numbers / routing_rules / ivr_flows が Backend に反映されていない
- **即時性**: STEER-112 では即時 POST（IngestPort）を想定していたが、運用方針として「全て Serversync 経由」に統一する

放置すると：
- Frontend UI に通話履歴・録音が表示されない
- 設定変更が通話処理に反映されない
- データ不整合が発生する

### 2.2 目的

**Transactional Outbox パターン**を用いた同期ワーカー（Serversync）を実装し、Backend と Frontend の DB を一方向または双方向で整合させる。

### 2.3 ユーザーストーリー

```
As a システム管理者
I want Backend と Frontend のデータが自動的に同期される
So that 手動でデータコピーする必要がなく、UI で最新の通話履歴・設定を確認できる

受入条件:
- [ ] Serversync が起動している場合、通話終了後 5分以内に Frontend に call_logs + recordings（メタ + ファイル）が反映される
- [ ] Serversync が停止している場合、outbox にデータが蓄積され、同期は行われない
- [ ] Serversync を再起動すると、蓄積されたデータから順次同期が開始される
- [ ] SIP サーバーと Serversync を独立して起動・停止できる（`systemctl start/stop serversync`）
- [ ] 録音ファイル（mixed.wav + meta.json）が Frontend へ転送され、Backend ローカルから削除される
- [ ] LINE 通知のみ Backend から直接送信される（Serversync 対象外）
- [ ] 同期失敗時は outbox に残り、次回ポーリングで自動リトライされる
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-02 |
| 起票理由 | Backend-Frontend 間のデータ同期機構の必要性 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (Sonnet 4.5) |
| 作成日 | 2026-02-07 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Issue #96 のステアリング作成、全データ同期を Serversync 経由に統一" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| - | - | - | - | |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-07 |
| 承認コメント | Serversync 独立プロセス化、契約のベースライン定義を承認。Frontend との意識合わせは別イシューで継続。 |

### 3.5 実装

| 項目 | 値 |
|------|-----|
| 実装者 | Codex |
| 実装日 | |
| 指示者 | @MasanoriSuda |
| 指示内容 | |
| コードレビュー | |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | |
| マージ日 | |
| マージ先 | BD-001, DD-009 (新規), STEER-112, contract.md |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| docs/steering/STEER-112_sot-reconstruction.md | 修正 | §6.1.3 同期フロー図から即時 POST を削除 |
| docs/contract.md | 修正 | §5.1 POST /api/ingest/call を削除、POST /api/ingest/recording-file 追加 |
| virtual-voicebot-backend/docs/design/basic/BD-001_architecture.md | 修正 | interface/sync/ モジュール説明追加 |
| virtual-voicebot-backend/docs/design/detail/DD-009_sync.md | 新規 | Serversync Worker 詳細設計 |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| **Cargo.toml** | 修正 | `[[bin]]` セクションに serversync 追加 |
| **src/bin/serversync.rs** | 新規 | Serversync 独立バイナリのエントリポイント |
| src/interface/sync/mod.rs | 新規 | Outbox Worker 実装 |
| src/interface/sync/worker.rs | 新規 | ポーリング・送信・リトライロジック |
| src/interface/sync/recording_uploader.rs | 新規 | 録音ファイル multipart POST |
| src/interface/db/postgres.rs | 修正 | SyncOutboxPort 実装追加 |
| src/interface/http/ingest.rs | 削除検討 | 即時 POST 不要になる可能性 |
| ~~virtual-voicebot-frontend/app/api/ingest/sync/route.ts~~ | ~~新規~~ | **別イシューで実装**（本 STEER では仕様定義のみ） |
| ~~virtual-voicebot-frontend/app/api/ingest/recording-file/route.ts~~ | ~~新規~~ | **別イシューで実装**（本 STEER では仕様定義のみ） |

---

## 5. 差分仕様（What / How）

### 5.1 アーキテクチャ方針の明確化

**決定事項（D-01）: 同期経路の統一**

- **原則**: LINE 通知以外の全データ同期は Serversync（Outbox Worker）を経由する
- **廃止**: 即時 POST（IngestPort による通話終了直後の POST /api/ingest/call）
- **対象**: 通話データ（call_logs, recordings）、設定データ（registered_numbers, routing_rules, ivr_flows, schedules, announcements）
- **例外**: LINE 通知のみ Backend から直接送信（既存 LineAdapter を維持）

**決定事項（D-02）: Serversync の独立プロセス化**

- **実装形態**: Serversync は **別バイナリ**として実装（`tokio::spawn` ではなく別 main）
- **独立性**:
  - SIP 送受信プロセス（`virtual-voicebot-backend`）が稼働中でも、Serversync が停止していれば同期は行われない
  - Serversync が停止中は、outbox にデータが蓄積され続ける
  - Serversync を起動すると、蓄積されたデータから順次同期が開始される
- **メリット**:
  - 独立再起動（SIP を止めずに Serversync だけ再起動可能）
  - 障害分離（Serversync クラッシュが SIP に影響しない）
  - リソース分離（CPU/メモリ制限を個別に設定可能）

**プロセス構成**:

```
┌─────────────────────────────┐
│ virtual-voicebot-backend    │  ← SIP/RTP 送受信
│ (src/main.rs)               │
│  - SIP/RTP プロトコル処理   │
│  - 通話制御                 │
│  - Backend DB 書き込み      │
│  - sync_outbox INSERT       │
└─────────────────────────────┘
              ↓ (DB 経由で連携)
┌─────────────────────────────┐
│ serversync                  │  ← 同期ワーカー
│ (src/bin/serversync.rs)     │
│  - sync_outbox ポーリング   │
│  - Frontend への POST       │
│  - 録音ファイル転送         │
└─────────────────────────────┘
```

---

### 5.2 Outbox Worker 設計（DD-009 へマージ）

#### 5.2.1 モジュール構成

```
src/interface/sync/
  ├── mod.rs              # OutboxWorker 公開
  ├── worker.rs           # ポーリング・送信ループ
  ├── recording_uploader.rs  # 録音ファイル multipart POST
  └── config.rs           # 設定（ポーリング間隔・バッチサイズ）
```

#### 5.2.2 OutboxWorker 主要ロジック

```rust
pub struct OutboxWorker {
    outbox_repo: Arc<dyn SyncOutboxPort>,
    http_client: reqwest::Client,
    config: SyncConfig,
}

pub struct SyncConfig {
    pub poll_interval_sec: u64,        // デフォルト 300 (5分)
    pub batch_size: i64,                // デフォルト 100
    pub frontend_base_url: String,      // 環境変数 FRONTEND_BASE_URL
    pub timeout_sec: u64,               // デフォルト 30
}

impl OutboxWorker {
    pub async fn run(&self) {
        loop {
            tokio::time::sleep(Duration::from_secs(self.config.poll_interval_sec)).await;
            if let Err(e) = self.process_batch().await {
                eprintln!("Outbox processing failed: {}", e);
            }
        }
    }

    async fn process_batch(&self) -> Result<(), SyncError> {
        let entries = self.outbox_repo.fetch_pending(self.config.batch_size).await?;
        for entry in entries {
            match self.send_entry(&entry).await {
                Ok(_) => {
                    self.outbox_repo.mark_processed(entry.id, Utc::now()).await?;
                }
                Err(e) => {
                    eprintln!("Failed to send entry {}: {}", entry.id, e);
                    // outbox に残る → 次回リトライ
                }
            }
        }
        Ok(())
    }

    async fn send_entry(&self, entry: &PendingOutboxEntry) -> Result<(), SyncError> {
        match entry.entity_type.as_str() {
            "recording_file" => self.send_recording_file(entry).await,
            _ => self.send_json_payload(entry).await,
        }
    }

    async fn send_json_payload(&self, entry: &PendingOutboxEntry) -> Result<(), SyncError> {
        let url = format!("{}/api/ingest/sync", self.config.frontend_base_url);
        let body = serde_json::json!({
            "entries": [{
                "entityType": entry.entity_type,
                "entityId": entry.entity_id,
                "payload": entry.payload,
                "createdAt": entry.created_at.to_rfc3339(),
            }]
        });
        self.http_client.post(url).json(&body).send().await?
            .error_for_status()?;
        Ok(())
    }

    async fn send_recording_file(&self, entry: &PendingOutboxEntry) -> Result<(), SyncError> {
        // payload から call_log_id, recording_id, file_path を抽出
        // storage/recordings/<callId>/mixed.wav + meta.json を multipart POST
        // 成功後: recordings.upload_status = 'uploaded', recordings.s3_url = レスポンスURL
        // ローカルファイル削除: rm -rf storage/recordings/<callId>/
    }
}
```

#### 5.2.3 録音ファイル転送の詳細

**送信先エンドポイント**: `POST /api/ingest/recording-file`

**リクエスト形式**: `multipart/form-data`

| Part 名 | 型 | 説明 |
|---------|-----|------|
| callLogId | text/plain | call_logs.id (UUID) |
| recordingId | text/plain | recordings.id (UUID) |
| audio | application/octet-stream | mixed.wav バイナリ |
| meta | application/json | meta.json 内容 |

**レスポンス**:
```json
{
  "fileUrl": "https://frontend.example.com/storage/recordings/{callLogId}/mixed.wav"
}
```

**Backend 側の処理**:
1. Frontend から fileUrl を受け取る
2. `UPDATE recordings SET upload_status = 'uploaded', s3_url = {fileUrl} WHERE id = {recordingId}`
3. `rm -rf storage/recordings/{callId}/`（ディレクトリごと削除）
4. `mark_processed(outbox_id, now())`

---

### 5.3 SyncOutboxPort Postgres 実装

**場所**: `src/interface/db/postgres.rs` に追加

```rust
impl SyncOutboxPort for PostgresAdapter {
    fn enqueue(&self, entry: NewOutboxEntry) -> SyncOutboxFuture<i64> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let rec = sqlx::query!(
                r#"
                INSERT INTO sync_outbox (entity_type, entity_id, payload)
                VALUES ($1, $2, $3)
                RETURNING id
                "#,
                entry.entity_type,
                entry.entity_id,
                entry.payload
            )
            .fetch_one(&pool)
            .await
            .map_err(|e| SyncOutboxError::WriteFailed(e.to_string()))?;
            Ok(rec.id)
        })
    }

    fn fetch_pending(&self, limit: i64) -> SyncOutboxFuture<Vec<PendingOutboxEntry>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let rows = sqlx::query!(
                r#"
                SELECT id, entity_type, entity_id, payload, created_at
                FROM sync_outbox
                WHERE processed_at IS NULL
                ORDER BY created_at ASC
                LIMIT $1
                "#,
                limit
            )
            .fetch_all(&pool)
            .await
            .map_err(|e| SyncOutboxError::ReadFailed(e.to_string()))?;

            Ok(rows.into_iter().map(|r| PendingOutboxEntry {
                id: r.id,
                entity_type: r.entity_type,
                entity_id: r.entity_id,
                payload: r.payload,
                created_at: r.created_at,
            }).collect())
        })
    }

    fn mark_processed(&self, id: i64, processed_at: DateTime<Utc>) -> SyncOutboxFuture<()> {
        let pool = self.pool.clone();
        Box::pin(async move {
            sqlx::query!(
                r#"
                UPDATE sync_outbox
                SET processed_at = $1
                WHERE id = $2
                "#,
                processed_at,
                id
            )
            .execute(&pool)
            .await
            .map_err(|e| SyncOutboxError::WriteFailed(e.to_string()))?;
            Ok(())
        })
    }
}
```

---

### 5.4 独立バイナリの実装

**新規ファイル**: `src/bin/serversync.rs`

```rust
// src/bin/serversync.rs

use std::sync::Arc;
use tokio::time::Duration;
use virtual_voicebot_backend::interface::db::PostgresAdapter;
use virtual_voicebot_backend::interface::sync::OutboxWorker;
use virtual_voicebot_backend::shared::config::SyncConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ログ初期化
    tracing_subscriber::fmt::init();

    // 環境変数読み込み
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let frontend_base_url = std::env::var("FRONTEND_BASE_URL")
        .expect("FRONTEND_BASE_URL must be set");
    let poll_interval_sec = std::env::var("SYNC_POLL_INTERVAL_SEC")
        .unwrap_or_else(|_| "300".to_string())
        .parse::<u64>()?;
    let batch_size = std::env::var("SYNC_BATCH_SIZE")
        .unwrap_or_else(|_| "100".to_string())
        .parse::<i64>()?;
    let timeout_sec = std::env::var("SYNC_TIMEOUT_SEC")
        .unwrap_or_else(|_| "30".to_string())
        .parse::<u64>()?;

    // DB 接続
    let postgres_adapter = Arc::new(
        PostgresAdapter::new(&database_url).await?
    );

    // Serversync 設定
    let sync_config = SyncConfig {
        poll_interval_sec,
        batch_size,
        frontend_base_url,
        timeout_sec,
    };

    // Outbox Worker 起動
    let outbox_worker = OutboxWorker::new(postgres_adapter, sync_config);

    tracing::info!("Serversync started (poll_interval={}s, batch_size={})",
        poll_interval_sec, batch_size);

    // メインループ（無限ループ）
    outbox_worker.run().await;

    Ok(())
}
```

**Cargo.toml への追加**:

```toml
[[bin]]
name = "serversync"
path = "src/bin/serversync.rs"
```

**起動コマンド**:

```bash
# SIP サーバー起動
cargo run --release

# Serversync 起動（別ターミナル）
cargo run --bin serversync --release
```

**systemd での運用**:

```ini
# /etc/systemd/system/virtual-voicebot-backend.service
[Unit]
Description=Virtual Voicebot Backend (SIP/RTP)

[Service]
ExecStart=/opt/virtual-voicebot/virtual-voicebot-backend
Restart=always

[Install]
WantedBy=multi-user.target
```

```ini
# /etc/systemd/system/serversync.service
[Unit]
Description=Serversync Worker
After=virtual-voicebot-backend.service

[Service]
ExecStart=/opt/virtual-voicebot/serversync
Restart=always
Environment="DATABASE_URL=postgresql://..."
Environment="FRONTEND_BASE_URL=http://localhost:3000"
Environment="SYNC_POLL_INTERVAL_SEC=300"
Environment="SYNC_BATCH_SIZE=100"

[Install]
WantedBy=multi-user.target
```

---

### 5.5 STEER-112 の修正（§6.1.3 同期フロー図）

**旧フロー（即時 POST あり）**:
```
通話終了
  → Backend DB + sync_outbox INSERT (TX)
  → 即時 POST /api/ingest/call  ← 【削除】
  → Outbox Worker (非同期)
```

**新フロー（Serversync 統一）**:
```
通話終了
  → Backend DB + sync_outbox INSERT (TX)
  → （即時送信なし、5分待つ）
  → Outbox Worker (5分間隔ポーリング)
    ├─ call_log / recording (JSON メタ) → POST /api/ingest/sync
    └─ recording_file (mixed.wav + meta.json) → POST /api/ingest/recording-file
        → 成功時: upload_status='uploaded', rm local
```

**修正箇所**:
- STEER-112 L191-215（設定系 CRUD フロー）: 「即時レスポンス」削除、Outbox 経由のみ
- STEER-112 L218-236（通話系フロー）: `POST /api/ingest/call` → `POST /api/ingest/sync` + `POST /api/ingest/recording-file` に変更

---

### 5.6 contract.md の修正

#### 削除エンドポイント

~~`POST /api/ingest/call`~~ → Outbox Worker 内部で使用、外部仕様から削除

#### 追加エンドポイント

**POST /api/ingest/recording-file**

**リクエスト**:
```
Content-Type: multipart/form-data

Parts:
  - callLogId: string (UUID)
  - recordingId: string (UUID)
  - audio: binary (mixed.wav)
  - meta: JSON (meta.json)
```

**レスポンス**:
```json
{
  "fileUrl": "https://frontend.example.com/storage/recordings/{callLogId}/mixed.wav"
}
```

**修正箇所**:
- contract.md L301: `POST /api/ingest/call` を削除または「Outbox Worker 内部専用」に注記
- contract.md L314-328: `POST /api/ingest/sync` の payload 例に `recording_file` エントリタイプを追加
- contract.md 新セクション: `POST /api/ingest/recording-file` 仕様追加

---

### 5.7 contract.md への追加（Frontend エンドポイント仕様）

> **注**: 以下の仕様を **contract.md §5.1** に追加する。実装は別イシューで行う。

#### POST /api/ingest/sync

**リクエスト**:
```json
{
  "entries": [
    {
      "entityType": "call_log" | "recording" | "registered_number" | ...,
      "entityId": "UUID",
      "payload": { /* エンティティ DTO */ },
      "createdAt": "ISO8601"
    }
  ]
}
```

**レスポンス**:
```json
{ "ok": true }
```

**処理内容**（別イシューで実装）:
- Frontend DB に各エンティティを upsert
- `entityType` に応じたテーブルに保存
- 既存エンティティは上書き（`id` で判定）

#### POST /api/ingest/recording-file

**リクエスト**: multipart/form-data（§5.2.3 参照）

**レスポンス**:
```json
{ "fileUrl": "https://frontend.../storage/recordings/{callLogId}/mixed.wav" }
```

**処理内容**（別イシューで実装）:
- `mixed.wav` + `meta.json` を Frontend ストレージに保存
- 保存先 URL を返却
- Backend はこの URL を `recordings.s3_url` に記録

---

---

**contract.md へのマージ後**:
- 上記 2 エンドポイントは contract.md §5.1 に追加される
- STEER-096 §5.7 は「マージ済み」に更新される
- Frontend 実装時は contract.md を参照する（STEER-096 は参照しない）

**別イシューで検討すべき事項**（contract.md に記載しない実装詳細）:
- Frontend DB スキーマ設計（Backend と異なる独自スキーマ可）
- 録音ファイル保存先（ローカル or S3/MinIO/R2）
- 認証/認可（Backend からの POST をどう検証するか）
- エラーハンドリング（upsert 失敗時の Backend への通知）

---

### 5.8 環境変数の追加

**Backend (.env)**:
```bash
FRONTEND_BASE_URL=http://localhost:3000   # Frontend API URL
SYNC_POLL_INTERVAL_SEC=300                 # 5分
SYNC_BATCH_SIZE=100                        # 1回のポーリングで取得する outbox エントリ数
SYNC_TIMEOUT_SEC=30                        # HTTP タイムアウト
```

**Frontend (.env)**:
```bash
NEXT_PUBLIC_BASE_URL=http://localhost:3000  # 録音配信 URL のベース
```

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #96 | STEER-096 | 起票 |
| STEER-096 | BD-001 (修正) | interface/sync/ モジュール追加 |
| STEER-096 | DD-009 (新規) | Serversync Worker 詳細設計 |
| STEER-096 | STEER-112 (修正) | §6.1.3 同期フロー図更新 |
| STEER-096 | contract.md (修正) | エンドポイント追加・削除 |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] 同期方針（Serversync 統一、LINE 例外）が明確か
- [ ] Outbox Worker のリトライ戦略が適切か
- [ ] 録音ファイル転送の multipart 仕様が実装可能か
- [ ] 既存 STEER-112 / contract.md との整合性があるか
- [ ] Frontend エンドポイントの受入条件が明確か

### 7.2 マージ前チェック（Approved → Merged）

- [ ] Backend Outbox Worker 実装完了
- [ ] SyncOutboxPort Postgres 実装完了
- [ ] 録音ファイル multipart POST 送信実装完了
- [ ] Frontend エンドポイント（スタブ）で疎通確認（200 OK 返却のみ）
- [ ] STEER-112 §6.1.3 同期フロー図の修正完了
- [ ] contract.md §5.1 へ POST /api/ingest/sync, POST /api/ingest/recording-file 追加完了
- [ ] Frontend エンドポイント実装は別イシューに起票済み

---

## 8. 備考

### 8.1 今後の拡張

- **同期間隔の動的変更**: 現在は環境変数固定。将来は `system_settings.sync_interval_sec` で調整可能に
- **S3 等外部ストレージ**: 現在は Frontend ローカル保存。将来は S3/MinIO/R2 に直接アップロード
- **Outbox クリーンアップ**: processed_at が 30 日以上経過したエントリを定期削除（別タスク）
- **リトライ上限**: 現在は無制限リトライ。失敗回数が一定を超えたら DLQ（Dead Letter Queue）に移動

### 8.2 運用観点

- **プロセス管理**:
  - SIP サーバー: `systemctl start virtual-voicebot-backend`
  - Serversync: `systemctl start serversync`
  - 両方起動: `systemctl start virtual-voicebot-backend serversync`
- **独立再起動**:
  - SIP 影響なし: `systemctl restart serversync`（通話処理は継続）
  - Serversync 停止中: outbox にデータ蓄積、再起動後に順次同期
- **監視**: Outbox の pending エントリ数、最古エントリの経過時間を Prometheus でメトリクス化
- **アラート**: pending エントリが 1000 件超、または最古エントリが 1 時間以上経過でアラート
- **ログ**: SIP ログ（`/var/log/voicebot/backend.log`）と Serversync ログ（`/var/log/voicebot/serversync.log`）を分離

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-07 | 初版作成 | Claude Code (Sonnet 4.5) |
| 2026-02-07 | D-02 追加: Serversync 独立プロセス化、`src/bin/serversync.rs` 実装、systemd 設定追加 | Claude Code (Sonnet 4.5) |
| 2026-02-07 | Frontend 実装を別イシューに分離、§5.7 を仕様定義のみに変更、§7.2 マージ前チェック更新 | Claude Code (Sonnet 4.5) |
| 2026-02-07 | Draft → Approved（承認者: @MasanoriSuda） | Claude Code (Sonnet 4.5) |
