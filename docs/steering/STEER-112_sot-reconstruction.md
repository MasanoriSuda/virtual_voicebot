# STEER-112: SoT 再構築（Frontend / Backend データモデル統一）

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-112 |
| タイトル | SoT 再構築（Frontend / Backend データモデル統一） |
| ステータス | Approved |
| 関連Issue | #112 |
| 優先度 | P0 |
| 作成日 | 2026-02-07 |
| 前提 | STEER-110（Backend DB 設計）Approved |

---

## 2. ストーリー（Why）

### 2.1 背景

Frontend と Backend のデータモデルに以下の矛盾・不整合が蓄積している：

| 矛盾 | 詳細 |
|------|------|
| **CallStatus 3者不一致** | contract.md: `ringing\|in_call\|ended\|error`、Frontend: `active\|completed\|failed`、Backend entity: `Setup\|Ringing\|Active\|Releasing\|Ended` |
| **ID 体系の混乱** | Frontend: `id` + `callId` + `callerNumber`（重複）。Backend: `CallId`(SIP) + `SessionId`(UUID) + DB `id`(UUID v7) + `external_call_id` |
| **IVR NodeType 不一致** | Frontend: `start\|menu\|input\|playback\|...`。Backend DB: `ANNOUNCE\|KEYPAD\|FORWARD\|...` |
| **Frontend 専用型の乱立** | `NumberGroup`, `RoutingFolder`, `Schedule`, `Announcement` 等が Backend に対応テーブルなし |
| **フィールド名不統一** | `startTime` vs `startedAt` vs `started_at`、`duration` vs `durationSec` |
| **contract.md 陳腐化** | MVP スコープのまま拡張されていない。STEER-110 の Outbox パターンと不整合 |

### 2.2 目的

1. **SoT マトリクス定義**: 全エンティティの SoT 所在と同期方向を明示
2. **contract.md v2**: Backend ↔ Frontend の API 契約を全面改訂
3. **Frontend 型定義の正規化**: Backend DB スキーマに整合する canonical 型を定義
4. **Backend DB 拡張**: `folders`, `schedules`, `schedule_time_slots`, `announcements` テーブル追加 + `call_logs.status` カラム追加

### 2.3 ユーザーストーリー

```
As a 開発チーム全体
I want to Frontend / Backend のデータモデルの単一真実源（SoT）を明確化
So that 型の矛盾・変換ミス・仕様の曖昧さを排除できる

受入条件:
- [ ] SoT マトリクスが全エンティティをカバーしている
- [ ] contract.md v2 が全 Public DTO を定義している
- [ ] Frontend canonical 型が Backend DB スキーマと整合している
- [ ] CallStatus が全層で統一されている
- [ ] IVR NodeType が全層で統一されている
- [ ] 新規テーブル（folders, schedules, announcements）の DDL が定義されている
- [ ] 同期方向・変換ルールが明示されている
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-06 |
| 起票理由 | 「フロント、バックがぐちゃぐちゃなので」 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (Opus 4.6) |
| 作成日 | 2026-02-07 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "SoT再構築を行いたい" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | @MasanoriSuda | - | - | |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-07 |
| 承認コメント | 3点の確認事項（破壊的変更・Folder汎用設計・ポリモーフィック参照）を含め承認 |

### 3.5 実装

| 項目 | 値 |
|------|-----|
| 実装者 | Codex（Backend DB マイグレーション + Frontend 型リファクタ） |
| 実装日 | - |
| 指示者 | @MasanoriSuda |
| 指示内容 | "本ステアリングに基づき Backend DB 拡張 + Frontend 型定義を修正" |
| コードレビュー | CodeRabbit (自動) |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | @MasanoriSuda |
| マージ日 | - |
| マージ先 | docs/contract.md（全面改訂）、BD-004（テーブル追加）、Frontend types.ts |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| docs/contract.md | **全面改訂** | MVP 契約 → v2（全エンティティ + SoT + 同期定義） |
| virtual-voicebot-backend/docs/design/basic/BD-004_call-routing-db.md | 修正 | folders, schedules, announcements テーブル追加、call_logs.status 追加 |
| virtual-voicebot-backend/docs/steering/STEER-110_backend-db-design.md | 参照 | 既存 11 テーブル（変更なし、追加のみ） |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| virtual-voicebot-frontend/lib/types.ts | **全面改訂** | canonical 型へリファクタ |
| virtual-voicebot-frontend/lib/api.ts | 修正 | 型変更に追従 |
| virtual-voicebot-frontend/lib/mock-data.ts | 修正 | 型変更に追従 |
| virtual-voicebot-backend/migrations/ | 追加 | 新規テーブル + call_logs.status |
| virtual-voicebot-backend/src/shared/ports/ | 追加 | SchedulePort, AnnouncementPort, FolderPort |
| virtual-voicebot-backend/src/interface/db/postgres.rs | 修正 | 新ポートの PostgreSQL 実装 |

---

## 5. 設計判断サマリ

壁打ちで確定した全設計判断を一覧化する。

| # | 論点 | 決定 | 根拠 |
|---|------|------|------|
| S-01 | CallStatus 正規値 | **`ringing \| in_call \| ended \| error`** | contract.md 準拠。call_logs に `status` VARCHAR 追加。Frontend は `active/completed/failed` を廃止 |
| S-02 | フォルダ構造 | **汎用 `folders` テーブル + 各エンティティに `folder_id` FK** | entity_type で判別。テーブル数を抑制 |
| S-03 | Schedule | **RD-004 FR-115 ベース。`schedules` + `schedule_time_slots`** | 営業時間内/外で異なるアクション |
| S-04 | Announcement | **`announcements` テーブル新設** | FR-112, FR-131 のアナウンス音声管理 |
| S-05 | Utterance | **MVP スコープ外** | contract.md 通り。Frontend の型は Future マーク |
| S-06 | SoT 原則 | **全エンティティの SoT は Backend DB** | Frontend は表示用コピー。Backend REST API 経由で CRUD |
| S-07 | IVR NodeType | **Backend DB 値に統一** | `ANNOUNCE\|KEYPAD\|FORWARD\|TRANSFER\|RECORD\|EXIT`。Frontend は表示ラベルで変換 |
| S-08 | フィールド命名 | **contract.md は camelCase、Backend DB は snake_case** | JSON API は camelCase、DB/Rust は snake_case。変換は Serde/API 層 |

---

## 6. 差分仕様（What / How）

### 6.1 SoT マトリクス

#### 6.1.1 基本原則

```
┌───────────────────────────────────────────────────────────┐
│                     SoT 原則                                │
│                                                           │
│  1. 全エンティティの SoT は Backend DB                      │
│  2. Frontend DB は表示用コピー（独自スキーマ可）            │
│  3. 設定系: Frontend → Backend REST → Backend DB            │
│           → Outbox → Frontend DB                           │
│  4. 通話系: Backend DB → Outbox → Frontend DB（一方向）    │
│  5. Backend DB の値が正。矛盾時は Backend を信頼            │
└───────────────────────────────────────────────────────────┘
```

#### 6.1.2 エンティティ別 SoT

| エンティティ | SoT | Backend DB テーブル | contract DTO | 同期方向 | CRUD 主体 |
|-------------|-----|-------------------|-------------|---------|-----------|
| **Call** | Backend DB | `call_logs` + `call_log_index` | `Call` | Backend → Frontend | Backend（SIP 処理で生成） |
| **Recording** | Backend DB | `recordings` | `Recording` | Backend → Frontend | Backend（録音エンジンで生成） |
| **SpamNumber** | Backend DB | `spam_numbers` | `SpamNumber` | 双方向 | Frontend（管理画面で CRUD） |
| **RegisteredNumber** | Backend DB | `registered_numbers` | `RegisteredNumber` | 双方向 | Frontend（管理画面で CRUD） |
| **RoutingRule** | Backend DB | `routing_rules` | `RoutingRule` | 双方向 | Frontend（管理画面で CRUD） |
| **IvrFlow** | Backend DB | `ivr_flows` | `IvrFlow` | 双方向 | Frontend（管理画面で CRUD） |
| **IvrNode** | Backend DB | `ivr_nodes` + `ivr_transitions` | `IvrNode` | 双方向 | Frontend（管理画面で CRUD） |
| **Schedule** | Backend DB | `schedules` + `schedule_time_slots` | `Schedule` | 双方向 | Frontend（管理画面で CRUD） |
| **Announcement** | Backend DB | `announcements` | `Announcement` | 双方向 | Frontend（管理画面で CRUD） |
| **Folder** | Backend DB | `folders` | `Folder` | 双方向 | Frontend（管理画面で CRUD） |
| **SystemSettings** | Backend DB | `system_settings` | `SystemSettings` | 双方向 | Frontend（管理画面で CRUD） |

#### 6.1.3 同期フロー

```
【設定系 CRUD（双方向）】

  Frontend UI
      │
      ▼ (ユーザー操作)
  Frontend App
      │
      ▼ POST/PUT/DELETE
  Backend REST API ──────────────────────┐
      │                                  │
      ▼                                  ▼
  Backend DB (SoT)          sync_outbox に INSERT
      │                     (同一 TX)
      │                                  │
      │                                  ▼
      │                     Outbox Worker (ポーリング)
      │                                  │
      │                                  ▼ POST
      │                          Frontend Ingest API
      │                                  │
      │                                  ▼
      │                          Frontend DB (コピー)
      │
      ▼ (即時レスポンス)
  Frontend UI 更新


