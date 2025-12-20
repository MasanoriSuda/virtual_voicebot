# Virtual Voicebot Contract (MVP)

## Scope
- Zoiper ↔ Rust自作 SIP/RTP voicebot backend が通話する
- Frontend は「通話終了後の履歴一覧」と「録音再生＋要約」を表示する
- ライブ中継や逐次発話表示は扱わず、終了後の履歴のみを対象とする

## Non-Goals (MVP)
- フロントから通話制御（発信/終話/保留など）は行わない（閲覧のみ）
- 録音の署名付き URL・認可の厳密化は Future

## MVP Constraints (HTTP)
- MVPでは frontend→backend の入力/制御APIは提供しない
- MVPのHTTPは `recordingUrl` の GET のみ
- 参照REST/SSEは将来追加（契約更新が必要）

## DTO Classification (MVP)
### Internal Event DTO（非公開・チャネル経由）
- sip↔session, rtp↔session, session↔app, app↔ai, app↔session など内部通信に用いる

### Public Read Model DTO（frontend向け読み取りモデル）
- Call, RecordingMeta（MVPで必須）
- Utterance はMVPでは定義しない（必要性が出た時点で追加）

### Transport DTO（HTTPレスポンス等の実体）
- `recordingUrl` の GET レスポンス（`audio/wav` のバイト列）
- Range 対応は将来（MVPでは未対応）

### Correlation ID Rules
- 外部DTO（Call/RecordingMeta）は `callId` を唯一の相関キーとする
- 内部イベントは `call_id` を必須とし、音声系は `stream_id` を併用する
- `session_id` を導入する場合は設計で明文化する（MVPは `call_id == session_id`）

### MVP Minimal Read Models（Public）
#### Call（Read Model）
- `callId`: string（必須）
- `startedAt`: string(ISO8601)（必須）
- `endedAt`: string(ISO8601)（任意）
- `recordingUrl`: string（任意）
- `status`: `"ringing" | "in_call" | "ended" | "error"`（必須）

#### RecordingMeta（Read Model）
- `callId`: string（必須）
- `path`: string（内部用。外部に出す場合はファイル名のみ推奨）
- `durationSec`: number（任意）
- `format`: `"wav"`（MVP固定）
- `mixedUrl`: string（必須。`recordingUrl` と同一でも可）

## Data Model

※この節は拡張例。MVPで必須な最小スキーマは「DTO Classification (MVP)」を正とし、追加項目は後方互換で追加可とする。

### Call
- `callId` は通話を一意に識別する
- `status`: `"ringing" | "in_call" | "ended" | "error"`（必要なら追加可）
- `summary` は通話中または通話後に更新されうる（最後に届いたものが正）

```json
{
  "callId": "c_123",
  "from": "sip:zoiper@example",
  "to": "sip:bot@example",
  "startedAt": "2025-12-13T00:00:00.000Z",
  "endedAt": null,
  "status": "in_call",
  "summary": "配送状況の確認。住所変更あり。",
  "durationSec": 123,
  "recordingUrl": null
}
```

## Delivery Model（バックエンド → フロントへのプッシュのみ）
- フロントは「終了済み通話の履歴と録音」を表示するだけを担当する。
- バックエンドは通話終了時に1回だけフロントの受信APIを叩いて、履歴と録音URL・要約を渡す。

## Callback API（バックエンド→フロント）
- **Call Ingest**: `POST /api/ingest/call`
  - バックエンドが通話終了後に送る。ペイロードに録音URLとメタ、要約を含める。
  - Payload:
  ```json
  {
    "callId": "c_123",
    "from": "sip:zoiper@example",
    "to": "sip:bot@example",
    "startedAt": "2025-12-13T00:00:00.000Z",
    "endedAt": "2025-12-13T00:05:00.000Z",
    "status": "ended",
    "summary": "配送状況の確認。住所変更あり。",
    "durationSec": 300,
    "recording": {
      "recordingUrl": "https://frontend.example/recordings/c_123/mixed.wav",
      "durationSec": 300,
      "sampleRate": 8000,
      "channels": 1
    }
  }
  ```
- フロントは受け取ったデータを内部DBに保存し、履歴一覧・詳細画面で表示する。ライブSSEや逐次発話の取り込みは行わない前提。

## Error Format (REST / SSE)
### REST Error
4xx/5xx は以下の形を推奨（最低限 `code` と `message`）

```json
{
  "error": {
    "code": "NOT_FOUND",
    "message": "callId not found",
    "requestId": "req_xxx"
  }
}
```

### SSE Error Event
- `type`: `"error"`
- `data` は REST と同等形式を推奨

```json
{
  "type": "error",
  "callId": "c_123",
  "data": {
    "error": { "code": "INTERNAL", "message": "..." }
  }
}
```

## Auth (MVP)
- MVP は認証なし（ローカル/閉域想定）
- 将来 `Authorization: Bearer ...` 等を導入する可能性あり

## Audio Playback (Future)
- `recordingUrl == null` の場合は「音声は準備中」表示
- `recordingUrl` は将来、署名付き URL 等に置き換わる可能性がある
- UI は URL を長期キャッシュしない前提（将来）

## Docs Setup
```bash
mkdir -p docs
nano docs/contract.md
```
