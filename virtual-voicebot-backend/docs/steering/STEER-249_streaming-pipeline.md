# STEER-249: LLM ストリーミング ＋ 文単位 TTS 先行再生（first-audio latency 改善）

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-249 |
| タイトル | LLM ストリーミング ＋ 文単位 TTS 先行再生（first-audio latency 改善） |
| ステータス | Draft |
| 関連Issue | #249 |
| 優先度 | P1 |
| 作成日 | 2026-02-25 |

---

## 2. ストーリー（Why）

### 2.1 背景

現状のボイスボットは ASR → LLM → TTS が完全直列であり、各ステージの最終結果を待ってから次へ進む。

```
（現状）
発話終了
  → ASR 全文確定（WAV 全読み → multipart POST → 転写完了）
  → LLM 全文確定（stream:false → body.text().await → parse）
  → TTS 全文合成（audio_query → synthesis → WAV 全受信 → 保存）
  → 再生開始
```

計測上の体感遅延は **3〜8 秒**（ローカル LLM + ローカル TTS 構成）になりやすい。

一方、再生側（[playback_service.rs](../../src/protocol/session/services/playback_service.rs)）は既に `step_playback()` によるフレーム逐次送出を行っており、「小さい WAV を早く渡す」だけで体感改善が即効する土台がある。

### 2.2 目的

**first-audio latency**（発話終了 → 最初の音声が相手に届くまでの時間）を短縮する。

第 1 段階として、コスト対効果が最も高い「LLM streaming 受信 ＋ 文単位 TTS 先行再生」を実装する。
ASR の真のストリーミング化・真の TTS ストリーミング化は **第 2 段階以降**（別 Issue）に分割する。

**目標レンジ（参考）**

| シナリオ | first-audio latency 目安 |
|---------|--------------------------|
| 現状（逐次） | 3〜8 秒 |
| 本 Issue（LLM streaming ＋ 文単位 TTS） | 1〜3 秒（体感差：大） |
| 第 2 段階（ASR partial/VAD 追加） | さらに 0.3〜1.5 秒改善余地 |

### 2.3 ユーザーストーリー

```
As a ボイスボット利用者（発信者）
I want to  質問後、応答の最初の一文をすばやく聞ける
So that    応答が早く感じられ、ボイスボットとの対話が自然になる

受入条件:
- [ ] LLM 全文完成前に最初の TTS ジョブが生成される
- [ ] LLM 全文完成前に最初の音声再生が開始される
- [ ] 既存の逐次動作が設定フラグで維持できる（既存経路を壊さない）
- [ ] フォールバック・タイムアウト挙動が既存と同等である
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-25 |
| 起票理由 | ボイスボット応答の体感遅延が大きいため、ストリーミング化を試行する |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Sonnet 4.6 |
| 作成日 | 2026-02-25 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "ストリーミング化ステアリング作成（#249）" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | @MasanoriSuda | - | - | |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | - |
| 承認日 | - |
| 承認コメント | |

### 3.5 実装

| 項目 | 値 |
|------|-----|
| 実装者 | Codex |
| 実装日 | - |
| 指示者 | @MasanoriSuda |
| 指示内容 | Approved 後に引き継ぎ |
| コードレビュー | - |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | - |
| マージ日 | - |
| マージ先 | - |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| docs/requirements/（AI 連携 RD） | 修正 | LLM ストリーミング要件、TTS キュー要件を追記 |
| docs/design/detail/（AI サービス DD） | 修正 | `LlmStreamPort` シグネチャ、`SentenceAccumulator` 設計、再生キュー設計 |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| `src/shared/ports/ai/llm.rs` | 追加 | `LlmStreamPort` トレイト（ストリーム戻り値版） |
| `src/service/ai/mod.rs` | 修正 | Ollama streaming 実装（`stream: true`、NDJSON パース） |
| `src/service/call_control/mod.rs` | 修正 | `handle_user_text()` でストリーム受信 → 文単位 TTS キュー投入 |
| `src/protocol/session/services/playback_service.rs` | 修正 | 再生キュー（`PlaybackQueue`）対応 |
| `src/shared/config.rs`（または環境変数） | 追加 | `VOICEBOT_STREAMING_ENABLED` フラグ |

---

## 5. 差分仕様（What / How）

### 5.1 前提：現状の確認

以下は既存コードの実測事実（調査時点）。

| 箇所 | 現状 |
|------|------|
| `call_control/mod.rs:384` | `AsrChunk { end: true }` を 1 回投げる（一括処理） |
| `ai/asr.rs:17` | μ-law をまとめて WAV 化し一時ファイル経由で ASR 呼び出し |
| `ai/mod.rs:339` | WAV 全読み後に multipart POST |
| `shared/ports/ai/llm.rs:5` | `LlmPort::generate_answer` 戻り値は `Result<String>` |
| `ai/mod.rs:1022` | Ollama リクエストに `stream: false` 固定 |
| `ai/mod.rs:387` | OpenAI は `resp.text().await` 後に parse（非ストリーム） |
| `shared/ports/ai/tts.rs:7` | `TtsPort::synth_to_wav` 戻り値は `Result<PathBuf>` |
| `ai/mod.rs:1390` | VOICEVOX は `/audio_query` → `/synthesis` で全 WAV 受信後に保存 |
| `playback_service.rs:53` | `step_playback()` はフレーム逐次 RTP 送信（すでに逐次）|

### 5.2 新規 Port（`LlmStreamPort`）

既存の `LlmPort`（戻り値 `Result<String>`）は **変更しない**。
ストリーム版は別トレイトを追加し、フラグ分岐で使い分ける。

```rust
// src/shared/ports/ai/llm.rs への追加

