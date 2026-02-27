# STEER-217: LLM クラウド優先 3 段フォールバック

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-217 |
| タイトル | LLM クラウド優先 3 段フォールバック |
| ステータス | Approved |
| 関連Issue | #217 |
| 優先度 | P0 |
| 作成日 | 2026-02-22 |

---

## 2. ストーリー（Why）

### 2.1 背景

Raspberry Pi 実機での動作確認で、LLM の速度・品質が実用水準に達しないことが判明した。
現行実装は「Gemini（クラウド）→ Ollama（localhost:11434 固定）→ 謝罪文」の 2 段構成であり、
以下の問題がある。

| 問題 | 詳細 |
|------|------|
| クラウド LLM がない場合に品質が落ちる | Pi 上 Ollama のみでは速度・精度ともに不十分 |
| ローカルサーバー URL が固定 | 別マシンの Ollama サーバーに切り替え不可（`mod.rs L356` に `localhost:11434` 固定） |
| 段別モデル設定がない | Pi は軽量モデルを使いたいが、`OLLAMA_MODEL` が全段共通 |
| フォールバック順序が暗黙的 | クラウド失敗時の挙動がコードに埋め込まれており、設定で変更不可 |
| `OLLAMA_URL` が未使用 | `docker-compose.dev.yml:14` に定義されているが、コードでは参照されていない |

### 2.2 目的

LLM を 3 段フォールバック構成（クラウド → ローカルサーバー → Pi）に変更し、
実用的な速度・品質を確保する。フォールバック順序・接続先・タイムアウト・モデルは設定で制御可能にする。

### 2.3 ユーザーストーリー

```text
As a システム管理者
I want to LLM バックエンドの優先順序と接続先を設定で変更したい
So that 環境（クラウドあり/なし/Pi 単体）に応じて最適な LLM 構成を選択できる

受入条件:
- [ ] クラウド LLM（Gemini）が利用可能な場合は最優先で使用する
- [ ] クラウド失敗時はローカルサーバー Ollama へ自動フォールバックする
- [ ] ローカルサーバー失敗時は Pi 上 Ollama へフォールバックする
- [ ] 各段の接続先 URL・モデル・タイムアウトは環境変数で設定可能である
- [ ] どの段で成功/失敗したかがログに記録される
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-22 |
| 起票理由 | Raspberry Pi 実機での LLM 速度・品質問題の解消 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Sonnet 4.6 |
| 作成日 | 2026-02-22 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "クラウド→ローカルサーバー→ラズパイの3段フォールバック。Codex 調査結果を元にステアリング作成" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | Codex | 2026-02-22 | NG | LLM_LOCAL_SERVER_URL フルエンドポイント・call_ollama_with_prompt スコープ・全段無効条件・§3.6/§4.1 不一致 の 4 件指摘 |
| 2 | Codex | 2026-02-22 | NG | AI_HTTP_TIMEOUT_MS 説明・§7.1 旧 URL・§9 OLLAMA_URL 同値表現 の 3 件指摘 |
| 3 | @MasanoriSuda | 2026-02-22 | OK | 全指摘解消確認 |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-22 |
| 承認コメント | Codex 2 ラウンドの指摘を全件解消。Approved。 |

### 3.5 実装

| 項目 | 値 |
|------|-----|
| 実装者 | Codex |
| 実装日 | 2026-02-22 |
| 実装内容 | `AiConfig` に LLM 段別 URL / enabled / model / timeout を追加、会話 LLM を `cloud -> local -> raspi` の 3 段フォールバック化、`LlmPort::generate_answer` に `call_id` を追加、段別ログと起動時警告を追加、`call_ollama_with_prompt` は互換維持し会話 LLM 専用 helper を新設 |
| 検証 | `cargo fmt` / `cargo test` PASS |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | 未定 |
| マージ日 | - |
| マージ先 | DD-006_ai.md §3, RD-001_product.md |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| docs/requirements/RD-001_product.md | 修正 | LLM 3 段フォールバック要件を追加 |
| docs/design/detail/DD-006_ai.md | 修正 | §3 LLM フォールバック仕様・設定項目を追加 |

### 4.2 影響するコード

| ファイル | 変更種別 | 概要 |
|---------|---------|------|
| `src/service/ai/mod.rs` L287 `handle_user_question_from_whisper_llm_only` | 修正 | LLM フォールバックを 3 段に拡張 |
| `src/service/ai/mod.rs` L326 `call_ollama_with_prompt` | **変更なし** | `intent.rs`・`weather.rs` の既存呼び出しへの波及を避けるため改変しない |
| `src/service/ai/mod.rs`（新規）`call_ollama_for_stage` | 追加 | 3 段フォールバック専用 helper。URL とモデルを引数で受け取り、`/api/chat` を呼ぶ（`call_ollama_with_prompt` と並立） |
| `src/shared/config/mod.rs` L858 `AiConfig` | 修正 | LLM 段別 URL / enabled / timeout / model フィールドを追加 |
| `src/shared/ports/ai/llm.rs` L6 `LlmPort::generate_answer` | 修正 | シグネチャに `call_id: String` を追加 |
| `src/service/ai/mod.rs` L449 `DefaultAiPort::generate_answer` | 修正 | Port 実装を新シグネチャに合わせて修正 |
| `src/service/ai/llm.rs` L59 `generate_answer` | 修正 | シグネチャに `call_id: &str` を追加し、段ごとのログを同関数内で出力する |
| `src/service/call_control/mod.rs` L478 | 修正 | `generate_answer(messages)` → `generate_answer(call_id, messages)` 呼び出し側を修正 |

---

## 5. 差分仕様（What / How）

### 5.1 フォールバック順序（確定）

```text
段 1: クラウド LLM（Gemini、GEMINI_API_KEY が設定されている場合）
  ↓ エラー or タイムアウト
