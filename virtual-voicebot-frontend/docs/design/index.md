# 設計書インデックス

> Virtual Voicebot Frontend の設計書一覧

| 項目 | 値 |
|------|-----|
| ステータス | Draft |
| 作成日 | 2026-02-03 |
| 関連Issue | #89, #99 |

---

## 概要

本ディレクトリには、Frontend の設計書を格納する。

| 種別 | 格納先 | 目的 | 対応テスト |
|------|--------|------|-----------|
| 基本設計（BD） | basic/ | **どう分けるか**を定義 | IT |
| 詳細設計（DD） | detail/ | **どう実装するか**を定義 | UT |

---

## ディレクトリ構造

```
design/
├── index.md           # 本ファイル
├── basic/             # 基本設計書（BD）
│   └── BD-001_frontend.md
└── detail/            # 詳細設計書（DD）
    └── (将来)
```

---

## 基本設計書（BD）一覧

| ID | 名称 | ステータス | 対応IT | 概要 |
|----|------|----------|--------|------|
| [BD-001](basic/BD-001_frontend.md) | Frontend 管理画面設計 | Draft | - | コンポーネント構成・データフロー |

→ 詳細は [basic/](basic/) を参照

---

## 参照

- [要件定義書 RD-005](../requirements/RD-005_frontend.md)
- [ステアリング STEER-099](../../../docs/steering/STEER-099_frontend-mvp.md)
