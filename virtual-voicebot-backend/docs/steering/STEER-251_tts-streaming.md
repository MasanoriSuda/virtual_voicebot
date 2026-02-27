# STEER-251: 真の TTS ストリーミング化（第3段階）

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-251 |
| タイトル | 真の TTS ストリーミング化（第3段階） |
| ステータス | Draft |
| 関連Issue | #251 |
| 前提 Issue | #249（LLM ストリーミング ＋ 文単位 TTS 先行再生）|
| 優先度 | P1 |
| 作成日 | 2026-02-26 |

---

## 2. ストーリー（Why）

### 2.1 背景

STEER-249 では LLM ストリーミング ＋ 文単位 TTS 先行再生を実装し、
全文確定待ちのレイテンシを大幅に削減した。

```
[STEER-249 後の状態]

LLM stream → SentenceAccumulator → 文1確定
                                       │
                                       ▼
                          synth_to_wav(文1)          ← HTTP POST /synthesis（WAV 一括受信: ~300–1000ms）
                                       │（WAV 全体受信後）
                                       ▼
                          AppEnqueueBotAudioFile     ← キュー投入 → 再生開始
                                       │
LLM stream → 文2確定 → synth_to_wav(文2) → ...（並列）
```

しかし **文単位の TTS 合成は依然として一括受信（HTTP POST → WAV 全体返却）** であり、
各文において「合成完了まで待機 → ファイル書き出し → キュー投入」の遅延（1文あたり 300〜1000ms）が残る。

「真の TTS ストリーミング」は、TTS サービスが音声バイトを順次（ストリーム形式）返す場合に
**WAV 全体の受信を待たずに再生を開始できる** 仕組みを実装することで、この残余遅延をさらに削減する。

### 2.2 目的

TTS 合成中に音声バイトを逐次受信し、十分なデータが揃った時点で再生を開始することで、
**文単位の first-audio latency をさらに短縮する**。

#249（LLM ストリーミング） + #250（ASR ストリーミング） + #251（TTS ストリーミング）で、
発話 → 応答 のフルパスを通じた latency を最大限削減する。

### 2.3 ユーザーストーリー

```
As a 電話ユーザー
I want to ボイスボットの応答音声が文節ごとに途切れなく素早く聴こえてほしい
So that 人間との会話に近い自然なテンポで対話できる

受入条件:
- [ ] VOICEVOX または OpenAI TTS がストリーミング API を持つ場合、音声バイトを逐次受信できる
- [ ] 文単位 TTS の latency が逐次経路より短縮される（採用した streaming TTS provider の first-byte 短縮効果）
- [ ] 既存逐次経路（VOICEBOT_TTS_STREAMING_ENABLED=false）で全既存テスト PASS
- [ ] TTS ストリーミングが利用不可の場合は逐次経路へ自動フォールバック
- NOTE: early-start（WAV 途中からの再生開始）は Phase A スコープ外（OQ-3 = 含まない）。
  Phase A の主価値は「TtsStreamPort インフラ整備」と「採用した streaming TTS provider の first-byte 短縮」。
  文単位の体感改善（即時再生）は別 Issue（Phase B: playback_service progressive read）で実現する。
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-26 |
| 起票理由 | STEER-249 §9.1「スコープ外（別 Issue 推奨）」で分割確定 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Sonnet 4.6 |
| 作成日 | 2026-02-26 |
| 指示者 | @MasanoriSuda |
| 指示内容 | Approved 後に引き継ぎ |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | @MasanoriSuda | 2026-02-26 | OK | 重大3・中2・軽1（Round 1 NG）→ 修正後 軽3（Round 2）→ 修正後 軽2（Round 3）→ 修正後 指摘なし（Round 4）で LGTM |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-26 |
| 承認コメント | lgtm |

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
| docs/requirements/（AI 連携 RD） | 修正 | TTS ストリーミング要件追加 |
| docs/design/detail/（AI サービス DD） | 修正 | `TtsStreamPort` シグネチャ、ストリーミング TTS クライアント設計 |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| `src/shared/ports/ai/tts.rs` | 追加 | `TtsStreamPort` トレイト |
| `src/service/call_control/mod.rs` | 修正 | `handle_user_text_streaming()` 内 TTS 呼び出しを `TtsStreamPort` 経由に切り替え、チャンク収集タスク追加 |
| `src/service/ai/mod.rs` | 追加 | VOICEVOX streaming HTTP クライアント（`call_voicevox_stream()`・**OQ-1 参照**）または OpenAI TTS streaming（**OQ-2 参照**）|
| `src/shared/config/mod.rs` | 追加 | `VOICEBOT_TTS_STREAMING_ENABLED`、`TTS_STREAMING_CONNECT_TIMEOUT_MS` 等の環境変数 |
| `src/protocol/session/services/playback_service.rs` | 修正 | early-start 採用時のみ（**OQ-3** で判断。不採用なら変更なし）|

---

## 5. 差分仕様（What / How）

### 5.1 前提：現状の確認

| 箇所 | 現状 |
|------|------|
| `src/shared/ports/ai/tts.rs:7` | `TtsPort::synth_to_wav()` — テキスト → `AiFuture<Result<PathBuf, TtsError>>`（WAV ファイルパス）|
| `src/service/ai/mod.rs:1934` | `synth_zundamon_for_stage()` — POST `/audio_query` → POST `/synthesis`（`response.bytes().await` で WAV 一括受信）|
| `src/service/call_control/mod.rs:810` | `handle_user_text_streaming()` — 文単位に `synth_to_wav()` を呼び出し → WAV 全体受信後に `AppEnqueueBotAudioFile` |
| `src/service/ai/mod.rs:158` | `http_client_for_stream` — streaming 用 HTTP クライアントが **既に存在**（LLM ストリーミングで利用中）|
| VOICEVOX Engine API | `/synthesis` が `Transfer-Encoding: chunked` をサポートするか未確認（**OQ-1**）|

### 5.2 `TtsStreamPort` 独立トレイト（ISP 準拠）

既存 `TtsPort`（`synth_to_wav` → `PathBuf`）は変更しない。
`LlmStreamPort`（STEER-249 §5.2）・`AsrStreamPort`（STEER-250 §5.2）と同方針で
**独立トレイト** として追加する。

```rust
// src/shared/ports/ai/tts.rs（追加）

