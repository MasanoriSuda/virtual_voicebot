<!-- SOURCE_OF_TRUTH: 着信ルーティングDB設計 -->
# BD-004_call-routing-db

> 着信ルーティング・IVRフロー管理のデータベース設計

| 項目 | 値 |
|------|-----|
| ID | BD-004 |
| ステータス | Approved |
| 作成日 | 2026-02-02 |
| 最終更新 | 2026-02-08 |
| 関連Issue | #92, #110, #138 |
| 対応RD | RD-004 |
| 対応STEER | STEER-110 |
| 対応IT | - |

---

## 1. 概要

### 1.1 目的

着信時の電話番号振り分け、IVRフロー制御、迷惑電話対策に必要なデータベース構造を定義する。

### 1.2 スコープ

- 発信者分類（4カテゴリ）のデータ構造
- アクションコード体系
- IVRフロー（木構造）のデータモデル
- 通話履歴・録音メタデータの永続化（STEER-110 追加）
- Frontend 同期基盤（STEER-110 追加）
- システム設定テーブル（STEER-110 追加）
- Rust側の状態管理方針

---

## 2. 発信者分類

### 2.1 4カテゴリ分類

着信時に発信者番号（Caller ID）を以下の4カテゴリに分類し、それぞれ異なるアクションを設定可能とする。

| カテゴリ | 説明 | 判定条件 | デフォルトアクション |
|---------|------|----------|---------------------|
| spam | 迷惑電話DB登録番号 | `spam_numbers` テーブルに一致 | RJ（即時拒否） |
| registered | 登録済み電話番号 | `registered_numbers` テーブルに一致 | VR（ボイスボット録音あり） |
| unknown | 未登録番号 | 上記いずれにも該当しない | IV（IVR） |
| anonymous | 非通知 | Caller ID が空/非通知 | IV（IVR） |

> **Note**: デフォルトアクションは管理画面から変更可能。上記は初期値。

### 2.2 分類フロー

```
着信
  │
  ▼
┌─────────────────┐
│ Caller ID 取得   │
└────────┬────────┘
         │
         ▼
    ┌─────────┐
    │ 非通知？ │─── Yes ──▶ anonymous
    └────┬────┘
         │ No
         ▼
┌─────────────────┐
│ spam_numbers    │─── Hit ──▶ spam
│ テーブル検索     │
└────────┬────────┘
         │ Miss
         ▼
┌─────────────────────┐
│ registered_numbers  │─── Hit ──▶ registered
│ テーブル検索         │
└────────┬────────────┘
         │ Miss
         ▼
      unknown
```

---

## 3. アクションコード体系

### 3.1 設計方針

- 2文字の英字コードで処理を表現
- 録音有無は別コードでペア化（AN/AR、VB/VR）
- IVR関連は `I` プレフィックスで統一

### 3.2 基本アクションコード

| コード | 名称 | 説明 | 録音 |
|--------|------|------|------|
| VB | Voicebot | AI応答（ボイスボット）開始（録音なし） | ❌ |
| VR | Voicebot+Record | AI応答（ボイスボット）開始（録音あり） | ✅ |
| NR | No Response | 応答なし（コール音のみ） | - |
| RJ | Reject | 即時拒否（話中音） | - |
| BZ | Busy | 話中応答 | - |
| AN | Announce | アナウンス再生（録音なし） | ❌ |
| AR | Announce+Record | アナウンス再生（録音あり） | ✅ |
| VM | Voicemail | 留守番電話 | ✅ |
| IV | IVR | IVRフローへ移行 | - |

### 3.3 録音オプション対応表

| 機能 | 録音なし | 録音あり |
|------|---------|---------|
| ボイスボット（AI応答） | VB | VR |
| アナウンス再生 | AN | AR |
| 留守番電話 | - | VM（常に録音） |

### 3.4 IVR内アクションコード

