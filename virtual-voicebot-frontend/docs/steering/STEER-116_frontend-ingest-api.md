# STEER-116: Frontend Ingest API 実装（Backend 同期受信側）

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-116 |
| タイトル | Frontend Ingest API 実装（Backend 同期受信側） |
| ステータス | Approved |
| 関連Issue | #116 |
| 優先度 | P0 |
| 作成日 | 2026-02-07 |

---

## 2. ストーリー（Why）

### 2.1 背景

- Backend Serversync（STEER-096）が Outbox Worker 経由で Frontend にデータを送信する
- Frontend は受信側 API（POST /api/ingest/sync, POST /api/ingest/recording-file）を実装する必要がある
- 現在の RD-005 は旧契約（POST /api/ingest/call）を参照しており、新契約への更新が必要

### 2.2 目的

- Backend Serversync からの同期データを受信し、Frontend DB に永続化する
- 録音ファイル（mixed.wav + meta.json）を受信し、Frontend ストレージに保存する
- RD-005 を新契約（contract.md §5.1）に準拠させる

### 2.3 ユーザーストーリー

```
As a システム管理者
I want Backend から送信された通話データ・録音が Frontend DB に自動保存される
So that Frontend UI で通話履歴・録音再生ができる

受入条件:
- [ ] POST /api/ingest/sync で通話ログ・録音メタデータが Frontend DB に upsert される
- [ ] POST /api/ingest/recording-file で録音ファイルが Frontend ストレージに保存される
- [ ] 保存された録音ファイルの URL が Backend に返却される
- [ ] entityType に応じて適切なテーブルに振り分けられる（call_log, recording 等）
- [ ] 既存エンティティは id で判定して上書きされる
- [ ] DB トランザクション内で upsert が行われる
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-07 |
| 起票理由 | Backend Serversync 実装完了後、Frontend 側受信 API が必要 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (Sonnet 4.5) |
| 作成日 | 2026-02-07 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Frontend Ingest API のステアリング作成" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| - | - | - | - | |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-07 |
| 承認コメント | Backend Serversync との連携仕様、Frontend DB スキーマ、録音ファイル保存処理の方針を承認。実装は Codex へ引き継ぎ。 |

### 3.5 実装

| 項目 | 値 |
|------|-----|
| 実装者 | Codex (Frontend) |
| 実装日 | |
| 指示者 | @MasanoriSuda |
| 指示内容 | |
| コードレビュー | |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | |
| マージ日 | |
| マージ先 | RD-005, DD-006 (新規), UT-006 (新規) |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| docs/requirements/RD-005_frontend.md | 修正 | §6.2 データ永続化、§6.3 依存関係を新契約に更新 |
| docs/design/detail/DD-006_ingest-api.md | 新規 | POST /api/ingest/sync, POST /api/ingest/recording-file の詳細設計 |
| docs/test/unit/UT-006_ingest-api.md | 新規 | 受信 API のユニットテスト仕様 |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| app/api/ingest/sync/route.ts | 新規 | POST /api/ingest/sync ハンドラ |
| app/api/ingest/recording-file/route.ts | 新規 | POST /api/ingest/recording-file ハンドラ（multipart） |
| lib/db/sync.ts | 新規 | Frontend DB への upsert ロジック |
| lib/storage/recording.ts | 新規 | 録音ファイル保存処理 |
| prisma/schema.prisma | 修正 | Frontend DB スキーマ（call_logs, recordings 等） |

---

## 5. 差分仕様（What / How）

### 5.1 アーキテクチャ方針

**決定事項（D-01）: Frontend DB スキーマ**

- **方針**: Backend DB とは **独立したスキーマ** を採用
- **理由**: Frontend は表示専用、Backend は SoT（Source of Truth）
- **MVP スコープ**: call_logs, recordings のみ実装
- **将来拡張**: registered_numbers, routing_rules, ivr_flows, schedules, announcements

**スキーマ設計原則**:
- Backend の DB スキーマを参考にするが、完全一致は不要
- Frontend に不要なカラムは省略可
- UI 表示に必要な正規化済みデータを格納

**決定事項（D-02）: 録音ファイル保存先**

- **MVP**: Frontend ローカルファイルシステム（`/storage/recordings/`）
- **将来**: S3/MinIO/R2 等の外部ストレージに移行可能な設計

---

### 5.2 POST /api/ingest/sync 実装

**エンドポイント**: `POST /api/ingest/sync`

**リクエスト** (contract.md §5.1 より):
```json
{
  "entries": [
    {
      "entityType": "call_log" | "recording" | "registered_number" | ...,
      "entityId": "019503a0-...",
      "payload": { /* エンティティ DTO */ },
      "createdAt": "2026-02-07T10:00:00Z"
    }
  ]
}
```

**レスポンス**:
```json
{ "ok": true }
```

**処理フロー**:
1. リクエストボディのバリデーション
2. DB トランザクション開始
3. `entries` を順次処理:
   - `entityType` に応じてテーブルを判定
   - `call_log` → `call_logs` テーブル
   - `recording` → `recordings` テーブル
   - その他（MVP 対象外）→ スキップまたはエラー
4. `payload` を該当テーブルに upsert（id で判定）
5. トランザクションコミット
6. `{ "ok": true }` を返却

**エラーハンドリング**:
- バリデーションエラー → 400 Bad Request
- DB エラー → 500 Internal Server Error（トランザクションロールバック）
- 未知の entityType → ログ出力 + スキップ（将来拡張対応）

**実装例（疑似コード）**:
```typescript
// app/api/ingest/sync/route.ts
import { NextRequest, NextResponse } from 'next/server';
import { upsertCallLog, upsertRecording } from '@/lib/db/sync';