【通話系（一方向: Backend → Frontend）】

  SIP/RTP 着信
      │
      ▼
  Backend 通話処理
      │
      ▼
  Backend DB (call_logs + recordings)
      │ + sync_outbox INSERT (同一 TX)
      │
      ▼
  Outbox Worker
      │
      ▼ POST /api/ingest/call
  Frontend Ingest API
      │
      ▼
  Frontend DB (calls + recordings)
```

---

### 6.2 Backend DB 追加テーブル DDL

> STEER-110 の 11 テーブルに **4 テーブル追加** + **1 カラム追加** + **5 カラム追加（folder_id）**。

#### 6.2.1 folders（汎用フォルダ — 木構造）

```sql
CREATE TABLE folders (
    id UUID NOT NULL PRIMARY KEY,
    parent_id UUID REFERENCES folders(id) ON DELETE CASCADE,
    entity_type VARCHAR(30) NOT NULL,
    -- 'phone_number' | 'routing_rule' | 'ivr_flow' | 'schedule' | 'announcement'
    name VARCHAR(100) NOT NULL,
    description TEXT,
    sort_order INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_folder_entity_type
        CHECK (entity_type IN (
            'phone_number', 'routing_rule', 'ivr_flow',
            'schedule', 'announcement'
        ))
);

CREATE INDEX idx_folders_parent ON folders(parent_id);
CREATE INDEX idx_folders_entity_type ON folders(entity_type);
-- 同一親フォルダ内での名前重複防止
CREATE UNIQUE INDEX uq_folders_parent_name
    ON folders(COALESCE(parent_id, '00000000-0000-0000-0000-000000000000'), entity_type, name);
```

#### 6.2.2 schedules（スケジュール — FR-115 時間帯ルーティング）

```sql
CREATE TABLE schedules (
    id UUID NOT NULL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    schedule_type VARCHAR(20) NOT NULL DEFAULT 'business',
    -- 'business' | 'holiday' | 'special' | 'override'
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    folder_id UUID REFERENCES folders(id) ON DELETE SET NULL,
    date_range_start DATE,
    -- NULL = 曜日ベース（毎週繰り返し）
    date_range_end DATE,
    action_type VARCHAR(20) NOT NULL,
    -- 'route' | 'voicemail' | 'announcement' | 'closed'
    action_target UUID,
    -- action_type に応じた参照先:
    --   route → routing_rules.id
    --   voicemail → NULL
    --   announcement → announcements.id
    --   closed → NULL
    -- FK なし（ポリモーフィック参照）
    action_code VARCHAR(2),
    -- 直接アクションコード指定（action_target の代替）
    version INT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_schedule_type
        CHECK (schedule_type IN ('business', 'holiday', 'special', 'override')),
    CONSTRAINT chk_schedule_action_type
        CHECK (action_type IN ('route', 'voicemail', 'announcement', 'closed'))
);

