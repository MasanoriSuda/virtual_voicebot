# AGENTS.md (repo root)

## 適用範囲

本ファイルはリポジトリ全体に適用される**共通ルール**を定義します。

- **Backend 固有の詳細**（依存方向、並行処理、テスト方針等）は [virtual-voicebot-backend/AGENTS.md](virtual-voicebot-backend/AGENTS.md) を参照してください。
- **Claude Code の責務**（ドキュメント/仕様担当）は [CLAUDE.md](CLAUDE.md) を参照してください。

---

## 0. 合意（チケット）ゲート（必須）

- GitHub Issue の参照（例：`Refs #123`）が提示されない限り、実装・改修を一切しない。
- 合意が無い場合はコードもdocsも変更せず、次の文面のみ返す：
  「Issue参照がないため作業できません。先にIssue化して `Refs #123` を提示してください。」

---

## ドキュメント更新の扱い（必須）

- 仕様/責務/フロー変更を伴う場合は、原則 Claude Code が docs を更新する。

- Codex は docs（docs/**, *.md）を **原則編集禁止**。
  docs更新が必要な場合は、編集せずに
  1) 変更案（diffまたは箇条書き）
  2) Yes/Noで答えられる質問
  を提示して停止し、Claude（またはオーナー）に引き継ぐ。

- **例外（オーナー許可）**：
  オーナーが GitHub Issue/PR またはプロンプトで明示的に許可した場合のみ、Codexは指定されたファイルに限り docs を編集してよい。
  - 許可形式（推奨）：`DOCS_OK: <file paths>`
  - 例：`DOCS_OK: docs/contract.md docs/recording.md`

- **許可不要な軽微修正（意味が変わらないもののみ）**：
  タイポ修正、リンク切れ修正のみ、最小差分で可。
  ※日付更新や表記ゆれ統一は「意味・運用解釈に影響し得る」ため、DOCS_OK が必要。

---

## レビュー任務（共通）

### 担当分担

| 担当 | レビュー観点 | 詳細定義 |
|------|-------------|---------|
| **Claude Code** | 仕様/ドキュメント整合性 | [CLAUDE.md](CLAUDE.md) |
| **Codex** | 実装/テスト観点 | [virtual-voicebot-backend/AGENTS.md](virtual-voicebot-backend/AGENTS.md) |

### レビュー出力フォーマット（必須）

- 指摘（重大/中/軽）に分類し、各項目に「根拠（該当ファイル/行/章）」を添える
- 修正案がある場合は「最小差分」を提案する

---

## レビュー結果の記録（必須）

- レビュー結果は必ず PR 本文に要約（重大/中/軽 + 根拠）として残す。
- レビュー全文は `docs/reviews/YYYY-MM-DD_issue-<n>.md` に保存する。
- 再発する指摘（2回以上）はルールに昇格する：`AGENTS.md` / `CONVENTIONS.md` へ追記する。
