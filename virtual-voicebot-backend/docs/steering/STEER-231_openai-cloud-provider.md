# STEER-231: OpenAI クラウドプロバイダー追加（ASR / LLM / TTS / weather 要約）

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-231 |
| タイトル | OpenAI クラウドプロバイダー追加（ASR / LLM / TTS / weather 要約） |
| ステータス | Approved |
| 関連Issue | #231 |
| 優先度 | P1 |
| 作成日 | 2026-02-24 |

---

## 2. ストーリー（Why）

### 2.1 背景

ローカルサーバー（Ollama 等）は常時稼働を前提とせず、ラズパイ環境では高負荷な LLM/TTS の実行が困難な場合がある。現状の cloud 対応は以下の通りで、OpenAI は未対応。

| 機能 | 現状の cloud provider | 問題 |
|------|----------------------|------|
| ASR | AWS Transcribe（`mod.rs:421`付近） | OpenAI 未対応 |
| LLM（通常会話） | Gemini（`mod.rs:507`付近） | OpenAI 未対応 |
| TTS | cloud なし（`mod.rs:116`/`mod.rs:901`付近） | cloud stage 自体が存在しない |
| weather 要約 LLM | local のみ（`src/service/ai/weather.rs:67`、`src/service/ai/mod.rs:646`） | cloud なし |

OpenAI を契約済みであり、軽量モデルで PoC を実施することで、ローカルサーバー不在でも AI 機能を継続利用したい。

### 2.2 目的

OpenAI を **クラウド内最優先プロバイダー** として ASR / LLM / TTS / weather 要約 LLM に追加する。

- 既存フォールバック（AWS Transcribe / Gemini / local / raspi）は維持する
- PoC フェーズとして、**モデルはコード固定（軽量）** で進める（env 化しない）
- TTS は cloud stage を新設して OpenAI を優先する（ずんだもん音声は PoC スコープ外・割り切り済み）
- weather 要約 LLM も通常会話 LLM と方針を揃えて OpenAI 優先にする

### 2.3 ユーザーストーリー

