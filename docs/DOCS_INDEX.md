# ドキュメント一覧 (DOCS_INDEX)

**ステータス**: Active
**最終更新**: 2026-02-07

> 本ファイルはリポジトリ内の全ドキュメントへの入口である。
> 旧体系（design.md, sip.md 等のフラットファイル）は 2026-02-07 に廃止し、
> V 字モデル体系（RD/BD/DD/process/steering）に一本化した。

---

## クイックリンク

| 目的 | ドキュメント |
|------|-------------|
| プロジェクト概要 | [README.md](../README.md) |
| AI エージェント指示 | [CLAUDE.md](../CLAUDE.md) |
| API 契約 | [contract.md](contract.md) |
| ドキュメント管理ポリシー | [DOCS_POLICY.md](DOCS_POLICY.md) |

---

## 1. リポジトリルート

| ファイル | 内容 | 正本 |
|---------|------|:----:|
| [README.md](../README.md) | リポジトリ概要、サブプロジェクト構成 | - |
| [CLAUDE.md](../CLAUDE.md) | AI/Claude Code 向け共通ガイド | ✓ |

---

## 2. docs/（システム横断ドキュメント）

| ファイル | 内容 | 正本 |
|---------|------|:----:|
| [DOCS_POLICY.md](DOCS_POLICY.md) | ドキュメント管理ポリシー | ✓ |
| [DOCS_INDEX.md](DOCS_INDEX.md) | 本ファイル（ドキュメント一覧） | - |
| [contract.md](contract.md) | Frontend ↔ Backend API 契約 | ✓ |
| [style/rust.md](style/rust.md) | Rust 固有スタイル | - |

### 2.1 要件仕様（docs/requirements/）

| ファイル | 内容 | 正本 |
|---------|------|:----:|
| [index.md](requirements/index.md) | 要件仕様一覧 | - |
| [RD-004_call-routing.md](requirements/RD-004_call-routing.md) | 電話番号振り分け・迷惑電話対策 | ✓ |

### 2.2 設計書（docs/design/）

| ファイル | 内容 | 正本 |
|---------|------|:----:|
| [index.md](design/index.md) | 設計書一覧 | - |

### 2.3 ステアリング（docs/steering/）

| ファイル | 内容 | ステータス |
|---------|------|----------|
| [STEER-099_frontend-mvp.md](steering/STEER-099_frontend-mvp.md) | Frontend MVP 仕様 | - |
| [STEER-112_sot-reconstruction.md](steering/STEER-112_sot-reconstruction.md) | SoT 再構築 | Approved |
| [STEER-113_docs-consolidation.md](steering/STEER-113_docs-consolidation.md) | 開発ガイドライン統合 | Draft |

### 2.4 レビュー（docs/reviews/）

| ファイル | 内容 |
|---------|------|
| [2025-12-27_issue-7.md](reviews/2025-12-27_issue-7.md) | Issue #7 レビュー |
| [2025-12-27_issue-7_codex.md](reviews/2025-12-27_issue-7_codex.md) | Issue #7 Codex レビュー |
| [2025-12-30_issue-8.md](reviews/2025-12-30_issue-8.md) | Issue #8 レビュー |

---

## 3. Backend（virtual-voicebot-backend/）

### 3.1 ルートドキュメント

| ファイル | 内容 | 正本 |
|---------|------|:----:|
| [README.md](../virtual-voicebot-backend/README.md) | Backend 概要 | - |
| [CLAUDE.md](../virtual-voicebot-backend/CLAUDE.md) | Claude Code Backend ガイド | ✓ |
| [AGENTS.md](../virtual-voicebot-backend/AGENTS.md) | AI/Codex 向け指示書 | ✓ |

### 3.2 プロセス定義（docs/process/）

| ファイル | 内容 | 正本 |
|---------|------|:----:|
| [v-model.md](../virtual-voicebot-backend/docs/process/v-model.md) | V 字モデル・成果物定義 | ✓ |
| [quality-gate.md](../virtual-voicebot-backend/docs/process/quality-gate.md) | 品質ゲート定義 | ✓ |
| [traceability.md](../virtual-voicebot-backend/docs/process/traceability.md) | トレーサビリティマトリクス | ✓ |

### 3.3 要件仕様（docs/requirements/）