use futures::Stream;
use std::pin::Pin;

pub type LlmStream = Pin<Box<dyn Stream<Item = Result<String, LlmError>> + Send>>;

pub trait LlmStreamPort: Send + Sync {
    /// LLM の出力をトークン単位のストリームとして受信する。
    /// 各 Item は "トークン断片文字列" または LlmError。
    fn generate_answer_stream(
        &self,
        call_id: String,
        messages: Vec<ChatMessage>,
    ) -> AiFuture<Result<LlmStream, LlmError>>;
}
```

> **設計論点 A**（オープン）: `LlmPort` に `generate_answer_stream` を追加してデフォルト実装を提供するか、それとも独立トレイトにするか。
> → 独立トレイト案を選択（既存 Port を壊さない、ISP 準拠）。

### 5.3 Ollama ストリーミング実装

Ollama は `POST /api/chat` に `stream: true` を送ると NDJSON で応答する。

```
{"model":"...","message":{"role":"assistant","content":"こん"},"done":false}
{"model":"...","message":{"role":"assistant","content":"にちは"},"done":false}
{"model":"...","message":{"role":"assistant","content":""},"done":true}
```

```rust
// ai/mod.rs への追加（Ollama streaming 呼び出し）

pub(super) async fn call_ollama_for_chat_stream(
    messages: &[ChatMessage],
    system_prompt: &str,
    model: &str,
    endpoint_url: &str,
    http_timeout: Duration,
) -> Result<LlmStream> {
    let client = http_client(http_timeout)?;
    // ...
    let req = OllamaChatRequest { model: ..., messages: ..., stream: true };
    let resp = client.post(endpoint_url).json(&req).send().await?;
    // resp.bytes_stream() を NDJSON 行単位に split → content フィールドを yield
    // done:true の行で Stream 終了
    // ...
}
```

> **設計論点 B**（オープン）: OpenAI クラウド LLM のストリーミング（SSE `data:` 形式）を今回スコープに含めるか。
> → 初回は **Ollama（ローカル）のみ** を対象とする。OpenAI streaming は別 Issue 推奨。

### 5.4 文単位アキュムレーター（`SentenceAccumulator`）

LLM トークンを蓄積し、文の区切りを検出したら `完成文字列` を返す。

```rust
// src/service/call_control/sentence_accumulator.rs（新規）

pub struct SentenceAccumulator {
    buf: String,
    max_chars: usize, // 設定可能。デフォルト 50 文字
}

impl SentenceAccumulator {
    /// トークン断片を追記し、文が完成したら Some(sentence) を返す。
    /// まだ文が完成していない場合は None。
    pub fn push(&mut self, token: &str) -> Option<String> { ... }