```text
As a オペレーター
I want to ローカルサーバーが停止していても AI 機能（ASR/LLM/TTS）を利用したい
So that ローカル環境の状態に依存せず安定したボイスボット会話を提供できる

受入条件:
- [ ] OPENAI_API_KEY を設定した環境で ASR が OpenAI（gpt-4o-mini-transcribe）優先で動作する
- [ ] OPENAI_API_KEY を設定した環境で LLM（通常会話）が OpenAI（gpt-4o-mini）優先で動作する
- [ ] OPENAI_API_KEY を設定した環境で TTS が OpenAI（gpt-4o-mini-tts）優先で動作する
- [ ] OPENAI_API_KEY を設定した環境で weather 要約 LLM が OpenAI 優先で動作する
- [ ] OpenAI が失敗した場合に既存フォールバック（AWS/Gemini/local/raspi）に落ちる
- [ ] OPENAI_API_KEY を設定しない環境では従来通りの挙動が維持される
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-24 |
| 起票理由 | ローカルサーバー不在時でも AI 機能（ASR/LLM/TTS）を利用できるようにしたい。OpenAI 契約済みのため PoC として最優先 cloud プロバイダーとして追加する |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Sonnet 4.6 |
| 作成日 | 2026-02-24 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "OpenAI を ASR/LLM/TTS の cloud 最優先として追加。既存フォールバック維持。PoC = 軽量モデル固定。weather 要約 LLM も含める。ずんだもん割り切り済み。" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | Codex | 2026-02-24 | NG | ①ファイルパス誤記（`src/service/weather.rs`→`src/service/ai/weather.rs`、`src/config/`→`src/shared/config/mod.rs`） ②`asr/llm/tts_stage_count`・`llm_cloud_enabled`・startup warning・all-stages-disabled 判定の更新要件が§5に未記載 ③§9の`CLOUD_TIMEOUT_MS`は実在しない（正: `ASR_CLOUD_TIMEOUT_MS`/`LLM_CLOUD_TIMEOUT_MS`） |
| 2 | Codex | 2026-02-24 | NG | ①§7.2 自動テスト項目が弱い（フォールバック・weather 独立経路が未カバー） ②§9 weather 緩和策の `OPENAI_LLM_ENABLED=false` は通常会話と共通フラグのため「weather 個別無効」は不正確 ③§5.6 TTS WAV フォーマット断定が §8 Q2（実測確認前提）と矛盾 |
| 3 | Codex | 2026-02-24 | NG | §7.2 自動テストで TTS cloud（OpenAI → local → raspi）フォールバックが未計画 |
| 4 | Codex | 2026-02-24 | OK | 前回残件解消（TTS 自動テスト追加・weather 緩和策文言・TTS WAV フォーマット断定修正）。Q1/Q2 はオーナー回答済みで確定 |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-24 |
| 承認コメント | lgtm |

### 3.5 実装

| 項目 | 値 |
|------|-----|
| 実装者 | Codex |
| 実装日 | 2026-02-24 |
| 指示者 | @MasanoriSuda |
| 指示内容 | OpenAI を cloud 最優先 provider として ASR/LLM/TTS/weather 要約に追加（既存フォールバック維持、PoC 固定モデル） |
| コードレビュー | `cargo test --lib` / `cargo clippy --lib -- -D warnings` / `cargo build --lib` 実行で確認 |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | - |
| マージ日 | - |
| マージ先 | `src/service/ai/mod.rs`、`src/service/ai/weather.rs`、`src/shared/config/mod.rs`、`.env.example` |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| `docs/steering/STEER-231_openai-cloud-provider.md`（本ファイル） | 新規 | 本 Issue の差分仕様 |

### 4.2 影響するコード

| ファイル | 変更種別 | 概要 |
|---------|---------|------|
| `src/service/ai/mod.rs` | 修正 | ASR cloud provider に OpenAI 追加（L421 付近）、LLM cloud provider に OpenAI 追加（L507 付近）、TTS に cloud stage 新設（L116/L901 付近）、`asr/llm/tts_stage_count`・`llm_cloud_enabled`・startup warning・all-stages-disabled 判定を OpenAI 含む形に更新（L156-L212 付近） |
| `src/service/ai/weather.rs` | 修正 | weather 要約 LLM に OpenAI 優先を追加（`src/service/ai/weather.rs:67`、`src/service/ai/mod.rs:646` 付近） |
| `src/shared/config/mod.rs` | 修正 | `OPENAI_API_KEY`・`OPENAI_BASE_URL`・各種 enabled フラグ・`TTS_CLOUD_TIMEOUT_MS` の読み込み追加（`src/shared/config/mod.rs:877`/`918`/`1021` 付近） |
| `.env.example` | 修正 | OpenAI 用サンプル環境変数追記 |

---

## 5. 差分仕様（What / How）

### 5.1 フォールバック順序（PoC 後の確定仕様）

| 機能 | フォールバック順序（左から優先） |
|------|-------------------------------|
| ASR | **OpenAI** → AWS Transcribe → local → raspi |
| LLM（通常会話） | **OpenAI** → Gemini → local → raspi |
| TTS | **OpenAI（新規 cloud stage）** → local → raspi |
| weather 要約 LLM | **OpenAI** → local（既存 `LLM_LOCAL_*` フォールバック維持） |

> **Note:** 各ステージの有効/無効は対応する設定キーの存在（`OPENAI_API_KEY` 等）で判定する。

### 5.2 追加する環境変数

| 変数名 | 必須/任意 | デフォルト | 説明 |
|--------|---------|-----------|------|
| `OPENAI_API_KEY` | 必須（未設定時は OpenAI ステージをスキップ） | なし | OpenAI API 認証キー |
| `OPENAI_BASE_URL` | 任意 | `https://api.openai.com/v1` | OpenAI API エンドポイント（プロキシ対応用） |
| `OPENAI_ASR_ENABLED` | 任意 | `true` | ASR での OpenAI 使用フラグ |
| `OPENAI_LLM_ENABLED` | 任意 | `true` | LLM での OpenAI 使用フラグ |
| `OPENAI_TTS_ENABLED` | 任意 | `true` | TTS での OpenAI 使用フラグ |
| `TTS_CLOUD_TIMEOUT_MS` | 任意 | `10000` | TTS cloud ステージのタイムアウト（ms）。現状 TTS に cloud がないため新規追加 |

