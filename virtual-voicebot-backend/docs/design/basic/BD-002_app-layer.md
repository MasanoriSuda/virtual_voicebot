<!-- SOURCE_OF_TRUTH: App層設計 -->
# app レイヤ詳細設計

**正本**: 本ファイルが app レイヤ I/F の正本です（2025-12-27 確定、Refs Issue #7 CX-3）

## 1. 目的・責務
- session からのイベントを受け取って「対話状態」を管理する
- ai::{asr,llm,tts} の呼び出し順番と依存関係を制御する
- session に「このセッションを続ける/切る」「この音声を流す」などの指示を出す

## 2. イベントフロー

### 必須フィールド規約（2025-12-28 追加、Refs Issue #7）
- **全イベント**: `call_id` / `session_id`（MVP では同一値、ai.md §1 参照）
- **PCM 系イベント**: 上記に加え `stream_id` を必須とする
- 参照: design.md §13.1、ai.md §1

### session → app イベント
| イベント | 用途 |
|----------|------|
| `CallStarted` | 通話開始通知（SDP 確定後） |
| `PcmReceived` | RTP から受信した PCM チャンク |
| `CallEnded` | 通話終了通知（BYE 受信等） |
| `SessionTimeout` | Session Timer タイムアウト |

### app → session イベント
| イベント | 用途 |
|----------|------|
| `BotAudioReady` | TTS 音声を RTP へ送信指示（payload: `PcmOutputChunk`） |
| `HangupRequested` | BYE 送信指示 |

> **補足**: `BotAudioReady` は PCM チャンク（`PcmOutputChunk`）を含む。session は受け取った PCM を rtp へ転送する。

### app → ai イベント
- [ai.md](ai.md) §5 参照

### ai → app イベント
- [ai.md](ai.md) §5 参照

## 3. 対話状態のモデル

### セッションごとの state
| 状態 | 説明 |
|------|------|
| `Idle` | 待機中（通話開始前） |
| `Listening` | ユーザ音声を ASR に送信中 |
| `Thinking` | LLM 応答待ち |
| `Speaking` | TTS 音声を再生中 |
| `Ended` | 通話終了 |

### LLM コンテキスト管理
- 履歴は app が保持し、LlmRequest に含めて渡す
- 履歴の最大長は config で制限

## 4. エラー/タイムアウト時の振る舞い

### エラーポリシー
| 条件 | アクション |
|------|----------|
| ASR/LLM/TTS 1回目失敗 | 謝罪音声で継続 |
| 連続失敗（config 管理、既定: 2回） | BYE 送信で終了 |
| タイムアウト | 謝罪音声 → 終了 |

### フォールバック
- ASR 失敗: 「聞き取れませんでした」
- LLM 失敗: 「少々お待ちください」
- TTS 失敗: 無音（ログのみ）

## 5. session / ai との責務境界

### session が知らなくてよいこと
- LLM のプロンプト構成
- ASR/TTS のストリーミング詳細
- 対話履歴

### ai が知らなくてよいこと
- 通話状態（SIP/RTP）
- セッションタイマー
- エラーポリシーの判断

### app だけが知っているべきこと
- 対話フロー全体の制御
- エラー発生時の継続/終了判断
- LLM コンテキストの構築

## 6. ストリーミング I/O ライフサイクル

### ASR 開始
- session Confirmed で app が PCM を ASR 入力チャネルへ送り始める
- 通話終了/停止指示でチャネルを閉じる

### TTS 開始
- app が発話テキストを決定した時点でリクエストを送る
- PCM チャンクを受領しつつ session 経由で rtp へ渡す

### 終了条件
- 通話終了、app 指示、連続エラーなどで終了シグナル受領時にチャネルを閉じる

## 7. 補助資料
- 概念フロー図: [voice_bot_flow.md](voice_bot_flow.md)（補助）
- AI I/F 詳細: [ai.md](ai.md)（正本）
