# STEER-218: TTS ローカルサーバー優先 2 段フォールバック

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-218 |
| タイトル | TTS ローカルサーバー優先 2 段フォールバック |
| ステータス | Approved |
| 関連Issue | #218 |
| 優先度 | P0 |
| 作成日 | 2026-02-22 |

---

## 2. ストーリー（Why）

### 2.1 背景

Raspberry Pi 実機での動作確認で、TTS（音声合成）の速度・品質が実用水準に達しないことが判明した。
現行実装は `synth_zundamon_wav`（`src/service/ai/mod.rs L666`）で VOICEVOX 互換 API を
`localhost:50021` 固定で呼ぶ単一経路であり、以下の問題がある。

| 問題 | 詳細 |
|------|------|
| 接続先 URL が固定 | Pi 上の VOICEVOX エンジンしか使えない。別ホストの高性能サーバーへ切り替え不可（`mod.rs L671/683` に `localhost:50021` 固定） |
| タイムアウトが共通値 | TTS は `/audio_query` + `/synthesis` の 2 回 HTTP を直列実行するため、RTT 増加の影響を受けやすいが、段別タイムアウトがない |
| フォールバックがない | ローカルサーバーへの接続失敗時に Pi で再試行する手段がない |
| `VOICEVOX_URL` が未使用 | `docker-compose.dev.yml:15` に定義済みだが、backend は参照していない |

### 2.2 目的

TTS を 2 段フォールバック構成（ローカルサーバー → Pi）に変更し、
実用的な速度・品質を確保する。接続先・タイムアウトは設定で制御可能にする。

### 2.3 ユーザーストーリー

```text
As a システム管理者
I want to TTS バックエンドの優先順序と接続先を設定で変更したい
So that 環境（高性能サーバーあり/なし/Pi 単体）に応じて最適な TTS 構成を選択できる

受入条件:
- [ ] ローカルサーバー（別ホスト Voicevox）が利用可能な場合は優先して使用する
- [ ] ローカルサーバー失敗時は Pi 上の VOICEVOX HTTP へ自動フォールバックする
- [ ] 各段の接続先 URL・タイムアウトは環境変数で設定可能である
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
| 起票理由 | Raspberry Pi 実機での TTS 速度・品質問題の解消 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Sonnet 4.6 |
| 作成日 | 2026-02-22 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "ローカルのサーバーに依頼して、それでもだめならラズパイで実行する 2 段フォールバック。Codex 調査結果を元にステアリング作成" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | Codex | 2026-02-22 | NG | §5.3 擬似コードのシグネチャ誤り・call_control 行番号誤り・AI_HTTP_TIMEOUT_MS 事実不整合・docker-compose.dev.yml 未記載 の 4 件指摘 |
| 2 | @MasanoriSuda | 2026-02-22 | OK | 全指摘解消確認 |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-22 |
| 承認コメント | Codex 1 ラウンドの指摘を全件解消。Approved。 |

### 3.5 実装

| 項目 | 値 |
|------|-----|
| 実装者 | Codex |
| 実装日 | 2026-02-22 |
| 実装内容 | TTS 2 段フォールバック（local server → raspi）を実装。`TtsPort` に `call_id` を追加して段別ログ（start/success/failure/all failed）を実装。`AiConfig` に `TTS_*` 設定を追加し、`docker-compose.dev.yml` の `VOICEVOX_URL` を `TTS_LOCAL_SERVER_BASE_URL` に置換。 |
| 検証結果 | `cargo fmt` / `cargo test -q` / `cargo clippy -- -D warnings` PASS（127 tests passed） |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | 未定 |
| マージ日 | - |
| マージ先 | DD-006_ai.md §TTS, RD-001_product.md |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| `docs/requirements/RD-001_product.md` | 修正 | TTS 2 段フォールバック要件を追加 |
| `docs/design/detail/DD-006_ai.md` | 修正 | TTS フォールバック仕様・設定項目を追加 |

### 4.2 影響するコード

| ファイル | 変更種別 | 概要 |
|---------|---------|------|
| `src/service/ai/mod.rs` L666 `synth_zundamon_wav` | 修正 | シグネチャに `call_id: &str` 追加・`out_path` は維持。2 段フォールバック制御を実装し `synth_zundamon_for_stage` を呼ぶ |
| `src/service/ai/mod.rs`（新規）`synth_zundamon_for_stage` | 追加 | 段別 helper。`base_url / text / timeout` を引数で受け取り `/audio_query` → `/synthesis` を呼び WAV bytes を返す |
| `src/shared/config/mod.rs` L858 `AiConfig` | 修正 | TTS 段別 base URL / enabled / timeout フィールドを追加 |
| `src/service/ai/mod.rs` L557 `DefaultAiPort::new()` | 修正 | 起動時警告（全段無効 / raspi URL 未設定）を追加 |
| `src/shared/ports/ai/tts.rs` L7 `TtsPort::synth_to_wav` | 修正 | シグネチャに `call_id: String` を追加（ASR #216・LLM #217 と統一） |
| `src/service/ai/mod.rs` L642 `DefaultAiPort::synth_to_wav` | 修正 | Port 実装を新シグネチャに合わせ `call_id` を `tts::synth_to_wav` へ伝播 |
| `src/service/ai/tts.rs` L6 `tts::synth_to_wav` | 修正 | シグネチャに `call_id: &str` 追加し `synth_zundamon_wav` へ伝播 |
| `src/service/call_control/mod.rs` L433 / L512 / L539 / L575 | 修正 | `synth_to_wav(call_id, ...)` 呼び出し側を 4 箇所すべて修正（通常応答・転送確認・転送失敗・セキュリティ応答） |
| `docker-compose.dev.yml` L15 | 修正 | `VOICEVOX_URL` を `TTS_LOCAL_SERVER_BASE_URL` に置き換え |

---

## 5. 差分仕様（What / How）

### 5.1 フォールバック順序（確定）

```text
段 1: ローカルサーバー TTS（TTS_LOCAL_SERVER_BASE_URL で指定、デフォルト http://localhost:50021）
  ↓ エラー or タイムアウト