段 2: ローカルサーバー LLM（別ホスト上の Ollama、ASR と同様に URL 設定化）
  ↓ エラー or タイムアウト
段 3: Pi LLM（Pi（別ホスト）上で起動した Ollama）
  ↓ 全段失敗
謝罪文フォールバック（現行動作を維持）
```

**実行方式（決定）:**
- 段 2・段 3 とも Ollama `/api/chat` エンドポイントを HTTP 経由で呼び出す。
- 各段で使用するモデルは個別の環境変数（`LLM_LOCAL_MODEL` / `LLM_RASPI_MODEL`）で設定可能。
- Pi の段 3 は別ホストを想定。

**スコープ（確定）:**
- 対象: 会話 LLM のみ（`generate_answer` 相当、`src/service/ai/mod.rs L287`）。
- 対象外: `intent.rs`・`weather.rs` の `call_ollama_with_prompt` 呼び出しは今回変更しない。

### 5.2 設定項目（新規追加）

以下を環境変数および `AiConfig` struct に追加する。

| 環境変数 | 型 | 説明 | デフォルト |
|---------|-----|------|-----------|
| `LLM_LOCAL_SERVER_URL` | String | 段 2 ローカルサーバー **フルエンドポイント** URL | `http://localhost:11434/api/chat`（現行 hardcode と同等、互換維持） |
| `LLM_LOCAL_SERVER_ENABLED` | bool | ローカルサーバー LLM を使用するか | `true` |
| `LLM_LOCAL_MODEL` | String | 段 2 で使用する Ollama モデル | `OLLAMA_MODEL` の値を引き継ぐ |
| `LLM_RASPI_URL` | String | 段 3 Pi LLM URL | デフォルトなし（`LLM_RASPI_ENABLED=true` 時は設定必須） |
| `LLM_RASPI_ENABLED` | bool | Pi LLM を使用するか | `false` |
| `LLM_RASPI_MODEL` | String | 段 3 で使用する Ollama モデル | `llama3.2:1b`（Pi 向け軽量モデル） |
| `LLM_CLOUD_TIMEOUT_MS` | u64 | 段 1 タイムアウト（ms） | `10000` |
| `LLM_LOCAL_TIMEOUT_MS` | u64 | 段 2 タイムアウト（ms） | `8000` |
| `LLM_RASPI_TIMEOUT_MS` | u64 | 段 3 タイムアウト（ms） | `15000` |