CREATE INDEX idx_schedules_active ON schedules(is_active) WHERE is_active;
CREATE INDEX idx_schedules_folder ON schedules(folder_id);
```

#### 6.2.3 schedule_time_slots（スケジュール時間帯）

```sql
CREATE TABLE schedule_time_slots (
    id UUID NOT NULL PRIMARY KEY,
    schedule_id UUID NOT NULL REFERENCES schedules(id) ON DELETE CASCADE,
    day_of_week SMALLINT,
    -- 0=Sunday ... 6=Saturday。NULL = date_range ベース
    start_time TIME NOT NULL,
    end_time TIME NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_day_of_week
        CHECK (day_of_week IS NULL OR (day_of_week >= 0 AND day_of_week <= 6)),
    CONSTRAINT chk_time_order
        CHECK (start_time < end_time)
);

CREATE INDEX idx_schedule_time_slots_schedule ON schedule_time_slots(schedule_id);
CREATE INDEX idx_schedule_time_slots_dow ON schedule_time_slots(day_of_week);
```

#### 6.2.4 announcements（アナウンス音声 — FR-112, FR-131）

```sql
CREATE TABLE announcements (
    id UUID NOT NULL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    announcement_type VARCHAR(20) NOT NULL DEFAULT 'custom',
    -- 'greeting' | 'hold' | 'ivr' | 'closed' | 'recording_notice' | 'custom'
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    folder_id UUID REFERENCES folders(id) ON DELETE SET NULL,
    audio_file_url TEXT,
    -- ローカル or S3 の音声ファイル URL
    tts_text TEXT,
    -- TTS 用テキスト（audio_file_url が NULL の場合に使用）
    duration_sec INT,
    language VARCHAR(10) NOT NULL DEFAULT 'ja',
    version INT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_announcement_type
        CHECK (announcement_type IN (
            'greeting', 'hold', 'ivr', 'closed', 'recording_notice', 'custom'
        )),
    CONSTRAINT chk_announcement_has_source
        CHECK (audio_file_url IS NOT NULL OR tts_text IS NOT NULL)
);

CREATE INDEX idx_announcements_type ON announcements(announcement_type);
CREATE INDEX idx_announcements_folder ON announcements(folder_id);
```

#### 6.2.5 既存テーブルへの変更

##### call_logs: status カラム追加

```sql
-- call_logs テーブルに status カラムを追加
-- ※ パーティションテーブルなので ALTER TABLE call_logs で親に追加
ALTER TABLE call_logs
    ADD COLUMN status VARCHAR(20) NOT NULL DEFAULT 'ringing';

ALTER TABLE call_logs
    ADD CONSTRAINT chk_call_status
        CHECK (status IN ('ringing', 'in_call', 'ended', 'error'));

CREATE INDEX idx_call_logs_status
    ON call_logs(status, started_at DESC);
```

##### 既存テーブル: folder_id 追加

```sql
-- 各エンティティテーブルに folder_id を追加
ALTER TABLE spam_numbers
    ADD COLUMN folder_id UUID REFERENCES folders(id) ON DELETE SET NULL;

ALTER TABLE registered_numbers
    ADD COLUMN folder_id UUID REFERENCES folders(id) ON DELETE SET NULL;

ALTER TABLE routing_rules
    ADD COLUMN folder_id UUID REFERENCES folders(id) ON DELETE SET NULL;

ALTER TABLE ivr_flows
    ADD COLUMN folder_id UUID REFERENCES folders(id) ON DELETE SET NULL;

