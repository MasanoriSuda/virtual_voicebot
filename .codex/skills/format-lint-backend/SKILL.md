---
name: format-lint-virtual-voicebot-backend
description: Format and lint ONLY the Rust project under virtual-voicebot-backend using cargo fmt and cargo clippy.
metadata:
  short-description: cargo fmt + clippy in backend subdir
---

## 目的
virtual-voicebot-backend/ 配下の Rust プロジェクトだけを format（rustfmt）し、lint（clippy）を実行して結果を返す。

## 前提
- リポジトリ直下に virtual-voicebot-backend/ が存在する
- virtual-voicebot-backend/ に Cargo.toml がある（または workspace のメンバーとして解決できる）
- Rust toolchain が導入済み（rustfmt / clippy を含む）

## 手順
1) backend に移動
- cd virtual-voicebot-backend

2) format（チェック→必要なら適用）
- cargo fmt --all
  - ※差分確認だけしたい場合は: cargo fmt --all -- --check

3) lint（clippy）
- cargo clippy --all-targets --all-features -- -D warnings

## 成功条件
- cargo fmt がエラーなしで完走
- cargo clippy が exit code 0 で完走（warning も失敗扱い）

## 失敗時の対応
- rustfmt/clippy が無い:
  - rustup component add rustfmt clippy
- workspace で backend がメンバー扱いで動かない:
  - cd した場所に Cargo.toml があるか確認し、なければ Cargo.toml のあるディレクトリに移動して再実行
- clippy が落ちた:
  - 最初の数件のエラーを抜粋し、必要なら該当ファイルと行を開いて修正案を提示
