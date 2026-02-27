# STEER-222: intent 分類ローカルサーバー優先 2 段フォールバック

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-222 |
| タイトル | intent 分類ローカルサーバー優先 2 段フォールバック |
| ステータス | Approved |
| 関連Issue | #222 |
| 優先度 | P0 |
| 作成日 | 2026-02-22 |

---

## 2. ストーリー（Why）

### 2.1 背景

Raspberry Pi 実機での動作確認で、intent 分類（ボイスボットの発話意図分類）の速度が実用水準に達しないことが判明した。
現行実装は `classify_intent`（`src/service/ai/intent.rs L75`）で `call_ollama_with_prompt` を
`localhost:11434/api/chat` 固定・`AI_HTTP_TIMEOUT_MS` 共通タイムアウトで呼ぶ単一経路であり、以下の問題がある。

| 問題 | 詳細 |
|------|------|
| 接続先 URL が固定 | 別ホストの高性能 Ollama サーバーへ切り替え不可（`call_ollama_with_prompt` 内の hardcode） |
| タイムアウトが共通値 | intent 分類は短文 JSON 応答のみのため、LLM 会話より短いタイムアウトが適切だが設定不可 |
| フォールバックがない | ローカルサーバーへの接続失敗時に Pi で再試行する手段がない |
| `OLLAMA_URL` が未使用 | `docker-compose.dev.yml:14` に定義済みだが backend は参照していない |

### 2.2 目的

intent 分類を 2 段フォールバック構成（ローカルサーバー → Pi）に変更し、
分類速度・可用性を向上させる。接続先・タイムアウト・モデルは設定で制御可能にする。

### 2.3 ユーザーストーリー

```text
As a システム管理者
I want to intent 分類バックエンドの優先順序と接続先を設定で変更したい
So that 環境（高性能サーバーあり/なし/Pi 単体）に応じて最適な分類構成を選択できる

受入条件:
- [ ] ローカルサーバー（別ホスト Ollama）が利用可能な場合は優先して使用する
- [ ] ローカルサーバー失敗時は Pi 上の Ollama へ自動フォールバックする
- [ ] 各段の接続先 URL・タイムアウト・モデルは環境変数で設定可能である
- [ ] どの段で成功/失敗したかがログに記録される
- [ ] Pi フォールバックは環境変数で無効化でき、無効時は失敗を即座に返す
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-22 |
| 起票理由 | Raspberry Pi 実機での intent 分類速度問題の解消 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Sonnet 4.6 |
| 作成日 | 2026-02-22 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "ローカルサーバー→ラズパイの 2 段フォールバック。Codex 調査結果を元にステアリング作成" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | Codex | 2026-02-22 | NG | §5.4 成功ログの `intent` フィールドが Q4（JSON不正=成功扱い）と矛盾。PII方針が出力JSON（query に生発話）をカバーしていない |
| 2 | Codex | 2026-02-22 | OK | 成功ログを `raw_len` 必須（`intent` はJSON parse成功時のみ任意）に修正済み。PII方針を入力/出力JSON本文ともに非出力に拡張済み。非ブロッカー：OLLAMA_URL 未使用整理は別Issueで対応可 |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-22 |
| 承認コメント | Codex レビュー OK 確認、全指摘解消を確認して承認 |

### 3.5 実装

| 項目 | 値 |
|------|-----|
| 実装者 | Codex |
| 実装日 | 2026-02-22 |
| 実装内容 | intent 分類の 2 段フォールバック（local server → raspi）を実装。`IntentPort` に `call_id` を追加し、段別ログ（start/success/failure/all failed）を実装。`AiConfig` に `INTENT_*` 設定（URL / enabled / model / timeout）を追加。intent 専用 Ollama helper を追加し、raw body を INFO 出力しない（PII 対策）。 |
| 検証結果 | `cargo fmt` / `cargo test -q` / `cargo clippy -q` PASS（129 tests passed） |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | 未定 |
| マージ日 | - |
| マージ先 | DD-006_ai.md §intent, RD-001_product.md |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| `docs/requirements/RD-001_product.md` | 修正 | intent 分類 2 段フォールバック要件を追加 |
| `docs/design/detail/DD-006_ai.md` | 修正 | intent フォールバック仕様・設定項目を追加 |

### 4.2 影響するコード

| ファイル | 変更種別 | 概要 |
|---------|---------|------|
| `src/service/ai/intent.rs` L75 `classify_intent` | 修正 | intent 分類ロジックを 2 段フォールバックに拡張（local server → raspi） |
| `src/service/ai/mod.rs`（新規）`call_ollama_for_intent_stage` | 追加 | intent 専用 helper。フルエンドポイント URL・モデル・タイムアウトを引数で受け取る（`call_ollama_with_prompt` は変更しない） |
| `src/shared/config/mod.rs` L858 `AiConfig` | 修正 | INTENT_* 段別 URL / enabled / timeout / model フィールドを追加 |
| `src/service/ai/mod.rs` L636 `DefaultAiPort::new()` | 修正 | 起動時警告（全段無効 / raspi URL 未設定）を追加 |
| `src/shared/ports/ai/intent.rs` L5 `IntentPort::classify_intent` | 修正 | シグネチャに `call_id: String` を追加（ASR #216・LLM #217・TTS #218 と統一） |
| `src/service/ai/mod.rs` L684 `DefaultAiPort::classify_intent` | 修正 | Port 実装を新シグネチャに合わせ `call_id` を `intent::classify_intent` へ伝播 |
| `src/service/call_control/mod.rs` L456 | 修正 | `classify_intent(call_id, text)` 呼び出し側を修正 |

> **注意:** `call_ollama_with_prompt`（`weather.rs` が使用）は変更しない。
> weather は今回のスコープ外（§5.1 参照）。

---

## 5. 差分仕様（What / How）

### 5.1 フォールバック順序（確定）

```text
段 1: ローカルサーバー intent 分類（INTENT_LOCAL_SERVER_URL で指定、デフォルト http://localhost:11434/api/chat）
  ↓ エラー or タイムアウト