-- schedules, announcements は CREATE TABLE で folder_id 定義済み
```

---

### 6.3 ER図（追加分）

```
┌────────────────────────────────────────────────────────────────────────┐
│                      フォルダ・スケジュール・アナウンス系                    │
│                                                                        │
│                     ┌──────────────────────┐                           │
│          ┌─────────▶│      folders         │◀──┐ (parent_id)          │
│          │          ├──────────────────────┤   │                       │
│          │          │ id (PK, UUID v7)      │───┘                       │
│          │          │ parent_id (FK, self)  │                           │
│          │          │ entity_type           │                           │
│          │          │ name                  │                           │
│          │          │ description           │                           │
│          │          │ sort_order            │                           │
│          │          │ created_at            │                           │
│          │          │ updated_at            │                           │
│          │          └──────────────────────┘                           │
│          │                    ▲                                         │
│          │   folder_id FK     │                                         │
│          │   ┌────────────────┼──────────────────────┐                 │
│          │   │                │                      │                 │
│   ┌──────┴───┴──┐   ┌────────┴─────────┐   ┌───────┴───────────┐    │
│   │ spam_numbers │   │ registered_nums  │   │ routing_rules     │    │
│   │ .folder_id   │   │ .folder_id       │   │ .folder_id        │    │
│   └──────────────┘   └──────────────────┘   └───────────────────┘    │
│                                                                        │
│   ┌──────────────────┐   ┌──────────────────────┐                     │
│   │ ivr_flows        │   │ STEER-110 既存       │                     │
│   │ .folder_id       │   │ テーブル群            │                     │
│   └──────────────────┘   └──────────────────────┘                     │
│                                                                        │
│   ┌──────────────────────┐       ┌──────────────────────┐             │
│   │     schedules        │       │    announcements      │             │
│   ├──────────────────────┤       ├──────────────────────┤             │
│   │ id (PK, UUID v7)     │       │ id (PK, UUID v7)     │             │
│   │ name                 │       │ name                  │             │
│   │ schedule_type        │       │ announcement_type     │             │
│   │ is_active            │       │ is_active             │             │
│   │ folder_id (FK)───────┼──┐    │ folder_id (FK)────────┼──┐         │
│   │ date_range_start     │  │    │ audio_file_url        │  │         │
│   │ date_range_end       │  │    │ tts_text              │  │         │
│   │ action_type          │  │    │ duration_sec          │  │         │
│   │ action_target        │  │    │ language              │  │         │
│   │ action_code          │  │    │ version               │  │         │
│   │ version              │  │    │ created_at            │  │         │
│   │ created_at           │  │    │ updated_at            │  │         │
│   │ updated_at           │  │    └──────────────────────┘  │         │
│   └──────────┬───────────┘  │                               │         │
│              │ 1:N          └─── folders ◀──────────────────┘         │
│              ▼                                                         │
│   ┌──────────────────────┐                                            │
│   │ schedule_time_slots  │                                            │
│   ├──────────────────────┤                                            │
│   │ id (PK, UUID v7)     │                                            │
│   │ schedule_id (FK)     │                                            │
│   │ day_of_week          │                                            │
│   │ start_time           │                                            │
│   │ end_time             │                                            │
│   │ created_at           │                                            │
│   └──────────────────────┘                                            │
└────────────────────────────────────────────────────────────────────────┘
```

---

### 6.4 正規 Status / Enum 定義

全層で共有する正規値を定義する。**Backend DB 値 = contract DTO 値 = Frontend 型値**。

#### 6.4.1 CallStatus

| 値 | 説明 | 遷移元 → 遷移先 |
|----|------|----------------|
| `ringing` | 着信中（呼出中） | → `in_call` or `ended` or `error` |
| `in_call` | 通話中（応答済み） | → `ended` or `error` |
| `ended` | 正常終了 | 終端 |
| `error` | エラー終了 | 終端 |

> **廃止**: Frontend の `active` / `completed` / `failed` は廃止。
> **不採用**: `missed` は独立ステータスとしない。`ended` + `answered_at IS NULL` で判定。

#### 6.4.2 CallerCategory

| 値 | 説明 |
|----|------|
| `spam` | 迷惑電話 DB 登録番号 |
| `registered` | 登録済み電話番号 |
| `unknown` | 未登録番号 |
| `anonymous` | 非通知 |

#### 6.4.3 ActionCode（基本）

| コード | 名称 |
|--------|------|
| `VB` | Voicebot（録音なし） |
| `VR` | Voicebot + Record |
| `NR` | No Response |
| `RJ` | Reject |
| `BZ` | Busy |
| `AN` | Announce（録音なし） |
| `AR` | Announce + Record |
| `VM` | Voicemail |
| `IV` | IVR |

#### 6.4.4 IVR NodeType（統一）

| Backend DB 値 | Frontend 表示ラベル | 説明 |
|--------------|-------------------|------|
| `ANNOUNCE` | アナウンス再生 | 音声ファイル / TTS を再生 |
| `KEYPAD` | メニュー選択 | DTMF 入力待ち |
| `FORWARD` | 外線転送 | 外部番号へ転送 |
| `TRANSFER` | 内線転送 | 内線番号へ転送 |
| `RECORD` | 録音 | 録音開始（留守番電話等） |
| `EXIT` | 終了 | 通話切断 |

> **廃止**: Frontend の `start` / `menu` / `input` / `playback` / `voicemail` / `hangup` / `condition` は廃止。
> `start` → ルートノードは `parent_id IS NULL` で判定（D-10）。

#### 6.4.5 EndReason

| 値 | 説明 |
|----|------|
| `normal` | 正常終了（BYE） |
| `cancelled` | 発信者キャンセル（CANCEL） |
| `rejected` | 拒否（RJ / BZ アクション） |
| `timeout` | タイムアウト |
| `error` | エラー |

#### 6.4.6 ScheduleType

| 値 | 説明 |
|----|------|
| `business` | 営業時間 |
| `holiday` | 休日 |
| `special` | 特別日 |
| `override` | 一時的上書き |

#### 6.4.7 AnnouncementType

| 値 | 説明 |
|----|------|
| `greeting` | ウェルカムメッセージ（FR-120） |
| `hold` | 保留音 |
| `ivr` | IVR メニュー音声 |
| `closed` | 営業時間外アナウンス |
| `recording_notice` | 録音通知（FR-131） |
| `custom` | カスタム |

---

### 6.5 contract.md v2（全文）

> 以下は `docs/contract.md` を置き換える全文。

```markdown
# Virtual Voicebot Contract v2

## 1. 概要

### 1.1 スコープ

- Zoiper ↔ Rust SIP/RTP voicebot backend が通話する
- Frontend は通話履歴・録音・設定管理を担当
- Backend DB が全エンティティの SoT（Single Source of Truth）

### 1.2 DB 構成

| DB | 配置 | 用途 |
|----|------|------|
| Backend DB | ローカル PostgreSQL（Raspberry Pi） | SoT。通話処理・ルーティング・録音・設定 |
| Frontend DB | 別 PostgreSQL | 表示用コピー + UI 固有データ |

同期: Backend → Frontend（Transactional Outbox + REST）

### 1.3 SoT 原則

1. 全エンティティの SoT は Backend DB
2. Frontend DB は表示用コピー（独自スキーマ可）
3. 矛盾時は Backend DB の値を信頼
4. 設定変更: Frontend → Backend REST API → Backend DB → Outbox → Frontend DB
5. 通話データ: Backend DB → Outbox → Frontend DB（一方向）

---

## 2. 正規 Enum 定義

### CallStatus
`"ringing" | "in_call" | "ended" | "error"`

### CallerCategory
`"spam" | "registered" | "unknown" | "anonymous"`

### ActionCode（基本 9 種）
`"VB" | "VR" | "NR" | "RJ" | "BZ" | "AN" | "AR" | "VM" | "IV"`

### EndReason
`"normal" | "cancelled" | "rejected" | "timeout" | "error"`

### IvrNodeType
`"ANNOUNCE" | "KEYPAD" | "FORWARD" | "TRANSFER" | "RECORD" | "EXIT"`

### ScheduleType
`"business" | "holiday" | "special" | "override"`

### AnnouncementType
`"greeting" | "hold" | "ivr" | "closed" | "recording_notice" | "custom"`

### RecordingType
`"full_call" | "ivr_segment" | "voicemail" | "transfer" | "one_way"`

### UploadStatus
`"local_only" | "uploading" | "uploaded" | "upload_failed"`

---