段 2: Pi TTS（TTS_RASPI_BASE_URL で指定、TTS_RASPI_ENABLED=true のときのみ）
  ↓ 全段失敗
呼び出し元で warn ログ・音声送信なし（現行動作を維持）
```

**実行方式（確定）:**
- 段 1・段 2 ともに VOICEVOX 互換 HTTP エンドポイント（`/audio_query` → `/synthesis`）を呼ぶ。
- URL 形式は **base URL**（例: `http://host:50021`）で指定し、エンドポイントパスはコードで付加する。
  - TTS は `/audio_query` と `/synthesis` の 2 エンドポイントを使うため、ASR/LLM と異なりフルエンドポイント URL ではなく base URL 形式が自然。
- Pi 段はリモートホスト上の VOICEVOX HTTP サーバーとして実装（最小差分）。
  プロセス起動制御（direct 実行）は別フェーズ。

**スコープ（確定）:**
- TTS ロジック本体の変更対象: `synth_zundamon_wav`（`src/service/ai/mod.rs L666`）および新規 `synth_zundamon_for_stage`。
- これに伴う追従修正: `TtsPort` / `DefaultAiPort::synth_to_wav` / `tts::synth_to_wav` / `call_control` 4 箇所 / `docker-compose.dev.yml`（§4.2 参照）。
- speaker_id は現行通り `3`（ずんだもん ノーマル）固定のまま（段別設定は今回スコープ外）。

### 5.2 設定項目（新規追加）

以下を環境変数および `AiConfig` struct に追加する。

| 環境変数 | 型 | 説明 | デフォルト |
|---------|-----|------|-----------|
| `TTS_LOCAL_SERVER_BASE_URL` | String | 段 1 ローカルサーバーの **base URL** | `http://localhost:50021`（現行 hardcode と同等、互換維持） |
| `TTS_LOCAL_SERVER_ENABLED` | bool | ローカルサーバー TTS を使用するか | `true` |
| `TTS_RASPI_BASE_URL` | String | 段 2 Pi の base URL | デフォルトなし（`TTS_RASPI_ENABLED=true` 時は設定必須） |
| `TTS_RASPI_ENABLED` | bool | Pi TTS を使用するか | `false` |
| `TTS_LOCAL_TIMEOUT_MS` | u64 | 段 1 タイムアウト（ms）。2 リクエスト合計 | `5000` |
| `TTS_RASPI_TIMEOUT_MS` | u64 | 段 2 タイムアウト（ms）。2 リクエスト合計 | `10000` |

