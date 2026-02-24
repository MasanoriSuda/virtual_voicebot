# STEER-241: intent の OpenAI クラウドプロバイダー追加

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-241 |
| タイトル | intent の OpenAI クラウドプロバイダー追加 |
| ステータス | Approved |
| 関連Issue | #241 |
| 優先度 | P1 |
| 作成日 | 2026-02-24 |

---

## 2. ストーリー（Why）

### 2.1 背景

STEER-231（#231）で ASR / LLM / TTS / weather 要約を OpenAI cloud 最優先化したが、**intent 分類だけ local / raspi の二段階フォールバックのまま**残っている。

| 課題 | 詳細 |
|------|------|
| PoC 環境でのタイムアウト遅延 | local LLM サーバー不達時に `INTENT_LOCAL_TIMEOUT_MS=3000ms` を消費してから raspi にフォールバックし、それも不達なら `INTENT_RASPI_TIMEOUT_MS=5000ms` 経過後に `general_chat` へ落ちる（`src/service/ai/intent.rs:86`） |
| 意図分類失敗による体感悪化 | intent 失敗 → general_chat フォールバック（`src/service/call_control/mod.rs:463`）では天気・転送などの専用フローが発動せず、UX が低下する |
| 構成上の非対称 | ASR / LLM / TTS は cloud 最優先だが intent だけ local 専用のため、「PC 上で local サーバーなし」構成では intent が常に失敗する |

### 2.2 目的

intent 分類に OpenAI Cloud ステージを追加し、**cloud → local → raspi** の順でフォールバックさせる。

- **Rust コード最小変更**（`intent.rs` に cloud stage 追加、`config/mod.rs` に設定項目追加）
- STEER-231 の確立したパターン（`openai_*_enabled` フラグ + `openai_base_url` / `openai_api_key` 共用）を踏襲
- local / raspi フォールバックは維持（既存構成を壊さない）

### 2.3 ユーザーストーリー

```text
As a PoC 運用者
I want to local LLM サーバーなし構成でも intent 分類を成功させたい
So that 天気・転送などの意図フローが cloud 経由で確実に発動する

受入条件:
- [ ] OPENAI_INTENT_ENABLED=true 設定時に OpenAI Chat Completions API で intent 分類が成功する
- [ ] cloud 失敗 / 未設定時に local → raspi にフォールバックする
- [ ] intent cloud 成功時のレイテンシが local timeout 待ち（3000ms）より短い
- [ ] OPENAI_INTENT_ENABLED=false（または未設定）時に既存の local / raspi 動作が変わらない
- [ ] cloud からの JSON 不正レスポンスは general_chat フォールバックになる（local/raspi と同一挙動。call_control 側の `parse_intent_json()` でハンドルされる）
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-24 |
| 起票理由 | intent の local/raspi 依存がボトルネックになっており、他の AI 機能と同様に OpenAI cloud を最優先化したい（#241） |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Sonnet 4.6 |
| 作成日 | 2026-02-24 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "intent も cloud 対応する #241。Codex 調査結果を踏まえて STEER-231 のパターンで作成してほしい" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | Codex | 2026-02-24 | NG | ①`call_openai_intent()` の戻り型 `Result<IntentResult>` が `classify_intent()` の契約 `Result<String>` と不整合 ②`intent_stage_count()` 更新だけでは `log_intent_startup_warnings()` の no-stages 文言・OPENAI_API_KEY 未設定 warning が漏れる ③`src/service/ai/mod.rs:463` 誤記→実体は `src/service/call_control/mod.rs:463` |
| 2 | Codex | 2026-02-24 | NG | ①§5.3 JSON バリデーション責務の二重定義（`:224` classify_intent() 側 vs `:241` 呼び出し元）②`env_bool` 引数不足・`env_string` が存在せず `env_non_empty` が正しい ③`call_openai_chat_completions()` が存在しない。実在するのは `call_openai_chat_for_stage()`（`src/service/ai/mod.rs:376`） |
| 3 | Codex | 2026-02-24 | NG | ①JSON 不正時挙動の矛盾: §5.3 は raw JSON 返し（call_control でパース）なのに §2.3/§7.2/§9 は「cloud スキップして次ステージ」前提（実際は general_chat フォールバック）②§5.2 未定義変数 `transcript`（classify_intent 引数は `text`） |

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
| 指示内容 | 承認済み STEER-241 に基づき、intent の OpenAI cloud ステージ（cloud→local→raspi）、`OPENAI_INTENT_*` 設定、startup warning 更新、`.env.example` 追記を実装する |
| コードレビュー | 未実施（実装後に `cargo fmt --all` / `cargo test --lib` / `cargo clippy --lib -- -D warnings` / `cargo build --lib` を実行して確認） |

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
| `docs/steering/STEER-241_intent-cloud-provider.md`（本ファイル） | 新規 | 本 Issue の差分仕様 |

### 4.2 影響するコード

| ファイル | 変更種別 | 概要 |
|---------|---------|------|
| `src/service/ai/intent.rs` | 修正 | Cloud ステージ追加（OpenAI → local → raspi の順に変更） |
| `src/shared/config/mod.rs` | 修正 | `AiConfig` に `openai_intent_enabled: bool` / `openai_intent_timeout: Duration` 追加、env 変数読み込み追加 |
| `.env.example` | 修正 | `OPENAI_INTENT_ENABLED` / `OPENAI_INTENT_TIMEOUT_MS` のサンプル追記 |
| `src/service/ai/mod.rs` | 修正（要確認） | ① `intent_stage_count()` 関数に cloud ステージを含める更新（§8 Q2・§5.6 参照）。② `log_intent_startup_warnings()` の no-stages 文言・`OPENAI_API_KEY` 未設定 warning 追加（§5.6 参照） |

---

## 5. 差分仕様（What / How）

### 5.1 フォールバック順序変更

**変更前:**

```
intent 分類
  ↓