| コード | 名称 | 説明 |
|--------|------|------|
| IA | IVR Announce | IVR内アナウンス再生 |
| IR | IVR Record | IVR内録音開始 |
| IK | IVR Keypad | DTMF入力待ち |
| IW | IVR Wait | 無音待機（タイムアウト付き） |
| IF | IVR Forward | 転送 |
| IT | IVR Transfer | 内線転送 |
| IB | IVR Voicebot | IVR内からボイスボットへ移行 |
| IE | IVR Exit | IVR終了（切断） |

### 3.5 コードチェイニング

アクションはチェイン可能。例：

```
AN → IV    # アナウンス再生後、IVRへ
IV → VR    # IVR完了後、ボイスボット（録音あり）
AR → IE    # 録音付きアナウンス後、切断
IV → VB    # IVR完了後、ボイスボット（録音なし）
```

---

## 4. データベース設計

### 4.1 設計方針（STEER-110 反映）

| 方針 | 説明 |
|------|------|
| DB エンジン | PostgreSQL 16+ |
| ID 戦略 | UUID v7（Rust 側で生成） |
| 電話番号 | E.164 形式、CHECK 制約で強制 |
| 削除戦略 | 設定系のみ論理削除（`deleted_at`）、通話系は TTL バッチ削除 |
| 同期方式 | Transactional Outbox + `synced_at` 併用 |
| パーティション | call_logs を月次レンジパーティション |
| IVR ルートノード | `root_node_id` 廃止。`parent_id IS NULL` = ルート |

### 4.2 ER図（STEER-110 全決定反映済み）

