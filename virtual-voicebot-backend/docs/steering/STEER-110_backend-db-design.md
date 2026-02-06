# STEER-110: バックエンド側データベース設計

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-110 |
| タイトル | バックエンド側データベース設計 |
| ステータス | Approved |
| 関連Issue | #110 |
| 優先度 | P0 |
| 作成日 | 2026-02-06 |

---

## 2. ストーリー（Why）

### 2.1 背景

現在の Backend は電話番号検索に **Tsurugi DB**（`phone_entries` テーブル）を使用しているが、以下の課題がある：

| 課題 | 詳細 |
|------|------|
| **スキーマの不足** | 現行は `phone_number` + `ivr_enabled` の2カラムのみ。BD-004 で定義した発信者4カテゴリ分類、アクションコード、IVR木構造に対応できない |
| **通話履歴の永続化なし** | 通話履歴（Call/Recording）がインメモリのみ。プロセス再起動で消失する |
| **録音メタデータ管理なし** | 録音ファイルのメタデータ（call_id, duration, S3 URL 等）をDBで管理していない |
| **Frontend 同期基盤なし** | Frontend へ履歴・設定を同期するためのデータ基盤が存在しない |
| **Tsurugi の運用負荷** | Tsurugi は NTT 開発の分散 OLTP DB だが、個人利用規模ではオーバースペックであり、PostgreSQL の方が運用・エコシステムともに適切 |

### 2.2 目的

Backend 側の PostgreSQL データベースを設計し、以下を実現する：

1. **BD-004 ルーティングテーブルの実装基盤**: 発信者分類・アクションコード・IVRフロー管理
2. **通話履歴の永続化**: call_logs テーブルで通話履歴を DB に記録
3. **録音メタデータ管理**: recordings テーブルで録音ファイルのメタデータを管理
4. **Frontend 同期の基盤**: Transactional Outbox + synced_at で Frontend との同期を保証
5. **Tsurugi → PostgreSQL 移行**: DB アダプタ層を PostgreSQL（SQLx）に置き換え

### 2.3 ユーザーストーリー

