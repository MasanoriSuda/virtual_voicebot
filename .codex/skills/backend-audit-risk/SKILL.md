---
name: backend-audit-risk
description: virtual-voicebot-backend/** のみを対象に、read-only で危険パターン（unwrap/expect/panic/unsafe等）を検出し要約（並列安全）。rg のみ使用。
metadata:
  short-description: backend risk scan (read-only)
---

## 目的
virtual-voicebot-backend/** のみを対象に、危険パターンを読み取り専用で検出し、件数と代表例を返す。

## ガード（絶対）
- 変更系コマンド禁止（編集、format、install、cargo build/test、git操作、rm/mv 等は禁止）
- 対象は virtual-voicebot-backend/** のみ
- コマンド実行前に必ず `cd virtual-voicebot-backend`
- 使ってよいコマンドは `rg` / `test` / `pwd` のみ
- 出力は「件数 + 代表例」形式
  - 各カテゴリ最大5件
  - 合計の抜粋は最大20行まで

## 実行手順（read-only）
0) スコープ確認
- `test -d virtual-voicebot-backend`
- `cd virtual-voicebot-backend`
- `pwd`（virtual-voicebot-backend 配下であることを確認）

1) 危険パターン検出（代表例だけ拾う）
- TODO系:
  - `rg -n "(TODO|FIXME|HACK)\b" -S .`
- unwrap/expect/panic:
  - `rg -n "\b(unwrap|expect)\b|panic!\s*\(" -S .`
- unsafe:
  - `rg -n "unsafe\s*\{" -S .`
- 広すぎる allow:
  - `rg -n "allow\((dead_code|unused|clippy::all)\)" -S .`

## 返すフォーマット
- Summary（3〜7行）
- Findings（カテゴリ別）
  - 件数（だいたいでOK）
  - 代表例（最大5件、ファイル:行:内容）
- Next steps（最大5個、優先順）