/// TTS 音声ストリーム — 1つの WAV ストリームを断片バイト列として順次 yield する。
///
/// ・各 item は音声バイト列の任意サイズの断片であり、各 item が独立した WAV ではない。
/// ・チャンク境界は実装依存（reqwest `bytes_stream()` は任意境界で分割される）。
///   WAV ヘッダが先頭 item に収まる保証はない。
/// ・adapter はチャンクを順番に concat した結果が完全な WAV になることを保証する（OQ-4 参照）。
/// ・呼び出し元（spawn タスク）はチャンクを concat して WAV ファイルに書き込む。
pub type TtsStream = Pin<Box<dyn Stream<Item = Result<Vec<u8>, TtsError>> + Send>>;

pub trait TtsStreamPort: Send + Sync {
    /// テキストを TTS ストリーミングで合成する。
    /// 戻り値の `TtsStream` は音声バイトを逐次 yield する。
    /// connect タイムアウトのみ実装内部で処理する（first-chunk / total は呼び出し元 spawn タスクで管理する）。
    /// speaker_id 等のプロバイダ固有パラメータは各実装が内部設定（環境変数 / config）から取得する。
    fn synth_stream(
        &self,
        call_id: String,
        text: String,
    ) -> AiFuture<Result<TtsStream, TtsError>>;
}
```

> **設計論点 A**（OQ-1）: VOICEVOX の `/synthesis` が `Transfer-Encoding: chunked` を
> サポートするか確認が必要。未サポートの場合は OpenAI TTS streaming（OQ-2）が主選択肢。
>
> **設計論点 B**（provider-neutral 設計）: `speaker_id: u32` 等の VOICEVOX 固有パラメータは
> トレイトシグネチャに含めない。各実装（`VoicevoxTtsStreamAdapter`・`OpenAiTtsStreamAdapter`）が
> それぞれの設定（環境変数・config）から取得する。これにより VOICEVOX / OpenAI TTS 双方を
> 同一の `TtsStreamPort` トレイトで抽象できる。

### 5.3 `AppWorker` への接続方針

STEER-249 の `llm_stream_port`・STEER-250 の `asr_stream_port` と同じく、
`AppWorker` に **別フィールド** として追加する（`AiServices` 集約トレイトは変更しない）。

```rust
// src/service/call_control/mod.rs（修正イメージ）

