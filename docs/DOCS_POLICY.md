# ドキュメントポリシー (DOCS_POLICY)

**ステータス**: ドラフト（レビュー待ち）
**作成日**: 2025-12-25

---

## 1. 目的

本ドキュメントは、virtual-voicebot リポジトリにおけるドキュメント管理の基本方針を定めます。

**目標**:
- 正本（Source of Truth）を明確にし、重複・矛盾を防ぐ
- ドキュメントの発見可能性を高める
- 更新漏れ・陳腐化を防ぐ

---

## 2. ドキュメント階層

```
virtual-voicebot/
├── README.md                    # リポジトリ概要（エントリポイント）
├── CONTRIBUTING.md              # 開発参加ガイド
├── STYLE.md                     # プロジェクト共通スタイル（正本）
├── PRINCIPLES.md                # 価値観・原則
├── CLAUDE.md                    # Claude Code向け指示書
├── AGENTS.md                    # 共通ルール（backendへの参照含む）
│
├── docs/
│   ├── DOCS_POLICY.md           # 本ドキュメント
│   ├── DOCS_INDEX.md            # ドキュメント一覧
│   ├── DOCS_ANALYSIS.md         # ドキュメント整理分析レポート
│   ├── contract.md              # Frontend ↔ Backend API契約
│   ├── style/
│   │   └── rust.md              # Rust固有スタイル（STYLE.mdに従属）
│   └── reviews/                 # レビュー結果保存先
│       └── YYYY-MM-DD_issue-N.md
│
└── virtual-voicebot-backend/
    ├── README.md                # Backend概要・クイックスタート
    ├── CLAUDE.md                # Claude Code向けバックエンド開発ガイド（正本）
    ├── AGENTS.md                # AI/Codex向け指示書（正本）
    │
    ├── docs/
    │   ├── design.md            # アーキテクチャ設計（正本）
    │   ├── sip.md               # SIP詳細設計（正本）
    │   ├── rtp.md               # RTP詳細設計（正本）
    │   ├── session.md           # Session詳細設計（正本）
    │   ├── ai.md                # AI連携設計（正本）
    │   ├── app.md               # App層設計（正本）
    │   ├── recording.md         # 録音設計（正本）
    │   ├── tests.md             # テスト計画
    │   ├── tests_e2e_sipp.md    # SIPp E2E手順（正本）
    │   ├── gap-analysis.md      # RFC準拠ギャップ分析・仕様（正本）
    │   └── impl/                # 実装計画
    │       └── PLAN.md          # 実装計画（バックログ統合）
    │
    ├── src/*/README.md          # モジュール概要（docs/*.mdへのリンク含む）
    └── test/README.md           # E2Eランナー使用方法
```

---

## 3. 正本ルール

### 3.1 正本の定義

**正本（Source of Truth）** とは、あるトピックについて唯一の権威ある情報源となるドキュメントです。

### 3.2 正本の識別

正本ファイルには、ファイル先頭に以下を記載します：

```markdown
<!-- SOURCE_OF_TRUTH: [トピック名] -->
```

例:
```markdown
<!-- SOURCE_OF_TRUTH: SIP詳細設計 -->
# SIP モジュール詳細設計 (`src/sip`)
```

### 3.3 正本一覧

| トピック | 正本ファイル | 補助ファイル |
|---------|-------------|-------------|
| アーキテクチャ | `docs/design.md` | - |
| API契約 | `docs/contract.md` | - |
| SIP設計 | `docs/sip.md` | `src/sip/README.md` |
| RTP設計 | `docs/rtp.md` | `src/rtp/README.md` |
| Session設計 | `docs/session.md` | `src/session/README.md` |
| AI連携設計 | `docs/ai.md` | `src/ai/README.md`, `docs/voice_bot_flow.md`（補助） |
| App層設計 | `docs/app.md` | `src/app/README.md` |
| 録音設計 | `docs/recording.md` | - |
| テスト計画/AC | `docs/tests.md` | `docs/tests_e2e_sipp.md` |
| RFC準拠/仕様 | `docs/gap-analysis.md` | - |
| SIPp E2E | `docs/tests_e2e_sipp.md` | `test/README.md` |
| AI/Codex指示 | `AGENTS.md` | - |
| Claude Code共通 | `CLAUDE.md`（root） | - |
| Claude Code Backend | `virtual-voicebot-backend/CLAUDE.md` | - |
| スタイル | `STYLE.md` | `docs/style/*.md` |

> **2025-12-27 追加**: ai.md, app.md, tests.md を正本に追加（Refs Issue #7 CX-3, CX-4）

### 3.4 矛盾時の優先順位

矛盾がある場合は以下の順で優先：

1. **正本ファイル** > 補助ファイル
2. **docs/*.md** > src/*/README.md
3. **STYLE.md** > docs/style/*.md
4. **design.md** > 個別モジュール設計
5. **I/F/AC（詳細仕様）** → `app.md/ai.md/tests.md` が `design.md` より優先

> **2025-12-28 追加**: I/F 詳細・AC は各正本が design.md より優先（Refs Issue #7）

---

## 4. ファイル命名規則

### 4.1 README.md

- 各ディレクトリの入口として配置
- 内容: 概要、クイックスタート、詳細ドキュメントへのリンク
- **Readme.md** は使用しない（大文字統一）

### 4.2 docs/ 配下

| パターン | 用途 | 例 |
|---------|------|-----|
| `{topic}.md` | 設計・仕様ドキュメント | `sip.md`, `rtp.md` |
| `tests_{type}.md` | テスト関連 | `tests_e2e_sipp.md` |
| `gap-analysis.md` | RFC準拠ギャップ分析・仕様 | - |

---

## 5. 更新ルール

### 5.1 コードと同時更新

仕様・責務・フローが変わる修正では、**先にドキュメントを更新**してからコードを変更する。

参照: `AGENTS.md §8 変更手順`

### 5.2 陳腐化防止

- `src/*/README.md` は詳細を持たず、`docs/*.md` へのリンクを中心とする
- 実装と乖離したドキュメントを発見した場合は、Issueを作成して追跡

---

## 6. アーカイブポリシー

### 6.1 アーカイブ対象

- 旧バージョンの設計ドキュメント
- 開発日誌（advenc_calendar/）

### 6.2 アーカイブ方法

**選択肢（要決定）**:
- A) `docs/archive/` ディレクトリに移動
- B) git履歴で管理し、ファイル自体は削除
- C) 現状維持

---

## 7. レビューチェックリスト

ドキュメント変更時は以下を確認：

- [ ] 正本ファイルを更新したか（補助ファイルだけの更新は避ける）
- [ ] 重複箇所がないか確認したか
- [ ] リンク切れがないか確認したか
- [ ] DOCS_INDEX.md に反映が必要か確認したか

---

*本ドキュメントは定期的にレビューし、運用に合わせて更新します。*