## 3. Public DTO（Read Model）

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

```json
{
  "id": "019503a0-1234-7000-8000-000000000001",
  "externalCallId": "c_20260207_001",
  "callerNumber": "+819012345678",
  "callerCategory": "registered",
  "actionCode": "VR",
  "status": "ended",
  "startedAt": "2026-02-07T10:00:00.000Z",
  "answeredAt": "2026-02-07T10:00:05.000Z",
  "endedAt": "2026-02-07T10:05:00.000Z",
  "durationSec": 300,
  "endReason": "normal"
}
```

### 3.2 Recording

| フィールド | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| id | string (UUID) | Yes | recordings.id |
| callLogId | string (UUID) | Yes | 紐付く Call の id |
| recordingType | RecordingType | Yes | 録音種別 |
| sequenceNumber | number | Yes | 同一通話内の録音順序 |
| recordingUrl | string | Yes | 録音ファイル URL |
| durationSec | number \| null | No | 録音時間（秒） |
| format | "wav" \| "mp3" | Yes | 音声フォーマット |
| fileSizeBytes | number \| null | No | ファイルサイズ |
| startedAt | string (ISO8601) | Yes | 録音開始日時 |
| endedAt | string (ISO8601) \| null | No | 録音終了日時 |

### 3.3 SpamNumber

| フィールド | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| id | string (UUID) | Yes | spam_numbers.id |
| phoneNumber | string | Yes | E.164 |
| reason | string \| null | No | 登録理由 |
| source | "manual" \| "import" \| "report" | Yes | 登録元 |
| folderId | string (UUID) \| null | No | フォルダ |
| createdAt | string (ISO8601) | Yes | 作成日時 |

### 3.4 RegisteredNumber

| フィールド | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| id | string (UUID) | Yes | registered_numbers.id |
| phoneNumber | string | Yes | E.164 |
| name | string \| null | No | 表示名 |
| category | CallerCategory | Yes | カテゴリ |
| actionCode | ActionCode | Yes | アクションコード |
| ivrFlowId | string (UUID) \| null | No | IVR フロー参照 |
| recordingEnabled | boolean | Yes | 録音有効 |
| announceEnabled | boolean | Yes | 録音通知有効 |
| notes | string \| null | No | メモ |
| folderId | string (UUID) \| null | No | フォルダ |
| version | number | Yes | 楽観ロック |
| createdAt | string (ISO8601) | Yes | 作成日時 |
| updatedAt | string (ISO8601) | Yes | 更新日時 |

### 3.5 RoutingRule

| フィールド | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| id | string (UUID) | Yes | routing_rules.id |
| callerCategory | CallerCategory | Yes | 対象カテゴリ |
| actionCode | ActionCode | Yes | アクションコード |
| ivrFlowId | string (UUID) \| null | No | IVR フロー参照 |
| priority | number | Yes | 優先度 |
| isActive | boolean | Yes | 有効フラグ |
| folderId | string (UUID) \| null | No | フォルダ |
| version | number | Yes | 楽観ロック |
| createdAt | string (ISO8601) | Yes | 作成日時 |
| updatedAt | string (ISO8601) | Yes | 更新日時 |

### 3.6 IvrFlow

| フィールド | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| id | string (UUID) | Yes | ivr_flows.id |
| name | string | Yes | フロー名 |
| description | string \| null | No | 説明 |
| isActive | boolean | Yes | 有効フラグ |
| folderId | string (UUID) \| null | No | フォルダ |
| nodes | IvrNode[] | Yes | ノード一覧（展開時のみ） |
| createdAt | string (ISO8601) | Yes | 作成日時 |
| updatedAt | string (ISO8601) | Yes | 更新日時 |

### 3.7 IvrNode

| フィールド | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| id | string (UUID) | Yes | ivr_nodes.id |
| flowId | string (UUID) | Yes | 所属フロー |
| parentId | string (UUID) \| null | Yes | 親ノード。null = ルート |
| nodeType | IvrNodeType | Yes | ノード種別 |
| actionCode | string \| null | No | アクションコード |
| audioFileUrl | string \| null | No | 音声ファイル URL |
| ttsText | string \| null | No | TTS テキスト |
| timeoutSec | number | Yes | タイムアウト秒数 |
| maxRetries | number | Yes | リトライ上限 |
| depth | number | Yes | 階層深度（0〜3） |
| exitAction | string | Yes | リトライ超過時アクション |
| transitions | IvrTransition[] | No | 遷移定義（展開時のみ） |

### 3.8 IvrTransition

| フィールド | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| id | string (UUID) | Yes | ivr_transitions.id |
| fromNodeId | string (UUID) | Yes | 遷移元ノード |
| inputType | "DTMF" \| "TIMEOUT" \| "INVALID" \| "COMPLETE" | Yes | 入力種別 |
| dtmfKey | string \| null | No | DTMF キー |
| toNodeId | string (UUID) \| null | No | 遷移先ノード |

### 3.9 Schedule

| フィールド | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| id | string (UUID) | Yes | schedules.id |
| name | string | Yes | スケジュール名 |
| description | string \| null | No | 説明 |
| scheduleType | ScheduleType | Yes | 種別 |
| isActive | boolean | Yes | 有効フラグ |
| folderId | string (UUID) \| null | No | フォルダ |
| dateRangeStart | string (date) \| null | No | 開始日 |
| dateRangeEnd | string (date) \| null | No | 終了日 |
| actionType | "route" \| "voicemail" \| "announcement" \| "closed" | Yes | アクション種別 |
| actionTarget | string (UUID) \| null | No | アクション先 |
| actionCode | string \| null | No | 直接アクションコード |
| timeSlots | ScheduleTimeSlot[] | Yes | 時間帯一覧 |
| version | number | Yes | 楽観ロック |

### 3.10 ScheduleTimeSlot

| フィールド | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| id | string (UUID) | Yes | schedule_time_slots.id |
| dayOfWeek | number \| null | No | 0=Sun ... 6=Sat |
| startTime | string (HH:mm) | Yes | 開始時刻 |
| endTime | string (HH:mm) | Yes | 終了時刻 |

### 3.11 Announcement