```
┌─── ルーティング系 ──────────────────────────────────────────────────┐
│                                                                      │
│  ┌───────────────────┐     ┌───────────────────────┐               │
│  │   spam_numbers    │     │  registered_numbers   │               │
│  ├───────────────────┤     ├───────────────────────┤               │
│  │ id (PK, UUID v7)  │     │ id (PK, UUID v7)      │               │
│  │ phone_number (UQ※)│     │ phone_number (UQ※)    │               │
│  │ reason            │     │ name                  │               │
│  │ source            │     │ category              │               │
│  │ deleted_at        │     │ action_code           │               │
│  │ created_at        │     │ ivr_flow_id (FK)──────┼──┐            │
│  │ updated_at        │     │ recording_enabled     │  │            │
│  └───────────────────┘     │ announce_enabled      │  │            │
│  ※ WHERE deleted_at IS NULL│ group_id (UUID※※)    │  │            │
│                             │ group_name            │  │            │
│                             │ notes, version        │  │            │
│                             │ deleted_at            │  │            │
│                             │ created_at, updated_at│  │            │
│                             └───────────────────────┘  │            │
│                             ※※ FK なし（削除済み対応） │            │
│                                  ▲                     │            │
│                                  │ (group_id)          │            │
│  ┌───────────────────────┐       │                     │            │
│  │  call_action_rules    │───────┘                     │            │
│  │    (#138 追加)        │                             │            │
│  ├───────────────────────┤                             │            │
│  │ id (PK, UUID v7)      │                             │            │
│  │ name                  │                             │            │
│  │ caller_group_id (FK※) │                             │            │
│  │ action_type           │                             │            │
│  │ action_config (JSONB) │                             │            │
│  │ priority              │                             │            │
│  │ is_active             │                             │            │
│  │ created_at, updated_at│                             │            │
│  └───────────────────────┘                             │            │
│  ※ FK なし（削除済みグループ対応）                      │            │
│                                                        │            │
│  ┌───────────────────────┐                             │            │
│  │    routing_rules      │                             │            │
│  ├───────────────────────┤                             │            │
│  │ id (PK, UUID v7)      │                             │            │
│  │ caller_category       │                             │            │
│  │ action_code           │                             │            │
│  │ ivr_flow_id (FK)──────┼─────────────────────────────┤            │
│  │ priority              │                             │            │
│  │ is_active, version    │                             │            │
│  │ created_at, updated_at│                             │            │
│  └───────────────────────┘                             ▼            │
│                                               ┌──────────────────┐ │
│                                               │    ivr_flows     │ │
│                                               ├──────────────────┤ │
│                                               │ id (PK, UUID v7) │ │
│                                               │ name             │ │
│                                               │ description      │ │
│                                               │ is_active        │ │
│                                               │ created_at       │ │
│                                               │ updated_at       │ │
│                                               └────────┬─────────┘ │
│                                                        │            │
│                                  ┌─────────────────────┘            │
│                                  ▼                                   │
│                       ┌──────────────────────┐                      │
│                       │     ivr_nodes        │◀──┐ (parent_id)     │
│                       ├──────────────────────┤   │                  │
│                       │ id (PK, UUID v7)      │───┘                 │
│                       │ flow_id (FK)          │                     │
│                       │ parent_id (FK, NULL=root)                   │
│                       │ node_type, action_code│                     │
│                       │ audio_file_url        │                     │
│                       │ tts_text, depth       │                     │
│                       │ timeout_sec           │                     │
│                       │ max_retries           │                     │
│                       │ exit_action           │                     │
│                       │ created_at, updated_at│                     │
│                       └──────────┬───────────┘                     │
│                                  │                                   │
│                       ┌──────────┴───────────┐                     │
│                       │   ivr_transitions    │                     │
│                       ├──────────────────────┤                     │
│                       │ id (PK, UUID v7)      │                     │
│                       │ from_node_id (FK)     │                     │
│                       │ input_type, dtmf_key  │                     │
│                       │ to_node_id (FK)       │                     │
│                       │ created_at            │                     │
│                       └──────────────────────┘                     │
└──────────────────────────────────────────────────────────────────────┘

┌─── 通話系 ───────────────────────────────────────────────────────────┐
│                                                                      │
│  ┌──────────────────────┐                                           │
│  │   call_log_index     │  ← FK 中間テーブル（非パーティション）     │
│  ├──────────────────────┤                                           │
│  │ id (PK, UUID v7)     │◀──────────────────────────┐               │
│  │ started_at           │                           │               │
│  └──────────┬───────────┘                           │               │
│             │ 1:1                                   │ 1:N           │
│             ▼                                       │               │
│  ┌──────────────────────┐                ┌──────────┴───────────┐   │
│  │  call_logs (PART.)   │                │     recordings       │   │
│  ├──────────────────────┤                ├──────────────────────┤   │
│  │ id (FK → index)      │                │ id (PK, UUID v7)     │   │
│  │ started_at           │                │ call_log_id (FK→idx) │   │
│  │ PK = (id, started_at)│                │ recording_type       │   │
│  │ external_call_id     │                │ sequence_number      │   │
│  │ sip_call_id          │                │ file_path, s3_url    │   │
│  │ caller_number (E.164)│                │ upload_status        │   │
│  │ caller_category      │                │ duration_sec, format │   │
│  │ action_code          │                │ file_size_bytes      │   │
│  │ ivr_flow_id          │                │ started_at, ended_at │   │
│  │ answered_at, ended_at│                │ synced_at            │   │
│  │ duration_sec         │                │ created_at           │   │
│  │ end_reason           │                └──────────────────────┘   │
│  │ version, synced_at   │                  ※ append-only            │
│  │ created_at           │                                           │
│  └──────────────────────┘                                           │
│  PARTITION BY RANGE (started_at) — 月次                              │
└──────────────────────────────────────────────────────────────────────┘

┌─── 同期・設定系 ─────────────────────────────────────────────────────┐
│                                                                      │
│  ┌──────────────────────┐     ┌────────────────────────────────┐    │
│  │    sync_outbox       │     │      system_settings           │    │
│  ├──────────────────────┤     ├────────────────────────────────┤    │
│  │ id (PK, BIGSERIAL)   │     │ id (PK, CHECK id=1)           │    │
│  │ entity_type          │     │ recording_retention_days (=90) │    │
│  │ entity_id (UUID)     │     │ history_retention_days (=365)  │    │
│  │ payload (JSONB)      │     │ sync_endpoint_url              │    │
│  │ created_at           │     │ default_action_code (='IV')    │    │
│  │ processed_at         │     │ max_concurrent_calls (=2)      │    │
│  └──────────────────────┘     │ extra (JSONB)                  │    │
│                                │ version, updated_at            │    │
│                                └────────────────────────────────┘    │
└──────────────────────────────────────────────────────────────────────┘
```