pub struct AppWorker {
    // ...（既存フィールド）
    tts_stream_port: Option<Arc<dyn TtsStreamPort>>,  // 追加（None = フラグ OFF or 未対応）
}
```

以下の条件をすべて満たす場合のみストリーミング経路へ進む。いずれかが欠けると逐次経路（`synth_to_wav()` 既存実装）へフォールバック。

- `VOICEBOT_STREAMING_ENABLED=true`（STEER-249 文単位 TTS モード。LLM streaming + SentenceAccumulator が前提、OQ-5 参照）
- `VOICEBOT_TTS_STREAMING_ENABLED=true`（本機能フラグ）
- `tts_stream_port.is_some()`（実装インスタンスが DI 注入済み）

### 5.4 ストリーミング TTS 呼び出し（`handle_user_text_streaming` 修正）

**フロー概要（OQ-3 = early-start 不採用の場合）:**

```
現行:
  文確定 → synth_to_wav(text)
         → await（WAV 全体受信: ~300–1000ms）
         → write_wav_to_tmp()
         → AppEnqueueBotAudioFile

変更後（tts_stream_port が Some の場合）:
  文確定 → synth_stream(text) → TtsStream 取得
         → spawn: チャンク収集タスク
              │ chunks 到着 → memory buffer に append
              │ ストリーム終了 → write_wav_to_tmp(buffer)
              │              → oneshot で tmp_path を返却
         → oneshot を await → AppEnqueueBotAudioFile（再生開始）
```

> OQ-3 = early-start 採用の場合は、一定バイト受信後に `AppEnqueueBotAudioFile` を先行送信し、
> 残チャンクを同一ファイルに append する（playback_service の progressive read 対応が別途必要）。

**擬似コード（`handle_user_text_streaming` 修正イメージ）:**

```rust
// src/service/call_control/mod.rs（修正イメージ）

