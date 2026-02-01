# 設計書インデックス（システム全体）

> Virtual Voicebot システム全体の設計書一覧

| 項目 | 値 |
|------|-----|
| ステータス | Draft |
| 作成日 | 2026-02-02 |
| 関連Issue | #89 |

---

## 概要

本ディレクトリには、**システム全体**（Backend/Frontend 横断）の設計書を格納する。

| 種別 | 格納先 | 目的 | 対応テスト |
|------|--------|------|-----------|
| 基本設計（BD） | basic/ | **どう分けるか**を定義 | IT |
| 詳細設計（DD） | detail/ | **どう実装するか**を定義 | UT |

- 各コンポーネント固有の設計書は各サブディレクトリに配置
  - Backend: [virtual-voicebot-backend/docs/design/](../../virtual-voicebot-backend/docs/design/index.md)
  - Frontend: 本ディレクトリ（v0 プロトタイプベース）

---

## ディレクトリ構造

```
docs/design/
├── index.md           # 本ファイル
├── basic/             # 基本設計書（BD）
│   └── BD-001_frontend.md
└── detail/            # 詳細設計書（DD）※将来
```

---

## 基本設計書（BD）一覧

| ID | 名称 | ステータス | 対応RD | 概要 |
|----|------|----------|--------|------|
| [BD-001](basic/BD-001_frontend.md) | Frontend 管理画面 | **Draft** | RD-005 | コンポーネント構成・データフロー・API設計 |

---

## 詳細設計書（DD）一覧

| ID | 名称 | ステータス | 対応BD | 概要 |
|----|------|----------|--------|------|
| （未作成） | - | - | - | - |

---

## コンポーネント別設計への参照

| コンポーネント | リンク | 概要 |
|---------------|--------|------|
| Backend | [design/index.md](../../virtual-voicebot-backend/docs/design/index.md) | SIP/RTP/Session/AI/Recording 等 |
| Frontend | 本ディレクトリ | 管理画面（Dashboard・発着信履歴等） |

---

## テンプレート

新規作成時は [virtual-voicebot-backend/docs/design/basic/TEMPLATE-BD.md](../../virtual-voicebot-backend/docs/design/basic/TEMPLATE-BD.md) を使用すること。

---

## 参照

- [プロセス定義書](../process/v-model.md) - §4.2, §4.3（予定）
- [要件仕様書インデックス](../requirements/index.md) - RD 一覧
- [Backend 設計インデックス](../../virtual-voicebot-backend/docs/design/index.md)

---

## 変更履歴

| 日付 | バージョン | 変更内容 | 作成者 |
|------|-----------|---------|--------|
| 2026-02-02 | 1.0 | 初版作成（BD-001 追加） | Claude Code |