[gate] local_enabled || raspi_enabled でなければ bail
  ↓
local ステージ（INTENT_LOCAL_TIMEOUT_MS=3000ms）
  ↓ 失敗
raspi ステージ（INTENT_RASPI_TIMEOUT_MS=5000ms）
  ↓ 失敗
"all intent stages failed" エラー → general_chat フォールバック
```

**変更後:**

```
intent 分類
  ↓
[gate] cloud_enabled || local_enabled || raspi_enabled でなければ bail
  ↓
cloud ステージ（OpenAI Chat Completions / OPENAI_INTENT_TIMEOUT_MS=3000ms）
  ↓ 失敗 or OPENAI_INTENT_ENABLED=false
local ステージ（INTENT_LOCAL_TIMEOUT_MS=3000ms）
  ↓ 失敗 or local_enabled=false
raspi ステージ（INTENT_RASPI_TIMEOUT_MS=5000ms）
  ↓ 失敗
"all intent stages failed" エラー → general_chat フォールバック（変更なし）
```

### 5.2 intent.rs の変更仕様（`src/service/ai/intent.rs`）

**変更箇所 1: gate（line 86 付近）**

```rust
// 変更前
if !ai_cfg.intent_local_server_enabled && !ai_cfg.intent_raspi_enabled {
    log::error!("[intent {call_id}] intent failed: reason=all intent stages disabled");
    anyhow::bail!("all intent stages failed");
}