段 2: Pi intent 分類（INTENT_RASPI_URL で指定、INTENT_RASPI_ENABLED=true のときのみ）
  ↓ 全段失敗
call_control 側で general_chat にフォールバック（現行動作を維持）
```

**実行方式（確定）:**
- 段 1・段 2 ともに Ollama `/api/chat` エンドポイントを HTTP 経由で呼ぶ。
- URL 形式は **フルエンドポイント URL**（例: `http://host:11434/api/chat`）で指定（LLM #217 と同形式）。
- Pi 段はリモートホスト上の Ollama HTTP サーバーとして実装（最小差分）。

**スコープ（確定）:**
- 対象: intent 分類のみ（`intent.rs L75` `classify_intent`）。
- 対象外: `weather.rs` の `call_ollama_with_prompt` 呼び出しは今回変更しない。
- `call_ollama_with_prompt` は後方互換維持。新規 `call_ollama_for_intent_stage` helper を並立追加。

### 5.2 設定項目（新規追加）

以下を環境変数および `AiConfig` struct に追加する。

| 環境変数 | 型 | 説明 | デフォルト |
|---------|-----|------|-----------|
| `INTENT_LOCAL_SERVER_URL` | String | 段 1 ローカルサーバーのフルエンドポイント URL | `http://localhost:11434/api/chat`（現行 hardcode 相当、互換維持） |
| `INTENT_LOCAL_SERVER_ENABLED` | bool | ローカルサーバー intent 分類を使用するか | `true` |
| `INTENT_LOCAL_MODEL` | String | 段 1 で使用する Ollama モデル | `OLLAMA_INTENT_MODEL` の値を引き継ぐ |
| `INTENT_RASPI_URL` | String | 段 2 Pi の URL | デフォルトなし（`INTENT_RASPI_ENABLED=true` 時は設定必須） |
| `INTENT_RASPI_ENABLED` | bool | Pi intent 分類を使用するか | `false` |
| `INTENT_RASPI_MODEL` | String | 段 2 で使用する Ollama モデル | `llama3.2:1b`（Pi 向け軽量モデル。LLM #217 の `LLM_RASPI_MODEL` と同じ推奨値） |
| `INTENT_LOCAL_TIMEOUT_MS` | u64 | 段 1 タイムアウト（ms） | `3000`（intent は短文 JSON のため LLM より短め） |
| `INTENT_RASPI_TIMEOUT_MS` | u64 | 段 2 タイムアウト（ms） | `5000` |

