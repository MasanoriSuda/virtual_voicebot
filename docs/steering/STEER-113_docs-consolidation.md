# STEER-113: 開発ガイドライン統合（CONTRIBUTING / STYLE / rust.md → DEVELOPMENT_GUIDE）

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-113 |
| タイトル | 開発ガイドライン統合（SoT 再作成 ver2） |
| ステータス | Draft |
| 関連Issue | #113 |
| 優先度 | P1 |
| 作成日 | 2026-02-07 |

---

## 2. ストーリー（Why）

### 2.1 背景

V 字モデル体系への移行（Issue #112 / STEER-112）により、Backend の正本は RD/BD/DD 体系に整理された。
しかし、**開発ガイドライン系ドキュメント**は旧体系のまま残っており、以下の問題がある：

1. **内容の重複**: CONTRIBUTING.md / STYLE.md / docs/style/rust.md / DEVELOPMENT_GUIDE.md の4ファイルに同一内容が分散
2. **正本の不明確さ**: 矛盾時にどのファイルを優先すべきか定義されていない
3. **更新漏れリスク**: 同じルールが複数箇所に記載されているため、片方だけ更新される危険がある

### 2.2 目的

4ファイルの内容を **DEVELOPMENT_GUIDE.md（既存・SOURCE_OF_TRUTH ヘッダーあり）** に統合し、
開発ガイドラインの正本を一本化する。

### 2.3 スコープ

| 対象 | やる/やらない |
|------|:----------:|
| CONTRIBUTING.md → DEVELOPMENT_GUIDE.md 統合 | やる |
| STYLE.md → DEVELOPMENT_GUIDE.md 統合 | やる |
| docs/style/rust.md → DEVELOPMENT_GUIDE.md 統合 | やる |
| 統合元3ファイルの削除 | やる |
| DOCS_INDEX.md / DOCS_POLICY.md の参照更新 | やる |
| DEVELOPMENT_GUIDE.md の内容リファクタリング | やらない（統合のみ、内容の書き換えは別イシュー） |
| root AGENTS.md の変更 | やらない（現状維持） |

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-07 |
| 起票理由 | Codex レビュー（#112 対応中）で旧体系ドキュメントの重複を発見 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-opus-4-6) |
| 作成日 | 2026-02-07 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "ステアリング自体はつくるべきだとおもっていますが、いかが？" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | | | | |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | |
| 承認日 | |
| 承認コメント | |

### 3.5 実装

| 項目 | 値 |
|------|-----|
| 実装者 | Claude Code（docs のみの変更のためコード変更なし） |
| 実装日 | |
| 指示者 | @MasanoriSuda |
| 指示内容 | |
| コードレビュー | N/A（docs のみ） |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | |
| マージ日 | |
| マージ先 | DEVELOPMENT_GUIDE.md, DOCS_INDEX.md, DOCS_POLICY.md |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| `virtual-voicebot-backend/docs/DEVELOPMENT_GUIDE.md` | 修正 | 統合先。CONTRIBUTING / STYLE / rust.md の内容を追記 |
| `CONTRIBUTING.md` | 削除 | 内容を DEVELOPMENT_GUIDE.md に統合後に削除 |
| `STYLE.md` | 削除 | 内容を DEVELOPMENT_GUIDE.md に統合後に削除 |
| `docs/style/rust.md` | 削除 | 内容を DEVELOPMENT_GUIDE.md に統合後に削除 |
| `docs/DOCS_INDEX.md` | 修正 | 削除ファイルの参照除去、DEVELOPMENT_GUIDE.md の記載を「統合済み」に更新 |
| `docs/DOCS_POLICY.md` | 修正 | §2 ツリーから削除ファイルを除去、§3.3 正本一覧を更新 |

### 4.2 影響するコード

なし（docs のみの変更）

---

## 5. 差分仕様（What / How）

### 5.1 統合方針

統合は「足りない内容の追加」のみ行い、既存の DEVELOPMENT_GUIDE.md の構造を壊さない。

#### 重複分析結果

