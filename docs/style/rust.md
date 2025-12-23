# Rust Style (Project)

このドキュメントは Rust 固有の細則です。共通基準はリポジトリ直下の `STYLE.md` を正本とします。  
矛盾がある場合は `STYLE.md` を優先し、本書は必要に応じて更新してください。

---

## 1. Tooling / Required checks

### 必須（PR前に通す）
- `cargo fmt --all`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --all-targets`

> 実行入口はプロジェクトの `make ci` / `just ci` に統一（個別コマンドの乱立を避ける）

### 推奨（必要に応じて）
- `cargo doc`（公開APIに触れた場合）
- `cargo deny` / `cargo audit`（依存追加・更新時）

---

## 2. Diff discipline（差分の規律）

- 目的外のリネーム・整形・移動は禁止（別PR）
- “ついでの改善”は極力しない（レビュー負担が増える）
- 大きな変更は分割する（目安：200〜400 LOC、5〜10 files）

---

## 3. Error handling

### 基本方針
- `panic!` / `unwrap()` / `expect()` は原則禁止（テスト除く）
- 失敗しうる処理は `Result<T, E>` で返す（呼び出し側が扱える形に）

### どのエラー型を使うか（推奨）
- ライブラリ：ドメインエラー（enum）を定義して返す
- アプリ/サービス：境界層では `anyhow::Result` などでまとめてもよい（ただしログとユーザー向け出力は整理）

### エラーメッセージ
- “ユーザー向け”と“内部向け”を混ぜない（外部に詳細を漏らさない）
- `map_err(|e| ...)` で文脈を足す場合、何が失敗したかがわかる文言にする

---

## 4. Logging / tracing

- ログは `tracing`（採用している場合）に統一
- 秘密情報（token/password/secret）や個人情報をログに出さない
- ループ内で大量に出るログはレベル・頻度を意識（debug/trace、サンプリング等）

推奨：
- 公開API/境界の入口で `info`、内部は `debug/trace` を基本
- エラー時は “原因 + 文脈” を出す（スタック的な情報は過剰に出さない）

---

## 5. Ownership / borrowing（読みやすさ優先）

- まずは読みやすい所有権設計にする（過度な最適化で `Rc<RefCell<_>>` 乱用しない）
- 引数は「必要最小限の権利」を渡す：
  - 読み取りのみ：`&T` / `&str`
  - 変更が必要：`&mut T`
  - 所有が必要：`T`
- 文字列は基本 `&str` で受け、保持が必要なら `String` にする

---

## 6. APIs & Modules

### Public API（破壊的変更は慎重に）
- `pub` を増やす前に設計意図を明確化
- `pub` 関数/型には doc comment（`///`）を付ける（少なくとも目的と使い方）
- 戻り値の型は呼び出し側が扱いやすい形に（`Result`、エラー型の安定性）

### モジュール構成
- “便利だから”で巨大な `mod.rs` に詰め込まない
- 1ファイルが肥大化したら責務で分割（目安：300〜500行で要検討）

---

## 7. Naming conventions

- 型/構造体/enum：名詞（`Request`, `Config`, `UserId`）
- 関数：動詞（`parse_*`, `build_*`, `validate_*`）
- boolean：`is_`, `has_`, `should_` を基本
- 略語は最小限（プロジェクトで共有される略語のみ）

---

## 8. Traits / Generics（汎用化の抑制）

- YAGNI：将来のための過剰なジェネリクス化は禁止
- まずは concrete type で書き、必要になってから抽象化する
- trait は “契約” なので、導入時は責務・利用箇所・代替案をPRに書く

---

## 9. `unsafe` policy

- 原則：`unsafe` を増やさない
- どうしても必要な場合：
  - `unsafe` は局所化（最小スコープ）
  - 安全性の不変条件（Safety invariant）をコメントで明記
  - 可能ならテスト・fuzz・検証（境界条件）を追加
  - “なぜ safe でできないか” をPR本文に書く

---

## 10. Concurrency / async

- `tokio` 等のランタイム前提の場合、ブロッキング処理を async 文脈に持ち込まない
- `Arc<Mutex<_>>` は最後の手段（まず設計で共有を減らす）
- `Send/Sync` 境界、キャンセル、タイムアウトは意識する（外部I/Oは特に）

---

## 11. Tests（Rust）

### 基本
- バグ修正は回帰テストを必須にする
- 境界条件（空、最大、異常系、タイムアウト等）を最低1つは含める

### テストの書き方
- Arrange / Act / Assert を意識（読みやすさ優先）
- 共有の準備コードはヘルパー関数化（ただし抽象化しすぎない）
- ランダム性が必要なら seed 固定で再現性を確保

---

## 12. Dependencies（依存追加）

- 新規依存追加は原則禁止（必要なら別PR + 理由 + 代替案 + 影響）
- 依存を増やす前に、標準ライブラリ/既存依存で解決できないか検討
- 依存更新はリスク（破壊的変更・脆弱性）をPRに明記

---

## 13. Examples (good patterns)

### Resultで返す（panicしない）
```rust
pub fn parse_port(s: &str) -> Result<u16, ParsePortError> {
    let port: u16 = s.parse().map_err(|_| ParsePortError::InvalidNumber)?;
    if port == 0 {
        return Err(ParsePortError::OutOfRange);
    }
    Ok(port)
}