> **注意:**
> - `INTENT_LOCAL_SERVER_URL` のデフォルト `http://localhost:11434/api/chat` は現行 hardcode 相当であり、**互換維持**（既存環境は URL 未設定でも動作継続）。
> - **URL は該当段の `*_ENABLED=true` のときのみ使用する。** `*_ENABLED=false` の段は URL 未設定でも起動エラーにならない。
> - **「全段無効」の判定:** `INTENT_LOCAL_SERVER_ENABLED=false` かつ `INTENT_RASPI_ENABLED=false` の場合を「有効段ゼロ」とする。この場合は起動時に警告ログを出し、intent 分類は call_control 側の既存フォールバック（general_chat）に直行する。
> - `INTENT_*` は `LLM_*` と独立した変数群とする（Q2 確定）。intent 分類はタイムアウトを短く設定できることが重要。
> - 現行の intent 分類は `AI_HTTP_TIMEOUT_MS`（`config::timeouts().ai_http`）を使用しているが、本変更後は `INTENT_*_TIMEOUT_MS` に移行する。`AI_HTTP_TIMEOUT_MS` は intent には適用しない。
> - `OLLAMA_URL`（compose L14）は引き続き未使用のまま。今回は `INTENT_LOCAL_SERVER_URL` で代替。

### 5.3 フォールバックロジック（intent.rs L75 の置き換え）

現行の `classify_intent`（単一経路固定）を、次の順序で試行するループに置き換える。

```text
// 【intent 専用 helper（新規）】
// call_ollama_for_intent_stage(endpoint_url, model, messages, prompt, timeout) -> Result<String>
//   Ollama /api/chat を呼び、JSON 文字列を返す（call_ollama_with_prompt は変更しない）
//   raw body はログ未出力（PII 対策：出力 JSON の query フィールドにユーザー発話が含まれる）

// 【フォールバック制御（既存関数を修正）】
// classify_intent(call_id: &str, text: String) -> Result<String>
//   ← 現行: classify_intent(text: String) -> Result<String>
for 各有効な intent 段（local, raspi の順）:
    call_ollama_for_intent_stage(endpoint_url, model, messages, prompt, timeout) を呼び出す
    成功（JSON 文字列を取得）:
        ログ出力「intent 成功: 段={local|raspi}, call_id=..., raw_len=...」（intent は JSON parse 成功時のみ任意追記）
        return Ok(json_string)
    失敗（エラー or タイムアウト）:
        ログ出力「intent 失敗: 段=..., 理由=..., call_id=..., 次段へ」
        次段へ続行
全段失敗:
    return Err(anyhow::anyhow!("all intent stages failed"))
    // 呼び出し元（call_control L456）で general_chat にフォールバック（現行動作を維持）
```

**フォールバック条件（確定）:**
- HTTP エラー / タイムアウト → 次段へ
- JSON 不正 / 空文字 / `<no response>` → **成功扱いのまま（シンプル実装優先、今回変更なし）**
  - JSON 不正の場合は call_control 側 router で general_chat に転落する現行動作を維持。

### 5.4 ログ要件（確定）

以下を構造化ログとして必ず記録する。

| イベント | ログレベル | 必須フィールド |
|---------|-----------|--------------|
| 各段で intent 試行開始 | DEBUG | `call_id`, `intent_stage`（local/raspi） |
| 各段で intent 成功 | INFO | `call_id`, `intent_stage`, `raw_len`（`intent` は JSON parse 成功時のみ任意追加） |
| 各段で intent 失敗 | WARN | `call_id`, `intent_stage`, `reason` |
| 全段失敗 | ERROR | `call_id`, `reason="all intent stages failed"` |

> **PII 方針:** intent 分類の入力テキスト本文・出力 JSON 本文ともにログに出力しない（出力 JSON の `query` フィールドにユーザー発話が含まれるため）。`text_len` / `raw_len` のみ出力可。`call_ollama_for_intent_stage` helper は既存の `Ollama raw body` INFO ログ（`call_ollama_with_prompt_internal` L608/L609）を使わず、raw body を未出力とする。

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #222 | STEER-222 | 起票 |
| STEER-222 | DD-006_ai.md §intent | 設計反映 |
| DD-006_ai.md §intent | intent.rs L75 / config/mod.rs L858 | 実装 |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] 2 段フォールバック順序が要件と一致しているか
- [ ] `INTENT_*` を `LLM_*` と独立した変数群とする方針に合意しているか
- [ ] `INTENT_LOCAL_SERVER_URL` の互換維持（デフォルト `http://localhost:11434/api/chat`）に合意しているか
- [ ] タイムアウト値（3000/5000 ms）が妥当か
- [ ] `call_ollama_with_prompt` 変更なし・新規 helper 追加方針に合意しているか
- [ ] weather が今回のスコープ外であることに合意しているか
- [ ] Q1〜Q5 の未確定点が全件解消しているか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] 実装が完了している
- [ ] コードレビューを受けている（CodeRabbit）
- [ ] 全段フォールバックの結合テストが PASS
- [ ] 段ごとの成功/失敗ログが出力されることを確認