### 4.3 テーブル定義

> DDL の完全版は [STEER-110 §6.4](../../steering/STEER-110_backend-db-design.md) を参照。
> 以下は BD レベルの論理構造を示す。

#### 4.3.1 ルーティング系

##### spam_numbers（迷惑電話DB — 論理削除対応）

| カラム | 型 | 備考 |
|--------|-----|------|
| id | UUID v7 (PK) | Rust 側で生成 |
| phone_number | VARCHAR(20) | E.164, 部分 UNIQUE (WHERE deleted_at IS NULL) |
| reason | VARCHAR(255) | |
| source | VARCHAR(50) | 'manual' / 'import' / 'report' |
| deleted_at | TIMESTAMPTZ | NULL = 有効 |
| created_at | TIMESTAMPTZ | |
| updated_at | TIMESTAMPTZ | |

##### registered_numbers（登録済み番号 — 論理削除 + 楽観ロック）

| カラム | 型 | 備考 |
|--------|-----|------|
| id | UUID v7 (PK) | Rust 側で生成 |
| phone_number | VARCHAR(20) | E.164, 部分 UNIQUE |
| name | VARCHAR(100) | |
| category | VARCHAR(50) | 'vip' / 'customer' / 'partner' / 'general' |
| action_code | VARCHAR(2) | デフォルト 'VR' |
| ivr_flow_id | UUID (FK) | → ivr_flows |
| recording_enabled | BOOLEAN | デフォルト TRUE |
| announce_enabled | BOOLEAN | デフォルト TRUE |
| group_id | UUID | 番号グループの不変ID（Frontend CallerGroup.id）、FK なし |
| group_name | VARCHAR(255) | 番号グループの表示名（Frontend CallerGroup.name） |
| notes | TEXT | |
| version | INT | 楽観的排他制御 |
| deleted_at | TIMESTAMPTZ | NULL = 有効 |
| created_at | TIMESTAMPTZ | |
| updated_at | TIMESTAMPTZ | |

> **Issue #138 追加**: `group_id` / `group_name` は Frontend CallerGroup（番号グループ）との対応を表す。
> - `group_id` は不変ID（UUID）で、Frontend で CallerGroup をリネームしても変わらない
> - `group_name` は表示名で、Frontend でリネームすると更新される
> - `call_action_rules.caller_group_id` と照合して番号グループ評価を実施（RD-004 FR-1.4 参照）

##### routing_rules（ルーティングルール — 楽観ロック）

| カラム | 型 | 備考 |
|--------|-----|------|
| id | UUID v7 (PK) | Rust 側で生成 |
| caller_category | VARCHAR(20) | 'spam' / 'registered' / 'unknown' / 'anonymous' |
| action_code | VARCHAR(2) | CHECK 制約 |
| ivr_flow_id | UUID (FK) | → ivr_flows |
| priority | INT | |
| is_active | BOOLEAN | |
| version | INT | 楽観的排他制御 |
| created_at | TIMESTAMPTZ | |
| updated_at | TIMESTAMPTZ | |

##### call_action_rules（着信アクションルール — Issue #138 追加）

| カラム | 型 | 備考 |
|--------|-----|------|
| id | UUID v7 (PK) | Frontend IncomingRule.id と同一値 |
| name | VARCHAR(255) | ルール名（例: "スパム拒否"） |
| caller_group_id | UUID | → registered_numbers.group_id、FK なし、NULL 許容 |
| action_type | VARCHAR(20) | 'allow' / 'deny' |
| action_config | JSONB | ActionCode + 詳細設定（RD-004 FR-2.2 参照） |
| priority | INT | 評価優先順位（小さいほど優先、Frontend 配列順） |
| is_active | BOOLEAN | TRUE = 有効 |
| created_at | TIMESTAMPTZ | |
| updated_at | TIMESTAMPTZ | |