| フィールド | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| id | string (UUID) | Yes | announcements.id |
| name | string | Yes | アナウンス名 |
| description | string \| null | No | 説明 |
| announcementType | AnnouncementType | Yes | 種別 |
| isActive | boolean | Yes | 有効フラグ |
| folderId | string (UUID) \| null | No | フォルダ |
| audioFileUrl | string \| null | No | 音声 URL |
| ttsText | string \| null | No | TTS テキスト |
| durationSec | number \| null | No | 再生時間 |
| language | string | Yes | 言語 (default: "ja") |
| version | number | Yes | 楽観ロック |

### 3.12 Folder

| フィールド | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| id | string (UUID) | Yes | folders.id |
| parentId | string (UUID) \| null | Yes | 親フォルダ。null = ルート |
| entityType | string | Yes | 所属エンティティ種別 |
| name | string | Yes | フォルダ名 |
| description | string \| null | No | 説明 |
| sortOrder | number | Yes | 並び順 |

### 3.13 SystemSettings

| フィールド | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| recordingRetentionDays | number | Yes | 録音保存日数 (default: 90) |
| historyRetentionDays | number | Yes | 履歴保存日数 (default: 365) |
| syncEndpointUrl | string \| null | No | Frontend API URL |
| defaultActionCode | ActionCode | Yes | デフォルトアクション |
| maxConcurrentCalls | number | Yes | 同時通話上限 |
| extra | object | Yes | 拡張設定 (JSONB) |
| version | number | Yes | 楽観ロック |

---

## 4. Correlation ID Rules

| レイヤー | ID | 説明 |
|---------|-----|------|
| DB PK | `id` (UUID v7) | 全テーブル共通の内部 PK |
| 通話相関 | `externalCallId` | アプリ層で生成する通話識別子 |
| SIP 相関 | `sipCallId` | SIP Call-ID ヘッダ値 |
| 同期相関 | `sync_outbox.id` | Outbox エントリの BIGSERIAL |

- 外部 DTO は `id`（UUID v7）を唯一の識別子とする
- `externalCallId` は表示・検索用の補助キー
- `sipCallId` は内部デバッグ用（外部 DTO では省略可）

---

## 5. API エンドポイント

### 5.1 Backend → Frontend（Sync / Ingest）

| メソッド | パス | 説明 |
|---------|------|------|
| POST | /api/ingest/call | 通話終了時のデータ投入 |
| POST | /api/ingest/sync | Outbox エントリの一括同期 |

#### POST /api/ingest/call

Backend が通話終了後に送信する。

```json
{
  "call": { /* Call DTO */ },
  "recordings": [ /* Recording DTO[] */ ]
}
```

#### POST /api/ingest/sync

Outbox Worker が未送信エントリを一括送信する。

```json
{
  "entries": [
    {
      "entityType": "registered_number",
      "entityId": "019503a0-...",
      "payload": { /* 該当エンティティの DTO */ },
      "createdAt": "2026-02-07T10:00:00Z"
    }
  ]
}
```

### 5.2 Frontend → Backend（CRUD）

| メソッド | パス | 説明 |
|---------|------|------|
| GET | /api/settings | システム設定取得 |
| PUT | /api/settings | システム設定更新 |
| GET | /api/spam-numbers | 迷惑番号一覧 |
| POST | /api/spam-numbers | 迷惑番号登録 |
| DELETE | /api/spam-numbers/:id | 迷惑番号削除（論理） |
| GET | /api/registered-numbers | 登録番号一覧 |
| POST | /api/registered-numbers | 登録番号追加 |
| PUT | /api/registered-numbers/:id | 登録番号更新 |
| DELETE | /api/registered-numbers/:id | 登録番号削除（論理） |
| GET | /api/routing-rules | ルーティングルール一覧 |
| POST | /api/routing-rules | ルーティングルール追加 |
| PUT | /api/routing-rules/:id | ルーティングルール更新 |
| DELETE | /api/routing-rules/:id | ルーティングルール削除 |
| GET | /api/ivr-flows | IVR フロー一覧 |
| POST | /api/ivr-flows | IVR フロー作成 |
| PUT | /api/ivr-flows/:id | IVR フロー更新 |
| DELETE | /api/ivr-flows/:id | IVR フロー削除 |
| GET | /api/schedules | スケジュール一覧 |
| POST | /api/schedules | スケジュール作成 |
| PUT | /api/schedules/:id | スケジュール更新 |
| DELETE | /api/schedules/:id | スケジュール削除 |
| GET | /api/announcements | アナウンス一覧 |
| POST | /api/announcements | アナウンス作成 |
| PUT | /api/announcements/:id | アナウンス更新 |
| DELETE | /api/announcements/:id | アナウンス削除 |
| GET | /api/folders | フォルダ一覧 |
| POST | /api/folders | フォルダ作成 |
| PUT | /api/folders/:id | フォルダ更新 |
| DELETE | /api/folders/:id | フォルダ削除 |
| GET | /recordings/:callId/:recordingId | 録音ファイル取得（Range 対応） |

### 5.3 録音ファイル配信

- `GET /recordings/:callId/:recordingId` → `audio/wav` バイト列
- **Range 対応必須**（`Accept-Ranges: bytes`, `206 Partial Content`）
- 不正 Range → `416 Range Not Satisfiable`

---

## 6. Error Format

### REST Error

```json
{
  "error": {
    "code": "NOT_FOUND",
    "message": "callId not found",
    "requestId": "req_xxx"
  }
}
```

### Optimistic Lock Error

```json
{
  "error": {
    "code": "CONFLICT",
    "message": "version mismatch: expected 3, got 2",
    "requestId": "req_xxx"
  }
}
```

---

## 7. Auth (MVP)

- MVP は認証なし（ローカル / 閉域想定）
- 将来 `Authorization: Bearer ...` 導入可能性あり

---

## 8. Non-Goals (MVP)