> **注意:**
> - `TTS_LOCAL_SERVER_BASE_URL` のデフォルト `http://localhost:50021` は現行 hardcode 相当であり、**互換維持**（既存環境は URL 未設定でも動作継続）。
> - **URL は該当段の `*_ENABLED=true` のときのみ使用する。** `*_ENABLED=false` の段は URL 未設定でも起動エラーにならない。
> - **「全段無効」の判定:** `TTS_LOCAL_SERVER_ENABLED=false` かつ `TTS_RASPI_ENABLED=false` の場合を「有効段ゼロ」とする。この場合は起動時に警告ログを出し、TTS 不可として扱う（呼び出し元の warn + 音声なしへ直行）。
> - タイムアウトは **段全体（`/audio_query` + `/synthesis` 2 リクエスト合計）** に適用する（各リクエストに分割しない）。これにより設定値が「1 回の発話に許容する最大待ち時間」として直感的に解釈できる。
> - `VOICEVOX_URL`（compose L15）は `TTS_LOCAL_SERVER_BASE_URL` に置き換えて廃止する（Q4 確定）。`docker-compose.dev.yml` の更新は §4.2 に含む。
> - 現行の TTS は `config::timeouts().ai_http`（= `AI_HTTP_TIMEOUT_MS`）を使用している（`mod.rs L667`）。本変更後は段別タイムアウト（`TTS_*_TIMEOUT_MS`）に移行し、`AI_HTTP_TIMEOUT_MS` は TTS には適用しない。

### 5.3 フォールバックロジック（mod.rs L666 周辺の置き換え）

現行の `synth_zundamon_wav`（単一 URL 固定）を、次の順序で試行するループに置き換える。

```text
// 【段別 helper（新規）】
// synth_zundamon_for_stage(base_url: &str, call_id: &str, text: &str, timeout: Duration) -> Result<Vec<u8>>
//   /audio_query → /synthesis を実行し WAV bytes を返す（ファイル書き込みは行わない）

// 【フォールバック制御（既存関数を修正）】
// synth_zundamon_wav(call_id: &str, text: &str, out_path: &str) -> Result<()>
//   ← 現行: synth_zundamon_wav(text: &str, out_path: &str) -> Result<()>
//   out_path は維持。call_id を追加し、各段に伝播する。
for 各有効な TTS 段（local, raspi の順）:
    synth_zundamon_for_stage(base_url, call_id, text, timeout) を呼び出す
    成功（wav_bytes を取得）:
        ログ出力「TTS 成功: 段={local|raspi}, call_id=..., wav_size=...」
        tokio::fs::write(out_path, &wav_bytes).await?
        return Ok(())
    失敗（エラー or タイムアウト）:
        ログ出力「TTS 失敗: 段=..., 理由=..., call_id=..., 次段へ」
        次段へ続行
全段失敗:
    return Err(anyhow::anyhow!("all TTS stages failed"))
    // 呼び出し元（call_control）で warn ログ・音声送信なし（現行動作を維持）
```

**フォールバック条件（確定）:**
- HTTP エラー / タイムアウト → 次段へ
- 空 WAV / 異常サイズ → **成功扱いのまま（シンプル実装優先、今回変更なし）**

### 5.4 ログ要件（確定）

以下を構造化ログとして必ず記録する。

| イベント | ログレベル | 必須フィールド |
|---------|-----------|--------------|
| 各段で TTS 試行開始 | DEBUG | `call_id`, `tts_stage`（local/raspi） |
| 各段で TTS 成功 | INFO | `call_id`, `tts_stage`, `wav_size` |
| 各段で TTS 失敗 | WARN | `call_id`, `tts_stage`, `reason` |
| 全段失敗 | ERROR | `call_id`, `reason="all TTS stages failed"` |

> **PII 方針:** TTS の合成テキスト本文はログに出力しない（`text_len` のみ出力可）。

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #218 | STEER-218 | 起票 |
| STEER-218 | DD-006_ai.md §TTS | 設計反映 |
| DD-006_ai.md §TTS | mod.rs L666 / config/mod.rs L858 | 実装 |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [x] 2 段フォールバック順序が要件と一致しているか
- [x] URL 形式（base URL）の採用に合意しているか
- [x] `TTS_LOCAL_SERVER_BASE_URL` の互換維持（デフォルト `http://localhost:50021`）に合意しているか
- [x] タイムアウトが「段全体（2 リクエスト合計）」方式に合意しているか
- [x] タイムアウト値（5000/10000 ms）が妥当か
- [x] `TTS_RASPI_ENABLED=false` デフォルトに合意しているか
- [x] Q1〜Q5 の未確定点が全件解消しているか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] 実装が完了している
- [ ] コードレビューを受けている（CodeRabbit）
- [ ] 全段フォールバックの結合テストが PASS
- [ ] 段ごとの成功/失敗ログが出力されることを確認

