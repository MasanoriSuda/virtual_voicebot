# 要件仕様書インデックス（Backend）

> Virtual Voicebot Backend の要件仕様書一覧

| 項目 | 値 |
|------|-----|
| ステータス | Draft |
| 作成日 | 2026-01-31 |
| 関連Issue | #69, #74 |

---

## 概要

本ディレクトリには、**Backend 固有**の要件仕様書（RD: Requirements Definition）を格納する。

- 各 RD は「**何を作るか**」を定義する
- 対応するテスト：システムテスト（ST）
- トレーサビリティ：RD ↔ ST

> **Note**: システム全体（Backend/Frontend 横断）の要件は [/docs/requirements/](../../../../docs/requirements/index.md) を参照

---

## 命名規則

```
RD-{連番}_{slug}.md

例:
- RD-001_call-reception.md
- RD-002_ai-conversation.md
- RD-003_recording.md
```

---

## 要件仕様書一覧

| ID | 名称 | ステータス | 対応ST | 概要 |
|----|------|----------|--------|------|
| [RD-001](RD-001_product.md) | プロダクト要求仕様 | Approved | ST-001 | プロダクトビジョン・機能要件・非機能要件 |
| [RD-002](RD-002_mvp.md) | MVP定義 | Approved | - | MVP スコープと最小動作定義 |
| [RD-003](RD-003_flow.md) | ボイスボットフロー | Approved | - | 通話フローの要件定義 |

---

## システム横断要件への参照

システム全体（Backend/Frontend 横断）の要件は以下を参照：

| ID | 名称 | 配置先 |
|----|------|--------|
| [RD-004](../../../../docs/requirements/RD-004_call-routing.md) | 電話番号振り分け・迷惑電話対策 | /docs/requirements/ |

---

## テンプレート

新規作成時は [TEMPLATE-RD.md](TEMPLATE-RD.md) を使用すること。

---

## 参照

- [プロセス定義書](../process/v-model.md) - §4.1 要件定義
- [品質ゲート定義](../process/quality-gate.md) - QG-1
- [システム全体要件](../../../../docs/requirements/index.md) - Backend/Frontend 横断要件