| ファイル | 内容 | 正本 |
|---------|------|:----:|
| [index.md](../virtual-voicebot-backend/docs/requirements/index.md) | 要件仕様一覧 | - |
| [RD-001_product.md](../virtual-voicebot-backend/docs/requirements/RD-001_product.md) | プロダクト要求仕様 | ✓ |
| [RD-002_mvp.md](../virtual-voicebot-backend/docs/requirements/RD-002_mvp.md) | MVP 定義 | ✓ |
| [RD-003_flow.md](../virtual-voicebot-backend/docs/requirements/RD-003_flow.md) | ボイスボットフロー | ✓ |

### 3.4 設計書（docs/design/）

| ファイル | 内容 | 正本 |
|---------|------|:----:|
| [index.md](../virtual-voicebot-backend/docs/design/index.md) | 設計書一覧 | - |

#### 基本設計（basic/）

| ファイル | 内容 | 正本 |
|---------|------|:----:|
| [BD-001_architecture.md](../virtual-voicebot-backend/docs/design/basic/BD-001_architecture.md) | システムアーキテクチャ | ✓ |
| [BD-002_app-layer.md](../virtual-voicebot-backend/docs/design/basic/BD-002_app-layer.md) | App 層設計 | ✓ |
| [BD-003_clean-architecture.md](../virtual-voicebot-backend/docs/design/basic/BD-003_clean-architecture.md) | クリーンアーキテクチャ | ✓ |
| [BD-003_dependency-diagram.md](../virtual-voicebot-backend/docs/design/basic/BD-003_dependency-diagram.md) | 依存関係図 | - |
| [BD-004_call-routing-db.md](../virtual-voicebot-backend/docs/design/basic/BD-004_call-routing-db.md) | 着信ルーティング DB 設計 | ✓ |

#### 詳細設計（detail/）

| ファイル | 内容 | 正本 |
|---------|------|:----:|
| [DD-001_tech-stack.md](../virtual-voicebot-backend/docs/design/detail/DD-001_tech-stack.md) | 技術スタック | ✓ |
| [DD-002_modules.md](../virtual-voicebot-backend/docs/design/detail/DD-002_modules.md) | モジュール設計 | ✓ |
| [DD-003_sip.md](../virtual-voicebot-backend/docs/design/detail/DD-003_sip.md) | SIP モジュール | ✓ |
| [DD-004_rtp.md](../virtual-voicebot-backend/docs/design/detail/DD-004_rtp.md) | RTP モジュール | ✓ |
| [DD-005_session.md](../virtual-voicebot-backend/docs/design/detail/DD-005_session.md) | Session モジュール | ✓ |
| [DD-006_ai.md](../virtual-voicebot-backend/docs/design/detail/DD-006_ai.md) | AI 連携 | ✓ |
| [DD-007_recording.md](../virtual-voicebot-backend/docs/design/detail/DD-007_recording.md) | 録音 | ✓ |

### 3.5 テスト仕様（docs/test/）

| ファイル | 内容 | 正本 |
|---------|------|:----:|
| [plan.md](../virtual-voicebot-backend/docs/test/plan.md) | テスト計画書 | ✓ |
| [system/ST-001_acceptance.md](../virtual-voicebot-backend/docs/test/system/ST-001_acceptance.md) | 受入テスト | ✓ |
| [system/ST-002_e2e-sipp.md](../virtual-voicebot-backend/docs/test/system/ST-002_e2e-sipp.md) | SIPp E2E テスト | ✓ |

### 3.6 ステアリング（docs/steering/）

| ファイル | ステータス | 関連 Issue |
|---------|----------|-----------|
| [index.md](../virtual-voicebot-backend/docs/steering/index.md) | Active | - |
| [STEER-085](../virtual-voicebot-backend/docs/steering/STEER-085_clean-architecture.md) | Draft | #52, #65, #85 |
| [STEER-095](../virtual-voicebot-backend/docs/steering/STEER-095_backend-refactoring.md) | Draft | #95 |
| [STEER-096](../virtual-voicebot-backend/docs/steering/STEER-096_serversync.md) | Approved | #96 |
| [STEER-108](../virtual-voicebot-backend/docs/steering/STEER-108_sip-core-engine-refactor.md) | Draft | #108 |
| [STEER-110](../virtual-voicebot-backend/docs/steering/STEER-110_backend-db-design.md) | Approved | #110 |

### 3.7 その他

| ファイル | 内容 |
|---------|------|
| [DEVELOPMENT_GUIDE.md](../virtual-voicebot-backend/docs/DEVELOPMENT_GUIDE.md) | 開発ガイドライン |
| [archive/](../virtual-voicebot-backend/docs/archive/) | アーカイブ（gap-analysis, impl-plan） |
| [reviews/2026-02-04_issue-95.md](../virtual-voicebot-backend/docs/reviews/2026-02-04_issue-95.md) | Issue #95 レビュー |

