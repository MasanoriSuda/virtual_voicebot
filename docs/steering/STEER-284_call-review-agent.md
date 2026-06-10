# STEER-284: 電話応対AIの通話後レビューエージェント

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-284 |
| タイトル | 電話応対AIの通話後レビューエージェント |
| ステータス | Approved |
| 関連Issue | #284 |
| 優先度 | P1 |
| 作成日 | 2026-06-11 |

---

## 2. ストーリー（Why）

### 2.1 背景

- 現行システムは通話録音を `mixed.wav` として保存し、Backend `sync_outbox` から Frontend へ通話ログ・録音メタデータ・録音ファイルを同期できる。
- Frontend の通話詳細には既に「録音」「文字起こし」「要約」タブがあるが、実データとしての文字起こし・要約・業務判断は未整備である。
- 録音を人間が聞き直す運用は時間がかかり、折り返し要否、未解決事項、クレーム予兆、回答の曖昧さを素早く把握できない。
- Issue #284 では、Zenn / AmiVoice 企画向けに「AmiVoice API と LLM で、電話応対ログを『聞く録音』から『判断できる通話カルテ』に変える」体験を作る方針が合意された。

### 2.2 目的

通話終了後に録音ファイルを AmiVoice API で文字起こしし、その結果を LLM で業務レビューに変換して、Frontend の通話詳細で確認できるようにする。

達成したい価値は以下とする。

1. **聞き直し負荷の削減**: 録音を開かなくても通話内容・要点・未解決事項を把握できる。
2. **業務判断の可視化**: 折り返し要否、クレーム予兆、予約変更、回答曖昧などの運用判断を表示する。
3. **根拠付きレビュー**: LLM の判断に証拠発話とタイムスタンプを添え、確認可能な形にする。
4. **既存同期との整合**: Backend DB を SoT とし、review 結果を outbox 経由で Frontend 表示用コピーへ同期する。

### 2.3 ユーザーストーリー