- Utterance（発話テキスト）— Future
- ライブ中継 / 逐次発話表示 — Future
- AI 自動迷惑判定 — Future
- 発信機能（UAC） — Future
- 複数回線対応 — Future
- 署名付き URL / 認可厳密化 — Future
```

---

### 6.6 Frontend Canonical 型定義

> 以下は `virtual-voicebot-frontend/lib/types.ts` のあるべき姿。実装変更は Codex へ引き継ぎ。

```typescript
// ============================================================
// Canonical Types — contract.md v2 準拠
// Backend DB スキーマと 1:1 対応
// ============================================================

// --- Enums ---

export type CallStatus = "ringing" | "in_call" | "ended" | "error"

export type CallerCategory = "spam" | "registered" | "unknown" | "anonymous"

export type ActionCode = "VB" | "VR" | "NR" | "RJ" | "BZ" | "AN" | "AR" | "VM" | "IV"

export type EndReason = "normal" | "cancelled" | "rejected" | "timeout" | "error"

export type IvrNodeType = "ANNOUNCE" | "KEYPAD" | "FORWARD" | "TRANSFER" | "RECORD" | "EXIT"

export type IvrInputType = "DTMF" | "TIMEOUT" | "INVALID" | "COMPLETE"

export type RecordingType = "full_call" | "ivr_segment" | "voicemail" | "transfer" | "one_way"

export type UploadStatus = "local_only" | "uploading" | "uploaded" | "upload_failed"

export type ScheduleType = "business" | "holiday" | "special" | "override"

export type AnnouncementType = "greeting" | "hold" | "ivr" | "closed" | "recording_notice" | "custom"

export type ScheduleActionType = "route" | "voicemail" | "announcement" | "closed"

export type FolderEntityType = "phone_number" | "routing_rule" | "ivr_flow" | "schedule" | "announcement"

// --- Core DTOs ---

export interface Call {
  id: string
  externalCallId: string
  callerNumber: string | null
  callerCategory: CallerCategory
  actionCode: string
  status: CallStatus
  startedAt: string           // ISO8601
  answeredAt: string | null
  endedAt: string | null
  durationSec: number | null
  endReason: EndReason
}

export interface Recording {
  id: string
  callLogId: string
  recordingType: RecordingType
  sequenceNumber: number
  recordingUrl: string
  durationSec: number | null
  format: "wav" | "mp3"
  fileSizeBytes: number | null
  startedAt: string
  endedAt: string | null
}

// --- Settings DTOs ---

export interface SpamNumber {
  id: string
  phoneNumber: string         // E.164
  reason: string | null
  source: "manual" | "import" | "report"
  folderId: string | null
  createdAt: string
}

export interface RegisteredNumber {
  id: string
  phoneNumber: string         // E.164
  name: string | null
  category: CallerCategory
  actionCode: ActionCode
  ivrFlowId: string | null
  recordingEnabled: boolean
  announceEnabled: boolean
  notes: string | null
  folderId: string | null
  version: number
  createdAt: string
  updatedAt: string
}

export interface RoutingRule {
  id: string
  callerCategory: CallerCategory
  actionCode: ActionCode
  ivrFlowId: string | null
  priority: number
  isActive: boolean
  folderId: string | null
  version: number
  createdAt: string
  updatedAt: string
}

// --- IVR DTOs ---

export interface IvrFlow {
  id: string
  name: string
  description: string | null
  isActive: boolean
  folderId: string | null
  nodes: IvrNode[]
  createdAt: string
  updatedAt: string
}

export interface IvrNode {
  id: string
  flowId: string
  parentId: string | null     // null = root node
  nodeType: IvrNodeType
  actionCode: string | null
  audioFileUrl: string | null
  ttsText: string | null
  timeoutSec: number
  maxRetries: number
  depth: number               // 0-3
  exitAction: string
  transitions: IvrTransition[]
}

export interface IvrTransition {
  id: string
  fromNodeId: string
  inputType: IvrInputType
  dtmfKey: string | null
  toNodeId: string | null
}

// --- Schedule DTOs ---

export interface Schedule {
  id: string
  name: string
  description: string | null
  scheduleType: ScheduleType
  isActive: boolean
  folderId: string | null
  dateRangeStart: string | null   // YYYY-MM-DD
  dateRangeEnd: string | null
  actionType: ScheduleActionType
  actionTarget: string | null
  actionCode: string | null
  timeSlots: ScheduleTimeSlot[]
  version: number
}

export interface ScheduleTimeSlot {
  id: string
  dayOfWeek: number | null    // 0=Sun ... 6=Sat
  startTime: string           // HH:mm
  endTime: string             // HH:mm
}

// --- Announcement DTOs ---

export interface Announcement {
  id: string
  name: string
  description: string | null
  announcementType: AnnouncementType
  isActive: boolean
  folderId: string | null
  audioFileUrl: string | null
  ttsText: string | null
  durationSec: number | null
  language: string
  version: number
}

// --- Folder DTO ---

export interface Folder {
  id: string
  parentId: string | null
  entityType: FolderEntityType
  name: string
  description: string | null
  sortOrder: number
}

// --- System Settings DTO ---

export interface SystemSettings {
  recordingRetentionDays: number
  historyRetentionDays: number
  syncEndpointUrl: string | null
  defaultActionCode: ActionCode
  maxConcurrentCalls: number
  extra: Record<string, unknown>
  version: number
}

// --- Future (MVP スコープ外) ---