> **設計方針**:
> - Frontend の `IncomingRule` を Backend DB に同期（Serversync 経由）
> - `caller_group_id` は `registered_numbers.group_id` を参照するが、FK 制約なし（削除済みグループ対応）
> - `action_config` (JSONB) の例:
>   ```json
>   { "actionCode": "BZ" }
>   { "actionCode": "IV", "ivrFlowId": "uuid-v7", "includeAnnouncement": true }
>   ```
> - RD-004 FR-1.1 の「段階2: 番号グループ評価」で使用

##### ivr_flows（IVRフロー定義 — root_node_id 廃止）

| カラム | 型 | 備考 |
|--------|-----|------|
| id | UUID v7 (PK) | Rust 側で生成 |
| name | VARCHAR(100) | |
| description | TEXT | |
| is_active | BOOLEAN | |
| created_at | TIMESTAMPTZ | |
| updated_at | TIMESTAMPTZ | |

> **D-10**: `root_node_id` は廃止。ルートノードは `ivr_nodes.parent_id IS NULL` で特定する。

##### ivr_nodes（IVRノード — depth 制約付き）

| カラム | 型 | 備考 |
|--------|-----|------|
| id | UUID v7 (PK) | Rust 側で生成 |
| flow_id | UUID (FK) | → ivr_flows, ON DELETE CASCADE |
| parent_id | UUID (FK) | → ivr_nodes, NULL = ルート |
| node_type | VARCHAR(20) | 'ANNOUNCE' / 'KEYPAD' / 'FORWARD' / 'TRANSFER' / 'RECORD' / 'EXIT' |
| action_code | VARCHAR(2) | |
| audio_file_url | TEXT | |
| tts_text | TEXT | |
| timeout_sec | INT | デフォルト 10 |
| max_retries | INT | デフォルト 3 |
| depth | SMALLINT | ルート=0, 最大 3（RD-004 FR-122） |
| exit_action | VARCHAR(2) | デフォルト 'IE' |
| created_at | TIMESTAMPTZ | |
| updated_at | TIMESTAMPTZ | |

##### ivr_transitions（IVR遷移）

| カラム | 型 | 備考 |
|--------|-----|------|
| id | UUID v7 (PK) | Rust 側で生成 |
| from_node_id | UUID (FK) | → ivr_nodes, ON DELETE CASCADE |
| input_type | VARCHAR(20) | 'DTMF' / 'TIMEOUT' / 'INVALID' / 'COMPLETE' |
| dtmf_key | VARCHAR(5) | '0'-'9', '*', '#', or NULL |
| to_node_id | UUID (FK) | → ivr_nodes |
| created_at | TIMESTAMPTZ | |

#### 4.3.2 通話系（STEER-110 追加）

##### call_log_index（FK 中間テーブル — 非パーティション）

> パーティションテーブル（call_logs）の PK にはパーティションキーを含める必要がある。
> 非パーティションの call_log_index を FK 参照先とすることで recordings → call_logs の紐付けを実現。

| カラム | 型 | 備考 |
|--------|-----|------|
| id | UUID v7 (PK) | call_logs.id と同一値 |
| started_at | TIMESTAMPTZ | TTL 削除用 |

##### call_logs（通話履歴 — 月次パーティション）

| カラム | 型 | 備考 |
|--------|-----|------|
| id | UUID v7 (FK) | → call_log_index |
| started_at | TIMESTAMPTZ | パーティションキー |
| (PK) | | = (id, started_at) |
| external_call_id | VARCHAR(64) | contract.md の callId に対応 |
| sip_call_id | VARCHAR(255) | デバッグ用 |
| caller_number | VARCHAR(20) | E.164, NULL = 非通知 |
| caller_category | VARCHAR(20) | 4 カテゴリ |
| action_code | VARCHAR(2) | |
| ivr_flow_id | UUID | FK なし（意図的） |
| answered_at | TIMESTAMPTZ | NULL = 未応答 |
| ended_at | TIMESTAMPTZ | |
| duration_sec | INT | |
| end_reason | VARCHAR(20) | 'normal' / 'cancelled' / 'rejected' / 'timeout' / 'error' |
| version | INT | 楽観的排他制御 |
| synced_at | TIMESTAMPTZ | NULL = 未同期 |
| created_at | TIMESTAMPTZ | |