> **Note:** PoC フェーズでは各機能のモデル名はコード固定とし、env 化しない（§8 Q1 参照）。

### 5.3 PoC 固定モデル

| 機能 | モデル | 備考 |
|------|--------|------|
| ASR | `gpt-4o-mini-transcribe` | 軽量転写モデル |
| LLM（通常会話・weather 要約共通） | `gpt-5-mini` | §8 Q1 確定（OpenAI 公式 Models ページで存在確認済み） |
| TTS | `gpt-4o-mini-tts` | §8 Q2 参照（音声フォーマット確認要） |

### 5.4 ASR 変更概要（`mod.rs` L421 付近）

```
// 変更前（cloud stage = AWS Transcribe のみ）:
AsrStage::Cloud => {
    // AWS Transcribe 呼び出し
}

// 変更後（cloud stage = OpenAI → AWS Transcribe の順）:
AsrStage::Cloud => {
    if openai_asr_enabled {
        // OpenAI Whisper API (gpt-4o-mini-transcribe) で WAV を multipart POST
        // 失敗時は次の provider にフォールバック
    }
    // OpenAI 未設定 or 失敗時: 既存 AWS Transcribe 呼び出し
}
```

### 5.5 LLM 変更概要（`mod.rs` L507 付近）

```
// 変更前（cloud stage = Gemini のみ）:
LlmStage::Cloud => {
    // Gemini 呼び出し
}

// 変更後（cloud stage = OpenAI → Gemini の順）:
LlmStage::Cloud => {
    if openai_llm_enabled {
        // OpenAI Chat Completions API (gpt-4o-mini) でメッセージ列を送信
        // 失敗時は次の provider にフォールバック
    }
    // OpenAI 未設定 or 失敗時: 既存 Gemini 呼び出し
}
```

### 5.6 TTS 変更概要（`mod.rs` L116/L901 付近）

```
// 変更前（TtsStage = Local / Raspi のみ）:
enum TtsStage { Local, Raspi }

// 変更後（TtsStage に Cloud 追加）:
enum TtsStage { Cloud, Local, Raspi }

TtsStage::Cloud => {
    // OpenAI TTS API (gpt-4o-mini-tts) で音声生成
    // WAV 形式で受け取り、実レスポンスのフォーマット（サンプルレート・ビット深度・チャンネル）を検証
    // フォーマット不一致の場合は変換処理を挟む or Local にフォールバック（§8 Q2 参照）
    // API 失敗時: Local にフォールバック
}
```

> **Note:** 現状の再生パイプラインは mono 16bit WAV かつ 24kHz 対応可能（`storage.rs:26`/`40` 参照）。OpenAI TTS の実際の WAV 出力フォーマットは実装時に検証する（§8 Q2）。

### 5.7 weather 要約 LLM 変更概要（`src/service/ai/weather.rs:67`、`src/service/ai/mod.rs:646` 付近）

```
// 変更前: call_ollama_for_weather() → LLM_LOCAL_* のみ
// 変更後: OpenAI LLM を優先試行 → 失敗時は既存 local フォールバック

async fn summarize_weather(...) {
    if openai_llm_enabled {
        // OpenAI Chat Completions API (gpt-4o-mini) で要約
        // 失敗時は次へ
    }
    // 既存 call_ollama_for_weather() (LLM_LOCAL_* 経由)
    // 全失敗時: fallback_summary() (weather.rs:84)
}
```

> **Note:** weather 要約 LLM は通常会話 LLM とは独立した呼び出し経路のため、個別に OpenAI 優先を追加する。

### 5.8 stage count / gate 更新（`src/service/ai/mod.rs` L156–L212 付近）

現行コードには各 AI 機能の有効ステージ数を集計し、0 の場合に起動時 warning や早期失敗を返す前段ゲートが存在する（`mod.rs:156`/`162`/`170`/`176`/`184`/`198`/`212` 付近）。OpenAI ステージを追加する際は以下も合わせて更新する。