export async function POST(req: NextRequest) {
  const { entries } = await req.json();

  // トランザクション内で一括処理
  await prisma.$transaction(async (tx) => {
    for (const entry of entries) {
      switch (entry.entityType) {
        case 'call_log':
          await upsertCallLog(tx, entry.payload);
          break;
        case 'recording':
          await upsertRecording(tx, entry.payload);
          break;
        default:
          console.warn(`Unknown entityType: ${entry.entityType}`);
      }
    }
  });

  return NextResponse.json({ ok: true });
}
```

---

### 5.3 POST /api/ingest/recording-file 実装

**エンドポイント**: `POST /api/ingest/recording-file`

**リクエスト** (contract.md §5.1 より): `multipart/form-data`

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

**処理フロー**:
1. multipart リクエストのパース
2. `callLogId`, `recordingId` の取得
3. `audio` バイナリを `/storage/recordings/{callLogId}/mixed.wav` に保存
4. `meta` JSON を `/storage/recordings/{callLogId}/meta.json` に保存
5. 保存先 URL を構築して返却

**ディレクトリ構成**:
```
virtual-voicebot-frontend/
  storage/
    recordings/
      <callLogId>/
        mixed.wav
        meta.json
```

**エラーハンドリング**:
- callLogId/recordingId 不正 → 400 Bad Request
- ファイル保存失敗 → 500 Internal Server Error
- ディスク容量不足 → 507 Insufficient Storage

**実装例（疑似コード）**:
```typescript
// app/api/ingest/recording-file/route.ts
import { NextRequest, NextResponse } from 'next/server';
import { saveRecordingFile } from '@/lib/storage/recording';
import { formDataToObject } from '@/lib/utils/formdata';

export async function POST(req: NextRequest) {
  const formData = await req.formData();

  const callLogId = formData.get('callLogId') as string;
  const recordingId = formData.get('recordingId') as string;
  const audioFile = formData.get('audio') as File;
  const metaJson = formData.get('meta') as string;

  // ファイル保存
  const fileUrl = await saveRecordingFile({
    callLogId,
    recordingId,
    audioFile,
    meta: JSON.parse(metaJson),
  });

  return NextResponse.json({ fileUrl });
}
```

---

### 5.4 Frontend DB スキーマ（MVP）

**Prisma スキーマ追加**:

```prisma
// prisma/schema.prisma