> `PARTITION BY RANGE (started_at)` — 月単位。TTL 削除は `DROP TABLE` で完了。

##### recordings（録音メタデータ — append-only）

| カラム | 型 | 備考 |
|--------|-----|------|
| id | UUID v7 (PK) | |
| call_log_id | UUID (FK) | → call_log_index, ON DELETE CASCADE |
| recording_type | VARCHAR(20) | 'full_call' / 'ivr_segment' / 'voicemail' / 'transfer' / 'one_way' |
| sequence_number | SMALLINT | 同一通話内の録音順序 |
| file_path | TEXT | ローカルファイルパス |
| s3_url | TEXT | NULL = 未アップロード |
| upload_status | VARCHAR(20) | 'local_only' / 'uploading' / 'uploaded' / 'upload_failed' |
| duration_sec | INT | |
| format | VARCHAR(10) | 'wav' / 'mp3' |
| file_size_bytes | BIGINT | |
| started_at | TIMESTAMPTZ | |
| ended_at | TIMESTAMPTZ | |
| synced_at | TIMESTAMPTZ | Frontend 同期日時 |
| created_at | TIMESTAMPTZ | |

> `updated_at` なし（append-only）。`synced_at` / `upload_status` のみ例外的に更新。

#### 4.3.3 同期・設定系（STEER-110 追加）

##### sync_outbox（Transactional Outbox）

| カラム | 型 | 備考 |
|--------|-----|------|
| id | BIGSERIAL (PK) | 順序保証（UUID v7 ではない） |
| entity_type | VARCHAR(30) | 'call_log' / 'recording' / 'phone_number' / 'ivr_flow' / 'routing_rule' |
| entity_id | UUID | FK なし（ポリモーフィック） |
| payload | JSONB | データスナップショット |
| created_at | TIMESTAMPTZ | |
| processed_at | TIMESTAMPTZ | NULL = 未送信 |

##### system_settings（システム設定 — 単一行ハイブリッド）

| カラム | 型 | 備考 |
|--------|-----|------|
| id | INT (PK) | CHECK id=1（単一行制約） |
| recording_retention_days | INT | デフォルト 90 |
| history_retention_days | INT | デフォルト 365 |
| sync_endpoint_url | TEXT | Frontend API URL |
| default_action_code | VARCHAR(2) | デフォルト 'IV' |
| max_concurrent_calls | INT | デフォルト 2 |
| extra | JSONB | ユーザー拡張設定 |
| version | INT | 楽観的排他制御 |
| updated_at | TIMESTAMPTZ | |

---

## 5. Rust 側状態管理

### 5.1 IVR ステートマシン

Rust 側では IVR の現在状態を保持し、DTMF 入力やタイムアウトに応じて次の状態を決定する。

```
┌─────────────────────────────────────────────────────────┐
│                    IvrStateMachine                       │
├─────────────────────────────────────────────────────────┤
│ - flow_id: Uuid                                          │
│ - current_node_id: Uuid                                  │
│ - retry_count: u32                                       │
│ - state: IvrState                                        │
├─────────────────────────────────────────────────────────┤
│ + new(flow_id) -> Self                                   │
│ + process_dtmf(key: char) -> NextAction                  │
│ + process_timeout() -> NextAction                        │
│ + process_complete() -> NextAction                       │
└─────────────────────────────────────────────────────────┘

enum IvrState {
    Idle,
    PlayingAnnounce,
    WaitingInput,
    Recording,
    Forwarding,
    Completed,
}

enum NextAction {
    PlayAudio(url),
    WaitDtmf { timeout_sec: u32 },
    Forward { destination: String },
    StartRecording,
    Exit,
    Error(String),
}
```

### 5.2 状態遷移

