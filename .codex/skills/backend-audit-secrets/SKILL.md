---
name: backend-audit-secrets
description: virtual-voicebot-backend/** のみを対象に、read-only で秘密/設定漏れっぽい箇所を検出して要約（並列安全）。rg/find/test のみ使用。
metadata:
  short-description: backend secrets scan (read-only)
---

## 目的
virtual-voicebot-backend/** のみを対象に、秘密情報・設定漏れの疑い（.env, pem, SECRET/TOKEN など）を読み取り専用で検出して要約する。

## ガード（絶対）
- 変更系コマンド禁止（編集、format、install、cargo build/test、git操作、rm/mv 等は禁止）
- 対象は virtual-voicebot-backend/** のみ
- コマンド実行前に必ず `cd virtual-voicebot-backend`
- 使ってよいコマンドは `rg` / `find` / `test` / `pwd` のみ
- 出力は「要約 + 抜粋（最大20行）」に制限
- 機密っぽい値は必ずマスクして表示（例: TOKEN=****、鍵ブロックは先頭行のみ等）

## 実行手順（read-only）
0) スコープ確認
- `test -d virtual-voicebot-backend`
- `cd virtual-voicebot-backend`
- `pwd`（virtual-voicebot-backend 配下であることを確認）

1) 文字列スキャン（見つけたらマスクして報告）
- AWS Access Key っぽい/秘密鍵ヘッダ:
  - `rg -n "(AKIA[0-9A-Z]{16}|-----BEGIN (RSA|OPENSSH) PRIVATE KEY-----)" -S .`
- 変数名っぽいもの:
  - `rg -n "(API_KEY|SECRET|TOKEN|PASSWORD)\s*[:=]" -S .`

2) ファイル名スキャン
- `find . -maxdepth 3 -type f \( -name ".env*" -o -name "*secret*" -o -name "*.pem" \)`

3) .gitignore チェック
- `test -f .gitignore && rg -n "(\.env|target/|\.pem)" -S .gitignore`

## 返すフォーマット
- Summary（3〜7行）
- Findings
  - Strings（マスク済みの抜粋、最大20行）
  - Files（該当ファイル一覧）
  - Gitignore（該当ルールの有無）
- Next steps（最大5個、優先順）