// 変更後
if !ai_cfg.openai_intent_enabled
    && !ai_cfg.intent_local_server_enabled
    && !ai_cfg.intent_raspi_enabled
{
    log::error!("[intent {call_id}] intent failed: reason=all intent stages disabled");
    anyhow::bail!("all intent stages failed");
}
```

**変更箇所 2: Cloud ステージの挿入（line 91 の前に追加）**

```rust
// Cloud ステージ（OpenAI Chat Completions）
if ai_cfg.openai_intent_enabled {
    if let Some(api_key) = &ai_cfg.openai_api_key {
        match call_openai_intent(
            text.as_str(),
            call_id,
            api_key,
            &ai_cfg.openai_base_url,
            &ai_cfg.openai_intent_model,
            ai_cfg.openai_intent_timeout,
        )
        .await
        {
            Ok(result) => {
                log::info!("[intent {call_id}] stage=cloud result={result:?}");
                return Ok(result);
            }
            Err(e) => {
                log::warn!("[intent {call_id}] stage=cloud failed: {e}");
            }
        }
    } else {
        log::warn!("[intent {call_id}] stage=cloud skipped: OPENAI_API_KEY not set");
    }
}
// （以降、既存の local / raspi ステージは変更なし）
```

> **Note:** `call_openai_intent()` 関数は `src/service/ai/mod.rs` に新規追加する（§5.3 参照）。

### 5.3 OpenAI intent ヘルパー関数（`src/service/ai/mod.rs` に新規追加）

OpenAI Chat Completions API を使って intent 分類する関数を追加する。既存の `call_openai_chat_for_stage()` パターンを踏襲する（`src/service/ai/mod.rs:376`）。

```rust
/// OpenAI Chat Completions API を使って intent プロンプトを送信し、
/// `choices[0].message.content` を raw JSON 文字列として返す。
/// local / raspi ステージと同一の契約（`Result<String>`）に揃える。
/// 通信エラー・content=None 時は Err を返しフォールバックを促す。
/// JSON の構造バリデーション（intent フィールドの検証）は classify_intent() 呼び出し元（call_control 側）で実施する。
async fn call_openai_intent(
    text: &str,
    call_id: &str,
    api_key: &str,
    base_url: &str,
    model: &str,
    timeout: Duration,
) -> anyhow::Result<String>
```

**処理フロー:**
1. 既存の intent プロンプト（local / raspi と同一のシステムプロンプト）を構築
2. `openai_base_url/chat/completions` に POST（Authorization: Bearer `openai_api_key`）
   - **JSON モード有効**（`response_format: { "type": "json_object" }`）を指定する（§8 Q3 確認済み）
   - system prompt に JSON 出力の明示指示が含まれていることを確認すること（OpenAI の JSON モード要件）
3. `choices[0].message.content` を `String` として返す（raw JSON 文字列）
   - JSON 構造のバリデーションは `classify_intent()` 呼び出し元（call_control 側: `src/service/call_control/mod.rs:467`, `src/service/call_control/router.rs:167`）で実施（local/raspi と共通の parse ロジックを再利用）
4. `content` が `None` または HTTP エラー時は `anyhow::bail!(...)` で Err を返す（フォールバック動作）
5. タイムアウトは `openai_intent_timeout`（`OPENAI_INTENT_TIMEOUT_MS` 環境変数）

### 5.4 config/mod.rs の変更仕様（`src/shared/config/mod.rs`）

**AiConfig 構造体に追加（line 877 付近）:**

```rust
// 変更後（追加フィールド）
pub openai_intent_enabled: bool,
pub openai_intent_timeout: Duration,
pub openai_intent_model: String,   // §8 Q1 確認済み: OPENAI_INTENT_MODEL、デフォルト "gpt-4o-mini"
```

**env 変数読み込みに追加（line 1058 付近）:**

```rust
openai_intent_enabled: env_bool("OPENAI_INTENT_ENABLED", false),
openai_intent_timeout: env_duration_ms("OPENAI_INTENT_TIMEOUT_MS", 3_000),
openai_intent_model: env_non_empty("OPENAI_INTENT_MODEL")
    .unwrap_or_else(|| "gpt-4o-mini".to_string()),
```

**ドキュメントコメント追加（line 943 付近）:**

```rust
/// - `OPENAI_INTENT_ENABLED`: enable OpenAI as the cloud intent stage; defaults to `false`.
/// - `OPENAI_INTENT_TIMEOUT_MS`: OpenAI intent timeout in milliseconds; defaults to `3000`.
/// - `OPENAI_INTENT_MODEL`: OpenAI model for intent classification; defaults to `"gpt-4o-mini"`.
```

### 5.5 .env.example の変更（`.env.example`）

```dotenv
# OpenAI（既存の ASR/LLM/TTS に加えて intent も cloud 対応）
# OPENAI_API_KEY=sk-...
# OPENAI_BASE_URL=https://api.openai.com/v1
# OPENAI_ASR_ENABLED=true
# OPENAI_LLM_ENABLED=true
# OPENAI_TTS_ENABLED=true
# OPENAI_INTENT_ENABLED=true       ← 追加
# TTS_CLOUD_TIMEOUT_MS=10000
# OPENAI_INTENT_TIMEOUT_MS=3000    ← 追加（デフォルト: 3000ms）
# OPENAI_INTENT_MODEL=gpt-4o-mini  ← 追加（デフォルト: gpt-4o-mini）
```

### 5.6 intent_stage_count() / log_intent_startup_warnings() の更新（`src/service/ai/mod.rs`）

`intent_stage_count()` と `log_intent_startup_warnings()` に cloud ステージを考慮した更新を加える。

**`intent_stage_count()` の更新（`src/service/ai/mod.rs:261` 付近）:**

cloud ステージは「`openai_intent_enabled` が true かつ `openai_api_key` が Some」のときのみ実効的に有効。この条件を helper 関数として整理し、stage count に反映する。

```rust
// helper（cloud ステージが実効的に有効かどうかを判定）
fn openai_intent_stage_enabled(ai_cfg: &config::AiConfig) -> bool {
    ai_cfg.openai_intent_enabled && ai_cfg.openai_api_key.is_some()
}

