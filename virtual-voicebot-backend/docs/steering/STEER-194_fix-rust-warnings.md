# STEER-194: Rust warning の解消

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-194 |
| タイトル | Rust warning の解消 |
| ステータス | Approved |
| 関連Issue | #194 |
| 優先度 | P2 |
| 作成日 | 2026-02-17 |

---

## 2. ストーリー（Why）

### 2.1 背景

現在、Backend コードベースで大量の Rust warning が発生している。

**warning の種類と影響**:

| 種別 | 説明 | 影響 |
|------|------|------|
| unused import | 使用されていない import 文 | コードの可読性低下、ビルド時間増加 |
| unused field | 読み取られていない構造体フィールド | 潜在的なバグの温床 |
| unused function | 使用されていない関数 | dead code の蓄積 |
| type privacy | 型の可視性の不整合 | API 設計の問題 |

**warning のベースライン**（2026-02-17 時点）:

| コマンド | warning 数 |
|---------|-----------|
| `cargo build --workspace` | 201 件 |
| `cargo clippy --workspace` | 247 件 |

**主要カテゴリ**（cargo build）:
1. unused function: 48 件
2. unused struct: 46 件
3. unused enum: 18 件
4. unused imports: 13 件
5. unused type alias: 10 件

### 2.2 目的

**運用上の問題を解消する**:

1. **不具合の見逃し防止**: 重要な warning が大量の warning に埋もれる
2. **CI/CD の健全性**: warning が多いと CI が通らない、または信頼性が低下
3. **開発効率の向上**: クリーンなビルド状態を維持し、新しい warning を即座に検知

### 2.3 ユーザーストーリー

```
As a Backend 開発者
I want to warning のないクリーンなビルド状態を維持したい
So that 新しい問題を即座に検知でき、コード品質を保てる

受入条件:
- [ ] `RUSTFLAGS="-D warnings" cargo build --workspace --all-targets --all-features` で warning が0件
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` で warning が0件
- [ ] 既存のテストがすべて PASS
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-17 |
| 起票理由 | 大量の warning により不具合を見逃す可能性、CI 運用上の問題 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Sonnet 4.5 |
| 作成日 | 2026-02-17 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "https://github.com/MasanoriSuda/virtual_voicebot/issues/194を立てました。詳細はcodexに任せればいいと思います" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|------------|
| 1 | | | | |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-17 |
| 承認コメント | コマンド修正、#[allow] 方針厳格化、スコープ整合済み |

### 3.5 実装（該当する場合）

| 項目 | 値 |
|------|-----|
| 実装者 | Codex (GPT-5) |
| 実装日 | |
| 指示者 | |
| 指示内容 | |
| コードレビュー | |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | |
| マージ日 | |
| マージ先 | - |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| - | - | ドキュメント変更なし（コード修正のみ） |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| src/**/*.rs | 修正 | unused import/field/function の削除 |
| src/**/*.rs | 修正 | 型の可視性修正（pub(crate) 等） |
| Cargo.toml | 修正 | 必要に応じて依存関係の整理 |

---

## 5. 差分仕様（What / How）

### 5.1 修正方針

**基本方針**: Codex に委譲するが、以下のガイドラインに従う

#### 5.1.1 unused import の対処

**優先順位1**: 完全に不要な import は削除

```rust
// 削除
use crate::protocol::sip::SipResponseBuilder; // 使用されていない
```

**`#[allow]` の使用は原則禁止**: 将来使用予定のコードでも、実装時に追加する方針とする。

例外的に `#[allow]` を使用する場合は、以下を必須とする：
- Issue 番号付きコメント
- 期限付き TODO コメント

```rust
// 例外的に許可する場合のみ
#[allow(unused_imports)]
use crate::protocol::sip::SipResponseBuilder; // TODO(#200): PRACK 実装時に使用予定（期限: 2026-03-31）
```

**優先順位2**: 一部のみ使用されている import は整理

```rust
// 修正前
use crate::shared::ports::{CallActionsPayload, CallerGroup, /* ... 10個 */};

// 修正後（使用されているもののみ）
use crate::shared::ports::CallerGroup;
```

#### 5.1.2 unused field の対処

**Option 1**: フィールドを実処理に接続する（推奨）

