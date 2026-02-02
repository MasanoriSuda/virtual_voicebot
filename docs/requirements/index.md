# 要件仕様書インデックス（システム全体）

> Virtual Voicebot システム全体の要件仕様書一覧

| 項目 | 値 |
|------|-----|
| ステータス | Draft |
| 作成日 | 2026-01-31 |
| 関連Issue | #74 |

---

## 概要

本ディレクトリには、**システム全体**（Backend/Frontend 横断）の要件仕様書（RD: Requirements Definition）を格納する。

- 各コンポーネント固有の要件は各サブディレクトリに配置
  - Backend: [virtual-voicebot-backend/docs/requirements/](../../virtual-voicebot-backend/docs/requirements/index.md)
  - Frontend: [virtual-voicebot-frontend/docs/requirements/](../../virtual-voicebot-frontend/docs/requirements/index.md)（予定）

---

## 命名規則

```
RD-{連番}_{slug}.md

例:
- RD-004_call-routing.md（システム全体の電話振り分け要件）
```

---

## 要件仕様書一覧

| ID | 名称 | ステータス | 対応ST | 概要 |
|----|------|----------|--------|------|
| [RD-004](RD-004_call-routing.md) | 電話番号振り分け・迷惑電話対策 | **Draft** | ST-003 | 番号振り分け・IVR・録音・管理画面（システム横断） |
| [RD-005](RD-005_frontend.md) | Frontend 管理画面 | **Draft** | - | Dashboard・発着信履歴・録音再生・文字起こし・要約（MVP） |

---

## コンポーネント別要件への参照

| コンポーネント | リンク | 概要 |
|---------------|--------|------|
| Backend | [requirements/index.md](../../virtual-voicebot-backend/docs/requirements/index.md) | SIP/RTP/AI連携等のバックエンド要件 |
| Frontend | [RD-005_frontend.md](RD-005_frontend.md) | 管理画面（Dashboard・発着信履歴等） |

---

## テンプレート

新規作成時は [virtual-voicebot-backend/docs/requirements/TEMPLATE-RD.md](../../virtual-voicebot-backend/docs/requirements/TEMPLATE-RD.md) を使用すること。

---

## 参照

- [プロセス定義書](../process/v-model.md) - §4.1 要件定義
- [品質ゲート定義](../process/quality-gate.md) - QG-1
- [Backend 要件インデックス](../../virtual-voicebot-backend/docs/requirements/index.md)

---

## 変更履歴

| 日付 | バージョン | 変更内容 | 作成者 |
|------|-----------|---------|--------|
| 2026-01-31 | 1.0 | 初版作成（RD-004移動に伴い新設） | @MasanoriSuda + Claude Code |
