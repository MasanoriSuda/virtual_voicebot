# ドキュメントポリシー (DOCS_POLICY)

**ステータス**: Active
**作成日**: 2025-12-25
**最終更新**: 2026-02-07

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
├── README.md                           # リポジトリ概要（エントリポイント）
├── CLAUDE.md                           # Claude Code 向け指示書
│
├── docs/                               # システム横断ドキュメント
│   ├── DOCS_POLICY.md                  # 本ドキュメント
│   ├── DOCS_INDEX.md                   # ドキュメント一覧
│   ├── contract.md                     # Frontend ↔ Backend API 契約（正本）
│   ├── requirements/                   # システム横断要件
│   │   └── RD-004_call-routing.md      # 電話番号振り分け・迷惑電話対策
│   ├── design/                         # システム横断設計
│   │   └── index.md
│   ├── steering/                       # システム横断ステアリング（Frontend-Backend 連携）
│   │   ├── STEER-112_sot-reconstruction.md
│   │   ├── STEER-113_docs-consolidation.md
│   │   ├── STEER-137_backend-integration-strategy.md
│   │   └── STEER-139_frontend-backend-sync-impl.md
│   ├── style/
│   │   └── rust.md                     # Rust 固有スタイル
│   └── reviews/                        # レビュー結果保存先
│       └── YYYY-MM-DD_issue-N.md
│
├── virtual-voicebot-backend/
│   ├── README.md                       # Backend 概要
│   ├── CLAUDE.md                       # Claude Code Backend ガイド（正本）
│   ├── AGENTS.md                       # AI/Codex 向け指示書（正本）
│   └── docs/
│       ├── process/                    # プロセス定義
│       │   ├── v-model.md              # V 字モデル定義（正本）
│       │   ├── quality-gate.md         # 品質ゲート定義（正本）
│       │   └── traceability.md         # トレーサビリティマトリクス（正本）
│       ├── requirements/               # 要件仕様
│       │   ├── RD-001_product.md       # プロダクト要求仕様（正本）
│       │   ├── RD-002_mvp.md           # MVP 定義（正本）
│       │   └── RD-003_flow.md          # ボイスボットフロー（正本）
│       ├── design/
│       │   ├── basic/                  # 基本設計
│       │   │   ├── BD-001_architecture.md
│       │   │   ├── BD-002_app-layer.md
│       │   │   ├── BD-003_clean-architecture.md
│       │   │   └── BD-004_call-routing-db.md
│       │   └── detail/                 # 詳細設計
│       │       ├── DD-001_tech-stack.md
│       │       ├── DD-002_modules.md
│       │       ├── DD-003_sip.md
│       │       ├── DD-004_rtp.md
│       │       ├── DD-005_session.md
│       │       ├── DD-006_ai.md
│       │       └── DD-007_recording.md
│       ├── test/                       # テスト仕様
│       │   ├── plan.md                 # テスト計画書（正本）
│       │   └── system/
│       │       ├── ST-001_acceptance.md
│       │       └── ST-002_e2e-sipp.md
│       ├── steering/                   # ステアリング（差分仕様）
│       │   ├── index.md
│       │   └── STEER-{N}_{slug}.md
│       └── archive/                    # 旧体系アーカイブ
│
└── virtual-voicebot-frontend/
    ├── README.md                       # Frontend 概要
    └── docs/
        ├── process/
        │   ├── v-model.md              # Frontend V 字モデル（正本）
        │   └── quality-gate.md         # Frontend 品質ゲート（正本）
        ├── requirements/
        │   └── RD-005_frontend.md      # 管理画面 MVP 要件（正本）
        ├── design/
        │   └── basic/
        │       └── BD-001_frontend.md  # 管理画面基本設計（正本）
        ├── test/
        │   └── plan.md                 # テスト計画書（正本）
        └── steering/
            └── index.md
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

#### システム横断

| トピック | 正本ファイル |
|---------|-------------|
| API 契約 | `docs/contract.md` |
| 電話番号振り分け要件 | `docs/requirements/RD-004_call-routing.md` |
| Claude Code 共通指示 | `CLAUDE.md`（root） |

#### Backend

