# STEER-216: ASR クラウド優先 3 段フォールバック

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-216 |
| タイトル | ASR クラウド優先 3 段フォールバック |
| ステータス | Approved |
| 関連Issue | #216 |
| 優先度 | P0 |
| 作成日 | 2026-02-22 |

---

## 2. ストーリー（Why）

### 2.1 背景

Raspberry Pi 実機での動作確認で、ASR の速度・品質が実用水準に達しないことが判明した。
現行実装は「AWS Transcribe（有効時）→ ローカル Whisper HTTP（localhost:9000 固定）」の 2 段構成であり、
以下の問題がある。

| 問題 | 詳細 |
|------|------|
| クラウド ASR がない場合に品質が落ちる | Pi 上 Whisper のみでは速度・精度ともに不十分 |
| ローカルサーバー URL が固定 | 別マシンの ASR サーバーに切り替え不可（mod.rs 内 `localhost:9000` 固定） |
| ASR 設定が未整備 | `AiConfig` に接続先・順序・タイムアウト設定がなく、環境変数のみで制御不能 |
| フォールバック順序が暗黙的 | クラウド失敗時の挙動がコードに埋め込まれており、設定で変更不可 |

### 2.2 目的

音声認識を 3 段フォールバック構成（クラウド → ローカルサーバー → Pi ローカル実行）に変更し、
実用的な速度・品質を確保する。フォールバック順序・接続先・タイムアウトは設定で制御可能にする。

### 2.3 ユーザーストーリー

```text
As a システム管理者
I want to ASR バックエンドの優先順序と接続先を設定で変更したい
So that 環境（クラウドあり/なし/Pi 単体）に応じて最適な ASR 構成を選択できる

受入条件:
- [ ] クラウド ASR が利用可能な場合は最優先で使用する
- [ ] クラウド失敗時はローカルサーバー ASR へ自動フォールバックする
- [ ] ローカルサーバー失敗時は Pi 上 ASR（HTTP サーバー方式）へフォールバックする
- [ ] 各段の接続先 URL と タイムアウトは環境変数で設定可能である
- [ ] どの段で成功/失敗したかがログに記録される
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-22 |
| 起票理由 | Raspberry Pi 実機での ASR 速度・品質問題の解消 |

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
| 1 | Codex | 2026-02-22 | OK | 前回 NG（AiConfig 参照先・call_id・AsrError 記法・章番号・URL互換性・ロールバック手順）はすべて解消済み。非ブロッカー補足: 起動時警告の実現位置・結合テストケース網羅は実装時に決定 |
| 2 | @MasanoriSuda | 2026-02-22 | OK | lgtm |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-22 |
| 承認コメント | lgtm |

### 3.5 実装

| 項目 | 値 |
|------|-----|
| 実装者 | Codex |
| 実装日 | 2026-02-22 |
| 実装内容 | `AiConfig` に ASR URL/有効フラグ/段別タイムアウトを追加、`service/ai/mod.rs` を 3 段フォールバック化、`call_id` 付き段別ログ追加、幻聴フィルタを各段判定へ移動 |
| 検証 | `cargo fmt` / `cargo test` PASS |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | 未定 |
| マージ日 | - |
| マージ先 | DD-006_ai.md §2, RD-001_product.md F-3 |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| docs/requirements/RD-001_product.md | 修正 | F-3 ASR に 3 段フォールバック要件を追加 |
| docs/design/detail/DD-006_ai.md | 修正 | §2 ASR フォールバック仕様・設定項目を追加 |

### 4.2 影響するコード

| ファイル | 変更種別 | 概要 |
|---------|---------|------|
| `src/service/ai/mod.rs` 付近 L92 | 修正 | ASR フォールバックを 3 段に拡張 |
| `src/shared/config/mod.rs` L858 | 修正 | `AiConfig` struct に ASR 設定フィールドを追加 |
| `src/service/ai/mod.rs` 付近 L121 | 修正 | ローカル ASR URL を設定化（ハードコード廃止） |
| `src/service/ai/mod.rs` L92 `transcribe_and_log` | 修正 | シグネチャに `call_id: &str` を追加し、段ごとのログを同関数内で出力する |
| `src/service/ai/asr.rs` 付近 L29 | 修正 | 幻聴フィルタを各段で実行するよう移動（Q2 確定） |

---

## 5. 差分仕様（What / How）

### 5.1 フォールバック順序（確定）

```text
段 1: クラウド ASR（AWS Transcribe、設定で有効化）
  ↓ エラー or タイムアウト