> **注意:**
> - `LLM_LOCAL_SERVER_URL` はフルエンドポイント URL（パス込み）で指定する。デフォルト `http://localhost:11434/api/chat` は現行 hardcode 相当であり、**互換維持**（既存環境は URL 未設定でも動作継続）。
> - `LLM_RASPI_URL` も同様にフルエンドポイント URL で指定する（例: `http://<raspi-ip>:11434/api/chat`）。
> - **URL は該当段の `*_ENABLED=true` のときのみ使用する。** `*_ENABLED=false` の段は URL 未設定でも起動エラーにならない。
> - **「全段無効」の判定:** `GEMINI_API_KEY` が未設定（クラウド段が実質無効）かつ `LLM_LOCAL_SERVER_ENABLED=false` かつ `LLM_RASPI_ENABLED=false` の場合を「有効段ゼロ」とする。この場合は起動時に警告ログを出し、LLM 不可として扱う（謝罪文フォールバックへ直行）。
> - `OLLAMA_MODEL` は `intent.rs`・`weather.rs` で引き続き使用する（今回変更なし）。
> - 既存の `AI_HTTP_TIMEOUT_MS` は TTS など LLM・ASR 以外の共通 HTTP 呼び出しで引き続き使用する。ASR は #216 の段別タイムアウト（`ASR_*_TIMEOUT_MS`）を、LLM は本ステアリングの段別タイムアウト（`LLM_*_TIMEOUT_MS`）を使う。
> - `OLLAMA_URL`（docker-compose にあるが未使用）は `LLM_LOCAL_SERVER_URL` に置き換える。

### 5.3 フォールバックロジック（mod.rs L287 周辺の置き換え）

現行の `handle_user_question_from_whisper_llm_only`（Gemini → Ollama 2 段）を、
次の順序で試行するループに置き換える。

```text
// generate_answer(call_id: &str, messages: Vec<ChatMessage>) -> anyhow::Result<String>
for 各有効な LLM 段（cloud, local, raspi の順）:
    当該段のタイムアウト付きで HTTP/SDK を呼び出す
    成功（テキスト返却）:
        ログ出力「LLM 成功: 段={cloud|local|raspi}, call_id=...」
        return Ok(text)
    失敗（エラー or タイムアウト）:
        ログ出力「LLM 失敗: 段=..., 理由=..., call_id=..., 次段へ」
        次段へ続行
全段失敗:
    return Err(anyhow::anyhow!("all LLM stages failed"))
    // 呼び出し元（LlmPort 実装）で LlmError::GenerationFailed にマップする
```

**フォールバック条件（確定）:**
- HTTP エラー / タイムアウト → 次段へ
- `<no response>` / 空文字返却 → **成功扱いのまま（現行動作を維持、今回変更なし）**

### 5.4 ログ要件（確定）

以下を構造化ログとして必ず記録する。

| イベント | ログレベル | 必須フィールド |
|---------|-----------|--------------|
| 各段で LLM 試行開始 | DEBUG | `call_id`, `llm_stage`（cloud/local/raspi） |
| 各段で LLM 成功 | INFO | `call_id`, `llm_stage`, `text_len` |
| 各段で LLM 失敗 | WARN | `call_id`, `llm_stage`, `reason` |
| 全段失敗 | ERROR | `call_id`, `reason="all LLM stages failed"` |

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #217 | STEER-217 | 起票 |
| STEER-217 | DD-006_ai.md §3 | 設計反映 |
| DD-006_ai.md §3 | mod.rs L287, L326, L356 / config/mod.rs L858 | 実装 |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] 3 段フォールバック順序が要件と一致しているか
- [ ] スコープ（会話 LLM のみ）の合意が得られているか
- [ ] 設定変数名・デフォルト値がレビュー済みか
- [ ] タイムアウト値（10000/8000/15000 ms）が妥当か
- [ ] `LLM_LOCAL_SERVER_URL` の互換維持（デフォルト `http://localhost:11434/api/chat`）に合意しているか
- [ ] 段別モデル設定（`LLM_LOCAL_MODEL` / `LLM_RASPI_MODEL`）に合意しているか
- [ ] `<no response>` を成功扱いとする現行動作の維持に合意しているか（Q2 解決済み）
- [ ] DD-006_ai.md §3 の変更範囲で実装者が迷わないか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] 実装が完了している
- [ ] コードレビューを受けている（CodeRabbit）
- [ ] 全段フォールバックの結合テストが PASS
- [ ] 段ごとの成功/失敗ログが出力されることを確認