    /// バッファに残った未送信テキストを強制フラッシュ（LLM done 時）。
    pub fn flush(&mut self) -> Option<String> { ... }
}
```

**文区切りルール（初期案）**

| 条件 | 説明 |
|------|------|
| 句点（`。！？`）を含んだ時点 | → 即フラッシュ |
| `max_chars` に達した時点 | → 読点（`、`）または空白直後にフラッシュ。なければ強制カット |
| LLM done 時 | → バッファ残余を `flush()` でフラッシュ |

> **設計論点 C**（オープン）: `max_chars` の初期値を何文字にするか（TTS 合成時間との兼ね合い）。
> → レビューで決定。暫定 50 文字。

### 5.5 `handle_user_text()` ストリーミング分岐

```rust
// src/service/call_control/mod.rs（修正イメージ）

async fn handle_user_text(&mut self, call_id: &CallId, trimmed: &str) -> anyhow::Result<()> {
    // ...（intent 分類は変更なし）

    if config::voicebot_streaming_enabled() {
        self.handle_user_text_streaming(call_id, messages).await
    } else {
        self.handle_user_text_sequential(call_id, messages).await // 既存経路
    }
}

async fn handle_user_text_streaming(&mut self, call_id: &CallId, messages: Vec<ChatMessage>) -> anyhow::Result<()> {
    let stream = self.ai_port.generate_answer_stream(call_id.to_string(), messages).await?;
    let mut acc = SentenceAccumulator::new(config::sentence_max_chars());
    let mut full_answer = String::new();

    tokio::pin!(stream);
    while let Some(chunk) = stream.next().await {
        let token = chunk?;
        full_answer.push_str(&token);
        if let Some(sentence) = acc.push(&token) {
            self.dispatch_tts_and_enqueue(call_id, &sentence).await;
        }
    }
    if let Some(tail) = acc.flush() {
        self.dispatch_tts_and_enqueue(call_id, &tail).await;
    }

    // 履歴は全文確定後に追加（既存と同じタイミング）
    self.push_history(user_query, full_answer);
    Ok(())
}
```

> **設計論点 D**（オープン）: TTS ジョブは `tokio::spawn` で並列化するか、直列で済ますか。
> → 初回は **直列**（前の TTS 完了を待ってから次の文を TTS 投入）。再生キューが整備されたら並列化を検討。

### 5.6 再生キュー対応

現状の `start_playback()` は呼ぶたびに前の再生をキャンセルする（`cancel_playback()` を冒頭で呼ぶ）。
連続送信に対応するために **再生キュー** を追加する。

```rust
// playback_service.rs（修正イメージ）

pub(crate) struct PlaybackState {
    pub(crate) frames: Vec<Vec<u8>>,
    pub(crate) index: usize,
}

// SessionCoordinator に追加するフィールド
playback_queue: VecDeque<Vec<Vec<u8>>>, // 追加キュー（WAV フレーム列）
```

```
新規イベント案: SessionOut::AppEnqueueBotAudioFile { path: String }
  → 現再生中であればキューに積む
  → 現再生なしであれば即 start_playback()
```

既存の `AppSendBotAudioFile`（即再生・上書き）は維持し、ストリーミング経路専用に `AppEnqueueBotAudioFile` を追加する。

> **設計論点 E**（オープン）: 既存の `AppSendBotAudioFile` に「enqueue フラグ」を追加するか、新規イベント型にするか。
> → 新規イベント型（既存シグナルの意味を変えない）を推奨。

### 5.7 機能フラグ

```toml
# .env または EnvironmentFile

# ストリーミングモード有効化（デフォルト false = 既存逐次動作）
VOICEBOT_STREAMING_ENABLED=false

