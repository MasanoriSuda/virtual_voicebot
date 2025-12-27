# ワークスペース整合性レビュー結果（Codex）

**Refs**: Issue #7
**レビュー日**: 2025-12-27
**担当**: Codex（実装/テスト観点）
**対象**: 実装/テスト観点での整合性確認

---

## 概要

Issue #7 に基づき、実装/テスト観点での整合性を確認しました。依存方向/タイムアウト/ログ・観測要件を重点的に確認しています。

---

## 指摘一覧

### 🔴 重大（整合性/運用リスク）

| # | 指摘 | 根拠 | 修正案（最小差分） |
|---|------|------|------------------|
| I-1 | **ASR/Transcribe の全文ログがデフォルトで出力され、PII が平文で残る** | `virtual-voicebot-backend/src/ai/mod.rs:90,123,388` にて `info!` で全文（AWS/Whisper、RAW JSON）を出力。AGENTS.md §7「PIIはデフォルトでログに出さない」に違反 | `info!` を `debug!` へ変更し、RAW JSON ログは削除または `LOG_PII=1` 等のフラグでガード。必要なら `call_id` を付与してメタ情報のみ出力 |

### 🟡 中（運用/品質リスク）

| # | 指摘 | 根拠 | 修正案（最小差分） |
|---|------|------|------------------|
| I-2 | **AWS Transcribe のポーリングが無期限で継続し、外部I/Oのタイムアウト要件を満たさない** | `virtual-voicebot-backend/src/ai/mod.rs:373-404` の `loop` が終了条件を持たず、AGENTS.md §5「外部I/Oは必ずtimeout」「無限リトライ禁止」に抵触 | `tokio::time::timeout` で全体に制限を掛けるか、`Instant` + 最大経過時間で打ち切り。例: `let deadline = Instant::now() + config::timeouts().ai_http;` を越えたら `anyhow::bail!` |

### 🟢 軽（改善推奨）

- 該当なし

---

## 付記（確認範囲）

- `app/http/ai` → `sip/rtp/transport` 直接参照なし（依存方向は維持）
- `session` → `ai` 直接参照なし（app 経由）
- `test/` の E2E は `Cargo.toml` で登録済み
