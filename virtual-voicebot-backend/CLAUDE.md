<!-- SOURCE_OF_TRUTH: Claude Code Backend開発ガイド -->
# CLAUDE.md (Backend)

> AI/Claude Code 向けバックエンド開発ガイド

## 概要

本ファイルは `virtual-voicebot-backend` の開発を進める上で AI（Claude Code / Codex）が遵守すべきルールを定義する。

**上位ドキュメント**: [/CLAUDE.md](../CLAUDE.md)（リポジトリ共通）、[AGENTS.md](AGENTS.md)（Backend 実装詳細）

---

## 1. ドキュメント階層（新構造）

### 1.1 プロセス定義 (`docs/process/`)

V字モデル・品質ゲート・トレーサビリティを定義する。

| ファイル | 内容 |
|---------|------|
| [v-model.md](docs/process/v-model.md) | プロセス定義書（V字モデル・ガバナンス） |
| [quality-gate.md](docs/process/quality-gate.md) | 品質ゲート定義 |
| traceability.md | トレーサビリティマトリクス（作成予定） |

### 1.2 ステアリング (`docs/steering/`)

イシュー単位の差分仕様。ストーリー（Why/Who/When）+ 仕様（What/How）を一元管理。

| ファイル | 内容 |
|---------|------|
| [TEMPLATE.md](docs/steering/TEMPLATE.md) | ステアリングテンプレート |
| STEER-xxx.md | イシューごとの差分仕様 |

### 1.3 本体仕様書（作成予定）

| ディレクトリ | 内容 |
|-------------|------|
| docs/requirements/ | 要件仕様書（RD） |
| docs/design/basic/ | 基本設計書（BD） |
| docs/design/detail/ | 詳細設計書（DD） |
| docs/test/unit/ | 単体テスト仕様（UT） |
| docs/test/integration/ | 結合テスト仕様（IT） |
| docs/test/system/ | システムテスト仕様（ST） |

### 1.4 旧ドキュメント（移行予定 → `docs/archive/`）

| ファイル | 内容 | 移行先 |
|---------|------|--------|
| [PRD.md](docs/PRD.md) | プロダクト要求仕様書 | docs/requirements/ |
| [FDD.md](docs/FDD.md) | 機能設計書 | docs/design/ |
| [TSD.md](docs/TSD.md) | 技術仕様書 | docs/design/detail/ |
| [design.md](docs/design.md) | アーキテクチャ設計 | docs/design/basic/ |
| [tests.md](docs/tests.md) | テスト計画 | docs/test/ |

---

## 2. タスク別コンテキストナビゲーション

### 2.1 SIP関連の作業

参照すべきドキュメント（この順で読む）:
1. [docs/steering/STEER-xxx.md](docs/steering/) （該当ステアリングがあれば）
2. docs/requirements/RD-001_sip-uas.md（作成予定）
3. docs/design/detail/DD-001_sip.md（作成予定）
4. 旧: [docs/sip.md](docs/sip.md)

### 2.2 AI連携の作業

参照すべきドキュメント:
1. [docs/steering/STEER-xxx.md](docs/steering/) （該当ステアリングがあれば）
2. docs/requirements/RD-002_ai-dialog.md（作成予定）
3. docs/design/detail/DD-005_ai.md（作成予定）
4. 旧: [docs/ai.md](docs/ai.md)

### 2.3 録音機能の作業

参照すべきドキュメント:
1. [docs/steering/STEER-xxx.md](docs/steering/) （該当ステアリングがあれば）
2. docs/requirements/RD-003_recording.md（作成予定）
3. docs/design/detail/DD-006_recording.md（作成予定）
4. 旧: [docs/recording.md](docs/recording.md)

---

## 3. 開発プロセス（ステアリング運用）

### 3.1 新規機能・変更の流れ

1. **イシュー起票**: GitHub Issue を作成
2. **ステアリング作成**: [TEMPLATE.md](docs/steering/TEMPLATE.md) をコピーして差分仕様を記述
3. **レビュー**: アーキテクト・POが確認
4. **承認**: 実装GOの判断
5. **実装**: Codexへ引き継ぎ（ステアリングをコンテキストとして渡す）
6. **マージ**: 差分を本体仕様書（RD/DD/UT等）へ反映

### 3.2 ドキュメント駆動の原則

- **仕様/責務/依存方向** に関わる変更は、必ずステアリングを先に作成
- コードがステアリング/docs と矛盾する場合、**正はステアリング/docs** とし、コードを追従させる
- 各段階で承認を得る（段取りを記録）

詳細は [プロセス定義書](docs/process/v-model.md) を参照。

---

## 4. 正本と優先順位

矛盾がある場合は以下の順で優先:

1. **ステアリング**（該当イシューの作業中）
2. **本体仕様書**（RD/BD/DD/UT/IT/ST）
3. **旧ドキュメント**（PRD/FDD/TSD等）
4. `src/*/README.md`

---

## 5. コーディング規約

詳細は [DEVELOPMENT_GUIDE.md](docs/DEVELOPMENT_GUIDE.md) を参照。

### 5.1 品質チェック

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

### 5.2 セキュリティ

- 音声・文字起こし・LLM 入出力は PII になりうる前提で扱う
- ログに原文（全文）を出さない（デバッグフラグで限定的に）

### 5.3 ID 規約

- すべての主要ログ/イベントに `call_id` を付与（必須）
- **MVP**: `call_id == session_id`（design.md §13.1 参照）
- メディア（RTP/PCM）単位は `stream_id` を併用

---

## 6. 参照リンク

### 6.1 プロセス定義

| ドキュメント | 内容 |
|-------------|------|
| [v-model.md](docs/process/v-model.md) | プロセス定義書 |
| [quality-gate.md](docs/process/quality-gate.md) | 品質ゲート定義 |
| [TEMPLATE.md](docs/steering/TEMPLATE.md) | ステアリングテンプレート |

### 6.2 開発ドキュメント

| ドキュメント | 内容 |
|-------------|------|
| [AGENTS.md](AGENTS.md) | AI/Codex 向け実装詳細 |
| [DEVELOPMENT_GUIDE.md](docs/DEVELOPMENT_GUIDE.md) | 開発ガイドライン |

### 6.3 旧仕様ドキュメント（移行予定）

| ドキュメント | 内容 |
|-------------|------|
| [PRD.md](docs/PRD.md) | プロダクト要求仕様書 |
| [FDD.md](docs/FDD.md) | 機能設計書 |
| [TSD.md](docs/TSD.md) | 技術仕様書 |
| [design.md](docs/design.md) | アーキテクチャ設計 |