# 文単位 TTS の最大文字数（デフォルト 50）
VOICEBOT_STREAMING_SENTENCE_MAX_CHARS=50
```

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #249 | STEER-249 | 起票 |
| STEER-249 | `LlmStreamPort` | 新規 Port 追加 |
| STEER-249 | `SentenceAccumulator` | 新規コンポーネント |
| STEER-249 | `AppEnqueueBotAudioFile` | 新規セッションイベント |
| STEER-249 | `PlaybackQueue` | 既存 playback 拡張 |
| `LlmStreamPort` | Ollama streaming 実装 | 実装 |
| `LlmStreamPort` | UT（streaming Port mock） | テスト |
| `SentenceAccumulator` | UT（境界値テスト） | テスト |

---

## 7. オープンクエスチョン（レビューで決める）

| # | 質問 | 選択肢 | 推奨案 |
|---|------|--------|--------|
| OQ-1 | `LlmStreamPort` は独立トレイトか `LlmPort` 拡張か？ | 独立 / 拡張 | 独立（ISP 準拠） |
| OQ-2 | OpenAI cloud LLM streaming を今回スコープに含めるか？ | 含む / 含まない | 含まない（別 Issue） |
| OQ-3 | `max_chars` 初期値は何文字か？ | 30 / 50 / 80 | 50（暫定） |
| OQ-4 | TTS ジョブは直列か並列か？ | 直列 / tokio::spawn 並列 | 直列（初回） |
| OQ-5 | `AppEnqueueBotAudioFile` は新規型か既存に enqueue フラグ追加か？ | 新規型 / フラグ追加 | 新規型 |
| OQ-6 | barge-in（ユーザー再発話時のキャンセル）を今回含めるか？ | 含む / 含まない | 含まない（設計論点が大きい） |
| OQ-7 | `mpsc` チャネルを bounded にするか？（バックプレッシャ） | bounded / unbounded | bounded（AGENTS.md 既存方針に従う） |
| OQ-8 | ストリーミング時の LLM タイムアウト設計は？（first-token / total） | first-token のみ / both | first-token + total の二段構え |

---

## 8. レビューチェックリスト

### 8.1 仕様レビュー（Review → Approved）

- [ ] ストーリー（Why）と目的が合意されているか
- [ ] `LlmStreamPort` のシグネチャが実装可能か（futures::Stream 依存 OK か）
- [ ] 文区切りルールが実運用に適しているか（日本語 TTS との相性）
- [ ] 再生キュー設計が既存 `cancel_playback()` / barge-in と矛盾しないか
- [ ] 機能フラグのデフォルト値（`false`）で既存動作が保証されるか
- [ ] タイムアウト方針が `AGENTS.md:116` の外部 I/O timeout 必須要件を満たすか
- [ ] オープンクエスチョン OQ-1〜OQ-8 が全て決定されているか

### 8.2 マージ前チェック（Approved → Merged）

- [ ] 実装完了（Codex）
- [ ] `VOICEBOT_STREAMING_ENABLED=false` で既存テスト全 PASS
- [ ] `VOICEBOT_STREAMING_ENABLED=true` で手動通話テスト実施
- [ ] CodeRabbit レビュー対応済み
- [ ] 本体仕様書への反映方針確認

---

## 9. 備考

### 9.1 スコープ外（別 Issue 推奨）

- **真の ASR ストリーミング**: `AsrPort` の Port 変更・ローカル whisper サーバーの WebSocket/chunk 対応まで影響が広い（第 2 段階）
- **真の TTS ストリーミング**: VOICEVOX の `/synthesis` streaming endpoint 対応（第 3 段階）
- **OpenAI LLM streaming**: SSE 形式パース実装（第 2 段階候補）
- **barge-in（割り込み再生キャンセル）**: TTS ジョブ・再生キューの停止 + キャンセルトークン設計が必要。既存 `AGENTS.md:127–128` の割り込み優先方針を踏まえた別設計が必要

### 9.2 未追跡ファイルの注意

`git status` にて `virtual-voicebot-backend/script/whisper_server_9010.py` が未追跡状態。
調査上は実験的ファイルと見られる。SoT として扱うかはオーナー判断が必要。

### 9.3 参照コード（調査時点のスナップショット）

| 参照 | 内容 |
|------|------|
| `src/service/call_control/mod.rs:384` | `AsrChunk { end: true }` 一括投入 |
| `src/service/call_control/mod.rs:425` | `handle_user_text()` 主フロー |
| `src/service/call_control/mod.rs:588` | TTS 呼び出し → `AppSendBotAudioFile` 送信 |
| `src/shared/ports/ai/llm.rs:5` | `LlmPort::generate_answer` 戻り値 `Result<String>` |
| `src/shared/ports/ai/tts.rs:7` | `TtsPort::synth_to_wav` 戻り値 `Result<PathBuf>` |
| `src/service/ai/mod.rs:1022` | Ollama `stream: false` 固定 |
| `src/protocol/session/services/playback_service.rs:18` | `start_playback()` 先頭で既存再生をキャンセル |
| `src/protocol/session/services/playback_service.rs:53` | `step_playback()` フレーム逐次 RTP 送信 |
| `virtual-voicebot-backend/AGENTS.md:116` | 外部 I/O は timeout 必須 |
| `virtual-voicebot-backend/AGENTS.md:127–128` | TTS→RTP 割り込み優先方針 |

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-25 | 初版作成（Draft） | Claude Sonnet 4.6 |