model CallLog {
  id              String    @id @default(dbgenerated("gen_random_uuid()")) @db.Uuid
  callId          String    @unique @db.VarChar(50)
  from            String    @db.VarChar(50)
  to              String    @db.VarChar(50)
  direction       String    @db.VarChar(20) // "inbound" | "outbound"
  status          String    @db.VarChar(20) // "answered" | "missed" | "busy"
  startedAt       DateTime
  endedAt         DateTime?
  durationSec     Int?
  createdAt       DateTime  @default(now())
  updatedAt       DateTime  @updatedAt

  recordings      Recording[]

  @@map("call_logs")
}

model Recording {
  id              String    @id @default(dbgenerated("gen_random_uuid()")) @db.Uuid
  callLogId       String    @db.Uuid
  recordingType   String    @db.VarChar(20) // "mixed" | "caller" | "bot"
  sequenceNumber  Int       @db.SmallInt
  filePath        String?   @db.VarChar(500)
  s3Url           String?   @db.VarChar(1000)
  uploadStatus    String    @db.VarChar(20) // "pending" | "uploaded" | "failed"
  durationSec     Int?
  format          String    @db.VarChar(10) // "wav" | "mp3"
  fileSizeBytes   BigInt?
  startedAt       DateTime
  endedAt         DateTime?
  createdAt       DateTime  @default(now())
  updatedAt       DateTime  @updatedAt

  callLog         CallLog   @relation(fields: [callLogId], references: [id])

  @@map("recordings")
}
```

**マイグレーション実行**:
```bash
cd virtual-voicebot-frontend
pnpm prisma migrate dev --name add_call_logs_recordings
```

---

### 5.5 RD-005 の更新

**修正箇所**:

1. **§6.2 データ永続化** (L213-218):

**旧**:
```markdown
- **方式**: Frontend 側で Server Sync 機構を構築
- **保存先**: Frontend ローカル DB（PostgreSQL）
- **同期タイミング**: Backend からの `POST /api/ingest/call` 受信時
```

**新**:
```markdown
- **方式**: Backend Serversync Worker が Outbox 経由で Frontend にデータ送信
- **保存先**: Frontend ローカル DB（PostgreSQL）
- **同期タイミング**: Backend Serversync が 5分間隔でポーリング + POST /api/ingest/sync 送信時
- **契約**: contract.md §5.1 参照（POST /api/ingest/sync, POST /api/ingest/recording-file）
```

2. **§6.3 依存関係** (L221-225):

**旧**:
```markdown
| 依存先 | 種別 | 説明 |
|--------|------|------|
| [contract.md](../../../docs/contract.md) | 必須 | Backend ↔ Frontend API 契約 |
| Backend `/api/ingest/call` | 必須 | 通話データ受信 API |
| 録音ファイル配信 | 必須 | recordingUrl からの音声取得 |
```

**新**:
```markdown
| 依存先 | 種別 | 説明 |
|--------|------|------|
| [contract.md](../../../docs/contract.md) | 必須 | Backend ↔ Frontend API 契約 |
| Backend Serversync (`POST /api/ingest/sync`) | 必須 | 通話データ・設定データ受信 API |
| Backend Serversync (`POST /api/ingest/recording-file`) | 必須 | 録音ファイル受信 API |
| 録音ファイル配信 (`GET /recordings/:callId/:recordingId`) | 必須 | recordingUrl からの音声取得（Range 対応） |
```

3. **§3.3 発着信履歴** (L102):

**旧**:
```markdown
| 制約 | Backend API `POST /api/ingest/call` で受信したデータを表示 |
```

**新**:
```markdown
| 制約 | Backend Serversync (`POST /api/ingest/sync`) で受信したデータを表示 |
```

---

### 5.6 詳細設計追加（DD-006_ingest-api.md へ）

**新規作成**: `docs/design/detail/DD-006_ingest-api.md`

```markdown
# DD-006: Ingest API 詳細設計