| 変数/判定 | 変更内容 |
|---------|---------|
| `asr_stage_count` | OpenAI ASR が有効な場合に +1 |
| `llm_cloud_enabled` / `llm_stage_count` | OpenAI LLM が有効な場合に cloud enabled = true / +1 |
| `tts_stage_count` | OpenAI TTS（新規 `TtsStage::Cloud`）が有効な場合に +1 |
| 起動時 warning | OpenAI のみ有効・他ステージ無効でも `"all stages disabled"` にならないよう条件更新 |
| all-stages-disabled 判定 | OpenAI-only 構成が有効構成として認識されるよう判定式を更新 |

> **Note:** 本更新を省略すると、OpenAI のみ有効で AWS/Gemini/local/raspi を無効化した構成で誤って「全ステージ無効」と判定され、OpenAI 呼び出し前にスキップ・失敗する実装漏れが発生する。

### 5.9 `.env.example` 追記サンプル

```dotenv
# OpenAI（PoC: ASR/LLM/TTS cloud 最優先）
# OPENAI_API_KEY=sk-...
# OPENAI_BASE_URL=https://api.openai.com/v1
# OPENAI_ASR_ENABLED=true
# OPENAI_LLM_ENABLED=true
# OPENAI_TTS_ENABLED=true
# TTS_CLOUD_TIMEOUT_MS=10000
```

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #231 | STEER-231 | 起票 |
| STEER-231 | `src/service/ai/mod.rs` L421/L507/L116/L901/L156-L212 | ASR/LLM/TTS cloud provider 追加・stage count/gate 更新 |
| STEER-231 | `src/service/ai/weather.rs:67` / `src/service/ai/mod.rs:646` | weather 要約 LLM OpenAI 化 |
| STEER-231 | `src/shared/config/mod.rs:877`/`918`/`1021` / `.env.example` | 環境変数追加 |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] フォールバック順序（OpenAI → 既存 cloud → local → raspi）が各機能で一貫しているか
- [ ] `OPENAI_API_KEY` 未設定時に従来挙動が完全に維持されるか（後方互換）
- [ ] TTS の cloud stage 新設が既存 `TtsStage` enum と整合しているか
- [ ] weather 要約 LLM が独立経路であることを踏まえた個別対応となっているか
- [ ] `TTS_CLOUD_TIMEOUT_MS` 等の新規設定にデフォルト値が明記されているか
- [ ] `asr_stage_count` / `llm_stage_count` / `tts_stage_count` / `llm_cloud_enabled` / 起動時 warning / all-stages-disabled 判定が OpenAI を含む形に更新されているか（§5.8・`src/service/ai/mod.rs` L156–L212 付近）
- [x] §8 Q1（LLM モデル名）・Q2（TTS 音声フォーマット）が回答済みか ← **Q1: `gpt-5-mini` 確定、Q2: 24kHz/16bit/mono・PoC 実測確認方針で合意**

### 7.2 マージ前チェック（Approved → Merged）

- [ ] `OPENAI_API_KEY` あり環境で ASR / LLM / TTS が OpenAI 優先で動作することを確認している
- [ ] `OPENAI_API_KEY` なし環境で従来と同じ挙動であることを確認している
- [ ] OpenAI API 障害時に AWS / Gemini / local / raspi に正常フォールバックすることを確認している
- [ ] weather 要約が OpenAI 優先で動作し、失敗時に `fallback_summary()` まで正常に落ちることを確認している
- [ ] 既存のテストがすべて PASS している
- [ ] **自動テスト（追加/更新）**: OpenAI ASR 失敗時に AWS Transcribe へフォールバックすることを検証するテストが存在する（mock OpenAI error → AWS 呼び出し確認）
- [ ] **自動テスト（追加/更新）**: OpenAI LLM 失敗時に Gemini へフォールバックすることを検証するテストが存在する（mock OpenAI error → Gemini 呼び出し確認）
- [ ] **自動テスト（追加/更新）**: weather 要約が OpenAI → `call_ollama_for_weather()` → `fallback_summary()` の順で落ちることを検証するテストが存在する
- [ ] **自動テスト（追加/更新）**: OpenAI TTS 失敗時に Local TTS へフォールバックすることを検証するテストが存在する（mock OpenAI error → local TTS 呼び出し確認）

---

## 8. 未確定点・質問