```
                        ┌─────────────┐
                        │    Idle     │
                        └──────┬──────┘
                               │ start()
                               ▼
                        ┌─────────────────┐
              ┌────────│ PlayingAnnounce │
              │         └────────┬────────┘
              │                  │ complete
              │                  ▼
              │         ┌─────────────────┐
     timeout/ │         │  WaitingInput   │◀─────────┐
     invalid  │         └────────┬────────┘          │
     (retry)  │                  │                   │
              │         ┌────────┴────────┐          │
              │         │                  │          │
              │    valid DTMF         invalid/timeout │
              │         │                  │          │
              │         ▼                  └──────────┘
              │  ┌─────────────┐                (retry < max)
              │  │ Next Node   │
              │  └──────┬──────┘
              │         │
              │         ▼
              │  ┌─────────────┐
              └─▶│ Completed   │ (retry >= max → IE)
                 └─────────────┘
```

### 5.3 DTMF 入力パターン

| パターン | 説明 | 遷移先決定 |
|----------|------|-----------|
| valid | transitions に定義された DTMF キー | `to_node_id` へ遷移 |
| invalid | transitions に未定義のキー | `INVALID` 遷移 or リトライ |
| timeout | `timeout_sec` 経過 | `TIMEOUT` 遷移 or リトライ |

### 5.4 リトライ超過時の動作

`max_retries` を超えた場合、`exit_action`（デフォルト: `IE` = 切断）を実行する。

---

## 6. データフロー

### 6.1 着信時の処理フロー

```
SIP INVITE 受信
      │
      ▼
┌─────────────────┐
│ Caller ID 抽出   │
└────────┬────────┘
         │
         ▼
┌─────────────────────────────────┐
│ DB 検索（分類判定）              │
│ 1. spam_numbers                 │
│ 2. registered_numbers           │
│ 3. routing_rules (by category)  │
└────────────────┬────────────────┘
                 │
                 ▼
        ┌────────────────┐
        │ action_code    │
        │ 取得           │
        └────────┬───────┘
                 │
    ┌────────────┴────────────┐
    │                         │
    ▼                         ▼
┌────────┐              ┌──────────┐
│ 即時系  │              │ IVR系    │
│ NC/RJ/ │              │ IV       │
│ AN/AR  │              └────┬─────┘
└────────┘                   │
                             ▼
                    ┌────────────────┐
                    │ IVR フロー読込  │
                    │ (ivr_flows +   │
                    │  ivr_nodes +   │
                    │  transitions)  │
                    └────────┬───────┘
                             │
                             ▼
                    ┌────────────────┐
                    │ IvrStateMachine│
                    │ 生成・開始      │
                    └────────────────┘
```

---

## 7. 非機能設計方針

### 7.1 性能

| 項目 | 方針 |
|------|------|
| 番号検索 | インデックス使用、O(log N)。NFR-100: 100ms 以内 |
| IVR フロー読込 | 開始時に全ノード/遷移をメモリ展開 |
| キャッシュ | routing_rules はアプリ起動時にキャッシュ |
| call_logs | 月次パーティションで古いデータの検索影響を分離 |
| 接続プール | SQLx 内蔵プール（max_connections = 5、Raspberry Pi 制約） |

### 7.2 可用性

| 項目 | 方針 |
|------|------|
| DB 障害時 | デフォルトルール（unknown → IV）適用 |
| IVR フロー不整合 | エラーログ + 切断 |
| TTL 運用 | recordings 90 日、call_logs 365 日、sync_outbox 送信済み 30 日 |

### 7.3 セキュリティ

| 項目 | 方針 |
|------|------|
| SQL インジェクション | パラメータバインド必須（SQLx） |
| 電話番号 | E.164 正規化して保存（`+819012345678` 形式） |

---

## 8. 前提条件・制約

### 8.1 前提条件

- PostgreSQL を使用（#62 決定事項）
- **環境構成方針**
  - 開発環境: Frontend / Backend / PostgreSQL すべてローカル（同一 PC or 同一ラズパイ、Docker 不使用）
  - 本番環境: 安定後 AWS に展開（マネージド DB 等を検討）
