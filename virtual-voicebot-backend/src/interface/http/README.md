# http モジュール

## 目的
- Frontend 向けの REST API と SSE を提供する。
- 録音ファイルの配信（必要なら Range 対応）を提供する。

## 詳細設計
- API契約（正本）: [docs/contract.md](../../../docs/contract.md)
- 録音設計: [DD-007_recording.md](../../docs/design/detail/DD-007_recording.md)

## 責務
- REST:
  - calls / utterances の参照 API を提供する
- Realtime:
  - SSE で call/utterance/summary のイベントを配信する
  - ping を定期送信し、切断に強い運用を可能にする
- Recording:
  - /api/recordings/{callId}/... の配信
  - 将来の署名URL化に備えた抽象化

## 禁止事項
- SIP/RTP のプロトコル処理をしない（それは sip/rtp の責務）
- セッション状態の本体を持たない（それは session の責務）
- ASR/LLM/TTS を直接呼ばない（それは app/ai の責務）
- 録音生成（エンコード/ミックス）はしない（それは media の責務）