| DEVELOPMENT_GUIDE.md の節 | CONTRIBUTING.md | STYLE.md | docs/style/rust.md | 統合アクション |
|--------------------------|:-:|:-:|:-:|------|
| §1 コーディング規約 | - | §2, §3, §4, §5 | §2〜§9 全体 | **重複**（既に網羅済み）→ 追加不要 |
| §2 相関 ID 規約 | - | - | - | DEVELOPMENT_GUIDE 固有 → 維持 |
| §3 モジュール設計ルール | - | - | - | DEVELOPMENT_GUIDE 固有 → 維持 |
| §4 テスト規約 | CONTRIBUTING §DoD | §6 | §11 | **重複** → 追加不要 |
| §5 Git ワークフロー | CONTRIBUTING §Workflow | §1 | §2 | **重複** → 追加不要 |
| §6 ドキュメント管理 | - | §9 | - | **重複** → 追加不要 |
| §7 セキュリティ | - | §4.2 | §4 | **重複** → 追加不要 |
| §8 運用・監視 | - | - | - | DEVELOPMENT_GUIDE 固有 → 維持 |
| §9 開発環境セットアップ | - | - | - | DEVELOPMENT_GUIDE 固有 → 維持 |
| §10 トラブルシューティング | - | - | - | DEVELOPMENT_GUIDE 固有 → 維持 |
| （未収録）PR ワークフロー | CONTRIBUTING §Workflow, §Review | §1, §10 | - | **追加**: §5 に PR チェックリスト/レビュー観点を補完 |
| （未収録）AI 利用ルール | CONTRIBUTING §AI-assisted | - | - | **追加**: §5 に AI 開発ルールを追加 |
| （未収録）Breaking changes | CONTRIBUTING §Breaking | §7 | - | **追加**: §5 or 新節に互換性ルールを追加 |
| （未収録）unsafe ポリシー | - | - | §9 | **追加**: §1 に unsafe ポリシーを追加 |
| （未収録）依存管理 | - | §8 | §12 | **追加**: 新節 or §1 に依存追加ルールを追加 |

#### 追加予定の内容

**§1.5 unsafe ポリシー（docs/style/rust.md §9 より）**
- 原則 `unsafe` を増やさない
- 必要な場合: 局所化、Safety invariant コメント、テスト追加、PR 本文に理由

**§5.4 PR レビュー観点（CONTRIBUTING.md §Review より）**
- 仕様/境界条件/エラーハンドリングの妥当性
- 可読性（説明可能か）
- テストカバレッジ
- 互換性・セキュリティ・データ安全性

**§5.5 AI 活用ルール（CONTRIBUTING.md §AI-assisted より）**
- PR 本文に AI に任せた作業を 1〜2 行で記載
- 不明瞭なコード/説明不能な抽象化は差し戻し対象
- テスト実行は人間が行う（明示依頼時を除く）

**§5.6 Breaking changes（CONTRIBUTING.md §Breaking より）**
- 公開 API / 設定ファイル / データスキーマ / FFI 境界の変更は設計メモ→実装 PR の順

**§12 依存管理（STYLE.md §8 + docs/style/rust.md §12 より）**
- 新規依存追加は慎重に（理由・代替案・影響を PR に記載）
- 標準ライブラリ/既存依存での解決を先に検討

### 5.2 削除するファイル

| ファイル | 削除条件 |
|---------|---------|
| `CONTRIBUTING.md` | §5 への統合完了後 |
| `STYLE.md` | 全内容が DEVELOPMENT_GUIDE.md に網羅されていることを確認後 |
| `docs/style/rust.md` | 全内容が DEVELOPMENT_GUIDE.md に網羅されていることを確認後 |

### 5.3 参照更新

**DOCS_INDEX.md**:
- §2 テーブルから `style/rust.md` 行を削除
- §1 テーブルに CONTRIBUTING.md / STYLE.md が含まれていれば削除
- §5 旧体系ファイルテーブルに CONTRIBUTING.md / STYLE.md / docs/style/rust.md を追加（移行先: DEVELOPMENT_GUIDE.md）

**DOCS_POLICY.md**:
- §2 ツリーから `style/rust.md` を削除
- §3.3 に影響があれば更新

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #113 | STEER-113 | 起票 |
| Issue #112 | Issue #113 | 先行（SoT 再構築の延長） |
| STEER-113 | DEVELOPMENT_GUIDE.md | 統合先 |
| STEER-113 | DOCS_INDEX.md | 参照更新 |
| STEER-113 | DOCS_POLICY.md | 参照更新 |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] 統合方針が明確か（何を追加し、何を追加しないか）
- [ ] 削除ファイルの全内容が統合先でカバーされているか
- [ ] DOCS_INDEX / DOCS_POLICY の更新漏れがないか
- [ ] 他ファイルから削除ファイルへの参照が残っていないか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] DEVELOPMENT_GUIDE.md に追加節が反映されている
- [ ] CONTRIBUTING.md / STYLE.md / docs/style/rust.md が削除されている
- [ ] DOCS_INDEX.md / DOCS_POLICY.md の参照が更新されている
- [ ] `grep -r "CONTRIBUTING\|STYLE\.md\|style/rust" --include="*.md"` で孤立参照がないこと

---

## 8. 備考

- DEVELOPMENT_GUIDE.md は Backend 配下（`virtual-voicebot-backend/docs/`）にあるが、統合元の CONTRIBUTING.md / STYLE.md はリポジトリルートにある。統合後は DEVELOPMENT_GUIDE.md が**リポジトリ全体の開発ガイドライン正本**となる点に注意。
- Frontend 固有の開発ガイドラインが必要になった場合は、`virtual-voicebot-frontend/docs/DEVELOPMENT_GUIDE.md` を別途作成すること（本ステアリングのスコープ外）。

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-07 | 初版作成 | Claude Code (claude-opus-4-6) |
