<!-- SOURCE_OF_TRUTH: AI連携設計 -->
# ai モジュール詳細設計（asr / llm / tts）

**配置**: `src/service/ai`
**正本**: 本ファイルが ai モジュール I/F の正本です（2025-12-27 確定、Refs Issue #7 CX-3）

## 1. 共通方針
- すべて「外部サービスのラッパー」として実装する
- モデル: イベント/チャネルベースを基本とし、1リクエスト1レスポンスの場面のみ Future 呼び出しを許容するハイブリッド
- ネットワーク再試行ポリシー、タイムアウトの基本値は config 管理

### 相関ID規約（2025-12-27 確定、Refs Issue #7）
- **MVP**: `session_id == call_id`（同一値）として扱う
- すべての DTO に `session_id`（= `call_id`）を必須とする
- メディア（PCM）系は `stream_id` を併用する
- 参照: design.md §13.1、AGENTS.md §3

## 2. ASR (service/ai/asr)

### 入力 DTO
```rust
AsrInputPcm {
    session_id: String,
    stream_id: String,
    pcm: Vec<i16>,      // 8000Hz, mono
    sample_rate: u32,   // = 8000
    channels: u8,       // = 1
    chunk_ms: u32,      // ≈ 20
}
```

### 出力 DTO
```rust
AsrResult {
    session_id: String,
    stream_id: String,
    text: String,
    is_final: bool,     // MVP: final 必須、partial 任意
    meta: Option<AsrMeta>,
}

AsrError {
    session_id: String,
    reason: String,
}
```

### チャネル構成
- service/call_control → service/ai/asr（入力チャネル）
- service/ai/asr → service/call_control（結果チャネル）
- バックプレッシャ: 入力キュー溢れ時に古い PCM を破棄（ログのみ）

## 3. LLM (service/ai/llm)

### 入力 DTO
```rust
LlmRequest {
    session_id: String,
    history: Vec<Message>,
    user_text: String,
    meta: Option<LlmMeta>,
}
```

### 出力 DTO
```rust
LlmResponse {
    text: String,
    action: Option<String>,
    end_flag: Option<bool>,
    meta: Option<LlmMeta>,
}

LlmError {
    session_id: String,
    reason: String,
}
```

### 呼び出しモデル
- Future で渡し、await で受け取る（1リクエスト1レスポンス）
- プロンプト組立: history/コンテキストは service/call_control が組み立てて渡す（service/ai/llm は純クライアント）
- ストリーミング応答: MVP では非対応、NEXT でトークンストリームを検討

## 4. TTS (service/ai/tts)

### 入力 DTO
```rust
TtsRequest {
    session_id: String,
    stream_id: String,
    text: String,
    options: Option<TtsOptions>,
}
```

### 出力 DTO
```rust
TtsPcmChunk {
    session_id: String,
    stream_id: String,
    pcm: Vec<i16>,      // 8000Hz, mono
    sample_rate: u32,   // = 8000
    is_last: bool,
}

TtsError {
    session_id: String,
    reason: String,
}
```

### チャネル構成
- service/call_control → service/ai/tts（リクエストチャネル）
- service/ai/tts → service/call_control（PCM/エラーチャネル）
- 終端: `is_last=true` で明示

## 5. service/call_control から見た I/F まとめ

### service/call_control → service/ai イベント
| イベント | 用途 |
|----------|------|
| `AsrInputPcm` | PCM チャンクを ASR に送信 |
| `LlmRequest` | テキスト + 履歴を LLM に送信 |
| `TtsRequest` | テキストを TTS に送信 |

### service/ai → service/call_control イベント
| イベント | 用途 |
|----------|------|
| `AsrResult` | 認識結果（partial/final） |
| `AsrError` | ASR エラー通知 |
| `LlmResponse` | LLM 応答 |
| `LlmError` | LLM エラー通知 |
| `TtsPcmChunk` | TTS 音声チャンク |
| `TtsError` | TTS エラー通知 |

### 必須フィールド
- `session_id`, `stream_id`（音声系）
- テキスト/PCM
- `is_final`/`is_last`（終端判定）
- `reason`（エラー時）

### キャンセル
- 通話終了時に ASR/TTS チャネルを閉じる
- 連続エラー時は service/call_control が BYE を選択可能

## 6. エラーポリシー
- design.md のエラーポリシーに従う
- 1回目は謝罪音声で継続
- 連続失敗で BYE を選べるようシグナルする

## 7. 現状の実装と差分メモ
- 現状: WAV 一時ファイルを経由し、HTTP (`/transcribe`, `/audio_query`, `/synthesis`) や AWS SDK を直接呼び出している
- 予定: service/call_control ↔ service/ai のチャネル経由で PCM/テキストを渡す形に置き換える（I/F 変更は別タスク）
- ポリシー（タイムアウト/リトライ/フォールバック）は現状のまま維持し、移行後も踏襲する想定

## 8. 補助資料
- 概念フロー図: [voice_bot_flow.md](voice_bot_flow.md)（補助）
