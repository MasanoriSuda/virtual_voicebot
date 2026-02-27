# STEER-210: ラズベリーパイ向け Ollama + llama モデル設定追加

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-210 |
| タイトル | ラズベリーパイ向け Ollama + llama モデル設定追加 |
| ステータス | Approved |
| 関連Issue | #210（親: #208） |
| 優先度 | P1 |
| 作成日 | 2026-02-21 |

---

## 2. ストーリー（Why）

### 2.1 背景

現行の LLM 呼び出しは Gemini API（クラウド）→ Ollama（ローカル、デフォルトモデル: `gemma3:4b`）の
2段フォールバック構成である（フォールバック: [`service/ai/mod.rs` L159](../../../src/service/ai/mod.rs) / デフォルトモデル: [`shared/config/mod.rs` L904](../../../src/shared/config/mod.rs)）。

ラズベリーパイ（#208）での動作を目指す場合、以下の状況がある。

- Gemini: クラウド依存・API キー必須のため、`GEMINI_API_KEY` 未設定時は `call_gemini` が Err を返し、error ログ出力後に Ollama へフォールバックする
- Ollama デフォルトモデル `gemma3:4b` はラズパイの RAM/CPU 制約下では重い可能性がある
- Ollama は `OLLAMA_MODEL` 環境変数でモデルを切り替えられるため、llama 系モデルへの変更はコード変更なしで実現できる
- llama.cpp 直接接続（OpenAI 互換）は Phase 2 以降で検討する

### 2.2 目的

既存コードを変更せず、環境変数 `OLLAMA_MODEL` に llama 系モデルを指定することで、
Raspberry Pi 上でより軽量な LLM をローカル実行できる構成をドキュメント化する。

### 2.3 ユーザーストーリー

```
As a ラズベリーパイで VoiceBot を運用したい開発者
I want to 推奨環境変数の設定だけで llama3.2 モデルに切り替えたい
So that コードビルドなしに、ラズパイ向けに最適化された LLM 構成で動作させられる

受入条件:
- [ ] OLLAMA_MODEL=llama3.2:1b を設定するだけで llama3.2 モデルが使われる（コード変更不要）
- [ ] GEMINI_API_KEY が未設定のとき、自動的に Ollama（= llama3.2:1b）が呼ばれる
- [ ] Gemini → Ollama フォールバックの挙動は現行のまま維持される
- [ ] ラズパイ向け推奨設定値が DD-006_ai.md に記載されている
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-21 |
| 起票理由 | Issue #210 参照 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-sonnet-4-6) |
| 作成日 | 2026-02-21 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "ラズベリーパイ向けにLLMにllamaを追加する処理を追加したい（OQ決定：B案・コード変更なし）" |

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

**コード変更不要。** 本ステアリングの成果物はドキュメント追記のみ。

| 項目 | 値 |
|------|-----|
| 実装者 | - （ドキュメント追記は Claude Code が実施） |
| 対象 | DD-006_ai.md、RD-002_mvp.md |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | - |
| マージ日 | - |
| マージ先 | `docs/design/detail/DD-006_ai.md`, `docs/requirements/RD-002_mvp.md` |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| `docs/design/detail/DD-006_ai.md` | 修正（追記） | §3 LLM 節に Raspberry Pi 向け推奨 `OLLAMA_MODEL` 値を追記 |
| `docs/requirements/RD-002_mvp.md` | 修正（追記） | 環境変数一覧テーブルに `OLLAMA_MODEL` の補足（ラズパイ推奨値）を追記 |

### 4.2 影響するコード

**なし。** 既存の `OLLAMA_MODEL` 読み取りコードは変更不要。

---

## 5. 差分仕様（What / How）

### 5.1 要件追記（RD-002_mvp.md の設定欄へマージ）

```markdown
## 環境変数: OLLAMA_MODEL（Raspberry Pi 向け推奨値）

現行デフォルト: `gemma3:4b`

Raspberry Pi での運用時は、RAM/CPU 制約に応じて以下を推奨する。

| モデル | 推奨シーン | 備考 |
|--------|----------|------|
| `llama3.2:1b` | **ラズパイ標準構成（推奨）** | 安定動作・応答速度重視 |
| `llama3.2:3b` | 品質重視（RAM 4GB 以上） | `llama3.2:1b` より応答品質が高い |
| `gemma3:4b`   | デスクトップ/サーバー（現行デフォルト） | ラズパイでは重い可能性あり |