| トピック | 正本ファイル | ID |
|---------|-------------|-----|
| プロセス定義 | `virtual-voicebot-backend/docs/process/v-model.md` | - |
| 品質ゲート | `virtual-voicebot-backend/docs/process/quality-gate.md` | - |
| トレーサビリティ | `virtual-voicebot-backend/docs/process/traceability.md` | - |
| プロダクト要求仕様 | `virtual-voicebot-backend/docs/requirements/RD-001_product.md` | RD-001 |
| MVP 定義 | `virtual-voicebot-backend/docs/requirements/RD-002_mvp.md` | RD-002 |
| ボイスボットフロー | `virtual-voicebot-backend/docs/requirements/RD-003_flow.md` | RD-003 |
| システムアーキテクチャ | `virtual-voicebot-backend/docs/design/basic/BD-001_architecture.md` | BD-001 |
| App 層設計 | `virtual-voicebot-backend/docs/design/basic/BD-002_app-layer.md` | BD-002 |
| クリーンアーキテクチャ | `virtual-voicebot-backend/docs/design/basic/BD-003_clean-architecture.md` | BD-003 |
| 着信ルーティング DB | `virtual-voicebot-backend/docs/design/basic/BD-004_call-routing-db.md` | BD-004 |
| 技術スタック | `virtual-voicebot-backend/docs/design/detail/DD-001_tech-stack.md` | DD-001 |
| モジュール設計 | `virtual-voicebot-backend/docs/design/detail/DD-002_modules.md` | DD-002 |
| SIP モジュール | `virtual-voicebot-backend/docs/design/detail/DD-003_sip.md` | DD-003 |
| RTP モジュール | `virtual-voicebot-backend/docs/design/detail/DD-004_rtp.md` | DD-004 |
| Session モジュール | `virtual-voicebot-backend/docs/design/detail/DD-005_session.md` | DD-005 |
| AI 連携 | `virtual-voicebot-backend/docs/design/detail/DD-006_ai.md` | DD-006 |
| 録音 | `virtual-voicebot-backend/docs/design/detail/DD-007_recording.md` | DD-007 |
| テスト計画 | `virtual-voicebot-backend/docs/test/plan.md` | - |
| 受入テスト | `virtual-voicebot-backend/docs/test/system/ST-001_acceptance.md` | ST-001 |
| SIPp E2E テスト | `virtual-voicebot-backend/docs/test/system/ST-002_e2e-sipp.md` | ST-002 |
| Claude Code Backend | `virtual-voicebot-backend/CLAUDE.md` | - |
| AI/Codex 指示 | `virtual-voicebot-backend/AGENTS.md` | - |

#### Frontend

| トピック | 正本ファイル | ID |
|---------|-------------|-----|
| プロセス定義 | `virtual-voicebot-frontend/docs/process/v-model.md` | - |
| 品質ゲート | `virtual-voicebot-frontend/docs/process/quality-gate.md` | - |
| 管理画面 MVP 要件 | `virtual-voicebot-frontend/docs/requirements/RD-005_frontend.md` | RD-005 |
| 管理画面基本設計 | `virtual-voicebot-frontend/docs/design/basic/BD-001_frontend.md` | BD-001 |
| テスト計画 | `virtual-voicebot-frontend/docs/test/plan.md` | - |

> **2026-02-07 更新**: 旧体系（design.md, sip.md 等のフラットファイル）から V 字モデル体系に全面移行（Refs Issue #112）

### 3.4 矛盾時の優先順位

矛盾がある場合は以下の順で優先：

1. **正本ファイル** > 補助ファイル（src/*/README.md 等）
2. **DD（詳細設計）** > **BD（基本設計）** > **RD（要件）**（I/F・境界条件は DD が優先）
3. **docs/contract.md** が Frontend/Backend 間の API 仕様に関する唯一の正本
4. **process/*.md**（プロセス定義）が運用ルールに関する正本

---

## 4. ファイル命名規則

### 4.1 README.md

- 各ディレクトリの入口として配置
- 内容: 概要、クイックスタート、詳細ドキュメントへのリンク
- **Readme.md** は使用しない（大文字統一）

### 4.2 docs/ 配下（V 字モデル体系）

| パターン | 用途 | 例 |
|---------|------|-----|
| `requirements/RD-{NNN}_{slug}.md` | 要件仕様 | `RD-001_product.md` |
| `design/basic/BD-{NNN}_{slug}.md` | 基本設計 | `BD-001_architecture.md` |
| `design/detail/DD-{NNN}_{slug}.md` | 詳細設計 | `DD-003_sip.md` |
| `test/plan.md` | テスト計画書 | - |
| `test/system/ST-{NNN}_{slug}.md` | システムテスト | `ST-001_acceptance.md` |
| `steering/STEER-{issue}_{slug}.md` | ステアリング（差分仕様） | `STEER-110_backend-db-design.md` |
| `process/*.md` | プロセス定義 | `v-model.md`, `quality-gate.md` |

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

- 旧体系の設計ドキュメント（design.md, sip.md, rtp.md 等 → V 字体系に移行済み）
- RFC 準拠ギャップ分析（gap-analysis.md）
- 旧実装計画（impl/PLAN.md）

### 6.2 アーカイブ方法

- `virtual-voicebot-backend/docs/archive/` ディレクトリに移動（2026-02-07 決定）
- 旧ファイルの移行先は [DOCS_INDEX.md §5](DOCS_INDEX.md) に記録

---

## 7. レビューチェックリスト

ドキュメント変更時は以下を確認：

- [ ] 正本ファイルを更新したか（補助ファイルだけの更新は避ける）
- [ ] 重複箇所がないか確認したか
- [ ] リンク切れがないか確認したか
- [ ] DOCS_INDEX.md に反映が必要か確認したか

---

*本ドキュメントは定期的にレビューし、運用に合わせて更新します。*
