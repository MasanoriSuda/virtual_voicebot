# Codex レビュー結果: Issue #92 関連

> BD-004 作成に先立つ既存コードベースのレビュー結果

| 項目 | 値 |
|------|-----|
| レビュー日 | 2026-02-02 |
| 関連Issue | #92 |
| レビュー実施 | Codex |
| 記録者 | Claude Code |

---

## 1. 指摘サマリ

| 重要度 | 件数 | 対応方針 |
|--------|------|----------|
| 重大 | 0 | - |
| 中 | 4 | BD-004 実装前に対応推奨（特に #3, #4） |
| 軽 | 2 | 後回し可 |

---

## 2. 指摘詳細（中）

### 2.1 SessionRegistry 共有Mutex問題

| 項目 | 内容 |
|------|------|
| 重要度 | 中 |
| 根拠箇所 | `types.rs:261-291`（`SessionMap = Arc<Mutex<HashMap<...>>>`） |
| 違反規約 | AGENTS.md:74-78（「1セッション=1タスク」「巨大Mutex禁止」） |
| 影響 | パフォーマンス、デッドロックリスク |

**最小差分案:**
- A) SessionRegistry を専用タスク + チャネル化（Actor化）
- B) 暫定として DashMap に置換してブロッキングMutex排除

---

### 2.2 CallId 二重定義

| 項目 | 内容 |
|------|------|
| 重要度 | 中 |
| 根拠箇所 | `identifiers.rs:3-14` と `types.rs:15-16` |
| 違反規約 | BD-003:102-106（DDD境界） |
| 影響 | 境界曖昧、保守性低下 |

**最小差分案:**
- `types.rs` の CallId を `entities::CallId` に置換
- 境界で変換する

---

### 2.3 STEER-085 設計と実装の乖離

| 項目 | 内容 |
|------|------|
| 重要度 | 中 |
| 根拠箇所 | STEER-085:526-598（SessionEvent/Command設計）vs `state_machine.rs:1-31`（薄いラッパのみ） |
| 実態 | 分岐が `session_coordinator.rs:334-420` に集中 |
| 影響 | 仕様と実装の不整合、IVR追加時に混乱 |

**最小差分案:**
- state machine に SessionEvent/Command を導入
- Coordinator から分岐を段階移行

---

### 2.4 SessionCoordinator God Object（1490行）

| 項目 | 内容 |
|------|------|
| 重要度 | 中（**BD-004 に直接影響**） |
| 根拠箇所 | `session_coordinator.rs:207-248`（責務多いフィールド）、`:334-420`（SIP/Outbound判断）、`:650-740`（B2BUA/IVR） |
| 違反規約 | BD-003（クリーンアーキテクチャ）、SRP |
| 影響 | IVR/コールルーティング追加で更に肥大化 |

**最小差分案:**
- IVR・B2BUA・Playback を別モジュールへ抽出
- Coordinator は orchestrate のみ残す

---

## 3. 指摘詳細（軽）

### 3.1 AiPort ドメインエラー喪失

| 項目 | 内容 |
|------|------|
| 重要度 | 軽 |
| 根拠箇所 | `ai_port.rs:3-24, 46-93` |
| 影響 | 観測性・拡張性に影響 |

**最小差分案:**
- 互換レイヤでも `AsrError` 等へマップ
- または `CompatError` enum で保持

---

### 3.2 RTP parser 規格外パケット対応

| 項目 | 内容 |
|------|------|
| 重要度 | 軽 |
| 根拠箇所 | `parser.rs:21-40` |
| 影響 | CSRC/extension 付きパケットで誤解釈リスク |

**最小差分案:**
- `csrc_count > 0` / `extension = true` は明示的に Err
- またはヘッダ長を計算してスキップ

---

## 4. 対応方針（提案）

### Phase 1: BD-004 実装前の磨き上げ（推奨）

| # | 対象 | 対応 | 理由 |
|---|------|------|------|
| 2.3 | STEER-085 乖離 | SessionEvent/Command 導入 | IVR状態管理と直結 |
| 2.4 | SessionCoordinator | IVR/B2BUA/Playback 分離 | BD-004 の IVR ロジック投入先を明確化 |

### Phase 2: BD-004 実装後

| # | 対象 | 対応 |
|---|------|------|
| 2.1 | SessionRegistry | Actor化 or DashMap |
| 2.2 | CallId 二重定義 | entities::CallId 統一 |
| 3.1 | AiPort | CompatError enum化 |
| 3.2 | RTP parser | CSRC/extension 対応 |

---

## 5. 次のアクション

- [ ] Phase 1 の Issue 起票（#2.3, #2.4）
- [ ] Phase 1 完了後、BD-004 実装を Codex へ引き継ぎ
- [ ] Phase 2 は別途バックログ化

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-02 | 初版作成 | Claude Code |
