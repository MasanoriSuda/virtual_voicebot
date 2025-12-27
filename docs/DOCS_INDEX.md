# ドキュメント一覧 (DOCS_INDEX)

**ステータス**: ドラフト
**最終更新**: 2025-12-27

---

## クイックリンク

| 目的 | ドキュメント |
|------|-------------|
| プロジェクト概要 | [README.md](../README.md) |
| 開発参加 | [CONTRIBUTING.md](../CONTRIBUTING.md) |
| コーディング規約 | [STYLE.md](../STYLE.md) |
| アーキテクチャ | [design.md](../virtual-voicebot-backend/docs/design.md) |
| API契約 | [contract.md](contract.md) |
| RFCギャップ分析 | [gap-analysis.md](../virtual-voicebot-backend/docs/gap-analysis.md) |

---

## 1. リポジトリルート

| ファイル | 内容 | 正本 |
|---------|------|:----:|
| [README.md](../README.md) | リポジトリ概要、サブプロジェクト構成 | - |
| [CONTRIBUTING.md](../CONTRIBUTING.md) | 開発参加ガイド | ✓ |
| [STYLE.md](../STYLE.md) | プロジェクト共通スタイルガイド | ✓ |
| [PRINCIPLES.md](../PRINCIPLES.md) | 価値観・原則 | ✓ |

---

## 2. docs/ (共通ドキュメント)

| ファイル | 内容 | 正本 |
|---------|------|:----:|
| [DOCS_POLICY.md](DOCS_POLICY.md) | ドキュメント管理ポリシー | ✓ |
| [DOCS_INDEX.md](DOCS_INDEX.md) | 本ファイル（ドキュメント一覧） | - |
| [DOCS_ANALYSIS.md](DOCS_ANALYSIS.md) | ドキュメント整理分析レポート | - |
| [contract.md](contract.md) | Frontend ↔ Backend API契約 | ✓ |
| [style/rust.md](style/rust.md) | Rust固有スタイル | - |
| [reviews/](reviews/) | レビュー結果保存先 | - |

---

## 3. Backend (virtual-voicebot-backend/)

### 3.1 ルートドキュメント

| ファイル | 内容 | 正本 |
|---------|------|:----:|
| [README.md](../virtual-voicebot-backend/README.md) | Backend概要、クイックスタート | - |
| [AGENTS.md](../virtual-voicebot-backend/AGENTS.md) | AI/Codex向け指示書 | ✓ |

### 3.2 設計ドキュメント (docs/)

| ファイル | 内容 | 正本 | 状態 |
|---------|------|:----:|------|
| [design.md](../virtual-voicebot-backend/docs/design.md) | アーキテクチャ設計 | ✓ | アクティブ |
| [sip.md](../virtual-voicebot-backend/docs/sip.md) | SIP詳細設計 | ✓ | アクティブ |
| [rtp.md](../virtual-voicebot-backend/docs/rtp.md) | RTP詳細設計 | ✓ | アクティブ |
| [session.md](../virtual-voicebot-backend/docs/session.md) | Session詳細設計 | ✓ | アクティブ |
| [ai.md](../virtual-voicebot-backend/docs/ai.md) | AI連携設計 | ✓ | アクティブ |
| [app.md](../virtual-voicebot-backend/docs/app.md) | App層設計 | ✓ | アクティブ |
| [recording.md](../virtual-voicebot-backend/docs/recording.md) | 録音設計 | ✓ | アクティブ |
| [voice_bot_flow.md](../virtual-voicebot-backend/docs/voice_bot_flow.md) | 対話フロー | - | 要確認 |
| [mvp.md](../virtual-voicebot-backend/docs/mvp.md) | MVP定義 | - | 要確認 |

### 3.3 テストドキュメント

| ファイル | 内容 | 正本 | 状態 |
|---------|------|:----:|------|
| [tests.md](../virtual-voicebot-backend/docs/tests.md) | テスト計画 | ✓ | アクティブ |
| [tests_e2e_sipp.md](../virtual-voicebot-backend/docs/tests_e2e_sipp.md) | SIPp E2E手順 | ✓ | アクティブ |
| [test/README.md](../virtual-voicebot-backend/test/README.md) | E2Eランナー使用方法 | - | 補助 |

### 3.4 分析ドキュメント

| ファイル | 内容 | 正本 | 状態 |
|---------|------|:----:|------|
| [gap-analysis.md](../virtual-voicebot-backend/docs/gap-analysis.md) | RFC準拠ギャップ分析・仕様 | ✓ | アクティブ |

### 3.5 実装計画 (docs/impl/)

| ファイル | 内容 | 正本 | 状態 |
|---------|------|:----:|------|
| [PLAN.md](../virtual-voicebot-backend/docs/impl/PLAN.md) | 実装ステップ計画 | ✓ | アクティブ |
| [TODO.md](../virtual-voicebot-backend/docs/impl/TODO.md) | 実装バックログ | ✓ | アクティブ |

### 3.6 モジュールREADME (src/)

| ファイル | 内容 | 詳細リンク先 |
|---------|------|-------------|
| [src/sip/README.md](../virtual-voicebot-backend/src/sip/README.md) | SIPモジュール概要 | docs/sip.md |
| [src/rtp/README.md](../virtual-voicebot-backend/src/rtp/README.md) | RTPモジュール概要 | docs/rtp.md |
| [src/session/README.md](../virtual-voicebot-backend/src/session/README.md) | Sessionモジュール概要 | docs/session.md |
| [src/transport/README.md](../virtual-voicebot-backend/src/transport/README.md) | Transportモジュール概要 | - |
| [src/ai/README.md](../virtual-voicebot-backend/src/ai/README.md) | AIモジュール概要 | docs/ai.md |
| [src/app/README.md](../virtual-voicebot-backend/src/app/README.md) | Appモジュール概要 | docs/app.md |
| [src/http/README.md](../virtual-voicebot-backend/src/http/README.md) | HTTPモジュール概要 | - |
| [src/media/README.md](../virtual-voicebot-backend/src/media/README.md) | Mediaモジュール概要 | - |

---

## 4. Frontend (virtual-voicebot-frontend/)

| ファイル | 内容 | 状態 |
|---------|------|------|
| [README.md](../virtual-voicebot-frontend/README.md) | Frontend概要 | 要確認 |

---

## 5. RFC仕様書 (spec/)

| ファイル | 内容 |
|---------|------|
| [rfc3261.txt](../virtual-voicebot-backend/spec/rfc3261.txt) | SIP Core |
| [rfc3262.txt](../virtual-voicebot-backend/spec/rfc3262.txt) | 100rel/PRACK |
| [rfc3263.txt](../virtual-voicebot-backend/spec/rfc3263.txt) | DNS SRV/NAPTR |
| [rfc3264.txt](../virtual-voicebot-backend/spec/rfc3264.txt) | Offer/Answer |
| [rfc3311.txt](../virtual-voicebot-backend/spec/rfc3311.txt) | UPDATE |
| [rfc3550.txt](../virtual-voicebot-backend/spec/rfc3550.txt) | RTP/RTCP |
| [rfc4028.txt](../virtual-voicebot-backend/spec/rfc4028.txt) | Session Timers |
| [rfc8866.txt](../virtual-voicebot-backend/spec/rfc8866.txt) | SDP |

---

## 凡例

| マーク | 意味 |
|-------|------|
| ✓ | 正本（Source of Truth） |
| - | 補助ドキュメント |
| **太字** | アクション必要 |

---

*本一覧は定期的に更新してください。陳腐化を防ぐため、ドキュメント追加・削除時は必ず反映してください。*
