# 設計書インデックス

> Virtual Voicebot Backend の設計書一覧

| 項目 | 値 |
|------|-----|
| ステータス | Draft |
| 作成日 | 2026-01-31 |
| 関連Issue | #69, #75, #76 |

---

## 概要

本ディレクトリには、システムの設計書を格納する。

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
│   ├── TEMPLATE-BD.md
│   └── BD-xxx.md
└── detail/            # 詳細設計書（DD）
    ├── TEMPLATE-DD.md
    └── DD-xxx.md
```

---

## 基本設計書（BD）一覧

| ID | 名称 | ステータス | 対応IT | 概要 |
|----|------|----------|--------|------|
| [BD-001](basic/BD-001_architecture.md) | システムアーキテクチャ | Approved | - | モジュール構成・データフロー |
| [BD-002](basic/BD-002_app-layer.md) | App層設計 | Approved | - | app レイヤ I/F・イベントフロー |

→ 詳細は [basic/](basic/) を参照

---

## 詳細設計書（DD）一覧

| ID | 名称 | ステータス | 対応UT | 概要 |
|----|------|----------|--------|------|
| [DD-001](detail/DD-001_tech-stack.md) | 技術スタック | Approved | - | 言語・依存クレート・開発ツール |
| [DD-002](detail/DD-002_modules.md) | モジュール設計 | Approved | - | 責務分離・レイヤ構造・RFC準拠 |
| [DD-003](detail/DD-003_sip.md) | SIPモジュール | Approved | - | SIPプロトコル処理・トランザクション |
| [DD-004](detail/DD-004_rtp.md) | RTPモジュール | Approved | - | RTP/RTCP処理 |
| [DD-005](detail/DD-005_session.md) | Sessionモジュール | Approved | - | コール制御・セッション管理 |
| [DD-006](detail/DD-006_ai.md) | AIモジュール | Approved | - | ASR/LLM/TTS連携 |
| [DD-007](detail/DD-007_recording.md) | Recordingモジュール | Approved | - | 録音生成・配信 |

→ 詳細は [detail/](detail/) を参照

---

## 参照

- [プロセス定義書](../process/v-model.md) - §4.2, §4.3
- [品質ゲート定義](../process/quality-gate.md) - QG-2, QG-3