async fn tts_with_streaming(
    &self,
    call_id: &CallId,
    text: &str,
    generation_id: PlaybackGenerationId,
) {
    let Some(port) = &self.tts_stream_port else {
        return self.tts_sequential(call_id, text, generation_id).await;
    };

    match port.synth_stream(call_id.to_string(), text.to_string()).await {
        Ok(stream) => {
            let (done_tx, done_rx) = oneshot::channel::<Result<PathBuf, TtsError>>();
            let tmp_path = tmp_wav_path(call_id);

            tokio::spawn(async move {
                let mut buf: Vec<u8> = Vec::new();
                tokio::pin!(stream);
                // NOTE: ストリーム全体タイムアウト → tokio::time::timeout(TTS_STREAMING_TOTAL_TIMEOUT_MS, async { ... }) で wrap する
                // NOTE: 最初のチャンク受信タイムアウト → 最初の stream.next() のみ tokio::select! + sleep(TTS_STREAMING_FIRST_CHUNK_TIMEOUT_MS) で保護する
                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(bytes) => buf.extend_from_slice(&bytes),
                        Err(e) => {
                            let _ = done_tx.send(Err(e));
                            return;
                        }
                    }
                }
                // ストリーム完了 → ファイル書き込み
                match write_wav_to_tmp(&tmp_path, &buf) {
                    Ok(()) => { let _ = done_tx.send(Ok(tmp_path)); }
                    Err(e) => { let _ = done_tx.send(Err(TtsError::IoError(e.to_string()))); }
                }
            });

            match done_rx.await {
                Ok(Ok(path)) => self.enqueue_audio(path, generation_id).await,
                Ok(Err(e)) => {
                    log::warn!("[tts stream {call_id}] failed: {e}; fallback to sequential");
                    self.tts_sequential(call_id, text, generation_id).await
                }
                Err(_) => {
                    log::warn!("[tts stream {call_id}] spawn task dropped; fallback to sequential");
                    self.tts_sequential(call_id, text, generation_id).await
                }
            }
        }
        Err(e) => {
            log::warn!("[tts stream {call_id}] synth_stream connection failed: {e}; fallback");
            self.tts_sequential(call_id, text, generation_id).await;
        }
    }
}
```

**タイムアウト責務**

| タイムアウト種別 | 担当箇所 | 設定キー |
|----------------|---------|---------|
| HTTP 接続（TCP + handshake） | `TtsStreamPort` 実装の connect_timeout | `TTS_STREAMING_CONNECT_TIMEOUT_MS`（新規） |
| 最初のチャンク受信 | spawn タスク内 `tokio::select!` | `TTS_STREAMING_FIRST_CHUNK_TIMEOUT_MS`（新規）|
| ストリーム全体 | spawn タスク内 `tokio::time::timeout` | `TTS_STREAMING_TOTAL_TIMEOUT_MS`（新規） |

タイムアウト超過時は `done_tx.send(Err(...))` → 呼び出し元が `tts_sequential()` へフォールバック。

### 5.5 VOICEVOX サーバーサイド（OQ-1）

現行実装（`src/service/ai/mod.rs:1934`）:
```rust
// 一括受信（現状）
let response = client.post(url).json(&query).send().await?;
let wav_bytes = response.bytes().await?;  // WAV 全体が届くまでブロック
```

VOICEVOX Engine が `Transfer-Encoding: chunked` をサポートする場合の変更イメージ:
```rust
// streaming 受信（変更案）
let response = client.post(url).json(&query).send().await?;
let mut stream = response.bytes_stream();  // reqwest の ByteStream
while let Some(chunk) = stream.next().await {
    yield chunk?;  // TtsStream へ中継
}
```

既存の `http_client_for_stream`（`src/service/ai/mod.rs:158`）をそのまま再利用できる。

> **OQ-1**: 現行 VOICEVOX Engine バージョンが `/synthesis` で chunked 転送をサポートするか、
> 実装前に以下の手順で確認し、結果を記録すること：
>
> ```bash
> # 1. /audio_query で query JSON を取得
> curl -s "http://localhost:50021/audio_query?text=テスト&speaker=3" -o /tmp/query.json
>
> # 2. /synthesis のレスポンスヘッダを確認（first-byte timing も記録）
> curl -s -X POST "http://localhost:50021/synthesis?speaker=3" \
>   -H "Content-Type: application/json" \
>   -d @/tmp/query.json \
>   -D -          \   ← レスポンスヘッダを stdout に表示
>   --no-buffer   \   ← バッファリング抑制（first-byte timing 確認用）
>   -o /dev/null
> ```
>
> 確認ポイント：
> - `Transfer-Encoding: chunked` → chunked 転送サポート → VOICEVOX streaming 実装可
> - `Content-Length: <固定値>` → 一括返却 → OpenAI TTS streaming を先行（OQ-2）

### 5.6 機能フラグ

```toml
# .env または EnvironmentFile

# TTS ストリーミングモード有効化（デフォルト false = 既存一括動作）
# NOTE: ON でも tts_stream_port が None の場合は逐次経路へフォールバック
# NOTE: VOICEBOT_STREAMING_ENABLED=true（STEER-249 文単位 TTS）が有効でない場合は逐次経路へフォールバック（OQ-5 参照）
VOICEBOT_TTS_STREAMING_ENABLED=false

# HTTP 接続タイムアウト ms
TTS_STREAMING_CONNECT_TIMEOUT_MS=3000

# 最初のチャンク受信タイムアウト ms
TTS_STREAMING_FIRST_CHUNK_TIMEOUT_MS=3000