---

## 8. 未確定点・質問

| # | 質問 | 選択肢 | 推奨 | オーナー回答 |
|---|------|--------|------|-------------|
| Q1 | `call_id` を段別ログに含めるか？（`IntentPort::classify_intent` のシグネチャ変更が必要。ASR #216・LLM #217・TTS #218 と同様） | Yes / No | **Yes（ASR/LLM/TTS と統一）** | **Yes（Port trait・内部関数・呼び出し側を修正）** @MasanoriSuda 2026-02-22 |
| Q2 | 設定変数を `INTENT_*` 新設にするか、`LLM_LOCAL_SERVER_URL` 等と共用するか | INTENT_* 新設 / LLM_* 共用 | **INTENT_* 新設（intent 専用タイムアウト設定が可能で柔軟性が高い）** | **INTENT_* 新設** @MasanoriSuda 2026-02-22 |
| Q3 | モデル設定を段別（`INTENT_LOCAL_MODEL` / `INTENT_RASPI_MODEL`）に分けるか | 段別 / OLLAMA_INTENT_MODEL 共通 | **段別（Pi 速度対策として `INTENT_RASPI_MODEL` に軽量モデルを指定できる）** | **段別（INTENT_LOCAL_MODEL / INTENT_RASPI_MODEL、デフォルト llama3.2:1b）** @MasanoriSuda 2026-02-22 |
| Q4 | フォールバック条件に JSON 不正・空文字・`<no response>` も含めて次段へ送るか | Yes（次段へ） / No（成功扱い） | **No（初期はエラー/タイムアウトのみ、JSON不正は call_control の general_chat フォールバックで対応）** | **No（成功扱いのまま）** @MasanoriSuda 2026-02-22 |
| Q5 | タイムアウト値（段 1: 3000 ms、段 2: 5000 ms）は妥当か | OK / 変更 | - | **OK（変更なし）** @MasanoriSuda 2026-02-22 |

---

## 9. リスク・ロールバック観点

| リスク | 影響 | 緩和策 |
|--------|------|--------|
| 全段フォールバックで遅延が累積 | intent 分類が遅くなり対話全体に影響 | タイムアウトを短め（3000/5000 ms）に設定し遅延を最小化 |
| Pi Ollama サーバーが未起動 | 段 2 がタイムアウトして遅延 | `INTENT_RASPI_ENABLED=false` をデフォルトにして影響ゼロ |
| JSON 不正応答がフォールバックされない | intent 分類ミスが増える | 今回は call_control 側 general_chat フォールバックで対応。次段フォールバックは別 Issue で対応可 |
| `call_ollama_with_prompt` は変更しないため weather には影響なし | - | 設計上の分離により波及なし |

**ロールバック手順:** 実装コミットを `git revert`。新規フラグを無効化する場合は `INTENT_LOCAL_SERVER_ENABLED=false` / `INTENT_RASPI_ENABLED=false` を設定し、旧 `localhost:11434` 固定動作に戻すには `INTENT_LOCAL_SERVER_URL=http://localhost:11434/api/chat`（デフォルト）を確認する。

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-22 | 初版作成（Codex 調査結果を元に差分仕様を記述） | Claude Sonnet 4.6 |
| 2026-02-22 | Q1〜Q5 オーナー回答反映（call_id 追加・INTENT_* 新設・段別モデル llama3.2:1b・JSON不正は成功扱い・タイムアウト値 OK）、§4.2 の Q1 条件分岐を確定変更に修正、§5.2 の INTENT_RASPI_MODEL デフォルト値を追加、§5.3 シグネチャ確定、§5.4 の "(Q1 確定後)" 注釈を削除 | Claude Sonnet 4.6 |
| 2026-02-22 | Codex NG 対応: §5.3 pseudocode の成功ログを `intent=` → `raw_len=`（JSON parse 成功時のみ任意追記）に修正、`call_ollama_for_intent_stage` raw body 未出力コメント追加。§5.4 成功ログ必須フィールドを `intent` → `raw_len` に修正。§5.4 PII 方針を入力/出力JSON本文ともに非出力・`call_ollama_with_prompt_internal` raw body ログ不使用に拡張 | Claude Sonnet 4.6 |
| 2026-02-22 | Codex レビュー OK 確認・オーナー承認によりステータスを Approved に更新。§3.3/§3.4 レビュー/承認記録を追記 | Claude Sonnet 4.6 |