```rust
// ログ出力、DB 保存、API レスポンスなどの実処理に接続
log::debug!("Session {} started", self.session_id);
// または
call_log.session_id = Some(self.session_id);
```

**Option 2**: フィールドを削除（本当に不要な場合のみ）

```rust
// 削除
// pub session_id: Uuid,
```

**Option 3**: 意図をコメントして `#[allow(dead_code)]` で許可（最終手段、原則禁止）

```rust
// 例外的に許可する場合のみ
#[allow(dead_code)]
pub session_id: Uuid, // TODO(#200): call_log 永続化時に使用予定（期限: 2026-03-31）
```

#### 5.1.3 type privacy の対処

**問題例**:
```
warning: type `SessState` is more private than the item `SessionCommand::Transition::0`
```

**修正**:
```rust
// 修正前
enum SessState { /* ... */ }
pub enum SessionCommand {
    Transition(SessState), // SessState が private だが public で露出
}

// 修正後
pub(crate) enum SessState { /* ... */ }
pub(crate) enum SessionCommand {
    Transition(SessState),
}
```

#### 5.1.4 優先順位

1. **P0**: 型の可視性の修正（API 設計の問題）
2. **P1**: unused import の削除（ビルド時間、可読性）
3. **P2**: unused field/function の削除または `#[allow]` 追加

### 5.2 受入条件

```markdown
## AC-1: warning ゼロ化

- [ ] `RUSTFLAGS="-D warnings" cargo build --workspace --all-targets --all-features` で warning が0件
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` で warning が0件
- [ ] 既存のテストがすべて PASS（`cargo test --workspace`）

## AC-2: コードレビュー

- [ ] 削除したコードが本当に不要か確認
- [ ] `#[allow(dead_code)]` の使用が最小限か確認（原則禁止、例外は Issue 番号 + 期限付き TODO 必須）
- [ ] 型の可視性が API 設計として正しいか確認
```

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #194 | STEER-194 | 起票 |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] 修正方針が明確か
- [ ] 削除基準が適切か
- [ ] `#[allow]` の使用方針が明確か

### 7.2 マージ前チェック（Approved → Merged）

- [ ] 実装が完了している
- [ ] コードレビューを受けている
- [ ] すべてのテストが PASS
- [ ] warning が0件になっている

---

## 8. 備考

### 8.1 スコープ外

以下は本ステアリングのスコープ外とし、将来対応とする：

- **CI での warning チェック強制化**: 以下のコマンドを CI に追加
  - `RUSTFLAGS="-D warnings" cargo build --workspace --all-targets --all-features`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - 理由: 今回は既存 warning の解消に集中し、CI 統合は別 Issue で対応
- **clippy lint の厳格化**: `clippy::pedantic` などの追加
- **新規 warning の防止**: pre-commit hook の導入

### 8.2 参考: 現在の warning 一覧

**cargo build 実行結果**（抜粋）:

```
warning: unused import: `SipResponseBuilder`
warning: type `SessState` is more private than the item `SessionCommand::Transition::0`
warning: field `query` is never read
warning: fields `id`, `session_id`, `from`, `to`, and `recordings` are never read
warning: unused imports: `CallActionsPayload`, `CallerGroup`, ...
warning: multiple associated functions are never used
warning: function `map_folder_read_err` is never used
```

**主な warning の所在**:
- `src/protocol/session/`: SessionCommand, SessState の可視性
- `src/main.rs`: 大量の unused import
- `src/shared/entities/`: unused field

### 8.3 実装時の注意点

**Codex への指示**:

1. **段階的な修正**: 一度にすべて修正せず、種別ごとに段階的に実施
   - ステップ1: 型の可視性修正
   - ステップ2: unused import 削除
   - ステップ3: unused field/function 対処

2. **テスト実行**: 各ステップで `cargo test --workspace` を実行

3. **最小差分**: 削除のみに留め、大きなリファクタリングは避ける

4. **`#[allow]` は原則禁止**: 例外的に使用する場合は Issue 番号 + 期限付き TODO 必須

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-17 | 初版作成 | Claude Sonnet 4.5 |
| 2026-02-17 | レビュー指摘対応: ベースライン明確化、コマンド修正、#[allow] 方針厳格化、AC-3 削除 | Claude Sonnet 4.5 |
| 2026-02-17 | 承認（ステータス: Approved） | @MasanoriSuda |