```
As a 電話応対システムの運用者
I want to 通話終了後に録音の文字起こし・要約・レビュー結果を自動生成したい
So that 録音をすべて聞かなくても、対応品質と次アクションを判断できる

受入条件:
- [ ] AC-1: 録音済み通話に対して、AmiVoice API による文字起こし結果が保存される
- [ ] AC-2: LLM により、通話概要・顧客の用件・未解決事項・次アクション・クレーム予兆が構造化 JSON として保存される
- [ ] AC-3: Frontend の通話詳細 Drawer に「レビュー」タブが追加され、レビュー結果を表示できる
- [ ] AC-4: レビュー結果には、判断根拠となる発話テキストとタイムスタンプが含まれる
- [ ] AC-5: 文字起こしまたはレビュー生成が失敗しても、通話ログ・録音ファイル同期は失敗扱いにしない
- [ ] AC-6: API キー、個人情報、全文音声データの内容をログに出力しない
- [ ] AC-7: `POST_CALL_REVIEW_ENABLED=false` の場合、既存の通話・録音同期挙動は変わらない
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-06-11 |
| 起票理由 | AmiVoice API と LLM を使い、通話録音を運用判断に使える通話カルテへ変換するため |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Codex (GPT-5) |
| 作成日 | 2026-06-11 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Issue #284 の電話応対AIの通話後レビューエージェントのステアリングファイル作成" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | @MasanoriSuda | 2026-06-11 | OK | Draft 内容を確認し「良いと思います」とコメント。OQ 方針も承認 |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-06-11 |
| 承認コメント | Open Questions の推奨方針で進める |

### 3.5 実装（該当する場合）

| 項目 | 値 |
|------|-----|
| 実装者 | Codex (GPT-5) |
| 実装日 | 2026-06-11 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "STEER-284 を Approved にして実装に入る" |
| コードレビュー | - |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | - |
| マージ日 | - |
| マージ先 | docs/contract.md, Backend RD/DD/UT, Frontend RD/DD/UT |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| `docs/contract.md` | 修正 | `recording` sync payload に通話後レビュー結果フィールドを追加 |
| `virtual-voicebot-backend/docs/requirements/RD-001_product.md` | 修正 | 通話後レビューエージェント要件を追加 |
| `virtual-voicebot-backend/docs/design/detail/DD-007_recording.md` | 修正 | 録音アップロード後の文字起こし・レビュー処理を追記 |
| `virtual-voicebot-backend/docs/test/unit/UT-xxx.md` | 追加 | review worker / AmiVoice client / LLM JSON parse の単体テスト |
| `virtual-voicebot-frontend/docs/requirements/RD-005_frontend.md` | 修正 | 通話詳細 Drawer のレビュー表示要件を追加 |
| `virtual-voicebot-frontend/docs/design/detail/DD-xxx.md` | 追加 | `CallReviewTab` 表示設計を追加 |

### 4.2 影響するコード

#### Backend

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| `virtual-voicebot-backend/migrations/YYYYMMDD_add_post_call_review_to_recordings.sql` | 追加 | `recordings` に transcript / summary / review / review status カラムを追加 |
| `virtual-voicebot-backend/src/shared/config/mod.rs` | 修正 | AmiVoice と通話後レビュー用の環境変数を追加 |
| `virtual-voicebot-backend/src/service/ai/post_call_review.rs` | 追加 | AmiVoice 文字起こし結果を LLM レビュー JSON に変換するユースケース |
| `virtual-voicebot-backend/src/service/ai/amivoice.rs` | 追加 | AmiVoice API クライアント |
| `virtual-voicebot-backend/src/interface/sync/worker.rs` | 修正 | `recording_file` upload 後に review worker を起動し、結果を outbox へ enqueue |
| `virtual-voicebot-backend/src/interface/db/postgres.rs` | 修正 | review 結果保存、`recording` sync payload 更新 |
| `virtual-voicebot-backend/src/shared/ports/sync_outbox_port.rs` | 修正 | review 結果保存と updated `recording` outbox enqueue のポートを追加 |

#### Frontend

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| `virtual-voicebot-frontend/lib/types.ts` | 修正 | `CallReview`, `CallReviewEvidence`, `CallDetail.review` を追加 |
| `virtual-voicebot-frontend/lib/db/sync.ts` | 修正 | `StoredRecording` に review fields を追加し、`recording` payload から normalize |
| `virtual-voicebot-frontend/lib/api.ts` | 修正 | `getCallDetail()` で review を返却 |
| `virtual-voicebot-frontend/components/calls/call-detail-drawer.tsx` | 修正 | 「レビュー」タブを追加 |
| `virtual-voicebot-frontend/components/calls/call-review-tab.tsx` | 追加 | 通話カルテ表示コンポーネント |

---

## 5. 差分仕様（What / How）

### 5.1 処理全体

MVP ではリアルタイム処理ではなく、通話終了後のバッチ処理として実装する。

```text
通話終了
  → Backend が call_log / recording / recording_file を sync_outbox に enqueue
  → serversync が recording_file を Frontend へ upload
  → Backend が recordings.s3_url を更新
  → post-call review worker が録音音声を AmiVoice API へ送信
  → AmiVoice transcript を LLM に渡して review JSON を生成
  → Backend recordings に transcript / summary / review を保存
  → Backend が updated recording payload を sync_outbox に enqueue
  → Frontend が recording payload を upsert
  → 通話詳細 Drawer で文字起こし・要約・レビューを表示
```

#### 非目標

- 通話中リアルタイム字幕は本 Issue の対象外。
- AI 応答生成パイプライン（既存 ASR → LLM → TTS）の挙動変更は対象外。
- 録音保存・upload の既存成功条件は変更しない。

### 5.2 Backend: DB スキーマ追加

`recordings` は既に `summaryText` / `transcriptJson` を Frontend 側で受けられる形になっているため、Backend 側の SoT カラムを追加する。

```sql
ALTER TABLE recordings
  ADD COLUMN transcript_json JSONB,
  ADD COLUMN summary_text TEXT,
  ADD COLUMN review_json JSONB,
  ADD COLUMN review_status VARCHAR(20) NOT NULL DEFAULT 'pending',
  ADD COLUMN review_error TEXT,
  ADD COLUMN reviewed_at TIMESTAMPTZ;

ALTER TABLE recordings
  ADD CONSTRAINT chk_recording_review_status
  CHECK (review_status IN ('pending', 'processing', 'completed', 'failed', 'skipped'));

CREATE INDEX idx_recordings_review_pending
  ON recordings(created_at)
  WHERE review_status IN ('pending', 'failed') AND upload_status = 'uploaded';