```
As a Backend開発者
I want to PostgreSQL による統合的なデータベース設計
So that 通話処理・履歴・録音・IVR・同期を一元管理できる

受入条件:
- [ ] PostgreSQL スキーマ定義（DDL）が完成している
- [ ] BD-004 のルーティングテーブルが含まれている（循環FK解消済み）
- [ ] 通話履歴テーブル（call_logs）が月次パーティション対応で定義されている
- [ ] 録音メタデータテーブル（recordings）が 1:N で定義されている
- [ ] 同期基盤（sync_outbox + synced_at）が定義されている
- [ ] システム設定テーブル（system_settings）が定義されている
- [ ] ER図が全決定事項を反映している
- [ ] マイグレーション戦略が定義されている
- [ ] Rust 側のアダプタ設計方針（SQLx）が定義されている
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-06 |
| 起票理由 | Backend DB の統合設計が必要 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (Opus 4.6) |
| 作成日 | 2026-02-06 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "バックエンド側のデータベースの設計を行う" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | @MasanoriSuda | 2026-02-07 | Approved | |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-07 |
| 承認コメント | 承認 |

### 3.5 実装（該当する場合）

| 項目 | 値 |
|------|-----|
| 実装者 | Codex |
| 実装日 | - |
| 指示者 | @MasanoriSuda |
| 指示内容 | "本ステアリングに基づき PostgreSQL スキーマ + SQLx アダプタを実装" |
| コードレビュー | CodeRabbit (自動) |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | @MasanoriSuda |
| マージ日 | - |
| マージ先 | BD-004（テーブル定義更新）、DD-新規（DB詳細設計） |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| virtual-voicebot-backend/docs/design/basic/BD-004_call-routing-db.md | 修正 | root_node_id 廃止、call_log_index / call_logs / recordings / sync_outbox / system_settings 追加、ER図更新 |
| virtual-voicebot-backend/docs/design/detail/DD-001_tech-stack.md | 修正 | Tsurugi → PostgreSQL(SQLx) への変更、UUID v7 採用を反映 |
| docs/contract.md | 確認 | Call / RecordingMeta データモデルとの整合性確認 |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| src/interface/db/tsurugi.rs | 削除 | Tsurugi アダプタを廃止 |
| src/interface/db/postgres.rs | 追加 | PostgreSQL アダプタ（SQLx）を新設 |
| src/shared/ports/phone_lookup.rs | 修正 | PhoneLookupPort を拡張（4カテゴリ分類対応） |
| src/shared/ports/call_repository.rs | 追加 | 通話履歴の永続化ポート |
| src/shared/ports/recording_repository.rs | 追加 | 録音メタデータの永続化ポート |
| migrations/ | 追加 | SQLx マイグレーションファイル |
| docker-compose.dev.yml | 修正 | PostgreSQL サービス追加 |

---

## 5. 設計判断サマリ

壁打ちで確定した全設計判断を一覧化する。DDL はこれらの決定に基づく。

| # | 論点 | 決定 | 根拠 |
|---|------|------|------|
| D-01 | PK の ID 戦略 | **UUID v7** | Raspberry Pi の SD カード / eMMC ではランダム I/O が遅い。UUID v7 はタイムスタンプ先頭で B-tree 挿入が順序的になり、ページ分割を抑制。Rust 側で `uuid::Uuid::now_v7()` 生成、DB の `DEFAULT gen_random_uuid()` は使用しない |
| D-02 | action_code 管理方式 | **VARCHAR(2) + CHECK 制約** | 拡張頻度が高い可能性があるが、現段階のコード数（基本9 + IVR内8 = 17種）はルックアップテーブルにするほどではない。追加時は `ALTER TABLE ... DROP/ADD CONSTRAINT` で対応 |
| D-03 | 電話番号正規化 | **E.164 形式**（`+819012345678`） | DB 一意性・検索・外部 API 連携を優先。ハイフン付き表示は UI 層で変換。CHECK 制約 `^\+[1-9][0-9]{1,14}$` を DB レベルで強制 |
| D-04 | 録音モデル | **1 Call : N Recording** | 転送・IVR 分割・片方向録音を考慮。Call に recording_url を直持ちしない。recording_type で種別を分類 |
| D-05 | SoT | **Call が集約ルート** | Recording は append-only な従属エンティティ。外部 call_id / SIP Call-ID は保持するが、内部 UUID v7 PK が主導 |
| D-06 | 削除戦略 | **設定系のみ論理削除** | call_logs / recordings は「削除されるべきでないデータ」。削除 API を提供しない。TTL バッチで物理削除。spam_numbers / registered_numbers のみ `deleted_at` カラム |
| D-07 | 同期方式 | **Transactional Outbox + synced_at 併用** | outbox で送信漏れゼロ保証、synced_at で最終同期日時のクイック参照。sync_log は廃止し sync_outbox に置換 |
| D-08 | call_logs パーティション | **初期から月次レンジパーティション** | `PARTITION BY RANGE (started_at)` で月単位。TTL 削除はパーティション DROP で完了。年間 12 パーティション + 1（作成中の月） |
| D-09 | パーティション FK 問題 | **中間テーブル `call_log_index`** | パーティションテーブルの PK にはパーティションキーを含める必要がある。非パーティションの call_log_index(id PK) を FK 参照先とすることで、recordings → call_log_index の FK が単一カラムで済む |
| D-10 | ivr_flows 循環 FK | **root_node_id 廃止** | `parent_id IS NULL` のノードをルートとする規約で解決。INSERT 順序問題・CASCADE デッドロック解消 |
| D-11 | callee_number | **廃止** | 現状は単一回線（ひかり電話）。着信先番号は system_settings で管理。将来の複数回線対応時に再追加 |
| D-12 | TTL | **履歴 365 日 / 録音 90 日** | 録音は S3 ファイル + DB メタデータを 90 日で削除。履歴（call_log_index + call_logs パーティション）は 365 日で DROP |
| D-13 | システム設定 | **ハイブリッド（単一行テーブル + JSONB 拡張カラム）** | 固定設定は専用カラム（型安全）、ユーザー拡張設定は JSONB カラム。1 テーブル・1 行 |

---

## 6. 差分仕様（What / How）

### 6.1 全体方針

| 方針 | 説明 |
|------|------|
| **DB エンジン** | PostgreSQL 16+ |
| **Rust クライアント** | SQLx（コンパイル時クエリ検証） |
| **ID 戦略** | UUID v7（Rust 側で `uuid::Uuid::now_v7()` 生成） |
| **タイムスタンプ** | TIMESTAMPTZ（UTC 保存） |
| **電話番号** | E.164 形式、CHECK 制約で強制 |
| **マイグレーション** | SQLx CLI (`sqlx migrate`) |
| **接続プール** | SQLx 内蔵プール（max_connections = 5） |
| **静的設定** | TOML ファイル（SIP 認証、ログレベル、S3 認証等） |
| **動的設定** | system_settings テーブル（録音保存期限、同期先 URL 等） |

### 6.2 Backend / Frontend DB の関係

```
┌──────────────────────────────────────────┐
│     Backend DB (ローカル PostgreSQL)       │
│                                          │
│  ┌─ ルーティング系 ───────────────────┐   │
│  │ spam_numbers (論理削除)           │   │
│  │ registered_numbers (論理削除)     │   │
│  │ routing_rules                    │   │
│  │ ivr_flows / ivr_nodes            │   │
│  │ ivr_transitions                  │   │
│  └───────────────────────────────────┘   │
│                                          │
│  ┌─ 通話系 ──────────────────────────┐   │
│  │ call_log_index (FK中間テーブル)    │   │     ┌──────────────────────┐
│  │ call_logs (月次パーティション)      │───┼────▶│  Frontend DB (RDS)   │
│  │ recordings (append-only)          │   │     │  (スキーマ別途設計)    │
│  └───────────────────────────────────┘   │     └──────────────────────┘
│                                          │        GET/POST (REST)
│  ┌─ 同期・設定 ──────────────────────┐   │
│  │ sync_outbox (Transactional Outbox)│   │
│  │ system_settings (単一行)          │   │
│  └───────────────────────────────────┘   │
└──────────────────────────────────────────┘
```

---

### 6.3 ER図（Backend DB 全体 — 全決定反映済み）

```
┌────────────────────────────────────────────────────────────────────────┐
│                         ルーティング系                                   │
│                                                                        │
│  ┌──────────────────────┐   ┌──────────────────────────┐              │
│  │    spam_numbers      │   │   registered_numbers     │              │
│  ├──────────────────────┤   ├──────────────────────────┤              │
│  │ id (PK, UUID v7)     │   │ id (PK, UUID v7)         │              │
│  │ phone_number (UQ※)   │   │ phone_number (UQ※)       │              │
│  │ reason               │   │ name                     │              │
│  │ source               │   │ category                 │              │
│  │ deleted_at           │   │ action_code              │              │
│  │ created_at           │   │ ivr_flow_id (FK)─────────┼──┐           │
│  │ updated_at           │   │ recording_enabled        │  │           │
│  └──────────────────────┘   │ announce_enabled         │  │           │
│  ※ WHERE deleted_at IS NULL │ notes                    │  │           │
│                             │ version                  │  │           │
│                             │ deleted_at               │  │           │
│                             │ created_at               │  │           │
│                             │ updated_at               │  │           │
│                             └──────────────────────────┘  │           │
│                                                           │           │
│  ┌──────────────────────────┐                             │           │
│  │     routing_rules        │                             │           │
│  ├──────────────────────────┤                             │           │
│  │ id (PK, UUID v7)         │                             │           │
│  │ caller_category          │                             │           │
│  │ action_code              │                             │           │
│  │ ivr_flow_id (FK)─────────┼─────────────────────────────┤           │
│  │ priority                 │                             │           │
│  │ is_active                │                             │           │
│  │ version                  │                             │           │
│  │ created_at               │                             │           │
│  │ updated_at               │                             │           │
│  └──────────────────────────┘                             │           │
│                                                           ▼           │
│                                                  ┌──────────────────┐ │
│                                                  │    ivr_flows     │ │
│                                                  ├──────────────────┤ │
│                                                  │ id (PK, UUID v7) │ │
│                                                  │ name             │ │
│                                                  │ description      │ │
│                                                  │ is_active        │ │
│                                                  │ created_at       │ │
│                                                  │ updated_at       │ │
│                                                  └────────┬─────────┘ │
│                                                           │           │
│                                     ┌─────────────────────┘           │
│                                     ▼                                 │
│                          ┌──────────────────────┐                     │
│                          │     ivr_nodes        │◀──┐ (parent_id)    │
│                          ├──────────────────────┤   │                 │
│                          │ id (PK, UUID v7)      │───┘                │
│                          │ flow_id (FK)          │                    │
│                          │ parent_id (FK, NULL=root)                  │
│                          │ node_type             │                    │
│                          │ action_code           │                    │
│                          │ audio_file_url        │                    │
│                          │ tts_text              │                    │
│                          │ timeout_sec           │                    │
│                          │ max_retries           │                    │
│                          │ depth                 │                    │
│                          │ exit_action           │                    │
│                          │ created_at            │                    │
│                          │ updated_at            │                    │
│                          └──────────┬───────────┘                    │
│                                     │                                 │
│                          ┌──────────┴───────────┐                    │
│                          │   ivr_transitions    │                    │
│                          ├──────────────────────┤                    │
│                          │ id (PK, UUID v7)      │                    │
│                          │ from_node_id (FK)     │                    │
│                          │ input_type            │                    │
│                          │ dtmf_key              │                    │
│                          │ to_node_id (FK)       │                    │
│                          │ created_at            │                    │
│                          └──────────────────────┘                    │
└────────────────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────────────────┐
│                           通話系                                       │
│                                                                        │
│  ┌──────────────────────────┐                                         │
│  │    call_log_index        │  ← FK 中間テーブル（非パーティション）     │
│  ├──────────────────────────┤                                         │
│  │ id (PK, UUID v7)         │◀──────────────────────────┐             │
│  │ started_at               │                           │             │
│  └──────────┬───────────────┘                           │             │
│             │ 1:1                                       │ 1:N         │
│             ▼                                           │             │
│  ┌──────────────────────────┐                ┌──────────┴───────────┐ │
│  │  call_logs (PARTITIONED) │                │     recordings       │ │
│  ├──────────────────────────┤                ├──────────────────────┤ │
│  │ id (FK → index)          │                │ id (PK, UUID v7)     │ │
│  │ started_at               │                │ call_log_id (FK→idx) │ │
│  │ PK = (id, started_at)   │                │ recording_type       │ │
│  │ external_call_id (UQ※)   │                │ sequence_number      │ │
│  │ sip_call_id              │                │ file_path            │ │
│  │ caller_number (E.164)    │                │ s3_url               │ │
│  │ caller_category          │                │ upload_status        │ │
│  │ action_code              │                │ duration_sec         │ │
│  │ ivr_flow_id              │                │ format               │ │
│  │ answered_at              │                │ file_size_bytes      │ │
│  │ ended_at                 │                │ started_at           │ │
│  │ duration_sec             │                │ ended_at             │ │
│  │ end_reason               │                │ synced_at            │ │
│  │ version                  │                │ created_at           │ │
│  │ synced_at                │                └──────────────────────┘ │
│  │ created_at               │                  ※ updated_at なし      │
│  └──────────────────────────┘                    (append-only)       │
│  ※ UQ = (external_call_id, started_at)                               │
│    パーティションキー含む複合UNIQUE                                      │
│                                                                        │
│  PARTITION BY RANGE (started_at)                                       │
│  ├── call_logs_2026_01                                                │
│  ├── call_logs_2026_02                                                │
│  └── ...                                                              │
└────────────────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────────────────┐
│                        同期・設定系                                     │
│                                                                        │
│  ┌──────────────────────────┐     ┌──────────────────────────────────┐│
│  │     sync_outbox          │     │       system_settings            ││
│  ├──────────────────────────┤     ├──────────────────────────────────┤│
│  │ id (PK, BIGSERIAL)       │     │ id (PK, CHECK id=1)             ││
│  │ entity_type              │     │ recording_retention_days (=90)  ││
│  │ entity_id (UUID)         │     │ history_retention_days (=365)   ││
│  │ payload (JSONB)          │     │ sync_endpoint_url               ││
│  │ created_at               │     │ default_action_code (='IV')     ││
│  │ processed_at             │     │ max_concurrent_calls (=2)       ││
│  └──────────────────────────┘     │ extra (JSONB)                   ││
│                                   │ version                         ││
│                                   │ updated_at                      ││
│                                   └──────────────────────────────────┘│
└────────────────────────────────────────────────────────────────────────┘
```

---

### 6.4 テーブル定義（DDL）

#### 6.4.1 ルーティング系

##### spam_numbers（迷惑電話DB — 論理削除対応）

```sql
CREATE TABLE spam_numbers (
    id UUID NOT NULL PRIMARY KEY,
    -- UUID v7: Rust 側で生成
    phone_number VARCHAR(20) NOT NULL,
    reason VARCHAR(255),
    source VARCHAR(50) NOT NULL DEFAULT 'manual',
    -- source: 'manual' | 'import' | 'report'
    deleted_at TIMESTAMPTZ,
    -- NULL = 有効、非 NULL = 論理削除済み
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_spam_phone_e164
        CHECK (phone_number ~ '^\+[1-9][0-9]{1,14}$'),
    CONSTRAINT chk_spam_source
        CHECK (source IN ('manual', 'import', 'report'))
);

