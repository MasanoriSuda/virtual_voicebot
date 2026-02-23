# STEER-224: 天気要約 LLM をローカルサーバー設定対応に変更

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-224 |
| タイトル | 天気要約 LLM をローカルサーバー設定対応に変更 |
| ステータス | Approved |
| 関連Issue | #224 |
| 優先度 | P1 |
| 作成日 | 2026-02-23 |

---

## 2. ストーリー（Why）

### 2.1 背景

天気 intent 後の応答生成（LLM 要約）が、general_chat の LLM 経路とは独立した実装になっており、
接続先が `localhost:11434` に固定されている。

具体的には：

| 問題 | 詳細 |
|------|------|
| 接続先 URL が固定 | `call_ollama_with_prompt()`（`mod.rs` L646-658）内で `"http://localhost:11434/api/chat"` を hardcode |
| モデルが `OLLAMA_MODEL` 固定 | `weather.rs` L62 で `config::ai_config().ollama_model` を参照。`LLM_LOCAL_MODEL` を無視 |
| タイムアウトが `AI_HTTP_TIMEOUT_MS` 共通値 | `config::timeouts().ai_http`（既定 20 秒）を使用。LLM 専用タイムアウト（`LLM_LOCAL_TIMEOUT_MS`）を反映しない |
| `LLM_LOCAL_SERVER_URL` が weather に未適用 | `AiConfig` に既に `llm_local_server_url` / `llm_local_model` / `llm_local_timeout` が存在するが `weather.rs` は参照しない |

結果として、バックエンドがラズパイ上で動作する場合は「ラズパイ固定」となり、
別ホストに高性能 Ollama サーバーを設定してもその恩恵を天気要約で受けられない。
localhost に Ollama がいなければ約 20 秒待ってタイムアウト WARN となり、`fallback_summary()` が返る
（機能停止ではないが品質低下）。

### 2.2 目的

天気要約の LLM 呼び出しでも `LLM_LOCAL_SERVER_URL` / `LLM_LOCAL_MODEL` / `LLM_LOCAL_TIMEOUT_MS`
を反映し、設定した接続先で要約できるようにする。

### 2.3 ユーザーストーリー

