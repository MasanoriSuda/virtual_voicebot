# app / ai インタフェース設計

## 全体方針
- モデル: イベント/チャネルベースを基本とし、1リクエスト1レスポンスの場面のみ Future 呼び出しを許容するハイブリッド。
- ストリーミング（ASR/TTS）はチャネルで双方向に運ぶ。LLM は 1 リクエスト 1 レスポンスの非同期呼び出し（トークンストリームは NEXT で拡張）。

## ASR I/F（音声→テキスト）
- 入力: `AsrInputPcm { session_id, stream_id, pcm: i16[], sample_rate=8000, channels=1, chunk_ms≈20 }` を app→ai::asr にチャネル送信。
- 出力: `AsrResult { session_id, stream_id, text, is_final, meta? }` を ai::asr→app へ。MVPでは partial は任意、final は必須。
- エラー: `AsrError { session_id, reason }` を返し、エラーポリシーに沿って app が謝罪/継続/終了を判断。
- チャネル構成: app→asr（入力）、asr→app（結果）の2本。バックプレッシャは入力キュー溢れ時に古いPCMを破棄（ログのみ）。

## LLM I/F（テキスト→応答テキスト/アクション）
- 呼び出し: `LlmRequest { session_id, history, user_text, meta? }` を app から ai::llm へ Future で渡し、`LlmResponse { text, action?, end_flag?, meta? }` を await で受け取る。
- プロンプト組立: history/コンテキストは app が組み立てて渡す（llm は純クライアント）。
- エラー/タイムアウト: `LlmError` 相当を返し、app が謝罪/フォールバック/終了を選択するシグナルとする。
- ストリーミング応答: MVP では非対応。NEXT でトークンストリームを検討。

## TTS I/F（テキスト→音声）
- 入力: `TtsRequest { session_id, stream_id, text, options? }` を app→ai::tts にチャネル送信。
- 出力: `TtsPcmChunk { session_id, stream_id, pcm: i16[], sample_rate=8000, is_last }` を ai::tts→app/rtp へストリーミング（20–40ms 程度）。
- エラー: `TtsError { session_id, reason }` を返し、この発話の失敗を明示。app が謝罪/次発話スキップ/終了を選択。
- チャネル構成: app→tts（リクエスト）、tts→app（PCM/エラー）の2本。終端は `is_last=true` で明示。

## 共通イベント設計
- app→ai: `AsrInputPcm`, `LlmRequest`, `TtsRequest`
- ai→app: `AsrResult`, `AsrError`, `LlmResponse`, `LlmError`, `TtsPcmChunk`, `TtsError`
- 必須フィールド: `session_id`, `stream_id`, テキスト/PCM、`is_final`/`is_last`、`reason`（エラー時）。
- エラー整合: design.md のエラーポリシーに従い、1回目は謝罪音声で継続、連続失敗で BYE を選べるようシグナルする。

## ストリーミング I/O ライフサイクル
- ASR開始: session Confirmed で app が PCM を ASR 入力チャネルへ送り始める。通話終了/停止指示でチャネルを閉じる。
- TTS開始: app が発話テキストを決定した時点でリクエストを送り、PCMチャンクを受領しつつ rtp へ渡す。
- 終了条件: 通話終了、app 指示、連続エラーなどで終了シグナル受領時にチャネルを閉じる。
- バックプレッシャ: 入力キューあふれ時は古い PCM を破棄（ログ）。TTS 出力滞留時は app 側で次発話を抑制する運用（MVP）。
