# Virtual Voicebot Contract (MVP)

## Scope
- Zoiper ↔ Rust自作 SIP/RTP voicebot backend が通話する
- Frontend は通話履歴一覧と通話詳細の会話ログ（LINE風）を表示する
- 将来的に録音音声を再生して内容を確認できるようにする

## Non-Goals (MVP)
- フロントから通話制御（発信/終話/保留など）は行わない（閲覧のみ）
- 録音の署名付き URL・認可の厳密化は Future

## Data Model

### Call
- `callId` は通話を一意に識別する
- `status`: `active` | `ended` | `failed`（必要なら追加可）
- `summary` は通話中または通話後に更新されうる（最後に届いたものが正）

```json
{
  "callId": "c_123",
  "from": "sip:zoiper@example",
  "to": "sip:bot@example",
  "startedAt": "2025-12-13T00:00:00.000Z",
  "endedAt": null,
  "status": "active",
  "summary": "配送状況の確認。住所変更あり。",
  "durationSec": 123,
  "recordingUrl": null
}
```

### Utterance
- `seq` は `callId` 内で単調増加かつ一意（発話ターン ID）
- `state`:
  - `partial`: 途中結果（同一 `seq` の表示を上書きしてよい）
  - `final`: 確定結果（同一 `seq` を `final` で上書きしてよい）
- `speaker`: `caller` | `bot` | `system`（MVP は `caller` / `bot` を想定）
- `startSec` / `endSec` は録音再生のシーク用（Future で活用）

```json
{
  "callId": "c_123",
  "seq": 37,
  "speaker": "caller",
  "state": "partial",
  "text": "きょうの配送…",
  "ts": "2025-12-13T00:01:02.120Z",
  "startSec": 12.34,
  "endSec": 15.67,
  "confidence": 0.86
}
```

## REST API
- **List Calls**: `GET /api/calls?limit=50&cursor={cursor}`  
  Response:
  ```json
  {
    "items": [],
    "nextCursor": null
  }
  ```
- **Get Call**: `GET /api/calls/{callId}` → `Call`
- **List Utterances (catch-up)**: `GET /api/calls/{callId}/utterances?afterSeq={number}` → `Utterance[]`（seq asc）  
  `afterSeq` は「最後に確定表示した seq」を渡す想定（再接続・追いつき用）

## Realtime (SSE)
- **Subscribe**: `GET /api/events?callId={callId}&afterSeq={number}`  
  `afterSeq` は任意。フロントが保持している「最後に確定表示した seq」を指定するのが推奨。  
  サーバ実装が簡単なら `afterSeq` 以降のイベントを可能な範囲で先に流し、その後 live に切り替える。  
  MVP で難しければ「追いつきは REST utterances で行う」運用でも OK（下記 Reconnect 参照）。

### Event Envelope
SSE の `data:` には JSON を流す（Envelope 形式）。

```json
{
  "type": "utterance.partial",
  "callId": "c_123",
  "data": {}
}
```

### Event Types
- `call.started` (data: `Call`)
- `call.ended` (data: `Call`)
- `utterance.partial` (data: `Utterance`)
- `utterance.final` (data: `Utterance`)
- `summary.updated` (data: `{ "summary": "..." }`)
- `ping` (data: `{ "ts": "..." }`) ※接続維持用

### Heartbeat
サーバは `ping` を 15〜30 秒に 1 回送る（プロキシ等で切断されにくくするため）

### Reconnect Strategy (MVP)
- フロントは切断・再接続を前提に実装する
- フロントは「最後に確定表示した seq（`final` を受けた seq）」を `lastSeq` として保持
- SSE が切れたら、まず追いつき: `GET /api/calls/{callId}/utterances?afterSeq=lastSeq`
- 次に SSE を再購読: `GET /api/events?callId={callId}&afterSeq=lastSeq`

## UI Rules
- `utterance.partial` は同一 `seq` のバブルを上書きする
- `utterance.final` が来たら確定表示する（同一 `seq` を `final` で上書き可）
- `startSec` / `endSec` がある場合、該当発話の「この部分を再生」でシーク再生できる（Future）

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