# ストリーム全体タイムアウト ms
TTS_STREAMING_TOTAL_TIMEOUT_MS=15000
```

> `TTS_STREAMING_EARLY_START_BYTES` は OQ-3 で early-start を採用した場合に追加する。

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #251 | STEER-251 | 起票 |
| STEER-249 §9.1 | STEER-251 | スコープ分割（前提 Issue） |
| STEER-251 | `TtsStreamPort` | 新規 Port 追加 |
| STEER-251 | `handle_user_text_streaming` 修正 | TTS 呼び出し切り替え |
| `TtsStreamPort` | VOICEVOX streaming 実装（`call_voicevox_stream()`） | 実装（OQ-1 確認後） |
| `TtsStreamPort` | OpenAI TTS streaming 実装 | 実装（OQ-2 採用の場合） |
| `TtsStreamPort` | UT（streaming Port mock） | テスト |

---

## 7. オープンクエスチョン

### 7.1 未決定（OQ-1〜6）

| # | 質問 | 選択肢 | 推奨案 |
|---|------|--------|--------|
| OQ-1 | VOICEVOX Engine の `/synthesis` は `Transfer-Encoding: chunked` をサポートするか？ | する / しない | **要確認**（実装前に `curl -D -` でレスポンスヘッダを確認し `Transfer-Encoding: chunked` か `Content-Length` 固定かを記録する。詳細は §5.5 参照。未サポートなら OpenAI TTS streaming を先行させる） |
| OQ-2 | OpenAI TTS streaming（`/audio/speech` with `stream=True`）を今回スコープに含めるか？ | 含む / 含まない | **含む**（OQ-1 で VOICEVOX 未サポートの場合のメイン選択肢。STEER-231 の OpenAI 統合基盤を活用。VOICEVOX サポート確認後でなくても先行実装可） |
| OQ-3 | early-start（WAV 途中からの再生開始）を今回スコープに含めるか？ | 含む（Phase A+B） / 含まない（Phase A のみ） | **含まない（Phase A のみ）**（playback_service の progressive read 対応が別途必要。まず `TtsStreamPort` インフラを整備し、early-start は別 Issue で） |
| OQ-4 | `TtsStream` item の内容は WAV チャンク（ヘッダ含む）か PCM raw か？ | WAV chunks / PCM raw | **WAV chunks**（VOICEVOX は WAV 形式を返す。OpenAI TTS streaming では `response_format=wav` を指定し WAV で受信する。MP3 は使用しない。変換は受信後に行う） |
| OQ-5 | TTS ストリーミングは `VOICEBOT_STREAMING_ENABLED=true`（STEER-249 文単位 TTS）が前提か？ | STREAMING_ENABLED=true が前提 / 独立した設定 | **VOICEBOT_STREAMING_ENABLED=true が前提**（逐次モードでは1文が長く TTS 時間が増大するため、LLM ストリーミング+文単位分割を先に有効にすべき） |
| OQ-6 | タイムアウト発生時のフォールバック先は逐次 TTS（`synth_to_wav`）か、エラー終了か？ | 逐次 TTS へフォールバック / Err を呼び出し元に返す | **逐次 TTS へフォールバック**（他 streaming Port（LLM/ASR）と同方針。体験継続を優先）|

---

## 8. レビューチェックリスト

### 8.1 仕様レビュー（Review → Approved）

- [ ] ストーリー（Why）と目的が合意されているか
- [ ] `TtsStreamPort` のシグネチャが VOICEVOX と OpenAI TTS streaming の両方を抽象できるか
- [ ] 既存 `TtsPort`（`synth_to_wav`）を変更せず独立トレイトとして追加されているか（ISP 準拠）
- [ ] 機能フラグ（`VOICEBOT_TTS_STREAMING_ENABLED=false`）で既存動作が保護されるか
- [ ] `tts_stream_port=None` 時に逐次経路へフォールバックするか
- [ ] タイムアウト方針が `AGENTS.md:116` の外部 I/O timeout 必須要件を満たすか（connect/first-chunk/total の 3 層）
- [ ] OQ-3（early-start スコープ外）が確定し、playback_service 変更が今回スコープ外であることが明確か
- [ ] OQ-1（VOICEVOX streaming サポート確認）のアクションが明確か
- [ ] OQ-1〜6 が全て決定されているか

### 8.2 マージ前チェック（Approved → Merged）

- [ ] 実装完了（Codex）
- [ ] `VOICEBOT_TTS_STREAMING_ENABLED=false` で既存テスト全 PASS
- [ ] `VOICEBOT_TTS_STREAMING_ENABLED=true` で手動通話テスト実施（TTS streaming chunk → playback フロー確認）
- [ ] CodeRabbit レビュー対応済み
- [ ] 本体仕様書への反映方針確認

---

## 9. 備考

### 9.1 スコープ外（別 Issue 推奨）

- **playback_service のプログレッシブ読み込み（early-start Phase B）**: WAV ファイル書き込み中に逐次読み出せるよう変更が必要。OQ-3 の early-start 採用時に必要
- **barge-in（割り込み再生キャンセル）**: TTS ジョブ・再生キューの停止 + キャンセルトークン設計（STEER-249 §9.1 参照）
- **VOICEVOX streaming endpoint 追加（OSS 貢献）**: OQ-1 が「未サポート」の場合は VOICEVOX Engine 本体への PR 検討

### 9.2 前提 Issue との関係

STEER-249（#249）の実装が完了している前提で本 Issue を実装する。
具体的には `AppEnqueueBotAudioFile` / `playback_queue` / `SentenceAccumulator` が実装済みであることが必要
（`handle_user_text_streaming()` の文単位 TTS キュー投入フローを流用するため）。

### 9.3 参照コード（調査時点のスナップショット）

| 参照 | 内容 |
|------|------|
| `src/shared/ports/ai/tts.rs:7` | `TtsPort::synth_to_wav()` — WAV ファイルパス返却（`PathBuf`）|
| `src/service/ai/mod.rs:1934` | `synth_zundamon_for_stage()` — `/audio_query` + `/synthesis`（一括受信）|
| `src/service/ai/mod.rs:158` | `http_client_for_stream` — streaming 用 HTTP クライアント（既存・再利用可）|
| `src/service/call_control/mod.rs:810` | `handle_user_text_streaming()` — 文単位 TTS 呼び出し現行実装 |
| `src/protocol/session/services/playback_service.rs:45` | `enqueue_playback()` — キュー登録 |
| `src/shared/ports/ai/llm.rs:9` | `LlmStream / LlmStreamPort` — 設計参考（同パターン） |
| `src/shared/ports/ai/asr.rs` | `AsrStreamPort` — 設計参考（同パターン） |
| `virtual-voicebot-backend/AGENTS.md:116` | 外部 I/O は timeout 必須 |

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-26 | 初版作成（Draft） | Claude Sonnet 4.6 |
| 2026-02-26 | レビュー指摘対応（軽微3件）: §5.2 `TtsStream` コメントを「1つの WAV ストリームの断片バイト列」と明文化、§2.3 受入条件に Phase A 価値の限定注記追加（early-start スコープ外）、OQ テーブル OQ-1 推奨案に §5.5 参照追記・§5.5 に curl ヘッダ確認手順を明記 | Claude Sonnet 4.6 |
| 2026-02-26 | レビュー指摘対応 Round 3 OK（軽2件）: [軽1] §2.3 Phase A 注記の `VOICEVOX streaming API の first-byte 短縮` → `採用した streaming TTS provider の first-byte 短縮` に統一（provider-neutral 化）。[軽2] §9.3 参照表の `TtsPort::synth_to_wav()` 説明を `WAV 全体返却` → `WAV ファイルパス返却（PathBuf）` に修正 | Claude Sonnet 4.6 |
| 2026-02-26 | レビュー指摘対応 Round 2 OK（軽3件）: [軽1] §2.3 受入条件 latency 文言を `VOICEVOX streaming API` → `採用した streaming TTS provider` に変更（provider-neutral 化）。[軽2] §5.2 `TtsStream` コメントを「先頭 item に WAV ヘッダ保証」から「チャンク境界は実装依存・任意サイズ断片・adapter が concat 完全WAV を保証」に緩和。[軽3] §5.4 擬似コード `tokio::spawn` ブロック内に first-chunk / total timeout の NOTE コメントを追加 | Claude Sonnet 4.6 |
| 2026-02-26 | レビュー指摘対応 Round 1 NG（重大3・中2・軽1）: [重大1] §5.1 `synth_to_wav()` 戻り値型を `Vec<u8>` → `PathBuf` に修正・§5.2 説明文も同期。[重大2] §5.2 タイムアウトコメントを connect のみ内部処理と明確化・§5.4 表の first-chunk 担当を spawn タスク内に統一（"または" 削除）。[重大3] §5.2 `synth_stream` シグネチャから `speaker_id: u32` を除去しプロバイダ固有パラメータを実装内部取得に変更・設計論点 B 追記・擬似コードの `SPEAKER_ID` 引数を削除・OQ-4 推奨案に OpenAI TTS は `response_format=wav` 固定（MP3 不使用）を明記。[中1] §5.4 擬似コード match を `Ok(Err(e))` / `Err(_)` の2アームに分離（無効 Rust 修正）。[中2] §5.3 ストリーミング有効条件に `VOICEBOT_STREAMING_ENABLED=true` を追加・§5.6 NOTE に OQ-5 参照追記。[軽1] §9.3 `asr.rs` 行から `TtsStream` の誤記を除去 | Claude Sonnet 4.6 |
