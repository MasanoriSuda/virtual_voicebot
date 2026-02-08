<!-- SOURCE_OF_TRUTH: Frontend ↔ Backend API契約 -->
# Virtual Voicebot Contract v2

> STEER-112 により v1（MVP）から全面改訂。全エンティティの SoT 定義・同期方向・Public DTO を統一。

| 項目 | 値 |
|------|-----|
| ステータス | Approved |
| 作成日 | 2025-12-13（v1） |
| 改訂日 | 2026-02-08（v2.2） |
| 関連Issue | #7, #112, #138, #139 |
| 関連ステアリング | STEER-112, STEER-137, STEER-139 |

---

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
| createdAt | string (ISO8601) | Yes | 作成日時 |
| updatedAt | string (ISO8601) | Yes | 更新日時 |

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
| createdAt | string (ISO8601) | Yes | 作成日時 |
| updatedAt | string (ISO8601) | Yes | 更新日時 |

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
| POST | /api/ingest/sync | Outbox エントリの一括同期（メタデータ） |
| POST | /api/ingest/recording-file | 録音ファイルのアップロード |

#### POST /api/ingest/sync

Serversync Worker が未送信エントリを一括送信する。

**リクエスト**:
```json
{
  "entries": [
    {
      "entityType": "call_log" | "recording" | "registered_number" | "routing_rule" | "ivr_flow" | "schedule" | "announcement",
      "entityId": "019503a0-...",
      "payload": { /* 該当エンティティの DTO */ },
      "createdAt": "2026-02-07T10:00:00Z"
    }
  ]
}
```

**レスポンス**:
```json
{ "ok": true }
```

**処理内容**:
- Frontend DB に各エンティティを upsert
- `entityType` に応じたテーブルに保存
- 既存エンティティは `id` で判定して上書き

#### POST /api/ingest/recording-file

Serversync Worker が録音ファイル（実体）を転送する。

**リクエスト**: `multipart/form-data`

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

**処理内容**:
- `mixed.wav` + `meta.json` を Frontend ストレージに保存
- 保存先 URL を返却
- Backend はこの URL を `recordings.s3_url` に記録

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

### 5.4 Frontend 設定公開 API（Backend Pull 用）— Issue #138

> Backend の Serversync が Frontend PoC の設定を Pull するための一時的 API。
> 将来的には Backend DB → Frontend DB の一方向同期に移行し、これらの API は廃止される（STEER-137 参照）。

| メソッド | パス | 説明 |
|---------|------|------|
| GET | /api/number-groups | 番号グループ一覧（CallerGroup） |
| GET | /api/call-actions | 着信アクションルール一覧（IncomingRule） |
| GET | /api/ivr-flows/export | IVR フロー定義一覧（IvrFlowDefinition、Frontend JSON から取得） |

#### GET /api/number-groups

Frontend の `number-groups.json` から番号グループ一覧を返す。

**レスポンス**:
```json
{
  "ok": true,
  "callerGroups": [
    {
      "id": "019503a0-1234-7000-8000-000000000001",
      "name": "スパム",
      "description": "迷惑電話",
      "phoneNumbers": ["+819012345678", "+819087654321"],
      "createdAt": "2026-02-08T00:00:00Z",
      "updatedAt": "2026-02-08T00:00:00Z"
    }
  ]
}
```

**処理内容**:
- Frontend の JSON ファイル（`number-groups.json`）から読み取り
- Backend Serversync が `registered_numbers.group_id` / `group_name` に保存

#### GET /api/call-actions

Frontend の `call-actions.json` から着信アクションルール一覧を返す。

**レスポンス**:
```json
{
  "ok": true,
  "rules": [
    {
      "id": "019503a0-1234-7000-8000-000000000002",
      "name": "スパム拒否",
      "callerGroupId": "019503a0-1234-7000-8000-000000000001",
      "actionType": "deny",
      "actionConfig": {
        "actionCode": "BZ"
      },
      "isActive": true,
      "createdAt": "2026-02-08T00:00:00Z",
      "updatedAt": "2026-02-08T00:00:00Z"
    }
  ],
  "anonymousAction": {
    "actionType": "deny",
    "actionConfig": {
      "actionCode": "BZ"
    }
  },
  "defaultAction": {
    "actionType": "allow",
    "actionConfig": {
      "actionCode": "VR",
      "recordingEnabled": false,
      "announceEnabled": false,
      "announcementId": null
    }
  }
}
```

**処理内容**:
- Frontend の JSON ファイル（`call-actions.json`）から読み取り
- Backend Serversync が `call_action_rules` テーブルに保存
- `anonymousAction` / `defaultAction` は `system_settings.extra` (JSONB) に保存

#### GET /api/ivr-flows/export（PoC Pull 用）

Frontend の `ivr-flows.json` から IVR フロー定義一覧を返す。

> **エンドポイント分離**: セクション 5.2 の `GET /api/ivr-flows` は Backend DB から取得する CRUD API。
> 本エンドポイント `/export` は Frontend JSON から取得する Backend Pull 用 API。

**レスポンス**:
```json
{
  "ok": true,
  "flows": [
    {
      "id": "019503a0-1234-7000-8000-000000000003",
      "name": "メインメニュー",
      "description": "受付振り分け",
      "isActive": true,
      "announcementId": "019503a0-1234-7000-8000-000000000004",
      "timeoutSec": 10,
      "maxRetries": 2,
      "invalidInputAnnouncementId": null,
      "timeoutAnnouncementId": null,
      "routes": [
        {
          "dtmfKey": "1",
          "label": "営業",
          "destination": {
            "actionCode": "VR"
          }
        }
      ],
      "fallbackAction": {
        "actionCode": "VR"
      },
      "createdAt": "2026-02-08T00:00:00Z",
      "updatedAt": "2026-02-08T00:00:00Z"
    }
  ]
}
```

**処理内容**:
- Frontend の JSON ファイル（`ivr-flows.json`）から読み取り
- Backend Serversync が `ivr_nodes` + `ivr_transitions` に変換して保存

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

---

## 変更履歴

| 日付 | バージョン | 変更内容 | 作成者 |
|------|-----------|---------|--------|
| 2025-12-13 | v1.0 | 初版作成（MVP） | @MasanoriSuda |
| 2025-12-27 | v1.1 | Range 対応必須化（Issue #7） | @MasanoriSuda + Claude Code |
| 2026-02-07 | v2.0 | 全面改訂（STEER-112）：SoT 原則・全エンティティ DTO・Enum 統一・API エンドポイント一覧 | Claude Code (Opus 4.6) |
| 2026-02-08 | v2.1 | Issue #138 反映：セクション 5.4「Frontend 設定公開 API（Backend Pull 用）」追加（GET /api/number-groups, GET /api/call-actions, GET /api/ivr-flows） | Claude Code (claude-sonnet-4-5) |
| 2026-02-08 | v2.2 | Issue #139 決定反映：GET /api/ivr-flows → GET /api/ivr-flows/export に変更（既存 CRUD API と責務分離） | Claude Code (claude-sonnet-4-5) |