-- 論理削除を考慮した部分 UNIQUE インデックス
CREATE UNIQUE INDEX uq_spam_numbers_phone
    ON spam_numbers(phone_number) WHERE deleted_at IS NULL;
```

##### registered_numbers（登録済み番号 — 論理削除 + 楽観ロック対応）

```sql
CREATE TABLE registered_numbers (
    id UUID NOT NULL PRIMARY KEY,
    phone_number VARCHAR(20) NOT NULL,
    name VARCHAR(100),
    category VARCHAR(50) NOT NULL DEFAULT 'general',
    -- category: 'vip' | 'customer' | 'partner' | 'general'
    action_code VARCHAR(2) NOT NULL DEFAULT 'VR',
    ivr_flow_id UUID REFERENCES ivr_flows(id) ON DELETE SET NULL,
    recording_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    announce_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    notes TEXT,
    version INT NOT NULL DEFAULT 1,
    -- 楽観的排他制御: UPDATE 時に WHERE version = $expected
    deleted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_registered_phone_e164
        CHECK (phone_number ~ '^\+[1-9][0-9]{1,14}$'),
    CONSTRAINT chk_registered_category
        CHECK (category IN ('vip', 'customer', 'partner', 'general')),
    CONSTRAINT chk_registered_action_code
        CHECK (action_code IN ('VB','VR','NR','RJ','BZ','AN','AR','VM','IV'))
);

