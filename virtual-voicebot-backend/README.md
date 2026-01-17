# Backend (Rust)

このディレクトリはバックエンド（Rust）実装を含みます。  
開発・テスト・実行の入口は本 README と、ルートのドキュメントを正本とします。

## Source of truth（正本）
- 共通の開発プロセス: `../CONTRIBUTING.md`
- 共通スタイル: `../STYLE.md`
- 原則（価値観）: `../PRINCIPLES.md`
- AI/自動化ルール（ある場合）: `../docs/ai/AI_RULES.md`
- Rust 固有スタイル: `../docs/style/rust.md`

---

## Quickstart

### Prerequisites
- Rust toolchain（`rust-toolchain.toml` がある場合はそれに従う）
- `make` または `just`（リポジトリの標準コマンドに従う）

### Build / Test
リポジトリ共通の入口コマンドを使います（個別コマンドの乱立を避ける）。

```bash
# repo root で実行
make ci
# または
just ci
