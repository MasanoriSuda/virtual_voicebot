# STEER-230: ASR/LLM/TTS フォールバック優先度変更（ローカルサーバー優先）

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-230 |
| タイトル | ASR/LLM/TTS フォールバック優先度変更（ローカルサーバー優先） |
| ステータス | Approved |
| 関連Issue | #230 |
| 優先度 | P0 |
| 作成日 | 2026-02-28 |

---

## 2. ストーリー（Why）

### 2.1 背景

STEER-216（ASR）・STEER-217（LLM）・STEER-231（OpenAI クラウドプロバイダー追加）で確立した
3 段フォールバック構成は「クラウド → ローカルサーバー → Raspberry Pi」順に設計・実装された。

しかし運用を通じて以下の課題が判明した。

| 問題 | 詳細 |
|------|------|
| クラウド優先では通常運用時の API コストが増大する | ローカルサーバーが稼働中でも常にクラウドを先に試みる |
| クラウド障害時のみローカルにフォールバックする設計 | 逆に「ローカルが障害時のみクラウドに退避する」運用が望ましい |
| ローカルサーバーが主力であるにもかかわらず第 2 段に位置する | 実環境ではローカルサーバーが常時稼働しており、クラウドは緊急退避用 |

対象サービス（コード上の実際の順序を確認した結果）:
- **ASR**: Local → Cloud → Raspi（`ASR_FALLBACK_ORDER` L380 — **実装済み**）
- **LLM**: Local → Cloud → Raspi（`LLM_FALLBACK_ORDER` L399 — **実装済み**）
- **TTS**: Cloud（OpenAI TTS） → Local → Raspi（L2000/L2100 — **未変更、本 Issue 対象**）

ASR/LLM はすでにローカル優先に変更済み。TTS のみクラウド優先が残っており、本 Issue で対応する。

### 2.2 目的

ASR・LLM・TTS のフォールバック順序を「**ローカルサーバー → クラウド → Raspberry Pi**」に変更し、
通常運用はローカルサーバーで完結させ、クラウドを緊急退避段として位置づける。

### 2.3 ユーザーストーリー