// Utterance / WebSocketMessage は MVP では定義しない。
// 必要性が出た時点で contract.md v3 で追加する。
```

---

### 6.7 廃止対象（Frontend 現行型からの削除/変更）

| 現行 Frontend 型 | 対応 | 理由 |
|-----------------|------|------|
| `CallStatus = "active" \| "completed" \| "failed"` | → `"ringing" \| "in_call" \| "ended" \| "error"` | S-01 |
| `Call.id` + `Call.callId`（重複） | → `Call.id` + `Call.externalCallId` | ID 体系統一 |
| `Call.from` + `Call.callerNumber`（重複） | → `Call.callerNumber` のみ | 重複排除 |
| `Call.startTime` | → `Call.startedAt` | フィールド名統一 |
| `Call.duration` + `Call.durationSec`（重複） | → `Call.durationSec` のみ | 重複排除 |
| `Call.to` | 廃止 | D-11: callee_number 廃止。system_settings で管理 |
| `Call.summary` | 廃止（Future） | Utterance と同様 MVP 外 |
| `Call.direction` | 廃止 | 現行は着信のみ。発信は Non-Goals |
| `Call.fromName` | 廃止 | Backend DB に対応カラムなし。registered_numbers.name で別途取得 |
| `CallDetail` | 廃止 | `Call` + `Recording[]` + `Utterance[]` を個別に取得 |
| `Utterance` / `WebSocketMessage` | Future マーク | MVP スコープ外 |
| `NumberGroup` | → `Folder` (entityType: "phone_number") | S-02 |
| `RoutingFolder` | → `Folder` (entityType: "routing_rule") | S-02 |
| `IvrFolder` | → `Folder` (entityType: "ivr_flow") | S-02 |
| `ScheduleFolder` | → `Folder` (entityType: "schedule") | S-02 |
| `AnnouncementFolder` | → `Folder` (entityType: "announcement") | S-02 |
| `IvrNodeType = "start" \| "menu" \| ...` | → `"ANNOUNCE" \| "KEYPAD" \| ...` | S-07 |
| `RoutingRuleType` | 廃止 | Backend DB に対応なし。`callerCategory` + `actionCode` で表現 |
| `PhoneNumber` | → `SpamNumber` / `RegisteredNumber` に分離 | Backend DB 準拠 |

---

### 6.8 マイグレーション追加ファイル

STEER-110 の 13 マイグレーションに追加：

```
migrations/
├── ... (STEER-110 の 001-013) ...
├── 20260207000001_create_folders.sql
├── 20260207000002_create_schedules.sql
├── 20260207000003_create_schedule_time_slots.sql
├── 20260207000004_create_announcements.sql
├── 20260207000005_add_folder_id_to_entities.sql
└── 20260207000006_add_status_to_call_logs.sql
```

---

## 7. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #112 | STEER-112 | 起票 |
| STEER-112 | docs/contract.md v2 | 全面改訂 |
| STEER-112 | BD-004 | テーブル追加 |
| STEER-110 D-01〜D-13 | STEER-112 SoT マトリクス | Backend DB 設計の統合 |
| RD-004 FR-115 | schedules + schedule_time_slots | 時間帯ルーティング |
| RD-004 FR-112 | announcements | お断りアナウンス |
| RD-004 FR-131 | announcements (recording_notice) | 録音通知アナウンス |
| RD-004 FR-120 | announcements (greeting) | ウェルカムメッセージ |
| contract.md v1 (Call) | contract.md v2 (Call) | DTO 拡張・正規化 |
| contract.md v1 (RecordingMeta) | contract.md v2 (Recording) | DTO 拡張・正規化 |

---

## 8. レビューチェックリスト

### 8.1 仕様レビュー（Review → Approved）

- [ ] SoT マトリクスが全エンティティをカバーしているか
- [ ] CallStatus が contract/Frontend/Backend で統一されているか
- [ ] IVR NodeType が contract/Frontend/Backend で統一されているか
- [ ] 新規テーブル DDL が PostgreSQL 16+ で実行可能か
- [ ] contract.md v2 の全 DTO が Backend DB スキーマと整合しているか
- [ ] Frontend canonical 型が contract.md v2 と一致しているか
- [ ] 同期方向（SoT → コピー）が矛盾なく定義されているか
- [ ] 廃止対象リストに漏れがないか
- [ ] STEER-110 との整合性（追加のみ、矛盾なし）

### 8.2 マージ前チェック（Approved → Merged）

- [ ] Backend マイグレーション追加分が正常に実行できる
- [ ] Frontend types.ts が canonical 型に更新されている
- [ ] docs/contract.md が v2 に置き換えられている
- [ ] 既存テストが全て PASS

---

## 9. 備考

### 9.1 スコープ外（別チケットで対応）

| 項目 | 理由 |
|------|------|
| Frontend DB スキーマ詳細設計 | Frontend 側で別途設計（本チケットは contract と型のみ） |
| Backend REST API 詳細設計（DD） | 本チケットはエンドポイント一覧のみ。パラメータ・バリデーション詳細は DD |
| Utterance / WebSocket 対応 | MVP スコープ外 |
| API 認証・認可 | MVP スコープ外 |

### 9.2 contract.md v1 からの破壊的変更

| v1 | v2 | 影響 |
|----|-----|------|
| `Call.status: "ringing" \| "in_call" \| "ended" \| "error"` | 変更なし | - |
| `Call.from` / `Call.to` (SIP URI) | 廃止。`callerNumber` (E.164) | SIP URI → E.164 |
| `Call.summary` | 廃止（Future） | Frontend のサマリ表示を削除 |
| `Call.recordingUrl` (直持ち) | 廃止。`Recording[]` で分離 | 1:N モデル（D-04） |
| `RecordingMeta.path` | → `Recording.recordingUrl` | フィールド名変更 |
| `RecordingMeta.mixedUrl` | 廃止。`recordingUrl` に統一 | 重複排除 |
| `POST /api/ingest/call` payload | 拡張（Call + Recording[]） | 後方互換なし |

---

## 10. Resolved Questions

| # | 質問 | 回答 | 決定日 |
|---|------|------|--------|
| S-Q1 | CallStatus の正規値 | `ringing \| in_call \| ended \| error` (contract.md 準拠) | 2026-02-07 |
| S-Q2 | フォルダ DB 構造 | 汎用 folders テーブル + folder_id FK | 2026-02-07 |
| S-Q3 | Schedule テーブル設計 | RD-004 FR-115 ベース。schedules + schedule_time_slots | 2026-02-07 |
| S-Q4 | Utterance の SoT | MVP スコープ外 | 2026-02-07 |
| S-Q5 | Frontend / Backend DB の関係 | 別スキーマ・別インスタンス。Backend が SoT | 2026-02-07 |
| S-Q6 | STEER-112 配置先 | ルート docs/steering/ | 2026-02-07 |

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-07 | 初版作成 | Claude Code (Opus 4.6) |
| 2026-02-07 | Review → Approved | @MasanoriSuda |