```

#### review_status の意味

| 値 | 意味 |
|----|------|
| `pending` | レビュー未処理 |
| `processing` | AmiVoice / LLM 処理中 |
| `completed` | transcript / review 生成完了 |
| `failed` | 処理失敗。録音同期自体は成功扱い |
| `skipped` | `POST_CALL_REVIEW_ENABLED=false` または対象外 |

### 5.3 Backend: 環境変数

| 環境変数 | 必須 | デフォルト | 説明 |
|----------|------|------------|------|
| `POST_CALL_REVIEW_ENABLED` | No | `false` | 通話後レビュー処理を有効化 |
| `POST_CALL_REVIEW_AUTO_RUN` | No | `true` | `recording_file` upload 成功後に自動実行 |
| `AMIVOICE_API_KEY` | Yes when enabled | なし | AmiVoice API 認証キー。ログ出力禁止 |
| `AMIVOICE_BASE_URL` | No | `https://acp-api.amivoice.com/v1` | AmiVoice API base URL |
| `AMIVOICE_ENGINE` | No | `-a-general` | AmiVoice 認識エンジン指定 |
| `AMIVOICE_TIMEOUT_MS` | No | `30000` | 文字起こし API timeout |
| `POST_CALL_REVIEW_LLM_MODEL` | No | 既存 LLM 設定に従う | review JSON 生成で使うモデル |
| `POST_CALL_REVIEW_TIMEOUT_MS` | No | `60000` | 文字起こし + LLM 全体 timeout |

### 5.4 Backend: AmiVoice 文字起こし

MVP では AmiVoice の同期 HTTP 文字起こしを利用する。

```text
POST {AMIVOICE_BASE_URL}/recognize
multipart/form-data:
  u = AMIVOICE_API_KEY
  d = AMIVOICE_ENGINE
  a = mixed.wav
```

処理ルール:

- 音声入力は `mixed.wav` を使用する。
- `recording_file` upload 後、Backend にローカルファイルが残っていればローカルパスを使う。
- ローカルファイルがない場合は `FRONTEND_BASE_URL/api/recordings/{callLogId}` から取得する。
- AmiVoice の生レスポンスはそのままログに出さない。
- transcript は Frontend 表示用に以下の正規形へ変換する。

```json
{
  "provider": "amivoice",
  "language": "ja-JP",
  "utterances": [
    {
      "seq": 1,
      "speaker": "unknown",
      "text": "配送状況を確認したいのですが",
      "timestamp": "2026-06-11T02:20:10.000Z",
      "isFinal": true,
      "startSec": 4.2,
      "endSec": 8.7,
      "confidence": 0.92
    }
  ],
  "rawProvider": {
    "name": "amivoice",
    "engine": "-a-general"
  }
}
```

MVP では speaker 分離が取得できない場合、`speaker = "unknown"` とする。AmiVoice 側の話者分離またはチャンネル分離を使える場合のみ `caller` / `bot` / `system` へ正規化する。

### 5.5 Backend: LLM レビュー JSON

AmiVoice transcript を LLM に渡し、以下の JSON schema に合う結果だけを保存する。

```json
{
  "version": 1,
  "summary": "配送状況確認と住所変更の相談。折り返しは不要。",
  "customerIntent": "配送状況の確認と住所変更",
  "responseEvaluation": {
    "status": "good",
    "notes": "必要事項を確認できているが、到着予定日の明示が不足している"
  },
  "unresolvedItems": [
    "到着予定日の確定"
  ],
  "nextActions": [
    {
      "type": "follow_up",
      "priority": "medium",
      "label": "配送予定日を確認してSMSで通知"
    }
  ],
  "riskSignals": [
    {
      "type": "complaint_risk",
      "severity": "low",
      "label": "再配達への不満が軽度に見られる"
    }
  ],
  "evidence": [
    {
      "label": "顧客の用件",
      "speaker": "unknown",
      "startSec": 4.2,
      "endSec": 8.7,
      "text": "配送状況を確認したいのですが"
    }
  ]
}
```

#### 保存ルール