```text
As a システム管理者
I want to ローカルサーバーを ASR/LLM/TTS の第 1 優先にしたい
So that 通常運用はローカルで完結し、クラウド障害に依存しない安定した運用ができる

受入条件:
- [ ] ASR ローカルサーバーが有効な場合、クラウドより先に試みる
- [ ] ASR ローカルサーバー失敗時のみクラウドへフォールバックする
- [ ] ASR クラウド失敗時は Pi へフォールバックする
- [ ] LLM ローカルサーバーが有効な場合、クラウド LLM より先に試みる
- [ ] LLM ローカルサーバー失敗時のみクラウド LLM へフォールバックする
- [ ] LLM クラウド失敗時は Pi Ollama へフォールバックする
- [ ] TTS ローカルサーバー（Zundamon/VOICEVOX）が有効な場合、OpenAI TTS より先に試みる
- [ ] TTS ローカルサーバー失敗時のみ OpenAI TTS へフォールバックする
- [ ] TTS クラウド失敗時は Pi へフォールバックする
- [ ] 環境変数名・デフォルト値は変更しない（既存設定ファイルへの影響ゼロ）
- [ ] どの段で成功/失敗したかがログに記録される（既存ログ仕様を維持）
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-23 |
| 起票理由 | ローカルサーバー優先の運用方針への変更 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Sonnet 4.6 |
| 作成日 | 2026-02-28 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "ASR/LLM/TTSの優先度をローカルサーバー→クラウド→ラズパイの順に変更" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1（再レビュー） | @MasanoriSuda | 2026-02-28 | OK | TTS スコープ追加（§5.3 新設）を含む再査読。指摘なし |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-28 |
| 承認コメント | lgtm（TTS スコープ追加含む再承認） |

### 3.5 実装（該当する場合）

| 項目 | 値 |
|------|-----|
| 実装者 | Codex |
| 実装日 | - |
| 指示者 | - |
| 指示内容 | - |
| コードレビュー | - |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | - |
| マージ日 | - |
| マージ先 | STEER-216 §5.1/§5.2/§5.3, STEER-217 §5.1/§5.2/§5.3, STEER-231 §5（TTS 段順序）, DD-006_ai.md §2・§3・§4 |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| `virtual-voicebot-backend/docs/steering/STEER-216_asr-cloud-fallback.md` | 修正 | §5.1/§5.2/§5.3 のクラウド優先記述をローカル優先に書き換え（変更履歴に `STEER-230 で順序変更` を追記） |
| `virtual-voicebot-backend/docs/steering/STEER-217_llm-cloud-fallback.md` | 修正 | §5.1/§5.2/§5.3 のクラウド優先記述をローカル優先に書き換え（変更履歴に `STEER-230 で順序変更` を追記） |
| `virtual-voicebot-backend/docs/steering/STEER-231_openai-cloud-provider.md` | 修正 | §5 TTS 段順序のクラウド優先記述をローカル優先に書き換え（変更履歴に `STEER-230 で順序変更` を追記） |
| `virtual-voicebot-backend/docs/design/detail/DD-006_ai.md` | 修正 | §2 ASR・§3 LLM・§4 TTS のフォールバック順序記述を更新 |

> **変更不要なドキュメント:**
> - `STEER-218_tts-local-fallback.md`: TTS 2 段（Local → Raspi）を定義した原典であり、今回の変更は段順序のみのため本体変更不要（参考情報として残す）
> - `STEER-222_intent-local-fallback.md`: すでにローカル優先のため対象外
> - `STEER-224_weather-llm-local.md`: ローカルサーバー単段のため対象外
> - `STEER-257_startup-ai-config-log.md`: ログ出力順序の変更は実装判断に委ねる（Q2 参照）

### 4.2 影響するコード

| ファイル | 変更種別 | 概要 |
|---------|---------|------|
| `src/service/ai/mod.rs` L1225 付近（`transcribe_and_log`） | 修正 | ASR フォールバックループ順序を `[local, cloud, raspi]` に変更 |
| `src/service/ai/mod.rs` L1353 付近（`generate_answer` 相当） | 修正 | LLM フォールバックループ順序を `[local, cloud, raspi]` に変更 |
| `src/service/ai/mod.rs` L2000 付近（TTS ストリーミングパス） | 修正 | TTS ストリーミングのフォールバック順序を `[local, cloud, raspi]` に変更 |
| `src/service/ai/mod.rs` L2100 付近（TTS 非ストリーミングパス） | 修正 | TTS 非ストリーミングのフォールバック順序を `[local, cloud, raspi]` に変更 |

> **変更不要なファイル:**
> - `src/shared/config/mod.rs`: 環境変数定義・デフォルト値はそのまま
> - Port trait シグネチャ: 変更なし

---

## 5. 差分仕様（What / How）

### 5.1 ASR フォールバック順序（変更後）

**変更前（STEER-216 §5.1）:**

```text
段 1: クラウド ASR（AWS Transcribe、USE_AWS_TRANSCRIBE=true のとき）
  ↓ エラー or タイムアウト
段 2: ローカルサーバー ASR（ASR_LOCAL_SERVER_ENABLED=true のとき）
  ↓ エラー or タイムアウト
段 3: Pi ASR（ASR_RASPI_ENABLED=true のとき）
  ↓ 全段失敗
謝罪音声フォールバック
```

**変更後:**

```text
段 1: ローカルサーバー ASR（ASR_LOCAL_SERVER_ENABLED=true のとき）
  ↓ エラー or タイムアウト
段 2: クラウド ASR（AWS Transcribe、USE_AWS_TRANSCRIBE=true のとき）
  ↓ エラー or タイムアウト