段 2: ローカルサーバー ASR（別ホスト上の whisper_server.py）
  ↓ エラー or タイムアウト
段 3: Pi ASR（Pi（別ホスト）上で起動した whisper_server.py、HTTP サーバー方式）
  ↓ 全段失敗
謝罪音声フォールバック（現行動作を維持）
```

**Pi 実行方式（決定）:** Pi（別ホスト）上で `whisper_server.py` を HTTP サーバーとして起動し、
backend は HTTP 経由で呼び出す。subprocess/組み込み方式は採用しない（責務境界を維持）。
段 2 ローカルサーバー・段 3 Pi とも別ホスト・ポート 9000 を使用する。

### 5.2 設定項目（新規追加）

以下を環境変数および `AiConfig` struct に追加する。

| 環境変数 | 型 | 説明 | デフォルト |
|---------|-----|------|-----------|
| `USE_AWS_TRANSCRIBE` | bool | クラウド ASR（AWS Transcribe）を使用するか | `false` |
| `ASR_LOCAL_SERVER_URL` | String | 段 2 ローカルサーバー URL | `http://localhost:9000/transcribe`（現行 hardcode と同等） |
| `ASR_LOCAL_SERVER_ENABLED` | bool | ローカルサーバー ASR を使用するか | `true` |
| `ASR_RASPI_URL` | String | 段 3 Pi ASR URL（**設定必須**、別ホスト） | 例: `http://<raspi-ip>:9000/transcribe` |
| `ASR_RASPI_ENABLED` | bool | Pi ASR を使用するか | `false` |
| `ASR_CLOUD_TIMEOUT_MS` | u64 | 段 1 タイムアウト（ms） | `5000` |
| `ASR_LOCAL_TIMEOUT_MS` | u64 | 段 2 タイムアウト（ms） | `3000` |
| `ASR_RASPI_TIMEOUT_MS` | u64 | 段 3 タイムアウト（ms） | `8000` |

> **注意:**
> - `USE_AWS_TRANSCRIBE` は現行の名称・動作を維持する（廃止しない）。
> - `ASR_LOCAL_SERVER_URL` のデフォルト値 `http://localhost:9000/transcribe` は現行 hardcode 相当であり、**互換維持**（既存環境は URL 未設定でも動作継続）。別ホストに変更する場合のみ明示的に設定する。
> - **URL は該当段の `*_ENABLED=true` のときのみ使用する。** `*_ENABLED=false` の段は URL 未設定でも起動エラーにならない。
> - 全段 `*_ENABLED=false`（有効段がゼロ）の場合は起動時に警告ログを出し、ASR 不可として扱う（謝罪フォールバックへ直行）。
> - 段 2・段 3 とも別ホストに変更する場合はポート 9000 を使用する。
> - 既存の `AI_HTTP_TIMEOUT_MS` は LLM/TTS で引き続き使用するが、ASR は上記個別値を使う。

### 5.3 フォールバックロジック（mod.rs L92 周辺の置き換え）

現行の ASR 呼び出し分岐を、次の順序で試行するループに置き換える。

```text
// transcribe_and_log(call_id: &str, wav_path: &str) -> anyhow::Result<String>
for 各有効な ASR 段（cloud, local, raspi の順）:
    当該段のタイムアウト付きで HTTP/SDK を呼び出す
    成功（テキスト返却）かつ幻聴フィルタ通過:
        ログ出力「ASR 成功: 段={cloud|local|raspi}, call_id=...」
        return Ok(text)
    失敗（エラー or タイムアウト or 空文字 or 幻聴フィルタ引っかかり）:
        ログ出力「ASR 失敗: 段=..., 理由=..., 次段へ」
        次段へ続行
全段失敗:
    return Err(anyhow::anyhow!("all ASR stages failed"))
    // 呼び出し元（AiPort 実装）で AsrError::TranscriptionFailed にマップする
```

**フォールバック条件（確定）:**
- HTTP エラー / タイムアウト → 次段へ
- 空文字返却 → 次段へ（無音または認識不能とみなす）
- 幻聴フィルタ: **各段の返却テキストに対して実行する**（Q2 確定）。フィルタに引っかかった場合は次段へ進む。全段失敗後に謝罪へ落ちる現行動作を維持する。

### 5.4 ログ要件（確定）

以下を構造化ログとして必ず記録する。

