<!-- SOURCE_OF_TRUTH: Codex単独運用共通指示 -->
# CLAUDE.md（Codex 単独運用互換ファイル）

## 役割（Codex）
- 本ファイルは歴史的なファイル名を維持しているが、現在の正は **Codex 単独運用の共通指示**です。
- Codex は、このリポジトリでは **ドキュメント/仕様/実装/テスト担当**です。
- 取り扱う成果物：
  - プロセス定義：virtual-voicebot-backend/docs/process/** , virtual-voicebot-frontend/docs/process/**
  - 要件仕様：docs/requirements/** , virtual-voicebot-backend/docs/requirements/** , virtual-voicebot-frontend/docs/requirements/**
  - 設計書：docs/design/** , virtual-voicebot-backend/docs/design/** , virtual-voicebot-frontend/docs/design/**
  - テスト仕様：virtual-voicebot-backend/docs/test/** , virtual-voicebot-frontend/docs/test/**
  - **ステアリング（差分仕様）：全フェーズ担当**
    - 新規ステアリングの作成（Status: Draft）
    - Review 時のレビュー指摘対応・修正（Status: Review）
    - 配置先: Backend → `virtual-voicebot-backend/docs/steering/`
    - 配置先: Frontend → `virtual-voicebot-frontend/docs/steering/`
    - 配置先: 横断（Frontend-Backend 連携）→ `docs/steering/`
  - README/CONTRIBUTING/規約ドキュメント
  - プロダクションコード、テスト、設定ファイル
- プロダクションコードの実装・修正は [virtual-voicebot-backend/AGENTS.md](virtual-voicebot-backend/AGENTS.md) の実装規約に従う。

### Codex の禁止事項

- **Status を勝手に変更しない**（Status 更新は人間が判断）
- **ストーリー（§2）変更は必ず Issue で合意を得てから行う**
- **意味が変わる docs 編集は `DOCS_OK` なしで行わない**（軽微修正を除く）

### 責務の境界（Codex 単独運用）

| 成果物 | Codex | オーナー | 条件 |
|--------|--------|----------|------|
| ステアリング（新規Draft作成） | ✓ 担当 | Status 判断 | Issue 参照必須 |
| ステアリング（Review時修正） | ✓ 担当 | 方針承認 | `DOCS_OK` 必須 |
| ステアリング（段取り更新） | ✓ 担当 | 必要時確認 | Status: Approved以降 |
| 本体仕様書（RD/BD/DD/UT等） | ✓ 担当 | 方針承認 | **DOCS_OK 必須** |
| README/CONTRIBUTING/規約 | ✓ 担当 | 方針承認 | **DOCS_OK 必須** |
| プロダクションコード | ✓ 担当 | レビュー/承認 | Issue 参照必須 |

## 開発アプローチ

本プロジェクトは **ストーリー駆動 × 仕様駆動** のハイブリッドを採用する。
詳細は [プロセス定義書](virtual-voicebot-backend/docs/process/v-model.md) を参照。

- **ストーリー駆動**: イシュー/ステアリング単位で Why/Who/When を管理
- **仕様駆動**: 詳細仕様を先に定義してからAIに実装させる

## AI エージェントの責務分担

| 観点 | Codex | CodeRabbit | オーナー |
|------|--------|------------|----------|
| **主担当** | 仕様/ドキュメント整合性、実装/テスト | コードレビュー | Status 判断、方針承認 |
| **レビュー対象** | docs 矛盾、V字トレース、依存方向、非同期境界、timeout/backpressure | コード品質、セキュリティ、パフォーマンス | 要件・運用判断 |
| **出力** | RD/BD/DD/ステアリング、質問リスト、最小差分コード修正 | PR コメント、改善提案 | 承認/差し戻し |
| **コード変更** | ✓ する | ❌ しない（指摘のみ） | 必要時のみ |
| **詳細定義** | 本ファイル (CLAUDE.md), [virtual-voicebot-backend/AGENTS.md](virtual-voicebot-backend/AGENTS.md) | GitHub App 設定 | Issue/PR コメント |

## 合意（チケット）ゲート
- **合意（チケット参照）が確認できない限り、作業を開始しないでください。**
- チケット参照の例：Issue番号、PR本文の `Refs #123`、チケットURL 等（本プロジェクトの運用に従う）
- 合意が無い場合は **一切の変更を行わず**、必ず次の文面のみ返してください：

> 合意（チケット参照）が確認できないため、ドキュメント作業を開始できません。  
> 先にチケット化し、参照（例：Refs #123）を提示してください。

## 仕様化の方針
- 推測で仕様を埋めない（「念のため」「今後の拡張」を理由に追加要求を捏造しない）。
- 事実・根拠（現状コード/ログ/既存仕様/チケット）に基づいて記述する。
- 仕様は「境界（入力/出力/エラー）」と「不変条件（内部で常に成り立つ前提）」を明確にする。

## 成果物の出力形式（必須）
ドキュメント変更を提案する場合は、必ず以下をセットで出してください：
1. 変更内容（差分または更新後テキスト）
2. 受入条件（Acceptance Criteria）
3. 未確定点・質問リスト（Open Questions）
4. リスク/ロールバック観点（必要に応じて）

## 追加：レビュー任務（必須）
Codex は VSCode ワークスペース全体のレビューを行い、Issue 参照と必要な `DOCS_OK` が確認できる範囲で修正する。

### レビュー観点
- docs（design/contract/recording）とコードの矛盾がないか
- 責務境界（http/session/media/ai）に違反がないか（禁止事項の確認）
- 仕様の不足・曖昧さ・将来の事故ポイント（受入条件に落ちるか）
- V字観点：要求→設計→テストのトレースが成立しているか
- 運用観点：ログ/監視/フォールバック/ロールバックが説明できるか

### レビュー出力フォーマット（必須）
- 指摘（重大/中/軽） + 根拠（該当 docs の章/該当コード箇所）
- 仕様/ドキュメントの修正案（文章案）
- 必要なら「質問リスト」（Yes/Noで決められる形）

## レビュー結果の記録（必須）
- レビュー結果は必ず PR 本文に要約（重大/中/軽 + 根拠）として残す。
- レビュー全文は `docs/reviews/YYYY-MM-DD_issue-<n>.md` に保存する（または保存案を提示する）。
- 同じ指摘が2回以上出た場合は、再発防止として `CLAUDE.md` / `AGENTS.md` / `CONVENTIONS.md` のいずれかにルールとして追記提案する。追記は `DOCS_OK` の範囲で行う。

## ステアリング運用

変更作業はイシュー単位で **ステアリングファイル** を作成して進める。

### ステアリングファイルの役割
- ストーリー（Why/Who/When）と差分仕様（What/How）を一元管理
- 承認後に本体仕様書（RD/DD/UT等）へマージ

### テンプレート
- Backend: [virtual-voicebot-backend/docs/steering/TEMPLATE.md](virtual-voicebot-backend/docs/steering/TEMPLATE.md)
- Frontend: [virtual-voicebot-frontend/docs/steering/TEMPLATE.md](virtual-voicebot-frontend/docs/steering/TEMPLATE.md)

### 運用フロー
1. イシュー起票
2. ステアリングファイル作成（Draft）
3. レビュー（Review）
4. 承認（Approved）
5. 実装（Codex が継続して担当）
6. コードレビュー（CodeRabbit が自動実施）
7. マージ（Merged）→ 本体仕様書へ反映

詳細は [プロセス定義書 §5](virtual-voicebot-backend/docs/process/v-model.md) を参照。

## プロセス参照ドキュメント

### Backend

| ドキュメント | 説明 |
|-------------|------|
| [プロセス定義書](virtual-voicebot-backend/docs/process/v-model.md) | V字モデル・成果物・ガバナンス定義 |
| [品質ゲート定義](virtual-voicebot-backend/docs/process/quality-gate.md) | フェーズ移行条件 |
| [ステアリングテンプレート](virtual-voicebot-backend/docs/steering/TEMPLATE.md) | 差分仕様のテンプレート |

### Frontend

| ドキュメント | 説明 |
|-------------|------|
| [プロセス定義書](virtual-voicebot-frontend/docs/process/v-model.md) | Frontend V字モデル・成果物定義 |
| [品質ゲート定義](virtual-voicebot-frontend/docs/process/quality-gate.md) | Frontend フェーズ移行条件 |
| [ステアリングテンプレート](virtual-voicebot-frontend/docs/steering/TEMPLATE.md) | Frontend 差分仕様のテンプレート |