段 3: Pi ASR（ASR_RASPI_ENABLED=true のとき）
  ↓ 全段失敗
謝罪音声フォールバック（現行動作を維持）
```

**フォールバックロジック（変更箇所のみ）:**

```text
// transcribe_and_log(call_id: &str, wav_path: &str) -> anyhow::Result<String>
// 変更前: [cloud, local, raspi] の順
// 変更後: [local, cloud, raspi] の順
for 各有効な ASR 段（local, cloud, raspi の順）:
    （ループ本体は STEER-216 §5.3 から変更なし）
    当該段のタイムアウト付きで HTTP/SDK を呼び出す
    成功（テキスト返却）かつ幻聴フィルタ通過 → return Ok(text)
    失敗 → 次段へ続行
全段失敗 → return Err(anyhow::anyhow!("all ASR stages failed"))
```

**既存設定との関係（変更なし）:**

| 環境変数 | デフォルト | 変更後の役割 |
|---------|-----------|------------|
| `ASR_LOCAL_SERVER_ENABLED` | `true` | 段 1（第 1 優先） |
| `ASR_LOCAL_SERVER_URL` | `http://localhost:9000/transcribe` | 段 1 URL |
| `ASR_LOCAL_TIMEOUT_MS` | `3000` | 段 1 タイムアウト |
| `USE_AWS_TRANSCRIBE` | `false` | 段 2（クラウド退避） |
| `ASR_CLOUD_TIMEOUT_MS` | `5000` | 段 2 タイムアウト |
| `ASR_RASPI_ENABLED` | `false` | 段 3 |
| `ASR_RASPI_URL` | （設定必須） | 段 3 URL |
| `ASR_RASPI_TIMEOUT_MS` | `8000` | 段 3 タイムアウト |

> **注意:** 環境変数名・デフォルト値・タイムアウト値はすべて STEER-216 から変更なし。
> 既存の `.env` ファイル・docker-compose を再設定する必要はない。

---

### 5.2 LLM フォールバック順序（変更後）

**変更前（STEER-217 §5.1）:**

```text
段 1: クラウド LLM（Gemini、GEMINI_API_KEY が設定されている場合）
  ↓ エラー or タイムアウト
段 2: ローカルサーバー LLM（LLM_LOCAL_SERVER_ENABLED=true のとき）
  ↓ エラー or タイムアウト
段 3: Pi LLM（LLM_RASPI_ENABLED=true のとき）
  ↓ 全段失敗
謝罪文フォールバック
```

**変更後:**

```text
段 1: ローカルサーバー LLM（LLM_LOCAL_SERVER_ENABLED=true のとき）
  ↓ エラー or タイムアウト
段 2: クラウド LLM（Gemini、GEMINI_API_KEY が設定されている場合）
  ↓ エラー or タイムアウト
段 3: Pi LLM（LLM_RASPI_ENABLED=true のとき）
  ↓ 全段失敗
謝罪文フォールバック（現行動作を維持）
```

**フォールバックロジック（変更箇所のみ）:**

```text
// generate_answer(call_id: &str, messages: Vec<ChatMessage>) -> anyhow::Result<String>
// 変更前: [cloud, local, raspi] の順
// 変更後: [local, cloud, raspi] の順
for 各有効な LLM 段（local, cloud, raspi の順）:
    （ループ本体は STEER-217 §5.3 から変更なし）
    当該段のタイムアウト付きで HTTP/SDK を呼び出す
    成功（テキスト返却） → return Ok(text)
    失敗（HTTP エラー / タイムアウト） → 次段へ続行
全段失敗 → return Err(anyhow::anyhow!("all LLM stages failed"))
```

> **`<no response>` / 空文字の扱い:** STEER-217 §5.3 の確定事項どおり成功扱いを維持する（変更なし）。

**既存設定との関係（変更なし）:**