- Backend は Rust + SQLx でDB接続
- SIP INVITE から Caller ID を取得可能
- UUID v7 は Rust 側で生成（`uuid::Uuid::now_v7()`）

### 8.2 制約

- IVR フローは木構造（循環禁止）、最大深度 3（RD-004 FR-122）
- 1つの IVR フロー内のノード数は 100 以下を推奨
- DTMF キーは 0-9, *, # のみ
- call_logs パーティションの PK は (id, started_at) の複合キー
- system_settings は単一行制約（id = 1）

---

## 9. 未確定事項（Open Questions）

### 解決済み

- [x] Q1: 本設計のスコープ → 仕様決定のみ
- [x] Q2: RD-004 との関係 → BD レベルで対応
- [x] Q3: DB 技術選定 → PostgreSQL 16+（Tsurugi から移行）
- [x] Q4: アクションコード体系 → VARCHAR(2) + CHECK 制約
- [x] Q5: 発信者分類 → 4カテゴリ
- [x] Q6: 非通知デフォルト → IVR
- [x] Q7: IVR リトライ超過時 → 切断（IE）
- [x] Q8: PK 戦略 → UUID v7（Rust 側生成）
- [x] Q9: 電話番号正規化 → E.164
- [x] Q10: 録音モデル → 1 Call : N Recording
- [x] Q11: 削除戦略 → 設定系のみ論理削除
- [x] Q12: 同期方式 → Transactional Outbox + synced_at
- [x] Q13: call_logs パーティション → 初期から月次レンジ
- [x] Q14: パーティション FK → call_log_index 中間テーブル
- [x] Q15: IVR 循環 FK → root_node_id 廃止

### 解決済み（Codex 指摘）

- [x] Q8: **DBスキーマ移行方針** → **SQLx migrate** を使用
  - Rust + SQLx 環境に整合、`migrations/` ディレクトリで管理

- [x] Q9: **ActionCode 適用優先度** → **registered_numbers 優先**（個別設定 > デフォルト）
  - 判定順序: spam_numbers → registered_numbers（個別 action_code）→ routing_rules（カテゴリデフォルト）

- [x] Q10: **ivr_nodes.node_type vs action_code** → **責務分離**
  - `node_type`: ノードの構造種別（ANNOUNCE / KEYPAD / FORWARD / EXIT 等）
  - `action_code`: そのノードで実行する処理コード（IA / IK / IF / IE 等）
  - node_type は木構造の意味、action_code は実行時の処理を規定

- [x] Q11: **IVR 木構造保証** → **両方**（DB + アプリ）
  - DB: `parent_id` による親子関係 + アプリ側で挿入/更新時に循環チェック
  - 理由: DBトリガーは複雑になるため、アプリ側で検証しつつ DB で基本構造を保証

- [x] Q12: **Caller ID 正規化ルール** → **E.164 正規化**
  - 国番号欠落時: 日本 `+81` を補完（デフォルト設定）
  - 先頭 `0` 除去: `090...` → `+8190...`
  - ハイフン/スペース除去: `090-1234-5678` → `+819012345678`
  - 保存形式: `+{国番号}{番号}` （例: `+819012345678`）

---

## 変更履歴

| 日付 | バージョン | 変更内容 | 作成者 |
|------|-----------|---------|--------|
| 2026-02-02 | 1.0 | 初版作成（#92 壁打ち結果） | Claude Code |
| 2026-02-02 | 1.1 | VB/VR（ボイスボット録音なし/あり）追加、IB追加、録音オプション対応表追加 | Claude Code |
| 2026-02-02 | 1.2 | Codex 指摘による Open Questions 追加（Q8〜Q12） | Claude Code |
| 2026-02-02 | 1.3 | Q8〜Q12 解決: SQLx migrate、優先度、責務分離、循環チェック、E.164正規化 | Claude Code |
| 2026-02-02 | 1.4 | 環境構成方針: 開発=ローカル（PC/ラズパイ）、本番=AWS | Claude Code |
