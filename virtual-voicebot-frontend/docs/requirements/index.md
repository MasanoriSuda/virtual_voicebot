# 要件仕様書インデックス（Frontend）

> Virtual Voicebot Frontend の要件仕様書一覧

| 項目 | 値 |
|------|-----|
| ステータス | Draft |
| 作成日 | 2026-02-05 |
| 関連Issue | #107 |

---

## 概要

本ディレクトリには、**Frontend 固有**の要件仕様書（RD: Requirements Definition）を格納する。

- 各 RD は「**何を作るか**」を定義する
- 対応するテスト：システムテスト（ST）
- トレーサビリティ：RD ↔ ST

---

## 命名規則

```
RD-{連番}_{slug}.md

例:
- RD-005_frontend.md
- RD-006_authentication.md
```

---

## 要件仕様書一覧

| ID | 名称 | ステータス | 対応ST | 概要 |
|----|------|----------|--------|------|
| [RD-005](RD-005_frontend.md) | Frontend 管理画面 MVP | Draft | - | Dashboard・発着信履歴・録音再生・文字起こし・要約 |

---

## システム横断要件への参照

システム全体（Backend/Frontend 横断）の要件は以下を参照：

| ID | 名称 | 配置先 |
|----|------|--------|
| [RD-004](../../../docs/requirements/RD-004_call-routing.md) | 電話番号振り分け・迷惑電話対策 | /docs/requirements/ |

---

## テンプレート

新規作成時は [TEMPLATE-RD.md](TEMPLATE-RD.md) を使用すること。

---

## 参照

- [プロセス定義書](../process/v-model.md) - §4.1 要件定義
- [品質ゲート定義](../process/quality-gate.md) - QG-1

---

## 変更履歴

| 日付 | バージョン | 変更内容 | 作成者 |
|------|-----------|---------|--------|
| 2026-02-05 | 1.0 | 初版作成 | Claude Code |
