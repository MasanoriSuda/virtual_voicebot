# ステアリングインデックス（Backend）

> Virtual Voicebot Backend のステアリング（差分仕様）一覧

| 項目 | 値 |
|------|-----|
| ステータス | Active |
| 作成日 | 2026-01-31 |

---

## 概要

本ディレクトリには、**Backend 固有**のステアリング（差分仕様）を格納する。

- ステアリングは「**イシュー単位の変更仕様**」を定義する
- 承認後に本体仕様書（RD/DD/UT等）へマージ

---

## 命名規則

```
STEER-{イシュー番号}_{slug}.md

例:
- STEER-085_clean-architecture.md
- STEER-080_cancel-handling.md
```

---

## ステアリング一覧

| ID | タイトル | ステータス | 関連Issue | 優先度 | 概要 |
|----|---------|----------|----------|--------|------|
| [STEER-085](STEER-085_clean-architecture.md) | クリーンアーキテクチャ移行（ISP準拠 + ファイル分割） | Draft | #52, #65, #85 | P0 | ISP準拠トレイト設計、エンティティ層新設、sip/mod.rs分割、Session分離 |
| [STEER-095](STEER-095_backend-refactoring.md) | Backend 磨き上げ（クリーンアーキテクチャ適合） | Draft | #95 | P1 | 現行実装と設計書の乖離を解消し、クリーンアーキテクチャに適合させる |
| [STEER-096](STEER-096_serversync.md) | Serversync実装（Backend-Frontend 同期機構） | Approved | #96 | P0 | Transactional Outbox Pattern、独立バイナリ、POST /api/ingest/sync + recording-file |
| [STEER-108](STEER-108_sip-core-engine-refactor.md) | 3層アーキテクチャへのリファクタリング | Draft | #108 | P1 | 全モジュールを Protocol/Service/Interface の3層構造に再構成し、依存方向を明確化 |
| [STEER-110](STEER-110_backend-db-design.md) | バックエンド側データベース設計 | Approved | #110 | P0 | PostgreSQL 統合 DB 設計（11テーブル、UUID v7、月次パーティション、Outbox 同期） |
| [STEER-123](STEER-123_recording-outbox-enqueue.md) | 録音データ sync_outbox エンキュー実装（Serversync バグフィックス） | Draft | #123 | P0 | 通話終了時に recording と recording_file を sync_outbox にエンキューするトランザクショナルライト実装 |
| [STEER-143](STEER-143_recording-enhancement.md) | Backend 録音実装強化（Phase 5） | Approved | #143 | P2 | 録音フラグ連動・録音メタデータ管理・Frontend 同期の整合実装 |
| [STEER-203](STEER-203_fix-bleg-rtp-timestamp-double-add.md) | B-leg → A-leg RTP タイムスタンプ二重加算バグ修正 | Approved | #203 | P0 | B2BUA 転送時に align_rtp_clock() と send_payload() でタイムスタンプが二重加算される問題を修正 |

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
- [詳細設計インデックス](../design/detail/index.md) - DD 一覧（予定）

---

## 変更履歴

| 日付 | バージョン | 変更内容 | 作成者 |
|------|-----------|---------|--------|
| 2026-01-31 | 1.0 | 初版作成 | Claude Code |
| 2026-02-06 | 1.1 | STEER-108 追加 | Claude Code |
| 2026-02-07 | 1.2 | STEER-095, STEER-110 追加 | Claude Code |
| 2026-02-07 | 1.3 | STEER-096, STEER-123 追加（Serversync 実装とバグフィックス） | Claude Code |
| 2026-02-11 | 1.4 | STEER-143 追加（Phase 5: 録音実装強化） | Codex |
| 2026-02-20 | 1.5 | STEER-203 追加（B-leg RTP タイムスタンプ二重加算バグ修正） | Claude Code |
