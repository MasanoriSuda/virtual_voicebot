# ステアリングインデックス（横断）

> Virtual Voicebot **システム横断**（Frontend-Backend 連携）のステアリング（差分仕様）一覧

| 項目 | 値 |
|------|-----|
| ステータス | Active |
| 作成日 | 2026-02-23 |

---

## 概要

本ディレクトリには、**Frontend / Backend 双方に影響する横断的**なステアリング（差分仕様）を格納する。

- ステアリングは「**イシュー単位の変更仕様**」を定義する
- 承認後に本体仕様書（RD/DD/UT等）へマージ
- Frontend 固有 → `virtual-voicebot-frontend/docs/steering/`
- Backend 固有 → `virtual-voicebot-backend/docs/steering/`

---

## 命名規則

```
STEER-{イシュー番号}_{slug}.md

例:
- STEER-137_backend-integration-strategy.md
- STEER-199_docker-compose-integration.md
```

---

## ステアリング一覧

| ID | タイトル | ステータス | 関連Issue | 優先度 | 概要 |
|----|---------|----------|----------|--------|------|
| [STEER-112](STEER-112_sot-reconstruction.md) | SoT 再構築（Frontend / Backend データモデル統一） | Approved | #112 | P0 | 全エンティティの SoT 所在と同期方向を明示するマトリクスを定義し、Frontend/Backend データモデルを統一する |
| [STEER-113](STEER-113_docs-consolidation.md) | 開発ガイドライン統合（SoT 再作成 ver2） | Draft | #113 | P1 | 4ファイルの内容を DEVELOPMENT_GUIDE.md に統合し、開発ガイドラインを一元化する |
| [STEER-137](STEER-137_backend-integration-strategy.md) | フロントエンド着信設定の Backend 連携統合戦略 | Approved | #137 | P0 | Frontend PoC と Backend を統合するための設計戦略を確定し、後続 Issue で詳細設計・実装を進める基盤を作る |
| [STEER-139](STEER-139_frontend-backend-sync-impl.md) | Frontend → Backend 同期実装（Phase 1: 同期基盤） | Approved | #139 | P0 | Backend の Serversync が Frontend PoC の設定を Pull して Backend DB に保存する同期基盤を構築する |
| [STEER-144](STEER-144_frontend-backend-integration.md) | Frontend UI の Backend 統合対応（Phase 6: E2E 動作確認） | Review | #144 | P1 | Phase 6 で Frontend UI と Backend の E2E 動作確認を実施し、統合対応を完了する |
| [STEER-153](STEER-153_role-split.md) | AI エージェント役割分担の変更と STEER 配置統一 | Merged | #153 | P0 | Claude Code の使用量を Draft 作成に集中させ、Review 以降の修正は Codex に委譲することで使用量上限問題を解決する |
| [STEER-166](STEER-166_announce-field-fix.md) | Frontend-Backend フィールド名不一致の修正（includeAnnouncement → announceEnabled） | Approved | #166 | P0 | Frontend-Backend 間のフィールド名を contract.md 仕様（announceEnabled）に統一し、不一致を解消する |
| [STEER-169](STEER-169_vr-actioncode-mismatch.md) | ActionCode "VR" の定義不整合修正 | Review | #169, #170 | P0 | ActionCode "VR" の定義を B2BUA 転送モードに統一し、Frontend/Backend 間の定義不整合を修正する |
| [STEER-173](STEER-173_call-history-action-details.md) | 発着信履歴のアクション詳細表示と IVR 経路追従 | Approved | #173 | P1 | 発着信履歴ページにアクション詳細を表示し、IVR 経路を追従して可視化する |
| [STEER-177](STEER-177_backend-sync-status-dashboard.md) | Backend 同期状態可視化ダッシュボード | Approved | #177 | P1 | Frontend のダッシュボードに Backend の着信アクション同期状態を可視化し、運用監視を可能にする |
| [STEER-199](STEER-199_docker-compose-integration.md) | Docker Compose 統合（Backend + Frontend） | Approved | #199 | P1 | モノレポ構成（Backend + Frontend）を統合的に管理できる Docker 環境を構築する |
| [STEER-213](STEER-213_fix-agent-doc-responsibility.md) | AI エージェントのドキュメント責務不整合の修正 | Approved | #213 | P0 | STEER-153 の変更によって生じた Claude Code のドキュメント責務に関する不整合を根本から修正する |
| [STEER-226](STEER-226_frontend-backend-split-config.md) | Frontend/Backend 別マシン構成の環境変数対応 | Draft | #226 | P1 | `.env.example` を実態に合わせて更新し、Frontend と Backend が別マシンで動作する構成の環境変数を整備する |
| [STEER-245](STEER-245_local-services-status-dashboard.md) | ローカルサービス死活監視ダッシュボード | Approved | #245 | P1 | Frontend のダッシュボードに ASR/LLM/TTS ローカルサービスの死活状態を表示する。Backend が probe を集約し、Frontend はウィジェットで可視化する |
| [STEER-266](STEER-266_incoming-call-popup.md) | 着信ポップアップ通知 | Draft | #266 | P1 | 着信（直接転送・IVR 転送）を契機に Frontend 画面上でポップアップを表示する。独立 fast-path ファイルキュー + serversync 新規 Worker（1秒ポーリング）で既存 sync_outbox に影響なく実装する |
| [STEER-267](STEER-267_ivr-vr-notify-fix.md) | DB IVR VR ルートの転送通知欠落バグ修正 | Approved | #267 | P1 | STEER-266 実装後に発覚した仕様漏れ。DB IVR の `"VR" =>` ルート（handlers/mod.rs L1304）に `notify_ivr_transfer_if_needed()` 呼び出しが欠落しており、IVR 転送時にポップアップが発火しない |

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

ステアリングの作成・運用手順は各サブプロジェクトの GUIDE.md を参照すること。

- Backend: [virtual-voicebot-backend/docs/steering/GUIDE.md](../../virtual-voicebot-backend/docs/steering/GUIDE.md)
- Frontend: [virtual-voicebot-frontend/docs/steering/GUIDE.md](../../virtual-voicebot-frontend/docs/steering/GUIDE.md)

---

## 参照

- [Backend ステアリングインデックス](../../virtual-voicebot-backend/docs/steering/index.md)
- [Frontend ステアリングインデックス](../../virtual-voicebot-frontend/docs/steering/index.md)
- [プロセス定義書（Backend）](../../virtual-voicebot-backend/docs/process/v-model.md) - §5 ステアリング運用
- [契約仕様書](../contract.md) - Frontend-Backend API 契約

---

## 変更履歴

| 日付 | バージョン | 変更内容 | 作成者 |
|------|-----------|---------|--------|
| 2026-02-23 | 1.0 | 初版作成（既存横断ステアリング STEER-112〜STEER-213 を収録、STEER-226 追加） | Claude Code |
| 2026-02-24 | 1.1 | STEER-245 追加（ローカルサービス死活監視ダッシュボード） | Claude Sonnet 4.6 |
| 2026-02-25 | 1.2 | STEER-245 ステータス Draft → Approved | @MasanoriSuda |
| 2026-02-28 | 1.3 | STEER-266 追加（着信ポップアップ通知） | Claude Sonnet 4.6 |
| 2026-02-28 | 1.4 | STEER-267 追加（DB IVR VR ルート転送通知欠落バグ修正） | Claude Sonnet 4.6 |