## 概要
Backend Serversync からのデータ受信 API の詳細設計

## DD-006-FN-01: POST /api/ingest/sync ハンドラ

### シグネチャ
```typescript
export async function POST(req: NextRequest): Promise<NextResponse>
```

### パラメータ
| パラメータ | 型 | 説明 |
|-----------|-----|------|
| req | NextRequest | リクエストオブジェクト |

### 処理フロー
1. リクエストボディをパース
2. DB トランザクション内で entries を upsert
3. { ok: true } を返却

### エラーケース
| エラー | 条件 | レスポンス |
|--------|------|-----------|
| バリデーションエラー | entries が配列でない | 400 Bad Request |
| DB エラー | upsert 失敗 | 500 Internal Server Error |

### トレース
- ← RD-005-FR-01
- → UT-006-TC-01

## DD-006-FN-02: POST /api/ingest/recording-file ハンドラ

### シグネチャ
```typescript
export async function POST(req: NextRequest): Promise<NextResponse>
```

### パラメータ
| パラメータ | 型 | 説明 |
|-----------|-----|------|
| req | NextRequest | multipart/form-data リクエスト |

### 処理フロー
1. multipart パース
2. 録音ファイルを /storage/recordings/{callLogId}/ に保存
3. { fileUrl } を返却

### エラーケース
| エラー | 条件 | レスポンス |
|--------|------|-----------|
| callLogId 不正 | UUID 形式でない | 400 Bad Request |
| ファイル保存失敗 | I/O エラー | 500 Internal Server Error |

### トレース
- ← RD-005-FR-02
- → UT-006-TC-02
```

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #116 | STEER-116 | 起票 |
| STEER-116 | RD-005 (修正) | 契約更新 |
| STEER-116 | DD-006 (新規) | 詳細設計 |
| DD-006-FN-01 | UT-006-TC-01 | 単体テスト |
| DD-006-FN-02 | UT-006-TC-02 | 単体テスト |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] contract.md §5.1 との整合性があるか
- [ ] Backend STEER-096 との連携が明確か
- [ ] Frontend DB スキーマが適切か
- [ ] エラーハンドリングが網羅的か
- [ ] RD-005 の修正内容が適切か

### 7.2 マージ前チェック（Approved → Merged）

- [ ] POST /api/ingest/sync 実装完了
- [ ] POST /api/ingest/recording-file 実装完了
- [ ] Frontend DB マイグレーション完了
- [ ] 録音ファイル保存処理実装完了
- [ ] Backend Serversync との疎通確認（200 OK 返却）
- [ ] RD-005 更新完了
- [ ] DD-006 作成完了

---

## 8. 備考

### 8.1 今後の拡張

- **S3 等外部ストレージ**: 現在は Frontend ローカル保存。将来は S3/MinIO/R2 に直接保存
- **認証/認可**: MVP では Backend からの POST を無条件で受け入れ。将来は API キー等で検証
- **同期エンティティ拡張**: registered_numbers, routing_rules 等の受信対応
- **リトライ通知**: Backend への同期失敗通知（現在は Backend 側でリトライ）

### 8.2 運用観点

- **ストレージ容量**: 録音ファイルが増加するため、定期的なクリーンアップが必要
- **監視**: POST /api/ingest/sync のレスポンスタイム、エラー率
- **ログ**: Frontend 側の受信ログ（`/var/log/voicebot/frontend.log`）に記録

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-07 | 初版作成 | Claude Code (Sonnet 4.5) |
| 2026-02-07 | Draft → Approved（承認者: @MasanoriSuda） | Claude Code (Sonnet 4.5) |