---

## 8. 未確定点・質問

| # | 質問 | 選択肢 | 推奨 | オーナー回答 |
|---|------|--------|------|-------------|
| Q1 | `call_id` を段別ログに含めるか？（`TtsPort::synth_to_wav` のシグネチャ変更が必要。ASR #216・LLM #217 と同様） | Yes / No | **Yes（ASR/LLM と統一）** | **Yes（Port trait・内部関数・呼び出し側を修正）** @MasanoriSuda 2026-02-22 |
| Q2 | タイムアウト方式は「段全体（`/audio_query` + `/synthesis` 2 リクエスト合計）」か「各 HTTP リクエストごと」か | 段全体 / 各リクエスト | **段全体（管理しやすく、発話単位の最大待ち時間として直感的）** | **段全体** @MasanoriSuda 2026-02-22 |
| Q3 | 空 WAV / 異常サイズ（HTTP 200 だが内容が不正）を失敗扱いにして次段フォールバックするか | Yes（次段へ） / No（成功扱い） | **No（シンプル実装優先、今回はエラー/タイムアウトのみ失敗扱い）** | **No（成功扱いのまま）** @MasanoriSuda 2026-02-22 |
| Q4 | `VOICEVOX_URL`（compose L15 定義済み・未使用）を `TTS_LOCAL_SERVER_BASE_URL` に統合して廃止するか | 統合廃止 / 互換エイリアスとして読む | **統合廃止（変数を整理し、`TTS_LOCAL_SERVER_BASE_URL` に一本化）** | **統合廃止** @MasanoriSuda 2026-02-22 |
| Q5 | タイムアウト値（段 1: 5000 ms、段 2: 10000 ms）は妥当か | OK / 変更 | - | **OK（変更なし）** @MasanoriSuda 2026-02-22 |

---

## 9. リスク・ロールバック観点

| リスク | 影響 | 緩和策 |
|--------|------|--------|
| 全段フォールバックで遅延が累積 | 対話応答が遅くなる | 段ごとにタイムアウトを短めに設定（設定可能） |
| Pi VOICEVOX サーバーが未起動 | 段 2 がタイムアウトして遅延 | `TTS_RASPI_ENABLED=false` をデフォルトにして影響ゼロ |
| `VOICEVOX_URL`（compose）が `TTS_LOCAL_SERVER_BASE_URL` に置き換わる | compose を参照していた場合に設定変更が必要 | `TTS_LOCAL_SERVER_BASE_URL` のデフォルトが現行 hardcode 相当のため単体実行互換は維持される |
| TTS は 2 HTTP リクエスト直列のため RTT の影響大 | リモート化でレイテンシ増加 | ローカルサーバー（LAN 内）を段 1 に置くことで RTT を最小化 |

**ロールバック手順:** 実装コミットを `git revert`。新規フラグを無効化する場合は `TTS_LOCAL_SERVER_ENABLED=false` / `TTS_RASPI_ENABLED=false` を設定し、旧 `localhost:50021` 固定動作に戻すには `TTS_LOCAL_SERVER_BASE_URL=http://localhost:50021`（デフォルト）を確認する。

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-22 | 初版作成（Codex 調査結果を元に差分仕様を記述） | Claude Sonnet 4.6 |
| 2026-02-22 | Q1〜Q5 オーナー回答反映（call_id 追加・タイムアウト段全体方式・空 WAV 成功扱い・VOICEVOX_URL 廃止・タイムアウト値 OK）、§4.2 の Q1 条件分岐を確定変更に修正、§5.3 シグネチャ確定、§5.4 の "(Q1 確定後)" 注釈を削除 | Claude Sonnet 4.6 |
| 2026-02-22 | Codex レビュー NG 指摘対応: §5.3 擬似コードを out_path 維持・段別 helper 分離の正確な構造に修正・§4.2 call_control 行番号を実際の呼び出し位置（L433/512/539/575）に修正し 4 箇所明記・tts.rs L6 を影響コードに追加・docker-compose.dev.yml L15 を影響コードに追加・§5.2 AI_HTTP_TIMEOUT_MS の説明を「現行は使用中・本変更後に段別タイムアウトへ移行」に修正 | Claude Sonnet 4.6 |