| イベント | ログレベル | 必須フィールド |
|---------|-----------|--------------|
| 各段で ASR 試行開始 | DEBUG | `call_id`, `asr_stage`（cloud/local/raspi） |
| 各段で ASR 成功 | INFO | `call_id`, `asr_stage`, `text_len` |
| 各段で ASR 失敗 | WARN | `call_id`, `asr_stage`, `reason` |
| 全段失敗 | ERROR | `call_id`, `reason="all ASR stages failed"` |

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #216 | STEER-216 | 起票 |
| STEER-216 | RD-001 F-3 | 要件追加 |
| RD-001 F-3 | DD-006_ai.md §2 | 設計反映 |
| DD-006_ai.md §2 | mod.rs L92, L121 / config/mod.rs L858 | 実装 |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] 3 段フォールバック順序が要件と一致しているか
- [ ] Pi ローカル実行を HTTP サーバー方式に限定することの合意が得られているか
- [ ] 設定変数名・デフォルト値がレビュー済みか
- [ ] タイムアウト値（5000/3000/8000 ms）が妥当か
- [ ] `USE_AWS_TRANSCRIBE` は廃止せず存続することに合意しているか（Q3 解決済み）
- [ ] `AI_HTTP_TIMEOUT_MS` は LLM/TTS 専用として引き続き使用し、ASR は個別タイムアウト変数を使うことに合意しているか
- [ ] DD-006_ai.md §2 の変更範囲で実装者が迷わないか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] 実装が完了している
- [ ] コードレビューを受けている（CodeRabbit）
- [ ] 全段フォールバックの結合テストが PASS
- [ ] 段ごとの成功/失敗ログが出力されることを確認

---

## 8. 未確定点・質問

| # | 質問 | 選択肢 | オーナー回答 |
|---|------|--------|-------------|
| Q1 | 各段のタイムアウト値（5000/3000/8000 ms）は妥当か？ | 変更可 | **OK（変更なし）** @MasanoriSuda 2026-02-22 |
| Q2 | 幻聴フィルタを各段で実行するか？（全段終了後のみでよいか） | 各段 / 全段後のみ | **各段で実行する** @MasanoriSuda 2026-02-22 |
| Q3 | `USE_AWS_TRANSCRIBE` を廃止し `ASR_CLOUD_ENABLED` に統一してよいか？ | Yes / No | **No（廃止しない、現行名称を存続）** @MasanoriSuda 2026-02-22 |
| Q4 | Pi の段 3 URL デフォルトを `localhost:9001` にしてよいか（段 2 と衝突回避）？ | Yes / 別ポート指定 | **別ホスト・ポート 9000 で統一（段 2・段 3 ともに別ホスト）** @MasanoriSuda 2026-02-22 |
| Q5 | クラウド ASR は引き続き AWS Transcribe 固定か、他サービス（GCP Speech 等）も将来考慮するか？ | AWS 固定 / 抽象化 | **AWS Transcribe 固定** @MasanoriSuda 2026-02-22 |

---

## 9. リスク・ロールバック観点

| リスク | 影響 | 緩和策 |
|--------|------|--------|
| 全段フォールバックで遅延が累積 | 対話応答が遅くなる | 段ごとにタイムアウトを短めに設定（設定可能） |
| Pi ローカル whisper_server.py が未起動 | 段 3 がタイムアウトして遅延 | `ASR_RASPI_ENABLED=false` をデフォルトにして影響ゼロ |

**ロールバック手順:** 実装コミットを `git revert`。新規フラグを無効化する場合は `ASR_LOCAL_SERVER_ENABLED=false` / `ASR_RASPI_ENABLED=false` を設定し、クラウド ASR のみ使用するなら `USE_AWS_TRANSCRIBE=true` を追加する。

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-22 | 初版作成（Codex 調査結果を元に差分仕様を記述） | Claude Sonnet 4.6 |
| 2026-02-22 | Q3/Q4 オーナー回答反映、リスク表・ロールバック手順・§7.1 チェックリストを最新仕様に更新 | Claude Sonnet 4.6 |
| 2026-02-22 | Q1/Q2/Q5 オーナー回答反映（タイムアウト値確定、幻聴フィルタ=各段、クラウド=AWS 固定）、§5.3 幻聴フィルタ記述を確定仕様に更新 | Claude Sonnet 4.6 |
| 2026-02-22 | Codex レビュー NG 指摘対応: URL と ENABLED フラグの関係明記・AiConfig 参照先修正（config/mod.rs L858）・call_id シグネチャ追加・AsrError 記法修正・章番号重複修正 | Claude Sonnet 4.6 |
| 2026-02-22 | Codex 再指摘対応: ASR_LOCAL_SERVER_URL をデフォルト http://localhost:9000/transcribe に変更（互換維持）・ロールバック手順に新規フラグを明記 | Claude Sonnet 4.6 |