CREATE UNIQUE INDEX uq_registered_numbers_phone
    ON registered_numbers(phone_number) WHERE deleted_at IS NULL;
```

##### routing_rules（ルーティングルール — 楽観ロック対応）

```sql
CREATE TABLE routing_rules (
    id UUID NOT NULL PRIMARY KEY,
    caller_category VARCHAR(20) NOT NULL,
    action_code VARCHAR(2) NOT NULL,
    ivr_flow_id UUID REFERENCES ivr_flows(id) ON DELETE SET NULL,
    priority INT NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    version INT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_routing_caller_category
        CHECK (caller_category IN ('spam', 'registered', 'unknown', 'anonymous')),
    CONSTRAINT chk_routing_action_code
        CHECK (action_code IN ('VB','VR','NR','RJ','BZ','AN','AR','VM','IV'))
);

CREATE INDEX idx_routing_rules_category
    ON routing_rules(caller_category, priority) WHERE is_active;
```

##### ivr_flows（IVRフロー定義 — root_node_id 廃止）

```sql
CREATE TABLE ivr_flows (
    id UUID NOT NULL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    -- root_node_id 廃止: ルートノードは parent_id IS NULL で特定
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

##### ivr_nodes（IVRノード — depth 制約付き）

```sql
CREATE TABLE ivr_nodes (
    id UUID NOT NULL PRIMARY KEY,
    flow_id UUID NOT NULL REFERENCES ivr_flows(id) ON DELETE CASCADE,
    parent_id UUID REFERENCES ivr_nodes(id) ON DELETE CASCADE,
    -- parent_id IS NULL = ルートノード（各 flow_id につき1つ）
    node_type VARCHAR(20) NOT NULL,
    action_code VARCHAR(2),
    audio_file_url TEXT,
    tts_text TEXT,
    timeout_sec INT NOT NULL DEFAULT 10,
    max_retries INT NOT NULL DEFAULT 3,
    depth SMALLINT NOT NULL DEFAULT 0,
    -- depth: ルート=0、RD-004 FR-122 により最大3階層
    exit_action VARCHAR(2) NOT NULL DEFAULT 'IE',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_node_type
        CHECK (node_type IN ('ANNOUNCE', 'KEYPAD', 'FORWARD', 'TRANSFER', 'RECORD', 'EXIT')),
    CONSTRAINT chk_node_depth
        CHECK (depth >= 0 AND depth <= 3),
    CONSTRAINT chk_node_exit_action
        CHECK (exit_action IN ('VB','VR','NR','RJ','BZ','AN','AR','VM','IV',
                               'IA','IR','IK','IW','IF','IT','IB','IE'))
);

CREATE INDEX idx_ivr_nodes_flow ON ivr_nodes(flow_id);
CREATE INDEX idx_ivr_nodes_flow_parent ON ivr_nodes(flow_id, parent_id);
```

##### ivr_transitions（IVR遷移）

```sql
CREATE TABLE ivr_transitions (
    id UUID NOT NULL PRIMARY KEY,
    from_node_id UUID NOT NULL REFERENCES ivr_nodes(id) ON DELETE CASCADE,
    input_type VARCHAR(20) NOT NULL,
    dtmf_key VARCHAR(5),
    -- '0'-'9', '*', '#', or NULL
    to_node_id UUID REFERENCES ivr_nodes(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_transition_input_type
        CHECK (input_type IN ('DTMF', 'TIMEOUT', 'INVALID', 'COMPLETE'))
);

CREATE INDEX idx_ivr_transitions_from ON ivr_transitions(from_node_id);
```

---

#### 6.4.2 通話系

##### call_log_index（FK 中間テーブル — 非パーティション）

```sql
CREATE TABLE call_log_index (
    id UUID NOT NULL PRIMARY KEY,
    -- call_logs / recordings からの FK 参照先
    started_at TIMESTAMPTZ NOT NULL
    -- TTL 削除時: この行を DELETE → recordings が CASCADE 削除
);
```

##### call_logs（通話履歴 — 月次パーティション）

```sql
CREATE TABLE call_logs (
    id UUID NOT NULL REFERENCES call_log_index(id),
    started_at TIMESTAMPTZ NOT NULL,
    -- パーティションキー

    PRIMARY KEY (id, started_at),
    -- パーティションテーブルの PK はパーティションキーを含む必要がある

    external_call_id VARCHAR(64) NOT NULL,
    -- アプリ層で生成する通話識別子（contract.md の callId に対応）
    sip_call_id VARCHAR(255),
    -- SIP Call-ID ヘッダ値（デバッグ用）
    caller_number VARCHAR(20),
    -- E.164 形式。NULL = 非通知
    caller_category VARCHAR(20) NOT NULL DEFAULT 'unknown',
    action_code VARCHAR(2) NOT NULL,
    ivr_flow_id UUID,
    -- FK なし（パーティションテーブルから非パーティションテーブルへの FK は可能だが、
    --   ivr_flows の削除時に call_logs の過去ログが影響を受けるべきではないため意図的に FK を張らない）
    answered_at TIMESTAMPTZ,
    -- NULL = 未応答（居留守、拒否等）
    ended_at TIMESTAMPTZ,
    duration_sec INT,
    end_reason VARCHAR(20) NOT NULL DEFAULT 'normal',
    version INT NOT NULL DEFAULT 1,
    synced_at TIMESTAMPTZ,
    -- NULL = 未同期
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_call_caller_e164
        CHECK (caller_number IS NULL OR caller_number ~ '^\+[1-9][0-9]{1,14}$'),
    CONSTRAINT chk_call_category
        CHECK (caller_category IN ('spam', 'registered', 'unknown', 'anonymous')),
    CONSTRAINT chk_call_action_code
        CHECK (action_code IN ('VB','VR','NR','RJ','BZ','AN','AR','VM','IV')),
    CONSTRAINT chk_call_end_reason
        CHECK (end_reason IN ('normal', 'cancelled', 'rejected', 'timeout', 'error'))

) PARTITION BY RANGE (started_at);

-- パーティションキーを含む複合 UNIQUE
CREATE UNIQUE INDEX uq_call_logs_external_id
    ON call_logs(external_call_id, started_at);

CREATE INDEX idx_call_logs_caller
    ON call_logs(caller_number, started_at DESC);

CREATE INDEX idx_call_logs_synced
    ON call_logs(started_at) WHERE synced_at IS NULL;

-- 初期パーティション作成（2026年分）
CREATE TABLE call_logs_2026_01 PARTITION OF call_logs
    FOR VALUES FROM ('2026-01-01') TO ('2026-02-01');
CREATE TABLE call_logs_2026_02 PARTITION OF call_logs
    FOR VALUES FROM ('2026-02-01') TO ('2026-03-01');
CREATE TABLE call_logs_2026_03 PARTITION OF call_logs
    FOR VALUES FROM ('2026-03-01') TO ('2026-04-01');
-- ... 以降は cron ジョブまたは pg_partman で自動作成
```

##### recordings（録音メタデータ — append-only）

```sql
CREATE TABLE recordings (
    id UUID NOT NULL PRIMARY KEY,
    call_log_id UUID NOT NULL REFERENCES call_log_index(id) ON DELETE CASCADE,
    -- call_log_index 経由で call_logs と紐付く
    -- ON DELETE CASCADE: TTL で call_log_index を削除すると recordings も削除
    recording_type VARCHAR(20) NOT NULL DEFAULT 'full_call',
    sequence_number SMALLINT NOT NULL DEFAULT 1,
    -- 同一通話内の録音順序
    file_path TEXT NOT NULL,
    -- ローカルファイルシステム上のパス
    s3_url TEXT,
    -- S3 アップロード後の URL。NULL = 未アップロード
    upload_status VARCHAR(20) NOT NULL DEFAULT 'local_only',
    duration_sec INT,
    format VARCHAR(10) NOT NULL DEFAULT 'wav',
    file_size_bytes BIGINT,
    started_at TIMESTAMPTZ NOT NULL,
    ended_at TIMESTAMPTZ,
    synced_at TIMESTAMPTZ,
    -- Frontend へメタデータを同期した日時
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- updated_at なし: append-only（synced_at / upload_status のみ例外的に更新）

    CONSTRAINT chk_recording_type
        CHECK (recording_type IN ('full_call', 'ivr_segment', 'voicemail', 'transfer', 'one_way')),
    CONSTRAINT chk_upload_status
        CHECK (upload_status IN ('local_only', 'uploading', 'uploaded', 'upload_failed')),
    CONSTRAINT chk_recording_format
        CHECK (format IN ('wav', 'mp3'))
);

CREATE INDEX idx_recordings_call_log_id ON recordings(call_log_id);
CREATE INDEX idx_recordings_synced ON recordings(synced_at) WHERE synced_at IS NULL;
CREATE INDEX idx_recordings_upload ON recordings(upload_status) WHERE upload_status != 'uploaded';
```

---

#### 6.4.3 同期・設定系

##### sync_outbox（Transactional Outbox）

```sql
CREATE TABLE sync_outbox (
    id BIGSERIAL PRIMARY KEY,
    -- BIGSERIAL: 順序保証（UUID v7 ではなく意図的に SERIAL）
    entity_type VARCHAR(30) NOT NULL,
    -- 'call_log' | 'recording' | 'phone_number' | 'ivr_flow' | 'routing_rule'
    entity_id UUID NOT NULL,
    -- FK なし（意図的: ポリモーフィック参照のため）
    payload JSONB NOT NULL,
    -- 送信時点のデータスナップショット
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMPTZ
    -- NULL = 未送信、非 NULL = 送信済み

    -- error_message 不要: 送信失敗時はそのまま残し再送。
    -- 恒久的失敗は processed_at を埋めて別途アラート。
);

CREATE INDEX idx_outbox_pending
    ON sync_outbox(created_at) WHERE processed_at IS NULL;
```

##### system_settings（システム設定 — 単一行ハイブリッド）

```sql
CREATE TABLE system_settings (
    id INT NOT NULL PRIMARY KEY DEFAULT 1,
    -- 単一行制約
    recording_retention_days INT NOT NULL DEFAULT 90,
    history_retention_days INT NOT NULL DEFAULT 365,
    sync_endpoint_url TEXT,
    -- Frontend API の URL（例: https://frontend.example/api）
    default_action_code VARCHAR(2) NOT NULL DEFAULT 'IV',
    max_concurrent_calls INT NOT NULL DEFAULT 2,
    extra JSONB NOT NULL DEFAULT '{}',
    -- ユーザー拡張設定用（型検証はアプリ層）
    version INT NOT NULL DEFAULT 1,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_single_row CHECK (id = 1),
    CONSTRAINT chk_retention_positive CHECK (
        recording_retention_days > 0 AND history_retention_days > 0
    ),
    CONSTRAINT chk_settings_action_code
        CHECK (default_action_code IN ('VB','VR','NR','RJ','BZ','AN','AR','VM','IV'))
);
```

---

### 6.5 初期データ（シードデータ）

```sql
-- デフォルトルーティングルール（UUID v7 は Rust 側で生成するため、シードでは固定 UUID 使用）
INSERT INTO routing_rules (id, caller_category, action_code, priority, is_active) VALUES
    ('019503a0-0000-7000-8000-000000000001', 'spam',       'RJ', 0, TRUE),
    ('019503a0-0000-7000-8000-000000000002', 'registered', 'VR', 0, TRUE),
    ('019503a0-0000-7000-8000-000000000003', 'unknown',    'IV', 0, TRUE),
    ('019503a0-0000-7000-8000-000000000004', 'anonymous',  'IV', 0, TRUE);

-- システム設定（1行のみ）
INSERT INTO system_settings (id) VALUES (1);
```

---

### 6.6 TTL / パーティション運用

| 対象 | TTL | 削除方式 | スケジュール |
|------|-----|---------|-------------|
| **recordings**（DB行 + S3 + ローカルファイル） | 90 日 | バッチ DELETE + S3 DeleteObject + ローカルファイル削除 | 日次 cron |
| **call_logs** パーティション | 365 日 | `DROP TABLE call_logs_YYYY_MM` | 月次 cron |
| **call_log_index** | 365 日 | `DELETE WHERE started_at < ...`（recordings CASCADE 後） | call_logs DROP 後に実行 |
| **sync_outbox** | 30 日（送信済み） | `DELETE WHERE processed_at < NOW() - 30 days` | 日次 cron |

TTL 削除の実行順序:

```
1. recordings の TTL 削除（90日超過）
   ├── S3 から録音ファイル削除
   ├── ローカルファイル削除
   └── recordings テーブルから DELETE

2. call_logs の TTL 削除（365日超過）
   ├── DROP TABLE call_logs_YYYY_MM  （パーティション丸ごと削除）
   └── DELETE FROM call_log_index WHERE started_at < ...
       └── → recordings は 90 日で先に削除済みのため CASCADE 対象なし

3. sync_outbox の掃除（30日超過の送信済み）
   └── DELETE FROM sync_outbox WHERE processed_at < NOW() - 30 days
```

---

### 6.7 マイグレーション戦略

| 項目 | 方針 |
|------|------|
| ツール | `sqlx-cli`（`sqlx migrate add / run / revert`） |
| 命名規則 | `YYYYMMDDHHMMSS_description.sql`（SQLx デフォルト） |
| ロールバック | 各マイグレーションに `DOWN` ファイルを用意 |
| Tsurugi 移行 | `phone_entries` データを `registered_numbers` + `routing_rules` に変換するワンショットスクリプト |

マイグレーションファイル構成：

```
migrations/
├── 20260206000001_create_ivr_flows.sql
├── 20260206000002_create_ivr_nodes.sql
├── 20260206000003_create_ivr_transitions.sql
├── 20260206000004_create_spam_numbers.sql
├── 20260206000005_create_registered_numbers.sql
├── 20260206000006_create_routing_rules.sql
├── 20260206000007_create_call_log_index.sql
├── 20260206000008_create_call_logs_partitioned.sql
├── 20260206000009_create_call_logs_initial_partitions.sql
├── 20260206000010_create_recordings.sql
├── 20260206000011_create_sync_outbox.sql
├── 20260206000012_create_system_settings.sql
└── 20260206000013_seed_defaults.sql
```

---

### 6.8 Rust 側アダプタ設計（概要）

> 詳細設計（DD）は本ステアリング承認後に別途作成。ここでは方針のみ記載。

#### 6.8.1 依存クレート

| クレート | 用途 |
|---------|------|
| `sqlx` (features: `runtime-tokio`, `postgres`, `uuid`, `chrono`) | PostgreSQL 接続・クエリ |
| `uuid` (features: `v7`) | UUID v7 生成 |
| `chrono` | タイムスタンプ |

#### 6.8.2 ポート（インターフェース）

```
shared/ports/
├── phone_lookup.rs         # 既存 → 4カテゴリ分類結果を返すよう拡張
├── call_repository.rs      # 新規: call_log_index + call_logs の書き込み
├── recording_repository.rs # 新規: recordings の追記 + upload_status 更新
├── sync_outbox_port.rs     # 新規: outbox への書き込み + 未送信取得
└── settings_port.rs        # 新規: system_settings の読み書き
```

#### 6.8.3 アダプタ構成

```
interface/db/
├── mod.rs
├── postgres.rs             # PostgreSQL 接続プール管理
├── phone_lookup_pg.rs      # PhoneLookupPort の PostgreSQL 実装
├── call_repo_pg.rs         # CallRepository の PostgreSQL 実装
├── recording_repo_pg.rs    # RecordingRepository の PostgreSQL 実装
├── sync_outbox_pg.rs       # SyncOutboxPort の PostgreSQL 実装
├── settings_pg.rs          # SettingsPort の PostgreSQL 実装
└── tsurugi.rs              # 削除予定
```

#### 6.8.4 トランザクション境界

| 操作 | トランザクション範囲 |
|------|---------------------|
| 通話開始記録 | `INSERT call_log_index` + `INSERT call_logs` + `INSERT sync_outbox` を同一 TX |
| 通話終了更新 | `UPDATE call_logs` + `INSERT sync_outbox` を同一 TX |
| 録音完了記録 | `INSERT recordings` + `INSERT sync_outbox` を同一 TX |
| 番号設定変更 | `UPDATE registered_numbers (version check)` + `INSERT sync_outbox` を同一 TX |

#### 6.8.5 Docker 開発環境

```yaml
# docker-compose.dev.yml に追加
services:
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_DB: voicebot
      POSTGRES_USER: voicebot
      POSTGRES_PASSWORD: voicebot_dev
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data
```

---

## 7. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #110 | STEER-110 | 起票 |
| STEER-110 | BD-004 §4 | テーブル定義の拡張・修正 |
| RD-004 FR-100〜103 | spam_numbers, registered_numbers, routing_rules | 番号振り分け要件 → テーブル |
| RD-004 FR-120〜125 | ivr_flows, ivr_nodes, ivr_transitions | IVR 要件 → テーブル |
| RD-004 FR-130〜133 | recordings, registered_numbers.recording_enabled/.announce_enabled | 録音要件 → テーブル |
| RD-004 FR-140 | call_log_index, call_logs | 発着信履歴要件 → テーブル |
| RD-004 FR-134 | system_settings.recording_retention_days | 録音保存期間要件 → 設定 |
| RD-004 NFR-100 | idx_spam_numbers, idx_registered_numbers, idx_routing_rules | 振り分け判定 100ms 以内 → インデックス |
| RD-004 NFR-101 | system_settings.max_concurrent_calls | 同時通話数 2 → 設定 |
| contract.md Call | call_logs.external_call_id | データモデル整合 |
| contract.md RecordingMeta | recordings | データモデル整合 |
| BD-004 §3.2 (アクションコード) | CHECK 制約 | コード体系の DB 強制 |

---

## 8. レビューチェックリスト

### 8.1 仕様レビュー（Review → Approved）

- [ ] 全テーブルの DDL が PostgreSQL 16+ で実行可能か
- [ ] BD-004 の既存テーブル定義との整合性（root_node_id 廃止反映）
- [ ] contract.md の Call / RecordingMeta モデルとの整合性
- [ ] RD-004 の機能要件がテーブルでカバーされているか
- [ ] E.164 CHECK 制約が正しいか
- [ ] インデックス設計が適切か（NFR-100: 振り分け判定 100ms 以内）
- [ ] NULL 許容 / NOT NULL の設計が業務ルールに合致しているか
- [ ] パーティション設計が正しく動作するか（FK 含む）
- [ ] 論理削除の部分 UNIQUE インデックスが正しいか
- [ ] Transactional Outbox のトランザクション境界が適切か
- [ ] TTL 削除順序が FK 制約に違反しないか

### 8.2 マージ前チェック（Approved → Merged）

- [ ] マイグレーションが正常に実行できる（`sqlx migrate run`）
- [ ] Rust アダプタのコンパイルが通る
- [ ] 既存テストが全て PASS
- [ ] docker-compose.dev.yml で PostgreSQL が起動する
- [ ] 初期パーティションが正しく作成される

---

## 9. 備考

### 9.1 スコープ外（別チケットで対応）

| 項目 | 理由 |
|------|------|
| Frontend DB スキーマ設計 | Frontend 側は別リポジトリ・別チケットで対応 |
| REST API エンドポイント詳細設計 | 本チケットは DB 設計のみ。API は別 DD で定義 |
| Tsurugi データ移行スクリプトの実装 | 本チケットは方針のみ。実装は Codex へ |
| pg_partman 導入 | 初期は手動 / cron でパーティション管理。必要に応じて導入 |

### 9.2 静的設定（ファイル管理）の項目

以下は DB に持たず TOML/YAML ファイルで管理する：

| 項目 | 理由 |
|------|------|
| SIP 認証情報（ユーザー名・パスワード） | 秘匿情報。Git 管理外の環境変数 or .env ファイル |
| S3 認証情報（アクセスキー・シークレット） | 同上 |
| サーバー bind アドレス・ポート | インフラ設定。起動時に固定 |
| ログレベル | 開発 / 本番で切替。環境変数 |
| PostgreSQL 接続文字列 | 同上 |

---

## 10. Resolved Questions

壁打ちで解決した全質問を記録する。

| # | 質問 | 回答 | 決定日 |
|---|------|------|--------|
| Q1 | 電話番号の正規化形式 | E.164 形式（+819012345678）。ハイフン表示は UI 層 | 2026-02-06 |
| Q2 | 1通話に複数録音を許容するか | Yes。1:N（recording_type で分類） | 2026-02-06 |
| Q3 | sync_log のリトライポリシー | sync_log 廃止 → Transactional Outbox に変更。未送信は outbox に残り続ける | 2026-02-06 |
| Q4 | PostgreSQL 接続プールサイズ | 5（Raspberry Pi 制約。リアルタイムと同期バッチで競合する場合は要調整） | 2026-02-06 |
| Q5 | SoT 決定のテーブル影響 | Call が SoT、Recording は append-only 従属。version カラム + 論理削除は設定系のみ | 2026-02-06 |
| Q6 | PK 戦略 | UUID v7（Rust 側で生成） | 2026-02-06 |
| Q7 | action_code 管理方式 | VARCHAR(2) + CHECK 制約 | 2026-02-06 |
| Q8 | 論理削除の適用範囲 | 設定系（spam_numbers, registered_numbers）のみ | 2026-02-06 |
| Q9 | 同期方式 | Transactional Outbox + synced_at 併用 | 2026-02-06 |
| Q10 | call_logs パーティション | 初期から月次レンジパーティション | 2026-02-06 |
| Q11 | IVR 循環 FK | root_node_id 廃止。parent_id IS NULL = ルート | 2026-02-06 |
| Q12 | callee_number | 廃止。system_settings で管理 | 2026-02-06 |
| Q13 | TTL 期間 | 履歴 365 日 / 録音 90 日 | 2026-02-06 |
| Q14 | システム設定テーブル | ハイブリッド（固定カラム + JSONB 拡張、単一行） | 2026-02-06 |
| Q15 | パーティション FK 問題 | call_log_index 中間テーブルで解決 | 2026-02-06 |

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-06 | 初版作成 | Claude Code (Opus 4.6) |
| 2026-02-06 | v2: 全設計判断（Q1〜Q15）反映。UUID v7、月次パーティション、Outbox、論理削除限定、中間テーブル、system_settings 等 | Claude Code (Opus 4.6) |