| 環境変数 | デフォルト | 変更後の役割 |
|---------|-----------|------------|
| `LLM_LOCAL_SERVER_ENABLED` | `true` | 段 1（第 1 優先） |
| `LLM_LOCAL_SERVER_URL` | `http://localhost:11434/api/chat` | 段 1 URL |
| `LLM_LOCAL_MODEL` | `OLLAMA_MODEL` の値 | 段 1 モデル |
| `LLM_LOCAL_TIMEOUT_MS` | `8000` | 段 1 タイムアウト |
| `GEMINI_API_KEY` | （未設定） | 段 2（クラウド退避） |
| `LLM_CLOUD_TIMEOUT_MS` | `10000` | 段 2 タイムアウト |
| `LLM_RASPI_ENABLED` | `false` | 段 3 |
| `LLM_RASPI_URL` | （設定必須） | 段 3 URL |
| `LLM_RASPI_MODEL` | `llama3.2:1b` | 段 3 モデル |
| `LLM_RASPI_TIMEOUT_MS` | `15000` | 段 3 タイムアウト |

> **注意:** 環境変数名・デフォルト値・タイムアウト値はすべて STEER-217 から変更なし。
> 既存の `.env` ファイル・docker-compose を再設定する必要はない。

---

### 5.3 TTS フォールバック順序（変更後）

**変更前（STEER-231 §5 で確立した順序）:**

```text
段 1: クラウド TTS（OpenAI TTS、OPENAI_TTS_ENABLED=true かつ OPENAI_API_KEY 設定済みのとき）
  ↓ エラー or タイムアウト
段 2: ローカルサーバー TTS（Zundamon/VOICEVOX、TTS_LOCAL_SERVER_ENABLED=true のとき）
  ↓ エラー or タイムアウト
段 3: Pi TTS（TTS_RASPI_ENABLED=true のとき）
  ↓ 全段失敗
warn ログ・音声なし（現行動作を維持）
```

**変更後:**

```text
段 1: ローカルサーバー TTS（Zundamon/VOICEVOX、TTS_LOCAL_SERVER_ENABLED=true のとき）
  ↓ エラー or タイムアウト
段 2: クラウド TTS（OpenAI TTS、OPENAI_TTS_ENABLED=true かつ OPENAI_API_KEY 設定済みのとき）
  ↓ エラー or タイムアウト
段 3: Pi TTS（TTS_RASPI_ENABLED=true のとき）
  ↓ 全段失敗
warn ログ・音声なし（現行動作を維持）
```

**フォールバックロジック（変更箇所のみ）:**

```text
// ストリーミングパス（mod.rs L2000 付近）
// 非ストリーミングパス（mod.rs L2100 付近）
// 変更前: [cloud, local, raspi] の順
// 変更後: [local, cloud, raspi] の順
各段の試行ロジック本体は変更なし（STEER-231 §5 から）
```

**既存設定との関係（変更なし）:**

| 環境変数 | デフォルト | 変更後の役割 |
|---------|-----------|------------|
| `TTS_LOCAL_SERVER_ENABLED` | `true` | 段 1（第 1 優先） |
| `TTS_LOCAL_SERVER_BASE_URL` | `http://localhost:50021` | 段 1 URL |
| `TTS_LOCAL_TIMEOUT_MS` | `5000` | 段 1 タイムアウト |
| `OPENAI_TTS_ENABLED` | `false` | 段 2（クラウド退避） |
| `TTS_CLOUD_TIMEOUT_MS` | `10000` | 段 2 タイムアウト |
| `TTS_RASPI_ENABLED` | `false` | 段 3 |
| `TTS_RASPI_BASE_URL` | （設定必須） | 段 3 URL |
| `TTS_RASPI_TIMEOUT_MS` | `10000` | 段 3 タイムアウト |

> **注意:** 環境変数名・デフォルト値・タイムアウト値はすべて STEER-231 から変更なし。
> デフォルト設定では `OPENAI_TTS_ENABLED=false` のためクラウド段はスキップされ、実質的な動作は変わらない。

