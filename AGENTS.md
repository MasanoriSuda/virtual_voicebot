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

## ドキュメント編集の原則（必須）

### ステアリング配置ルール

**正規配置（3箇所のみ）:**
- **横断（Frontend-Backend 連携）**: `docs/steering/STEER-*.md`
- **Backend 専用**: `virtual-voicebot-backend/docs/steering/STEER-*.md`
- **Frontend 専用**: `virtual-voicebot-frontend/docs/steering/STEER-*.md`

**禁止:**
- `docs/STEER-*.md`（root 直下）への配置

### 差分最小の原則

docs 修正は原則、**全文再生成ではなく最小差分（patch）で行うこと**。

**推奨手法:**
- Edit tool 使用（old_string → new_string）
- 変更箇所のみ明示的に指摘
- 全文コピペは禁止

**例外:**
- 新規ファイル作成時（Draft 作成等）
- 大規模リファクタリング（DOCS_OK で明示的に許可された場合）

**理由:**
- SoT（Single Source of Truth）の意図しない破壊を防ぐ
- レビュー負荷を最小化する
- 変更履歴を明確にする

---

## ドキュメント更新の扱い（必須）

### 原則
- 仕様/責務/フロー変更を伴う場合は、原則 Claude Code が docs を更新する。
- Codex は docs（docs/**, *.md）を **原則編集禁止**。

### 例外1: ステアリングファイル（Review以降の修正のみ）

**Codex は以下の条件を満たす場合、既存ステアリングファイルを修正可能：**

1. **対象ファイル**: 以下のいずれか（**既存のみ**）
   - `docs/steering/STEER-*.md`（横断）
   - `virtual-voicebot-backend/docs/steering/STEER-*.md`（Backend）
   - `virtual-voicebot-frontend/docs/steering/STEER-*.md`（Frontend）
2. **ステータス条件**: `Status: Review` または `Status: Approved`
3. **イシュー番号**: メタ情報に関連Issueが記載されていること
4. **修正範囲**: 以下のセクションのみ
   - §3.3 レビュー（レビュー記録の追記）
   - §3.5 実装（実装段取りの更新）
   - §5 差分仕様（レビュー指摘に基づく修正）
   - §6 受入条件（レビュー指摘に基づく修正）
   - §7 未確定点/質問（質問の解消）
5. **レビュー要件**: 修正内容は Claude Code またはオーナーのレビューを受けること（運用例外: Claude Code 使用量不足時は Codex セルフレビュー可）
6. **差分最小**: 全文再生成ではなく、最小差分で修正すること

**禁止事項:**
- **Codex による新規ステアリングファイルの作成**（Draft 作成は Claude Code の責務）
- Status: Draft のステアリングを修正すること
- **ストーリー（§2）の変更**（Why の変更は Issue で合意すべき）
- Status の勝手な変更（Status 更新は人間が判断）
- 本体仕様書（RD/BD/DD等）の無許可修正
- **`docs/STEER-*.md`（root 直下）への配置**（正規3箇所のみ使用）

### 例外2: オーナー許可（DOCS_OK）

**オーナーが明示的に許可した場合のみ、Codex は以下を修正可能：**

- 本体仕様書（docs/requirements/**, docs/design/**, docs/test/**）
- README/CONTRIBUTING/規約ドキュメント（*.md）
- その他の docs ファイル

**許可形式（必須）:**
```
DOCS_OK: <file paths>
```
- **パス列挙必須**（`DOCS_OK: true` のような全許可は不可）
- 複数ファイルはスペース区切り
- 例：`DOCS_OK: docs/design/basic/BD-003.md README.md`

**オプション: スコープ指定**
```
DOCS_OK_SCOPE: #<issue number>
```
- 指定した Issue/PR のスコープ内のみ有効
- 例：`DOCS_OK_SCOPE: #153`

**許可の記載場所:**
- GitHub Issue 本文
- GitHub PR 本文
- プロンプト（直接指示）

**許可の有効範囲:**
- 指定されたファイルのみ
- 指定された Issue/PR のスコープ内のみ

### 例外3: 軽微修正（許可不要）

**以下は DOCS_OK なしで修正可能（意味が変わらない編集のみ）:**
- タイポ修正
- リンク切れ修正
- Markdown 整形（改行、インデント、コードブロック）
- 見出し番号ズレ修正
- 箇条書き整形
- 表のカラム幅調整・整形
- 最終更新日の自動更新

**DOCS_OK が必要な変更:**
- 要件/責務/フロー/手順/権限/テスト条件の変更
- 作成日/承認日/レビュー日の変更（手順や運用の解釈が変わる）
- 表記ゆれ統一（用語定義に関わる場合）

**判断基準:**
「この変更で運用・解釈・判断が変わるか？」→ Yes なら DOCS_OK 必要

### その他の docs 更新が必要な場合

上記例外に該当しない docs 更新が必要な場合は、編集せずに
1) 変更案（diffまたは箇条書き）
2) Yes/Noで答えられる質問
を提示して停止し、Claude Code（またはオーナー）に引き継ぐ。

### 想定外変更検知時の運用（停止条件の明確化）

- 作業開始時に `git status --short` を確認する。
- 想定外変更を検知した場合の扱い:
  - 担当外領域（例: Codex作業中の `docs/**`）は、該当ファイルを編集しない前提で「警告のみ」で継続可。
  - 担当領域（今回編集予定のファイル群）に想定外変更がある場合は停止し、ユーザー確認を行う。
- 停止時は、検知ファイルパスと停止理由を1〜2行で明示する。

**注意:**
- `docs/steering/STEER-*.md` が未追跡ファイルとして検知された場合、Status を確認すること。
- `Status: Draft` のステアリングは編集禁止（§1.3.1 参照）。調査・報告のみ可能。
- `Status: Review` または `Status: Approved` のステアリングのみ修正可能。

---

## レビュー運用（必須）

### ステアリング Review 時のレビュー手順

1. **Codex が修正**: ステアリング §5 等をレビュー指摘に基づき修正
2. **PR 作成**: 修正内容を PR にして以下をチェック
3. **レビュー依頼**: Claude Code またはオーナーに依頼（運用例外: Claude Code 使用量不足時は Codex がレビュー実施）

**レビュー要件（共通）:**
- 単なる形式チェックではなく、[CLAUDE.md](CLAUDE.md) に定義された観点から実質的レビューを行う
- 指摘がある場合は、重大/中/軽に分類して根拠（該当 docs 章/行）とともに提示
- 「オーナーが承認済み」であっても、仕様観点から独立したレビューを実施
- レビュー結果は PR コメントまたは `docs/reviews/` に記録
- 運用例外で Codex がレビューする場合は、`Codexレビュー→指摘→Codex修正` の流れを許可する

**レビュー判定と続行条件（必須）:**
- レビュー結果は `OK` / `NG` を明示する。
- `OK` の場合、Codex は当該作業を続行してよい。
- `NG` の場合、オーナーの修正方針承認を得るまで Codex は続行しない。
- オーナー承認後、Codex は承認内容に沿って最小差分で修正する。

**PR テンプレート要件（推奨）:**

以下のチェックリストを PR に含めることを推奨：

```markdown
## Review Checklist (for STEER edits)

- [ ] Claude Code / Owner reviewed（運用例外: Claude 使用量不足時は Codex self-review 可）
- [ ] DOCS_OK included (required for SoT docs edits)
- [ ] ストーリー（§2）を変更していない
- [ ] 差分は最小限（全文再生成していない）
```

**CODEOWNERS 活用（推奨）:**

```
# ステアリングファイルは Claude Code またはオーナーの承認必須
docs/steering/** @owner-username
virtual-voicebot-backend/docs/steering/** @owner-username
virtual-voicebot-frontend/docs/steering/** @owner-username
```

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