---

## 4. Frontend（virtual-voicebot-frontend/）

### 4.1 ルートドキュメント

| ファイル | 内容 | 正本 |
|---------|------|:----:|
| [README.md](../virtual-voicebot-frontend/README.md) | Frontend 概要 | - |

### 4.2 プロセス定義（docs/process/）

| ファイル | 内容 | 正本 |
|---------|------|:----:|
| [v-model.md](../virtual-voicebot-frontend/docs/process/v-model.md) | Frontend V 字モデル | ✓ |
| [quality-gate.md](../virtual-voicebot-frontend/docs/process/quality-gate.md) | Frontend 品質ゲート | ✓ |

### 4.3 要件仕様（docs/requirements/）

| ファイル | 内容 | 正本 |
|---------|------|:----:|
| [index.md](../virtual-voicebot-frontend/docs/requirements/index.md) | 要件仕様一覧 | - |
| [RD-005_frontend.md](../virtual-voicebot-frontend/docs/requirements/RD-005_frontend.md) | 管理画面 MVP 要件 | ✓ |

### 4.4 設計書（docs/design/）

| ファイル | 内容 | 正本 |
|---------|------|:----:|
| [index.md](../virtual-voicebot-frontend/docs/design/index.md) | 設計書一覧 | - |
| [BD-001_frontend.md](../virtual-voicebot-frontend/docs/design/basic/BD-001_frontend.md) | 管理画面基本設計 | ✓ |

### 4.5 テスト仕様（docs/test/）

| ファイル | 内容 | 正本 |
|---------|------|:----:|
| [plan.md](../virtual-voicebot-frontend/docs/test/plan.md) | テスト計画書 | ✓ |

### 4.6 ステアリング（docs/steering/）

| ファイル | ステータス | 関連 Issue |
|---------|----------|-----------|
| [index.md](../virtual-voicebot-frontend/docs/steering/index.md) | Active | - |
| [STEER-107](../virtual-voicebot-frontend/docs/steering/STEER-107_frontend-docs-structure.md) | Draft | #107 |

---

## 5. 旧体系ファイル（廃止済み）

以下のファイルは V 字モデル体系移行に伴い **2026-02-07 付で廃止**。
内容は対応する RD/BD/DD に移行済み。

| 旧ファイル | 移行先 | 状態 |
|-----------|--------|------|
| `virtual-voicebot-backend/docs/design.md` | BD-001, BD-002, BD-003 | 削除済み |
| `virtual-voicebot-backend/docs/sip.md` | DD-003_sip.md | 削除済み |
| `virtual-voicebot-backend/docs/rtp.md` | DD-004_rtp.md | 削除済み |
| `virtual-voicebot-backend/docs/session.md` | DD-005_session.md | 削除済み |
| `virtual-voicebot-backend/docs/ai.md` | DD-006_ai.md | 削除済み |
| `virtual-voicebot-backend/docs/app.md` | BD-002_app-layer.md | 削除済み |
| `virtual-voicebot-backend/docs/recording.md` | DD-007_recording.md | 削除済み |
| `virtual-voicebot-backend/docs/PRD.md` | RD-001_product.md | 削除済み |
| `virtual-voicebot-backend/docs/FDD.md` | BD 体系に統合 | 削除済み |
| `virtual-voicebot-backend/docs/TSD.md` | DD 体系に統合 | 削除済み |
| `virtual-voicebot-backend/docs/mvp.md` | RD-002_mvp.md | 削除済み |
| `virtual-voicebot-backend/docs/tests.md` | test/plan.md | 削除済み |
| `virtual-voicebot-backend/docs/tests_e2e_sipp.md` | ST-002_e2e-sipp.md | 削除済み |
| `virtual-voicebot-backend/docs/gap-analysis.md` | archive/ に移動 | アーカイブ |
| `virtual-voicebot-backend/docs/impl/PLAN.md` | archive/ に移動 | アーカイブ |
| `virtual-voicebot-backend/docs/impl/TODO.md` | GitHub Issues に移行 | 削除済み |

---

## 凡例

| マーク | 意味 |
|-------|------|
| ✓ | 正本（Source of Truth） |
| - | 補助ドキュメント / インデックス |

---

*本一覧はドキュメント追加・削除時に必ず更新すること。*