```text
As a システム管理者
I want to 天気要約の LLM 接続先を LLM_LOCAL_SERVER_URL で制御したい
So that general_chat と同じローカルサーバーを天気要約でも使用し、応答品質を確保できる

受入条件:
- [ ] LLM_LOCAL_SERVER_URL を設定したとき、天気要約の LLM 呼び出しが設定先を使う
- [ ] バックエンド実行ホスト上に localhost:11434 が無くても、設定先が生きていれば天気要約が成功する
- [ ] 天気要約失敗時は従来どおり fallback_summary() が返る
- [ ] general_chat / intent 分類の既存挙動に回帰がない
- [ ] 天気要約失敗ログに接続先 endpoint と timeout_ms が含まれる
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-23 |
| 起票理由 | 天気要約 LLM が localhost 固定で、LLM_LOCAL_SERVER_URL 設定が反映されない問題の解消 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Sonnet 4.6 |
| 作成日 | 2026-02-23 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "天気後の LLM 応答が localhost 固定になっている問題を解消。Codex 調査結果を元にステアリング作成" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | Codex | 2026-02-23 | NG | `LLM_LOCAL_SERVER_ENABLED` 未考慮・helper 委譲と raw body INFO 出力の矛盾・テスト観点不足（3点） |
| 2 | Codex | 2026-02-23 | OK | 全指摘解消を確認 |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-23 |
| 承認コメント | Codex レビュー OK 確認、全指摘解消を確認して承認 |

### 3.5 実装

| 項目 | 値 |
|------|-----|
| 実装者 | Codex |
| 実装日 | 2026-02-23 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "STEER-224（承認済み）に基づき、weather 要約 LLM に `LLM_LOCAL_*` 設定反映・enabled チェック・失敗ログ拡張・weather helper 追加を実装" |
| コードレビュー | - |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | - |
| マージ日 | - |
| マージ先 | DD-006_ai.md §weather, RD-001_product.md |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| `docs/requirements/RD-001_product.md` | 修正 | 天気要約 LLM の接続先設定化要件を追加 |
| `docs/design/detail/DD-006_ai.md` | 修正 | `summarize_weather` の LLM 設定参照仕様を追記 |

### 4.2 影響するコード

| ファイル | 変更種別 | 概要 |
|---------|---------|------|
| `src/service/ai/weather.rs` L67 `summarize_weather` | 修正 | `call_ollama_with_prompt` → `call_ollama_for_weather` helper を呼ぶ形に変更 |
| `src/service/ai/mod.rs` | 追加 | `call_ollama_for_weather` helper 関数を追加（`pub(super)`） |

> **注意:** `call_ollama_with_prompt` / `call_ollama_with_prompt_internal` の signature・可視性は変更しない（他の呼び出し元への波及防止）。

---

## 5. 差分仕様（What / How）

### 5.1 変更方針（確定）

天気要約の LLM 呼び出しを以下のとおり変更する。

| 項目 | 変更前 | 変更後 |
|------|--------|--------|
| 接続先 URL | `"http://localhost:11434/api/chat"`（hardcode） | `config::ai_config().llm_local_server_url`（`LLM_LOCAL_SERVER_URL` 環境変数） |
| モデル | `config::ai_config().ollama_model`（`OLLAMA_MODEL`） | `config::ai_config().llm_local_model`（`LLM_LOCAL_MODEL`） |
| タイムアウト | `config::timeouts().ai_http`（`AI_HTTP_TIMEOUT_MS`、既定 20 秒） | `config::ai_config().llm_local_timeout`（`LLM_LOCAL_TIMEOUT_MS`、既定 8 秒） |
| local 有効判定 | なし（無条件に呼ぶ） | `LLM_LOCAL_SERVER_ENABLED=false` の場合は LLM 呼び出しをスキップし `fallback_summary()` を返す |
| 失敗時動作 | `fallback_summary()` を返す（維持） | 変更なし（維持） |

> **`LLM_LOCAL_SERVER_ENABLED` の扱い:**
> `AiConfig.llm_local_server_enabled`（既定 `true`）が `false` の場合は weather LLM 呼び出しをスキップし、
> 直ちに `fallback_summary()` を返す。これにより general_chat 側の local stage enable/disable と挙動が一致する。
> `LLM_LOCAL_SERVER_ENABLED=false` は「ローカル Ollama を使わない」という意図の設定であり、weather 要約も同様に無効化するのが自然な解釈。

**設定変数追加なし:**
`AiConfig` には既に `llm_local_server_url` / `llm_local_model` / `llm_local_timeout` が存在する。
新規の `WEATHER_LLM_*` 変数は追加しない（設定項目の肥大化防止・既存設定との整合）。

**スコープ（確定）:**
- 対象: `weather.rs` の `summarize_weather()` のみ
- 対象外: 天気 API 取得ロジック（`fetch_weather_report`）・weatherキャッシュ
- 対象外: 多段フォールバック（raspi / cloud を weather に追加することは今回しない）
- 対象外: intent / general_chat / TTS の既存ロジック

### 5.2 実装方針（確定）

**weather 専用 helper 関数を追加する（STEER-222 の `call_ollama_for_intent_stage` と同パターン）。**

```rust
// mod.rs に追加
pub(super) async fn call_ollama_for_weather(
    messages: &[ChatMessage],
    system_prompt: &str,
    model: &str,
    endpoint_url: &str,
    http_timeout: Duration,
) -> Result<String>
// → call_ollama_with_prompt_internal には委譲しない。
//    intent helper 同様に個別実装し、raw body の INFO ログを出力しない。
```

- `weather.rs` はこの helper を呼ぶ
- `call_ollama_with_prompt` / `call_ollama_with_prompt_internal` の signature・可視性は変更しない
- `call_ollama_with_prompt_internal` は現状 raw body を INFO 出力するため（`mod.rs` L697-698）、
  委譲すると weather 呼び出しでも raw body が INFO 出力される。
  weather データ（場所名・気温等）は PII ではないが、ログ肥大化防止の観点から raw body 出力は行わない。
  そのため helper は `call_ollama_with_prompt_internal` に委譲せず、intent helper と同様に個別実装する。

### 5.3 `summarize_weather` の変更（weather.rs L51-73）

現行の `call_ollama_with_prompt` 呼び出しを以下のとおり置き換える。

```text
// 変更前（weather.rs L62-73）
let model = config::ai_config().ollama_model.clone();
match super::call_ollama_with_prompt(&messages, &prompt, &model).await {
    Ok(text) => text,
    Err(err) => {
        log::warn!("[weather] summarization failed: {err:?}");
        fallback_summary(report)
    }
}