// 変更後（cloud を含む stage count）
fn intent_stage_count(ai_cfg: &config::AiConfig) -> usize {
    usize::from(openai_intent_stage_enabled(ai_cfg))
        + usize::from(ai_cfg.intent_local_server_enabled)
        + usize::from(ai_cfg.intent_raspi_enabled)
}
```

**`log_intent_startup_warnings()` の更新（`src/service/ai/mod.rs:141` 付近）:**

cloud ステージ追加に伴い、以下の warning を追加する。

```rust
// 追加 warning ①: OPENAI_INTENT_ENABLED=true だが OPENAI_API_KEY 未設定
if ai_cfg.openai_intent_enabled && ai_cfg.openai_api_key.is_none() {
    log::warn!("intent: OPENAI_INTENT_ENABLED=true but OPENAI_API_KEY is not set; cloud stage will be skipped");
}

// 追加 warning ②: no-stages 文言に cloud も含める（既存の no-stages チェックを更新）
// intent_stage_count() が更新済みのため、既存の no-stages warning は自動的に cloud も考慮される
```

> **Note:** `intent_stage_count()` が 0 のとき既存の no-stages warning が出力される。`openai_intent_stage_enabled()` を stage count に含めることで、cloud が設定済みなら no-stages にならない。`OPENAI_INTENT_ENABLED=true` + `OPENAI_API_KEY` 未設定のケースは「設定ミス」として warning ①で検出する。

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #241 | STEER-241 | 起票 |
| STEER-231 | STEER-241 | ASR/LLM/TTS の cloud 対応パターンを踏襲。`openai_api_key` / `openai_base_url` は共用 |
| STEER-241 | `src/service/ai/intent.rs` | Cloud ステージ追加（gate 修正 + cloud ブロック挿入） |
| STEER-241 | `src/service/ai/mod.rs` | `call_openai_intent()` ヘルパー追加、`intent_stage_count()` 更新、`openai_intent_stage_enabled()` helper 追加、`log_intent_startup_warnings()` 更新（§5.6） |
| STEER-241 | `src/shared/config/mod.rs` | `openai_intent_enabled` / `openai_intent_timeout` / `openai_intent_model` 追加 |
| STEER-241 | `src/service/ai/intent.rs:86` | gate 変更の対象行 |
| STEER-241 | `src/service/call_control/mod.rs:463` | general_chat フォールバックは変更なし（intent 全失敗時の挙動を維持） |
| STEER-241 | `src/shared/config/mod.rs:1058-1059` | INTENT_LOCAL_TIMEOUT_MS / INTENT_RASPI_TIMEOUT_MS は変更なし |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] フォールバック順序が cloud → local → raspi で一貫しているか（STEER-231 との整合）
- [ ] `OPENAI_INTENT_ENABLED=false` 時に既存動作が変わらないことが仕様上で保証されているか
- [ ] JSON 不正レスポンス時に general_chat フォールバックになる挙動が明確か（call_control 側 `parse_intent_json()` で処理、local/raspi と同一挙動）
- [ ] `OPENAI_API_KEY` 未設定時に cloud ステージをスキップする挙動が明確か
- [x] §8 Q1（モデル名）の回答が反映されているか（`OPENAI_INTENT_MODEL`、デフォルト `gpt-4o-mini`）
- [x] §8 Q2（`intent_stage_count()` 更新要否）の回答が反映されているか（Codex が確認して対応）
- [x] §8 Q3（JSON モード使用可否）の回答が反映されているか（JSON モード使用で確定）

### 7.2 マージ前チェック（Approved → Merged）

- [ ] `OPENAI_INTENT_ENABLED=true` で cloud intent 分類が成功する
- [ ] cloud 失敗時に local → raspi にフォールバックする
- [ ] `OPENAI_INTENT_ENABLED=false` で既存の local / raspi 動作が変わらない
- [ ] cloud から JSON 不正レスポンスが返った場合に general_chat フォールバックになることを確認する（`call_control/router.rs:167` の `parse_intent_json()` で処理）
- [ ] 既存の unit テストが PASS する

---

## 8. 未確定点・質問

| # | 質問 | 選択肢 | 推奨 | オーナー回答 |
|---|------|--------|------|-------------|
| Q1 | intent cloud ステージで使う OpenAI モデル名をどうするか | A: `OPENAI_INTENT_MODEL` 環境変数として独立（デフォルト `gpt-4o-mini`）/ B: 既存の LLM モデルを共用（`openai_llm_model`） | **A を推奨**（intent は分類タスクで応答速度とコストを優先したい。`gpt-4o-mini` が安価・高速） | **A に確定**。`OPENAI_INTENT_MODEL`（デフォルト `gpt-4o-mini`）を独立フィールドとして追加。§5.4 に反映済み |
| Q2 | `src/service/ai/mod.rs` の `intent_stage_count()` 関数を cloud 含む形に更新が必要か | 必要 / 不要（関数が使われていなければスキップ可） | **Codex が確認して判断**（`intent_stage_count()` の使われ方を調査してから決定） | **Codex が確認して対応**。使われていれば cloud ステージ分も `+1` する。§4.2 の影響コード欄に「要確認」として記載済み |
| Q3 | OpenAI API の JSON モード（`response_format: { type: "json_object" }`）を使うか | 使う（JSON 不正を減らせる。ただし system prompt に JSON 出力を明示する必要あり）/ 使わない（既存 local/raspi と同じプロンプトで対応） | **使うことを推奨**（JSON 不正によるフォールバックを減らし、cloud ステージの成功率を上げる。既存プロンプトは既に JSON 出力指示のはず） | **使うで確定**。§5.3 処理フロー step 2 に `response_format: json_object` を追記済み |

---

## 9. リスク・ロールバック観点

| リスク | 影響 | 緩和策 |
|--------|------|--------|
| OpenAI API レート制限 / 障害 | cloud ステージ失敗 → local/raspi フォールバック（UX 影響小） | フォールバック設計で吸収 |
| コスト増（intent 呼び出し追加） | 1ターンあたり ASR + intent + LLM + TTS の 4 呼び出しになる | PoC 用途では許容範囲 |
| JSON 不正レスポンス | call_control 側の `parse_intent_json()` でパース失敗 → general_chat フォールバック（local/raspi と同一挙動） | JSON モード使用（§8 Q3）で低減 |
| `OPENAI_API_KEY` 未設定での OPENAI_INTENT_ENABLED=true | cloud ステージをスキップして local/raspi に進む（警告ログ出力） | §5.2 で `api_key.is_none()` チェック済み |

**ロールバック:** `OPENAI_INTENT_ENABLED=false`（または未設定）にするだけで即座に既存の local/raspi 動作に戻る。コード変更のロールバックは不要。

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-24 | 初版作成（Codex 調査結果を元に差分仕様を記述。STEER-231 の cloud 対応パターンを踏襲） | Claude Sonnet 4.6 |
| 2026-02-24 | §8 Q1/Q2/Q3 オーナー回答記録（全推奨採用）。§5.2 Q1 参照コメント削除、§5.3 JSON モード確定（response_format: json_object）、§5.4 openai_intent_model フィールド確定、§5.5 OPENAI_INTENT_MODEL 追加、§7.1 Q* チェック済みに更新、§6 トレーサビリティ更新 | Claude Sonnet 4.6 |
| 2026-02-24 | §3.3 Round 1 NG 記録。①§5.3 戻り型 `Result<IntentResult>` → `Result<String>` に修正（local/raspi と契約統一）②§4.2/§5.6 新規追加: `openai_intent_stage_enabled()` helper・`log_intent_startup_warnings()` 更新要件（OPENAI_API_KEY 未設定 warning・no-stages 文言）③§2.1/§6 の `src/service/ai/mod.rs:463` → `src/service/call_control/mod.rs:463` に修正 | Claude Sonnet 4.6 |
| 2026-02-24 | §3.3 Round 2 NG 記録。①§5.3 `:224` の責務記述を `classify_intent() 呼び出し元（call_control 側）` に統一・`:241` に call_control 参照先追加 ②§5.4 `env_bool` に第2引数 `false` 追加・`env_string` → `env_non_empty` に修正 ③§5.3 `call_openai_chat_completions()` → `call_openai_chat_for_stage()`（`src/service/ai/mod.rs:376`）に修正 | Claude Sonnet 4.6 |
| 2026-02-24 | §3.3 Round 3 NG 記録（Option B 採用）。①§2.3/§7.2/§9 の「JSON 不正時 cloud スキップ→次ステージ」を「JSON 不正時 general_chat フォールバック（call_control/router.rs:167 で処理、local/raspi と同一挙動）」に修正 ②§5.2/§5.3 `transcript` → `text.as_str()` / `text: &str` に修正 | Claude Sonnet 4.6 |
| 2026-02-24 | §3.3 Round 4 NG 記録。§7.1 の「JSON 不正レスポンス時にフォールバックする（エラー返し）」を「general_chat フォールバックになる挙動（call_control 側 parse_intent_json()）」に修正 | Claude Sonnet 4.6 |
| 2026-02-24 | §3.3 Round 5 OK 記録。§1 ステータス Draft → Approved、§3.4 承認者記録（@MasanoriSuda, lgtm） | Claude Sonnet 4.6 |