| # | 質問 | 選択肢 | 推奨 | オーナー回答 |
|---|------|--------|------|-------------|
| Q1 | PoC での LLM モデル名: Codex 調査では `gpt-5-mini` と記載されているが、2026-02-24 時点の OpenAI API で利用可能なモデルか確認が必要 | `gpt-4o-mini`（現行軽量 chat モデル） / `gpt-5-mini`（存在確認要） | **`gpt-4o-mini` を推奨**（既知の軽量モデル）。`gpt-5-mini` が利用可能と確認できれば差し替え可 | **ChatGPT 調査結果（2026-02-24）: `gpt-5-mini` は OpenAI 公式 Models 一覧および個別ページで存在確認済み**。PoC では `gpt-5-mini` を使用する |
| Q2 | OpenAI TTS（gpt-4o-mini-tts）の WAV 出力フォーマット（サンプルレート・ビット深度・チャンネル）が再生パイプライン（`storage.rs:26`/`40`）と互換か | 互換 / 変換処理が必要 | `storage.rs` の現状実装は 24kHz 対応可能と Codex 調査で確認済み。実装時に実際の API レスポンスで検証を推奨 | **ChatGPT 調査結果（2026-02-24）: PCM 出力は仕様上 24kHz / 16bit（little-endian）。Realtime API の `pcm16` 要件も 24kHz / 16bit / mono**。再生パイプラインとのフォーマット互換性は実装環境依存のため、PoC で実測確認が妥当 |

---

## 9. リスク・ロールバック観点

| リスク | 影響 | 緩和策 |
|--------|------|--------|
| `OPENAI_API_KEY` 設定ミスによる全 AI 機能の cloud 段階スキップ | ASR/LLM/TTS が local/raspi に落ちて動作継続するが cloud 品質が得られない | `OPENAI_API_KEY` 未設定ログを起動時に出力する |
| TTS 音声フォーマット不一致（サンプルレート/チャンネル数） | 再生音声が破損する可能性 | §8 Q2 で実装時に検証。不一致の場合は `TtsStage::Cloud` を一時無効化してロールバック可 |
| weather 要約 LLM の cloud 化によるコスト増 | 天気問い合わせごとに OpenAI 呼び出しが発生 | `OPENAI_LLM_ENABLED=false` で LLM 系（通常会話・weather 要約）の OpenAI を一括無効化可能。weather 要約のみの個別無効化は本 Issue スコープ外（別 Issue 対応） |
| OpenAI API 障害時の応答遅延 | cloud タイムアウト分だけ応答が遅れる | `TTS_CLOUD_TIMEOUT_MS`（新規）/ 既存 `ASR_CLOUD_TIMEOUT_MS` / `LLM_CLOUD_TIMEOUT_MS`（`src/shared/config/mod.rs:896`/`904`/`1044`/`1056` 付近）で制御 |

**ロールバック手順:** `OPENAI_API_KEY` 環境変数を削除または `OPENAI_*_ENABLED=false` を設定することで即時フォールバック可能。コード変更自体を revert する場合は対象 PR を revert する。

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-24 | 初版作成（Codex 調査結果を元に差分仕様を記述） | Claude Sonnet 4.6 |
| 2026-02-24 | §3.3 Round 1 NG 記録、①ファイルパス修正（weather/config）②§5.8 stage count/gate 追記③§7.1 チェック追加④§9 タイムアウト変数名修正 | Claude Sonnet 4.6 |
| 2026-02-24 | §3.3 Round 2 NG 記録、①§7.2 自動テスト項目追加②§9 weather 緩和策文言修正（LLM一括無効化に統一）③§5.6 TTS WAVフォーマット断定を弱める | Claude Sonnet 4.6 |
| 2026-02-24 | §3.3 Round 3 NG 記録、§7.2 TTS cloud→local フォールバック自動テスト項目追加 | Claude Sonnet 4.6 |
| 2026-02-24 | §3.3 Round 4 OK 記録、§8 Q1/Q2 オーナー回答記録、§5.3 LLM モデル名 `gpt-5-mini` に確定、§7.1 Q1/Q2 チェック済みに更新 | Claude Sonnet 4.6 |
| 2026-02-24 | §1 ステータス Draft → Approved、§3.4 承認者記録 | @MasanoriSuda |