- `summary_text` には `review.summary` を保存する。
- `transcript_json` には §5.4 の正規 transcript を保存する。
- `review_json` には §5.5 の review JSON を保存する。
- JSON parse に失敗した場合は `review_status='failed'` とし、`review_error` に短い理由のみ保存する。
- `review_error` に transcript 全文や API key を含めない。

### 5.6 Backend: sync payload

review 完了後、既存の `recording` entity を再度 `sync_outbox` へ enqueue する。

```json
{
  "id": "0190...",
  "callLogId": "0190...",
  "recordingType": "full_call",
  "sequenceNumber": 1,
  "filePath": "storage/recordings/.../mixed.wav",
  "s3Url": "http://frontend:3000/storage/recordings/{callLogId}/mixed.wav",
  "uploadStatus": "uploaded",
  "durationSec": 180,
  "format": "wav",
  "fileSizeBytes": 1234567,
  "startedAt": "2026-06-11T02:20:00.000Z",
  "endedAt": "2026-06-11T02:23:00.000Z",
  "summaryText": "配送状況確認と住所変更の相談。折り返しは不要。",
  "transcriptJson": { "provider": "amivoice", "utterances": [] },
  "reviewStatus": "completed",
  "reviewJson": { "version": 1, "summary": "...", "nextActions": [] },
  "reviewedAt": "2026-06-11T02:24:00.000Z"
}
```

Frontend は同じ `recording.id` で upsert するため、初回同期済みの録音メタデータを review 付き payload で上書きできる。

### 5.7 Backend: エラー処理

| ケース | 処理 |
|--------|------|
| `POST_CALL_REVIEW_ENABLED=false` | `review_status='skipped'` を保存し、review API は呼ばない |
| `AMIVOICE_API_KEY` 未設定 | 起動時 warning。review 実行時は `failed` |
| AmiVoice timeout / 5xx | `failed`。録音 upload / outbox processed は成功扱い |
| LLM timeout / JSON parse error | `failed`。transcript が取れていれば `transcript_json` は保存可能 |
| Frontend recording fetch 失敗 | ローカルファイルがない場合のみ `failed` |
| `mixed.wav` が空 / 破損 | `failed` |

`recording_file` upload 成功後の review 失敗は、録音ファイル表示を壊さない。review 再実行は別 Issue または本 Issue 後続タスクで扱う。

### 5.8 Frontend: 型追加

```typescript
export interface CallReviewEvidence {
  label: string
  speaker: "caller" | "bot" | "system" | "unknown"
  startSec: number | null
  endSec: number | null
  text: string
}

export interface CallReview {
  version: number
  summary: string
  customerIntent: string
  responseEvaluation: {
    status: "good" | "needs_attention" | "poor" | "unknown"
    notes: string
  }
  unresolvedItems: string[]
  nextActions: Array<{
    type: "follow_up" | "confirm" | "escalate" | "none" | "other"
    priority: "low" | "medium" | "high"
    label: string
  }>
  riskSignals: Array<{
    type: "complaint_risk" | "confusion" | "urgent" | "other"
    severity: "low" | "medium" | "high"
    label: string
  }>
  evidence: CallReviewEvidence[]
}

export interface CallDetail extends Call {
  // 既存フィールド...
  reviewStatus?: "pending" | "processing" | "completed" | "failed" | "skipped"
  review?: CallReview | null
}
```

### 5.9 Frontend: 通話詳細 Drawer

`components/calls/call-detail-drawer.tsx` のタブに「レビュー」を追加する。

表示項目:

- レビュー状態バッジ
  - `completed`: 完了
  - `pending` / `processing`: 生成中
  - `failed`: 生成失敗
  - `skipped`: 無効
- 通話概要
- 顧客の用件
- 応答評価
- 未解決事項
- 次アクション
- リスクシグナル
- 証拠発話

表示ルール:

- `reviewStatus='completed'` かつ `review` がある場合はレビュー内容を表示する。
- `pending` / `processing` の場合は「レビュー生成中」を表示する。
- `failed` の場合は「レビュー生成に失敗しました」を表示し、録音・文字起こしタブは通常通り利用可能にする。
- `skipped` の場合は「通話後レビューは無効です」を表示する。
- evidence の `startSec` / `endSec` は `mm:ss` 形式で表示する。音声シーク連携は後続改善とする。

### 5.10 テスト観点

#### Backend unit / integration

