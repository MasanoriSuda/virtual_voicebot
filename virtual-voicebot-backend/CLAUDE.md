<!-- SOURCE_OF_TRUTH: Claude Code Backend開発ガイド -->
# CLAUDE.md (Backend)

> AI/Claude Code 向けバックエンド開発ガイド

## 概要

本ファイルは `virtual-voicebot-backend` の開発を進める上で AI（Claude Code / Codex）が遵守すべきルールを定義する。

**上位ドキュメント**: [/CLAUDE.md](../CLAUDE.md)（リポジトリ共通）、[AGENTS.md](AGENTS.md)（Backend 実装詳細）

---

## 1. ドキュメント階層

### 1.1 永続的ドキュメント (`docs/`)

バックエンドの「何を作るか」「どう作るか」を定義する恒久的なドキュメント。基本設計・方針が変わらない限り更新しない。

| ファイル | 内容 |
|---------|------|
| [design.md](docs/design.md) | アーキテクチャ設計（正本） |
| [sip.md](docs/sip.md) | SIP 詳細設計 |
| [rtp.md](docs/rtp.md) | RTP 詳細設計 |
| [session.md](docs/session.md) | Session 詳細設計 |
| [app.md](docs/app.md) | App 層設計（I/F 正本） |
| [ai.md](docs/ai.md) | AI 連携設計（I/F 正本） |
| [recording.md](docs/recording.md) | 録音設計 |
| [tests.md](docs/tests.md) | テスト計画・受入条件（AC 正本） |
| [gap-analysis.md](docs/gap-analysis.md) | RFC 準拠ギャップ分析 |

### 1.2 統合ドキュメント (`docs/`)

仕様・設計・開発ガイドを統合した包括的なドキュメント。

| ファイル | 内容 |
|---------|------|
| [PRD.md](docs/PRD.md) | プロダクト要求仕様書 |
| [FDD.md](docs/FDD.md) | 機能設計書 |
| [TSD.md](docs/TSD.md) | 技術仕様書 |
| [DEVELOPMENT_GUIDE.md](docs/DEVELOPMENT_GUIDE.md) | 開発ガイドライン |

### 1.3 作業単位のドキュメント (`docs/impl/`)

特定の開発作業における「今回何をするか」を定義する一時的なファイル。

| ファイル | 内容 |
|---------|------|
| [PLAN.md](docs/impl/PLAN.md) | 実装ステップ計画 |
| [TODO.md](docs/impl/TODO.md) | 実装バックログ |

---

## 2. 開発プロセス

### 2.1 新規機能開発

1. **影響分析**: 既存 `docs/*.md` を読み、影響範囲を特定
2. **ドキュメント更新**: 変更が必要な正本を先に更新
3. **承認取得**: ドキュメント更新後、確認・承認を得てから実装
4. **実装**: ドキュメントに沿ってコードを書く
5. **品質チェック**: lint / 型チェック / テスト

### 2.2 ドキュメント駆動の原則

- **仕様/責務/依存方向** に関わる変更は、必ず `docs/*.md` を先に更新
- コードが docs と矛盾する場合、**正は docs** とし、コードを追従させる
- 1ファイルごとに作成・更新し、各段階で承認を得る

---

## 3. 正本と優先順位

矛盾がある場合は以下の順で優先（[DOCS_POLICY.md](../docs/DOCS_POLICY.md) §3.4 参照）:

1. **I/F 詳細**: `app.md` / `ai.md` / `tests.md` が `design.md` より優先
2. **docs/*.md** > `src/*/README.md`
3. **design.md** > 個別モジュール設計

---

## 4. コーディング規約

詳細は [DEVELOPMENT_GUIDE.md](docs/DEVELOPMENT_GUIDE.md) を参照。

### 4.1 品質チェック

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

### 4.2 セキュリティ

- 音声・文字起こし・LLM 入出力は PII になりうる前提で扱う
- ログに原文（全文）を出さない（デバッグフラグで限定的に）

### 4.3 ID 規約

- すべての主要ログ/イベントに `call_id` を付与（必須）
- **MVP**: `call_id == session_id`（design.md §13.1 参照）
- メディア（RTP/PCM）単位は `stream_id` を併用

---

## 5. 図表・ダイアグラム

### 5.1 記載場所

設計図やダイアグラムは関連する永続ドキュメントに直接記載。独立した `diagrams/` フォルダは作成しない。

### 5.2 記述形式（優先順）

1. **Mermaid 記法**（推奨）: Markdown に直接書ける、バージョン管理が容易
2. **アスキーアート**: シンプルな図表向け
3. **画像ファイル**: 複雑な図のみ `docs/images/` に配置

---

## 6. 参照リンク

### 6.1 開発ドキュメント

| ドキュメント | 内容 |
|-------------|------|
| [AGENTS.md](AGENTS.md) | AI/Codex 向け実装詳細 |
| [design.md](docs/design.md) | アーキテクチャ設計 |
| [DEVELOPMENT_GUIDE.md](docs/DEVELOPMENT_GUIDE.md) | 開発ガイドライン |

### 6.2 仕様ドキュメント

| ドキュメント | 内容 |
|-------------|------|
| [PRD.md](docs/PRD.md) | プロダクト要求仕様書 |
| [FDD.md](docs/FDD.md) | 機能設計書 |
| [TSD.md](docs/TSD.md) | 技術仕様書 |

### 6.3 共通ドキュメント

| ドキュメント | 内容 |
|-------------|------|
| [/DOCS_POLICY.md](../docs/DOCS_POLICY.md) | ドキュメントポリシー |
| [/DOCS_INDEX.md](../docs/DOCS_INDEX.md) | ドキュメント一覧 |
| [/STYLE.md](../STYLE.md) | プロジェクト共通スタイル |
