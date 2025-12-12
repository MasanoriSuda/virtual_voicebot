# Virtual Voicebot Contract (MVP)

## Scope
- Zoiper ↔ Rust自作SIP/RTP voicebot backend が通話
- Frontend は通話履歴一覧と通話詳細の会話ログ（LINE風）を表示
- 将来的に録音音声を再生して内容を確認できるようにする

## Data Model

### Call
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
- `GET /api/calls` → `Call[]`
- `GET /api/calls/{callId}` → `Call`
- `GET /api/calls/{callId}/utterances?afterSeq={number}` → `Utterance[]`（seq asc）

## Realtime (WebSocket or SSE)
- WebSocket: `GET ws://{HOST}/api/ws?callId={callId}`
- SSE: `GET http://{HOST}/api/events?callId={callId}`

### Event Envelope
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

## UI Rules
- `utterance.partial` は同一 `seq` のバブルを上書きする
- `utterance.final` が来たら確定表示する（同一 `seq` を final で上書き可）
- `startSec`/`endSec` がある場合、該当発話の「この部分を再生」でシーク再生できる

## Audio Playback (Future)
- `recordingUrl == null` の場合は「音声は準備中」表示
- `recordingUrl` は将来、署名付きURL等に置き換わる可能性がある

## Docs Setup (Optional)
```bash
mkdir -p docs
nano docs/contract.md
```