| ID | 対象 | 入力 | 期待結果 |
|----|------|------|----------|
| T-01 | config | env 未設定 | `POST_CALL_REVIEW_ENABLED=false` |
| T-02 | AmiVoice client | 正常レスポンス fixture | transcript 正規形に変換される |
| T-03 | AmiVoice client | timeout | review_status が `failed` になる |
| T-04 | LLM review parser | schema 準拠 JSON | `review_json` と `summary_text` が保存される |
| T-05 | LLM review parser | JSON 破損 | transcript は保存可能、review は `failed` |
| T-06 | sync payload | review 完了録音 | `summaryText` / `transcriptJson` / `reviewJson` が payload に含まれる |
| T-07 | recording_file flow | review 失敗 | recording upload は成功扱いで processed される |

#### Frontend unit / component

| ID | 対象 | 入力 | 期待結果 |
|----|------|------|----------|
| T-08 | `normalizeRecording` | review fields あり | `StoredRecording` に保持される |
| T-09 | `getCallDetail` | review completed | `CallDetail.review` が返る |
| T-10 | `CallReviewTab` | completed | 概要・次アクション・証拠発話が表示される |
| T-11 | `CallReviewTab` | failed | 録音表示を壊さず失敗表示になる |
| T-12 | `CallDetailDrawer` | 4タブ構成 | 録音・文字起こし・要約・レビューが切り替えられる |

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #284 | STEER-284 | 起票 |
| STEER-284 | RD-Backend: 通話後レビュー要件 | 要件追加 |
| STEER-284 | RD-Frontend: 通話詳細レビュー表示要件 | 要件追加 |
| RD-Backend | DD-Backend: post-call review worker | 設計 |
| RD-Frontend | DD-Frontend: CallReviewTab | 設計 |
| DD-Backend | UT-Backend T-01〜T-07 | 単体/結合テスト |
| DD-Frontend | UT-Frontend T-08〜T-12 | 単体/コンポーネントテスト |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] AmiVoice API の利用方式（同期 HTTP / 非同期 HTTP）が MVP として妥当か
- [ ] review 結果の SoT が Backend DB であることに矛盾がないか
- [ ] `recording_file` upload 成功と review 失敗が分離されているか
- [ ] API キー・個人情報・文字起こし全文をログに出さない設計になっているか
- [ ] Frontend の通話詳細表示が既存「録音」「文字起こし」「要約」タブと整合しているか
- [ ] テストケースが失敗系を含んでいるか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] Backend 実装が完了している
- [ ] Frontend 実装が完了している
- [ ] `POST_CALL_REVIEW_ENABLED=false` で既存同期が回帰しない
- [ ] AmiVoice API key 未設定時に安全に無効化/失敗扱いになる
- [ ] `cargo test` が PASS
- [ ] Frontend の lint/test/build が PASS
- [ ] 実通話録音 1 件で、録音再生・文字起こし・レビュー表示を確認済み

---

## 8. Open Questions

| ID | 質問 | 推奨回答 | 状態 |
|----|------|----------|------|
| OQ-1 | AmiVoice は同期 HTTP で足りるか、長時間録音向けに非同期 HTTP も必要か | MVP は同期 HTTP。長時間通話は後続 Issue | 確定 |
| OQ-2 | 話者分離を MVP に含めるか | AmiVoice が安定して返せる場合のみ採用。不可なら `unknown` | 確定 |
| OQ-3 | review 失敗時の手動再実行 UI/API を含めるか | MVP では含めず、Backend CLI/API を後続に分割 | 確定 |
| OQ-4 | Zenn 記事用にサンプル通話データを匿名化して添付するか | 実番号・実音声は使わず、匿名化/合成サンプルを使用 | 確定 |

---

## 9. 備考

- 本 Issue はコンテスト向け PoC の性格が強いため、まずは「通話後バッチで review を表示できる」ことを優先する。
- review JSON は将来的に検索・集計できるが、本 Issue では通話詳細表示に限定する。
- 通話録音や文字起こしには個人情報が含まれる可能性が高いため、ログ・記事・スクリーンショットでは匿名化を必須とする。

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-06-11 | 初版作成 | Codex (GPT-5) |
| 2026-06-11 | オーナー承認により Approved 化。OQ-1〜4 を確定 | Codex (GPT-5) |