設定例（Raspberry Pi 標準構成）:
OLLAMA_MODEL=llama3.2:1b
# GEMINI_API_KEY は未設定（call_gemini が失敗し error ログ出力後、Ollama へフォールバック）
```

---

### 5.2 詳細設計追記（DD-006_ai.md §3 LLM 節へマージ）

```markdown
### Raspberry Pi 向け設定（OLLAMA_MODEL 切り替え）

コード変更なしで llama 系モデルへ切り替えられる。
`GEMINI_API_KEY` を未設定にすると `call_gemini` が Err を返し、error ログ出力後に Ollama へフォールバックする（毎回 error ログが出る点に注意）。

| 環境変数 | ラズパイ推奨値 | デフォルト |
|---------|-------------|----------|
| `OLLAMA_MODEL` | `llama3.2:1b` | `gemma3:4b` |
| `GEMINI_API_KEY` | （未設定） | （任意） |
| `OLLAMA_INTENT_MODEL` | `llama3.2:1b` | `OLLAMA_MODEL` と同値 |

LLM フロー（ラズパイ運用時の実際の経路）:
```
call_gemini → Err（API キーなし）
  → call_ollama（OLLAMA_MODEL=llama3.2:1b）→ Ok(answer)
```

注意事項:
- Ollama サーバーがラズパイ上で起動していること（`http://localhost:11434`）
- `llama3.2:1b` モデルを事前に `ollama pull llama3.2:1b` でダウンロードしておくこと
- モデルのダウンロードには十分なストレージが必要（1B モデルで約 1.3 GB）

### Phase 2 予定（本ステアリング対象外）

llama.cpp 直接接続（OpenAI 互換 `/v1/chat/completions`）は別イシューで対応する。
Phase 2 では `LLM_PROVIDER` 環境変数と `call_llama_cpp` クライアントを追加する想定。
```

---

### 5.3 テストケース追加

**なし。** コード変更がないため新規テストは不要。
既存の Ollama フォールバックテスト（存在する場合）に `OLLAMA_MODEL` の切り替えが
既存テストを壊さないことを確認する（動作確認レベル）。

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #210 | STEER-210 | 起票 |
| Issue #208 | Issue #210 | 親イシュー（Raspberry Pi 動作） |
| STEER-210 | RD-002_mvp.md §設定 | 環境変数補足追記 |
| STEER-210 | DD-006_ai.md §3 LLM | Raspberry Pi 向け設定補足追記 |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] 推奨モデル（`llama3.2:1b` / `llama3.2:3b`）の選択根拠が明確か
- [ ] 既存フォールバック動作（Gemini → Ollama）との整合性があるか
- [ ] ドキュメント追記箇所（DD-006、RD-002）が適切か
- [ ] Phase 2 の llama.cpp 対応範囲が明確に「対象外」とされているか
- [ ] Open Questions が全て解消されているか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] DD-006_ai.md への追記が完了している
- [ ] RD-002_mvp.md への追記が完了している
- [ ] 既存テストが PASS（コード変更なしなので破壊がないことの確認）

---

## 8. 決定済み事項（旧 Open Questions）

| # | 質問 | 決定 | 根拠 |
|---|------|------|------|
| OQ-1 | llama.cpp 新規クライアントか、`OLLAMA_MODEL` 変更のみか | **(B) `OLLAMA_MODEL` 変更のみ**（コード変更なし） | 既存 `OLLAMA_MODEL` 読み取りコードで対応可能（Phase 2 で llama.cpp を追加） |
| OQ-2 | 推奨モデルのサイズ/名称は？ | **`llama3.2:1b`（デフォルト推奨）、`llama3.2:3b`（品質重視）** | ラズパイで安定しやすいのは 1B。TinyLlama は品質が落ちやすいため非推奨 |
| OQ-3 | Gemini → Ollama フォールバックは維持するか？ | **維持**。`GEMINI_API_KEY` 未設定時は `call_gemini` が失敗してから Ollama へフォールバック（error ログあり） | 互換性維持のため。llama.cpp 固定は Phase 2 で別対応 |

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-21 | 初版作成 | Claude Code (claude-sonnet-4-6) |
| 2026-02-21 | OQ 全解消を受け全面改訂（B案・コード変更なし方針へ）| Claude Code (claude-sonnet-4-6) |
| 2026-02-21 | レビュー指摘修正（call_gemini フォールバック表現・根拠リンク行番号）| Claude Code (claude-sonnet-4-6) |