---

### 5.4 Intent・Weather（変更なし）

| サービス | 現在の優先度 | 本 Issue での変更 |
|---------|------------|-----------------|
| Intent（STEER-222） | ローカル → Pi | 変更なし（クラウド段なし） |
| Weather LLM（STEER-224） | ローカルのみ | 変更なし（クラウド段なし） |

---

### 5.5 デフォルト動作への影響

変更前後でデフォルト設定のまま起動した場合：

| サービス | デフォルトでの変更前後の動作 |
|---------|----------------------|
| ASR（`USE_AWS_TRANSCRIBE=false`） | クラウド段がスキップされる点は同じ。ループ順序の変更のみ |
| LLM（`GEMINI_API_KEY` 未設定, `OPENAI_API_KEY` 未設定） | クラウド段がスキップされる点は同じ |
| TTS（`OPENAI_TTS_ENABLED=false`） | クラウド段がスキップされる点は同じ |

> **影響:** デフォルト設定ではクラウド段が有効でないため、実質的な動作は変わらない。
> クラウド段が有効な環境（`USE_AWS_TRANSCRIBE=true` / `GEMINI_API_KEY` or `OPENAI_API_KEY` 設定済み / `OPENAI_TTS_ENABLED=true`）のみ、ローカルサーバーが優先されるよう動作が変わる。

---

### 5.6 ログ要件（既存仕様を維持）

STEER-216 §5.4・STEER-217 §5.4 のログ仕様から変更なし。
`asr_stage` / `llm_stage` / `tts_stage` の値として `local / cloud / raspi` をそのまま使用する。

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #230 | STEER-230 | 起票 |
| STEER-230 | STEER-216 §5.1/§5.2/§5.3 | ASR フォールバック順序を修正 |
| STEER-230 | STEER-217 §5.1/§5.2/§5.3 | LLM フォールバック順序を修正 |
| STEER-230 | STEER-231 §5（TTS 段） | TTS フォールバック順序を修正 |
| STEER-230 | DD-006_ai.md §2・§3・§4 | 設計書フォールバック順序を修正 |
| STEER-216 §5.3 | src/service/ai/mod.rs L1225 付近（`transcribe_and_log` ループ順） | 実装対象 |
| STEER-217 §5.3 | src/service/ai/mod.rs L1353 付近（`generate_answer` ループ順） | 実装対象 |
| STEER-230 §5.3 | src/service/ai/mod.rs L2000 付近（TTS ストリーミング ループ順） | 実装対象 |
| STEER-230 §5.3 | src/service/ai/mod.rs L2100 付近（TTS 非ストリーミング ループ順） | 実装対象 |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] ASR のフォールバック順序が `[local, cloud, raspi]` に変更されていることを確認
- [ ] LLM のフォールバック順序が `[local, cloud, raspi]` に変更されていることを確認
- [ ] TTS のフォールバック順序が `[local, cloud, raspi]` に変更されていることを確認（ストリーミング・非ストリーミング両パス）
- [ ] 環境変数名・デフォルト値が変更されていないことを確認
- [ ] Intent・Weather が対象外であることに合意
- [ ] デフォルト設定での動作変化が許容範囲内であることを確認
- [ ] ログ仕様（`asr_stage` / `llm_stage` / `tts_stage` の値）が既存仕様と整合していることを確認
- [ ] STEER-216・STEER-217・STEER-231 への参照が正確か
- [ ] Q1〜Q3 の未確定点が解消されているか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] 実装が完了している
- [ ] コードレビューを受けている（CodeRabbit）
- [ ] ASR・LLM・TTS 全段フォールバックの結合テストが PASS（ローカル優先順での試行を確認）
- [ ] ローカルサーバーが第 1 段として試行されるログが出力されることを確認（ASR/LLM/TTS 各サービス）
- [ ] TTS ストリーミング・非ストリーミング両パスで順序が変更されていることを確認
- [ ] 既存の `.env` / docker-compose を変更せずに動作することを確認

