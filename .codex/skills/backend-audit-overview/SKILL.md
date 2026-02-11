---
name: backend-audit-overview
description: virtual-voicebot-backend/** のみを対象に、read-only で構造と Cargo 概要を要約する（並列安全）。rg/find/ls/test のみ使用。
metadata:
  short-description: backend overview (read-only)
---

## 目的
virtual-voicebot-backend/** のみを対象に、読み取り専用で「構造」「Cargo.toml の概要」「主要モジュールの雰囲気」を短く要約して返す。

## ガード（絶対）
- 変更系コマンド禁止（編集、format、install、cargo build/test、git操作、rm/mv 等は禁止）
- 対象は virtual-voicebot-backend/** のみ
- コマンド実行前に必ず `cd virtual-voicebot-backend`
- 使ってよいコマンドは `ls` / `find` / `rg` / `test` / `pwd` のみ
- 出力は「要約 + 重要箇所の抜粋（最大20行）」に制限

## 実行手順（read-only）
0) スコープ確認
- `test -d virtual-voicebot-backend`
- `test -f virtual-voicebot-backend/Cargo.toml`
- `cd virtual-voicebot-backend`
- `pwd`（virtual-voicebot-backend 配下であることを確認）

1) 構造
- `ls -la`
- `find . -maxdepth 2 -type f \( -name "Cargo.toml" -o -name "README*" -o -name "*.rs" \) | head -n 200`

2) Cargo 概要
- `rg -n "^\s*\[package\]|\[workspace\]|\[dependencies\]|\[features\]|\[bin\]|\[lib\]" -S Cargo.toml **/Cargo.toml`

## 返すフォーマット
- Summary（3〜7行）
- Findings
  - Structure（要点）
  - Cargo overview（依存/feature/ワークスペースの雰囲気）
  - Extract（最大20行）
- Next steps（最大5個、優先順）