---

## 8. 未確定点・質問

| # | 質問 | 選択肢 | オーナー回答 |
|---|------|--------|-------------|
| Q1 | `call_id` を段別ログに含めるか？（`LlmPort::generate_answer` のシグネチャ変更が必要） | Yes / No | **Yes（call_id を追加。Port trait・内部関数・呼び出し側を修正）** @MasanoriSuda 2026-02-22 |
| Q2 | `<no response>` / 空文字を失敗扱いにして次段へフォールバックするか？ | Yes / No | **No（現行通り成功扱い）** @MasanoriSuda 2026-02-22 |
| Q3 | タイムアウト値（10000/8000/15000 ms）は妥当か？ | 変更可 | **OK（変更なし）** @MasanoriSuda 2026-02-22 |
| Q4 | `LLM_RASPI_MODEL` のデフォルト `llama3.2:1b` で良いか（DD-006_ai.md §3.LLM で推奨モデルとして定義されている）？ | OK / 変更 | **OK** @MasanoriSuda 2026-02-22 |
| Q5 | スコープを今後 intent / weather にも広げる予定があるか？（ある場合は別 Issue で対応） | Yes / No | **会話 LLM のみ** @MasanoriSuda 2026-02-22 |

---

## 9. リスク・ロールバック観点

| リスク | 影響 | 緩和策 |
|--------|------|--------|
| 全段フォールバックで遅延が累積 | 対話応答が遅くなる | 段ごとにタイムアウトを短めに設定（設定可能） |
| Pi Ollama サーバーが未起動 | 段 3 がタイムアウトして遅延 | `LLM_RASPI_ENABLED=false` をデフォルトにして影響ゼロ |
| `OLLAMA_URL`（compose）が `LLM_LOCAL_SERVER_URL` に置き換わる | compose を参照していた場合に設定変更が必要 | `LLM_LOCAL_SERVER_URL` はフルエンドポイント形式（`/api/chat` パス込み）のため compose 側は更新が必要。ただし `LLM_LOCAL_SERVER_URL` 未設定のデフォルトで単体実行互換は維持される |

**ロールバック手順:** 実装コミットを `git revert`。新規フラグを無効化する場合は `LLM_LOCAL_SERVER_ENABLED=false` / `LLM_RASPI_ENABLED=false` を設定し、クラウドのみ使用するなら `GEMINI_API_KEY` が設定済みであることを確認する。

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-22 | 初版作成（Codex 調査結果を元に差分仕様を記述） | Claude Sonnet 4.6 |
| 2026-02-22 | Q1〜Q5 オーナー回答反映（call_id 追加・タイムアウト値・LLM_RASPI_MODEL・スコープ確定）、§4.2 に Port trait と呼び出し側の変更を追記、§5.3・§5.4 を call_id 対応仕様に更新 | Claude Sonnet 4.6 |
| 2026-02-22 | Codex レビュー NG 指摘対応: LLM_LOCAL_SERVER_URL をフルエンドポイント URL に修正（localhost:11434/api/chat）・call_ollama_with_prompt を変更しない方針に変更し新規 helper 追加を明記・「全段無効」の判定条件に GEMINI_API_KEY 有無を明記・§4.1 に RD-001_product.md を追加 | Claude Sonnet 4.6 |
| 2026-02-22 | Codex レビュー 2 回目 NG 指摘対応: §5.2 注意の AI_HTTP_TIMEOUT_MS 説明を ASR (#216) との整合に合わせ修正・§7.1 チェックリストの URL を `http://localhost:11434/api/chat` に更新・§9 OLLAMA_URL リスク行の「同値」表現をフルエンドポイント形式と compose 側更新要否を明記した記述に修正 | Claude Sonnet 4.6 |
