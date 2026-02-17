---
name: backend-audit-orchestrator
description: Orchestrate 3 read-only backend audits in parallel and merge results. No file modifications.
---

## 目的
virtual-voicebot-backend/** の read-only 監査を 3 つの独立タスクに分割し、可能なら並列実行して統合レポートを返す。

## ガード（絶対）
- このスキル自体は変更系コマンドを実行しない（原則コマンド実行なし）
- 子スキル以外でのコマンド実行は禁止
- 対象は virtual-voicebot-backend/** のみ

## 実行計画（並列）
以下の3タスクを「別タスク」として同時に実行：
- Task A: $backend-audit-overview
- Task B: $backend-audit-risk
- Task C: $backend-audit-secrets

## 統合のルール
- A/B/C の結果をそのまま貼らず、要点だけ統合する
- 重複はまとめる
- Next steps は優先順位つき最大5つ

## 出力フォーマット
- Summary（3〜7行）
- Findings（A/B/C）
- Next steps（最大5個）
