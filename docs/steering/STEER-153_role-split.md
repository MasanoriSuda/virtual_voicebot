# STEER-153: AI エージェント役割分担の変更と STEER 配置統一

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-153 |
| タイトル | AI エージェント役割分担の変更と STEER 配置統一 |
| ステータス | Merged |
| 関連Issue | #153 |
| 優先度 | P0 |
| 作成日 | 2026-02-11 |

---

## 2. ストーリー（Why）

### 2.1 背景

現行の運用では、Claude Code がドキュメントの作成から修正まで全てを担当しており、Claude Code の使用量が上限に早期到達する問題が発生していた。

### 2.2 目的

Claude Code の使用量を Draft 作成に集中させ、Review 以降の修正は Codex に委譲することで、使用量上限の問題を解決する。

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-11 |
| 起票理由 | Claude Code の使用量上限対策 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Sonnet 4.5 |
| 作成日 | 2026-02-11 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "役割分担の変更と STEER 配置統一" |

### 3.5 実装

| 項目 | 値 |
|------|-----|
| 実装者 | Claude Sonnet 4.5 |
| 実装日 | 2026-02-11 |
| 実装内容 | ドキュメント更新と STEER 配置移動 |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ者 | @MasanoriSuda |
| マージ日 | 2026-02-11 |
| コミットハッシュ | dd917bb |

---

## 4. スコープ

### 4.1 対象（含む）

- AGENTS.md (root): ドキュメント更新の扱いを全面改訂
- CLAUDE.md: 役割・禁止事項・責務の境界を明記
- AGENTS.md (backend): 修正権限・Status 更新権限を追加
- v-model.md (backend/frontend): RACI に「レビュー指摘対応」を追加
- TEMPLATE.md (backend/frontend): 修正権限・Status 更新ルールを追加
- STEER 配置統一: docs/STEER-*.md（root 直下）を正規3箇所に移動

### 4.2 対象外（除く）

- プロダクションコードの変更
- 本体仕様書（RD/BD/DD）の内容変更

---

## 5. 差分仕様（What / How）

### 5.1 役割分担の変更

| 成果物 | Claude Code | Codex | 条件 |
|--------|-------------|-------|------|
| ステアリング（新規Draft作成） | ✓ 担当 | ❌ 禁止 | - |
| ステアリング（Review時修正） | ❌ 禁止 | ✓ 可 | Status: Review/Approved + レビュー必須 |
| ステアリング（段取り更新） | - | ✓ 担当 | Status: Approved以降 |
| 本体仕様書（RD/BD/DD/UT等） | ✓ 担当 | △ 可 | **DOCS_OK 必須** |
| README/CONTRIBUTING/規約 | ✓ 担当 | △ 可 | **DOCS_OK 必須** |

### 5.2 STEER 配置統一

**正規配置（3箇所のみ）:**
- 横断（Frontend-Backend 連携）: `docs/steering/STEER-*.md`
- Backend 専用: `virtual-voicebot-backend/docs/steering/STEER-*.md`
- Frontend 専用: `virtual-voicebot-frontend/docs/steering/STEER-*.md`

**廃止:**
- `docs/STEER-*.md`（root 直下）→ 0件に整理完了

---

## 6. 受入条件（Acceptance Criteria）

- [x] AC-1: 「Codex による新規ステアリング作成は禁止」が明記されていること
- [x] AC-2: 軽微修正の範囲が適切に拡大されていること（Markdown整形等を含む）
- [x] AC-3: DOCS_OK の形式がパス列挙必須で明記されていること
- [x] AC-4: レビュー運用（PR チェックボックス、CODEOWNERS）が推奨として記載されていること
- [x] AC-5: Claude Code の「Review 以降編集禁止」が明記されていること
- [x] AC-6: 差分最小の原則が明記されていること
- [x] AC-7: Status 更新権限が明記されていること
- [x] AC-8: v-model.md（Backend/Frontend）の RACI に「レビュー指摘対応」フェーズが追加されていること
- [x] AC-9: TEMPLATE.md（Backend/Frontend）に修正権限・Status 更新ルールが追加されていること
- [x] AC-10: `docs/STEER-*.md`（root 直下）が 0件であること
- [x] AC-11: ステアリング配置ルールが AGENTS.md に明記されていること（正規3箇所）
- [x] AC-12: DOCS_INDEX.md / DOCS_POLICY.md が更新されていること

---

## 7. 影響範囲

### 7.1 変更ファイル

- AGENTS.md (root)
- CLAUDE.md
- virtual-voicebot-backend/AGENTS.md
- virtual-voicebot-backend/docs/process/v-model.md
- virtual-voicebot-backend/docs/steering/TEMPLATE.md
- virtual-voicebot-frontend/docs/process/v-model.md
- virtual-voicebot-frontend/docs/steering/TEMPLATE.md
- docs/DOCS_INDEX.md
- docs/DOCS_POLICY.md

### 7.2 移動ファイル

| 移動元 | 移動先 | 分類 |
|--------|--------|------|
| docs/STEER-099_frontend-mvp.md | virtual-voicebot-frontend/docs/steering/ | Frontend |
| docs/STEER-137_backend-integration-strategy.md | docs/steering/ | 横断 |
| docs/STEER-139_frontend-backend-sync-impl.md | docs/steering/ | 横断 |
| docs/STEER-140_rule-evaluation-engine.md | virtual-voicebot-backend/docs/steering/ | Backend |
| docs/STEER-141_actioncode-phase3.md | virtual-voicebot-backend/docs/steering/ | Backend |
| docs/STEER-142_ivr-db-schema-design.md | virtual-voicebot-backend/docs/steering/ | Backend |

---

## 8. リスク/ロールバック観点

| リスク | 影響 | 緩和策 |
|--------|------|--------|
| Codex がストーリー（§2）を勝手に変更 | Issue との不整合 | ストーリー変更を明示的に禁止 |
| Codex が新規ステアリングを作成 | Claude Code の責務侵食 | 新規作成を明示的に禁止 |
| 修正権限の混乱 | 責務の境界が曖昧になる | Claude Code またはオーナーのレビュー必須化 |
| 本体仕様書の無秩序な修正 | 仕様の整合性崩壊 | DOCS_OK 許可制の徹底 |

**ロールバック手順:**
- コミット dd917bb を `git revert` で取り消し
- 移動したファイルを元の位置に戻す