// 変更後（概要）
let ai_cfg = config::ai_config();
if !ai_cfg.llm_local_server_enabled {
    log::debug!("[weather] LLM_LOCAL_SERVER_ENABLED=false, skipping LLM summarization");
    return fallback_summary(report);
}
let model = ai_cfg.llm_local_model.clone();
let endpoint_url = ai_cfg.llm_local_server_url.clone();
let timeout = ai_cfg.llm_local_timeout;
match super::call_ollama_for_weather(&messages, &prompt, &model, &endpoint_url, timeout).await {
    Ok(text) => text,
    Err(err) => {
        log::warn!(
            "[weather] summarization failed: endpoint={endpoint_url} timeout_ms={} err={err:?}",
            timeout.as_millis()
        );
        fallback_summary(report)
    }
}
```

### 5.4 ログ要件

| イベント | ログレベル | 必須フィールド |
|---------|-----------|--------------|
| 天気要約成功 | （既存のまま変更なし） | - |
| 天気要約失敗 | WARN | `endpoint`（接続先 URL）, `timeout_ms`（タイムアウト値）, `err`（エラー内容） |

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #224 | STEER-224 | 起票 |
| STEER-224 | DD-006_ai.md §weather | 設計反映 |
| DD-006_ai.md §weather | weather.rs L51 / mod.rs | 実装 |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] `LLM_LOCAL_*` 共用方針（新規 `WEATHER_LLM_*` 変数を追加しない）に合意しているか
- [ ] モデルを `OLLAMA_MODEL` → `LLM_LOCAL_MODEL` に変更することに合意しているか
- [ ] タイムアウトを `AI_HTTP_TIMEOUT_MS`（20 秒）→ `LLM_LOCAL_TIMEOUT_MS`（8 秒）に変更することに合意しているか
- [ ] `call_ollama_for_weather` helper 追加方針に合意しているか
- [ ] 多段フォールバック（raspi / cloud）を weather に追加しないことに合意しているか
- [ ] `fallback_summary()` の維持に合意しているか
- [ ] Q3（LLM_LOCAL_MODEL 運用影響）が疎通確認で解消する前提に合意しているか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] 実装が完了している
- [ ] コードレビューを受けている（CodeRabbit）
- [ ] 既存テスト（`weather.rs` L340-）が PASS している
- [ ] `LLM_LOCAL_SERVER_URL` を別ホストに設定した場合の動作確認ができている
- [ ] localhost:11434 不在環境で `fallback_summary()` が返ることを確認している
- [ ] `summarize_weather` が `LLM_LOCAL_SERVER_ENABLED=false` のとき LLM を呼ばず `fallback_summary()` を返すことを検証するテストケースを追加している（モック HTTP または単体テストで endpoint 選択を確認する。実装コストが高い場合は手動試験手順書を §8 に追記する）

---

## 8. 未確定点・質問

| # | 質問 | 選択肢 | 推奨 | オーナー回答 |
|---|------|--------|------|-------------|
| Q1 | 実装手段として weather 専用 helper を追加するか、`call_ollama_with_prompt_internal` の可視性を変更するか | A: helper 追加 / B: 可視性変更 | **A: helper 追加（STEER-222 の `call_ollama_for_intent_stage` と同パターン。誤用リスクが低い）** | **A: helper 追加** @MasanoriSuda 2026-02-23 |
| Q2 | タイムアウトを `LLM_LOCAL_TIMEOUT_MS`（8 秒）に合わせるか、独立した設定変数（`WEATHER_LLM_TIMEOUT_MS`）を新設するか | 共用 / 新設 | **共用（設定変数の肥大化防止。天気要約は general_chat より短文のため 8 秒で十分と判断）** | **共用（LLM_LOCAL_TIMEOUT_MS を使用）** @MasanoriSuda 2026-02-23 |
| Q3 | `OLLAMA_MODEL`（現行）から `LLM_LOCAL_MODEL` への変更で運用影響はないか（既存 `.env` のモデル設定を確認） | OK / 影響あり | **要確認（`LLM_LOCAL_MODEL` が未設定の場合は `OLLAMA_MODEL` をフォールバックするため互換維持）** | **疎通確認時に検証する** @MasanoriSuda 2026-02-23 |

---

## 9. リスク・ロールバック観点

| リスク | 影響 | 緩和策 |
|--------|------|--------|
| `LLM_LOCAL_SERVER_URL` 未設定環境でのデフォルト変更 | デフォルトは `http://localhost:11434/api/chat` のため変更前と同じ動作になり影響なし | `AiConfig` のデフォルト値（`mod.rs` L1017-1018）が互換を保証 |
| `LLM_LOCAL_TIMEOUT_MS`（8 秒）への短縮 | タイムアウト 20 秒 → 8 秒に短縮。LLM が遅い環境では `fallback_summary()` に落ちやすくなる | `fallback_summary()` はテキスト品質低下のみで機能停止なし。必要なら `LLM_LOCAL_TIMEOUT_MS` を引き上げて対応可 |
| `call_ollama_with_prompt` 既存呼び出し元への波及 | helper 追加方針のため既存関数の変更なしで波及ゼロ | - |
| `LLM_LOCAL_MODEL` 設定不一致（Q3） | モデルが意図と異なる可能性 | 疎通確認時に `.env` を確認して解消 |

**ロールバック手順:** 実装コミットを `git revert`。または `LLM_LOCAL_SERVER_URL=http://localhost:11434/api/chat`（デフォルト）を確認し、実質変更前と同じ接続先を使用する。

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-23 | 初版作成（Codex 調査結果を元に差分仕様を記述） | Claude Sonnet 4.6 |
| 2026-02-23 | Q1（helper 追加）・Q2（LLM_LOCAL_TIMEOUT_MS 共用）オーナー確定。Q3 は疎通確認時対応に変更。§4.2・§5.2・§7.1・§9 を更新 | Claude Sonnet 4.6 |
| 2026-02-23 | レビュー NG 対応: §5.1 に `LLM_LOCAL_SERVER_ENABLED` 無効時のスキップ仕様を追加。§5.2 を「委譲なし個別実装」に修正（raw body INFO 出力防止）。§5.3 の変更後擬似コードに enabled チェックを追加・PII 注記を削除。§7.2 に endpoint/timeout 選択のテストケース追加を追記 | Claude Sonnet 4.6 |