---

## 8. 未確定点・質問

| # | 質問 | 選択肢 | オーナー回答 |
|---|------|--------|-------------|
| Q1 | `ASR_LOCAL_TIMEOUT_MS` のデフォルト 3000 ms（段 2 クラウドの 5000 ms より短い）は段 1 として妥当か？ローカルサーバーが第 1 優先になるためタイムアウト値を見直すか | このまま / 延長する | **このまま（3000 ms 据え置き）。local timeout 起因の cloud 遷移率を運用で監視し、必要に応じ別 Issue で調整する** |
| Q2 | STEER-257（起動時設定ログ）の出力項目順序もローカル優先に揃えるか（例: `asr_local_enabled` → `asr_cloud_enabled` の順で出力） | 揃える / そのまま | **そのまま。STEER-257 の出力項目にクラウド段は含まれておらず、実装ログもローカル中心のため変更不要** |
| Q3 | STEER-216・STEER-217 本体ドキュメントの §5.1/§5.2/§5.3 のクラウド優先記述（ループ順序テキスト・設定テーブルの「段番号」・ロジック擬似コード）をマージ時に書き換えるか（注記追加のみでは §5.2/§5.3 に旧順序が残り文書内不整合になる） | 書き換える / 注記を追加 | **書き換える。経緯保持のため変更履歴に `STEER-230 で順序変更` を 1 行追記する** |

---

## 9. リスク・ロールバック観点

| リスク | 影響 | 緩和策 |
|--------|------|--------|
| ローカルサーバーが停止中に第 1 段タイムアウトが発生 | クラウド到達まで `ASR_LOCAL_TIMEOUT_MS`（3000 ms）の遅延が加わる | ローカル稼働前提の運用とし、停止時は手動で `ASR_LOCAL_SERVER_ENABLED=false` に切り替える |
| ローカル LLM の応答品質がクラウドより低い場合 | 通常運用の応答品質が低下する可能性 | LLM モデルをチューニングしてから順序変更を適用する |
| TTS ローカル停止中に第 1 段タイムアウトが発生 | クラウド到達まで `TTS_LOCAL_TIMEOUT_MS`（5000 ms）の遅延が加わる | 停止時は手動で `TTS_LOCAL_SERVER_ENABLED=false` に切り替える |
| クラウドへのフォールバックが発生した場合のコスト | 従来より少なくなる（通常はローカルで完結するため） | 影響なし（むしろ改善） |

**ロールバック手順:** 実装コミットを `git revert`。緊急回避として各 `*_LOCAL_SERVER_ENABLED=false` を設定すると段 1 をスキップしクラウドが第 1 優先に戻る（変更前動作相当）。

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-28 | 初版作成 | Claude Sonnet 4.6 |
| 2026-02-28 | レビュー指摘対応: §4.1 STEER-257 の Q 参照番号を Q2 に修正・§4.1 STEER-216/217 の修正範囲を §5.1/§5.2/§5.3 に拡張・§4.2 mod.rs 行番号を実際値（L1225/L1353）に更新・§8 Q1〜Q3 にレビュー推奨回答を反映 | Claude Sonnet 4.6 |
| 2026-02-28 | TTS をスコープに追加: 実コード確認により TTS も Cloud(OpenAI) → Local → Raspi のクラウド優先と判明。タイトル・§2・§3.6・§4・§5.3(新規)・§5.4/§5.5/§5.6 繰り下げ・§6・§7・§9 を更新 | Claude Sonnet 4.6 |
| 2026-02-28 | レビュー指摘対応: §2.1 ASR/LLM の実装状態を正確な順序（Local → Cloud → Raspi、実装済み）に修正・§3.3 に TTS スコープ追加による再レビュー注記を追加 | Claude Sonnet 4.6 |
