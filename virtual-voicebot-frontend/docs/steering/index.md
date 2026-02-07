# ステアリングインデックス（Frontend）

> Virtual Voicebot Frontend のステアリング（差分仕様）一覧

| 項目 | 値 |
|------|-----|
| ステータス | Active |
| 作成日 | 2026-02-05 |

---

## 概要

本ディレクトリには、**Frontend 固有**のステアリング（差分仕様）を格納する。

- ステアリングは「**イシュー単位の変更仕様**」を定義する
- 承認後に本体仕様書（RD/DD/UT等）へマージ

---

## 命名規則

```
STEER-{イシュー番号}_{slug}.md

例:
- STEER-107_frontend-docs-structure.md
- STEER-110_call-detail-page.md
```

---

## ステアリング一覧

| ID | タイトル | ステータス | 関連Issue | 優先度 | 概要 |
|----|---------|----------|----------|--------|------|
| [STEER-107](STEER-107_frontend-docs-structure.md) | フロントエンド SoT ドキュメント体系の構築 | Approved | #107 | P1 | ディレクトリ構造・テンプレート・インデックス・プロセス定義の整備 |
| [STEER-116](STEER-116_frontend-ingest-api.md) | Frontend Ingest API 実装（Backend 同期受信側） | Approved | #116 | P0 | POST /api/ingest/sync, POST /api/ingest/recording-file の受信実装、Frontend DB への upsert、録音ファイル保存 |
| [STEER-119](STEER-119_ui-backend-integration.md) | Frontend UI と Backend の連携実装 | Approved | #119 | P0 | モックデータから実データへ切り替え、lib/api.ts の Prisma 実装、KPI 集計、録音ファイル配信、AC-1〜AC-14 検証 |
| [STEER-129](STEER-129_announce-audio-add.md) | アナウンスタブ音声追加機能 | Approved | #129 | P1 | WAV アップロード + VoiceVox TTS 生成、アナウンス管理操作（削除/切替/名称変更）、JSON ストア永続化 |
| [STEER-132](STEER-132_call-action-ui.md) | 着信時アクション決定 UI | Approved | #132 | P1 | 番号グループ × ルール評価による着信アクション設定、Allow(VR/IV/VM) / Deny(BZ/NR/AN)、JSON ストア永続化 |

---

## ステータス定義

| ステータス | 説明 |
|-----------|------|
| Draft | 作成中 |
| Review | レビュー中 |
| Approved | 承認済み（実装待ち） |
| Merged | 本体仕様書へマージ完了 |

---

## 運用ガイド

ステアリングの作成・運用手順は [GUIDE.md](GUIDE.md) を参照すること。

---

## テンプレート

新規作成時は [TEMPLATE.md](TEMPLATE.md) を使用すること。

---

## 参照

- [プロセス定義書](../process/v-model.md) - §5 ステアリング運用
- [要件仕様インデックス](../requirements/index.md) - RD 一覧
- [設計書インデックス](../design/index.md) - BD/DD 一覧

---

## 変更履歴

| 日付 | バージョン | 変更内容 | 作成者 |
|------|-----------|---------|--------|
| 2026-02-05 | 1.0 | 初版作成 | Claude Code |
| 2026-02-07 | 1.1 | STEER-116 追加（Frontend Ingest API 実装） | Claude Code |
| 2026-02-07 | 1.2 | STEER-116 承認（Approved） | Claude Code |
| 2026-02-07 | 1.3 | STEER-119 追加（Frontend UI と Backend の連携実装） | Claude Code |
| 2026-02-07 | 1.4 | STEER-119 承認（Approved） | Claude Code |
| 2026-02-08 | 1.5 | STEER-129 追加（アナウンスタブ音声追加機能） | Claude Code |
| 2026-02-08 | 1.6 | STEER-129 承認（Approved） | Claude Code |
| 2026-02-08 | 1.7 | STEER-132 追加（着信時アクション決定 UI） | Claude Code |
| 2026-02-08 | 1.8 | STEER-132 承認（Approved） | Claude Code |
