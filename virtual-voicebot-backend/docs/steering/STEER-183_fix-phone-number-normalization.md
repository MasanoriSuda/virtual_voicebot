# STEER-183: 電話番号正規化不具合修正

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-183 |
| タイトル | 電話番号正規化不具合修正（090... → E.164 変換対応） |
| ステータス | Approved |
| 関連Issue | #183 |
| 優先度 | P0 |
| 作成日 | 2026-02-16 |

---

## 2. ストーリー（Why）

### 2.1 背景

**問題**:
- 番号通知あり（例: 09028894539）で着信したにもかかわらず、以下の不具合が発生している：
  1. **ルール評価失敗**: RuleEvaluator の `normalize_phone_number()` が E.164 形式（+始まり）のみ許可しているため、090... 形式の番号が `InvalidPhoneNumber` エラーになる
  2. **レガシー IVR への誤遷移**: ルール評価失敗時に defaultAction へフォールバックせず、既定分岐で legacy IVR に遷移してしまう
  3. **履歴の非通知化**: call_logs.caller_number の抽出も E.164 限定の正規化を使用しているため、090... は NULL 保存 → Frontend で「非通知」表示される

**根本原因**:
- 現行実装（STEER-140）では電話番号を E.164 形式（+8190...）前提で処理している
- 実際の SIP INVITE の Caller ID は日本国内番号（090...）で来る可能性があるが、国内番号→E.164 変換ロジックが存在しない
- ルール評価失敗時のフォールバック先が明確に実装されていない

**影響**:
- 番号通知ありの着信が正しく処理されず、ユーザー体験が著しく低下
- 通話履歴が「非通知」として記録され、後から発信者を特定できない
- 登録済み番号・番号グループのルールが適用されず、既定動作になる

### 2.2 目的

Caller ID の電話番号正規化ロジックを修正し、日本国内番号（090...）を E.164 形式（+8190...）に変換できるようにする。

**達成目標**:
- 090... 形式の Caller ID が +8190... に正規化され、ルール評価が正常動作する
- **電話番号正規化失敗時**に defaultAction へフォールバックする（legacy IVR 直行を防ぐ）
- call_logs.caller_number に正規化後の番号が保存される（実装箇所は調査中）
- 真の非通知（空文字/"anonymous"/"withheld"）は従来どおり anonymousAction で処理される

**注**: DB アクセスエラーなど、正規化以外の評価失敗時は従来どおり `Err` を返す

### 2.3 ユーザーストーリー

```
As a システム管理者
I want to 日本国内番号（090...）からの着信が正しくルール評価され、登録済み番号として扱われる
So that 番号グループごとの設定（録音あり/なし、拒否など）が適用され、通話履歴にも正しく記録される

受入条件:
- [ ] 09028894539 で着信した場合、+819028894539 に正規化される
- [ ] 正規化後の番号で registered_numbers / call_action_rules / routing_rules の評価が行われる
- [ ] 評価結果に基づいて正しい ActionCode が実行される（legacy IVR 直行しない）
- [ ] call_logs.caller_number に +819028894539 が保存される（NULL にならない）
- [ ] 空文字/"anonymous"/"withheld" は従来どおり anonymousAction で処理される
- [ ] 不正な文字列（例: "abc"）は normalize_phone_number_e164() で InvalidPhoneNumber エラー、evaluate() は Ok(defaultAction) を返す
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-16 |
| 起票理由 | 番号通知あり着信が非通知扱いになる不具合が発生（#183） |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-sonnet-4-5) |
| 作成日 | 2026-02-16 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Issue #183 のステアリングファイル作成" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | Codex | 2026-02-16 | 要修正 | 重大2件（フォールバック要件矛盾、RoutingError 参照先不整合）、中2件（テスト対象混在、call log 未確定）、軽1件（mod.rs 新規表記） → 全て修正完了 |
| 2 | Codex | 2026-02-16 | OK | 前回 NG 項目全件解消。フォールバック範囲明確化、RoutingError pub use 整理、UT 統一、mod.rs 修正表記に変更。Approved 判定可能。 |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-16 |
| 承認コメント | Codex レビュー通過（2回実施、全指摘対応完了）。実装フェーズへ引き継ぎ。 |

### 3.5 実装（該当する場合）

| 項目 | 値 |
|------|-----|
| 実装者 | - |
| 実装開始日 | - |
| 実装完了日 | - |
| PR番号 | - |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ日 | - |
| マージ先 | - |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| RD-004 | 修正 | FR-1.2（番号正規化）に日本国内番号→E.164 変換を追加 |
| DD-004-XX（新規） | 追加 | normalize_phone_number() の詳細設計（変換ロジック） |
| UT-004-XX（新規） | 追加 | normalize_phone_number() のテストケース |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| src/service/routing/evaluator.rs | 修正 | normalize_phone_number() の変換ロジック修正（090... → +8190...） |
| src/service/routing/evaluator.rs | 修正 | evaluate() のエラーハンドリング修正（正規化失敗時に defaultAction を返す） |
| src/service/routing/mod.rs | 修正 | normalize_phone_number_e164() 公開関数追加、RoutingError を pub use |
| （SessionCoordinator 系） | 調査中 | call log 保存時の caller_number 抽出を normalize_phone_number_e164() で共通化（実装時に確定） |

---

## 5. 差分仕様（What / How）

### 5.1 要件修正（RD-004 へマージ）

#### FR-1.2: 番号正規化（修正）

```markdown
## FR-1.2: 番号正規化

発信者番号（Caller ID）と DB 内の電話番号は **E.164 形式**（`+819012345678`）で比較する。

**正規化ルール**:
1. 空白 ` `、ハイフン `-`、括弧 `()`、`+` 以外の記号を除去
2. 以下の順序で E.164 形式に変換：
   - 既に `+` で始まる場合 → そのまま（E.164 形式と判定）
   - `0` で始まる日本国内番号（例: 090...）→ `+81` + 先頭の `0` を除去（例: +8190...）
   - その他 → 変換不可（InvalidPhoneNumber エラー）
3. 変換後の形式チェック:
   - `+` で始まること
   - `+` を除く部分が数字のみ（8〜16文字）であること

**非通知の扱い**（FR-1.3 参照）:
- Caller ID が空、"anonymous"、"withheld" の場合、正規化せず anonymousAction を適用

**変換例**:
| 入力 | 出力 | 備考 |
|------|------|------|
| `09028894539` | `+819028894539` | 日本国内番号（先頭 0 除去） |
| `090-2889-4539` | `+819028894539` | ハイフン除去後に変換 |
| `+819028894539` | `+819028894539` | 既に E.164 形式 |
| `+1234567890` | `+1234567890` | 他国の E.164 形式 |
| `abc` | エラー | 不正文字列 |
| `""` | anonymousAction | 非通知（空文字） |
| `anonymous` | anonymousAction | 非通知（キーワード） |

**制約**:
- MVP では日本国内番号（`0` 始まり）のみ対応
- 他国の国内番号（例: 米国の 1-XXX-XXX-XXXX）は将来対応
```

---

### 5.2 詳細設計追加（DD-004-XX へマージ）

#### DD-004-FN-01: normalize_phone_number()

**シグネチャ**:
```rust
fn normalize_phone_number(&self, phone_number: &str) -> Result<String, RoutingError>
```

**入力**:
| パラメータ | 型 | 説明 |
|-----------|-----|------|
| phone_number | &str | 生の Caller ID（090..., +819..., 空文字, "anonymous" など） |

**出力**:
| 型 | 説明 |
|----|------|
| Ok(String) | E.164 形式の電話番号（例: `+819028894539`） |
| Err(RoutingError::InvalidPhoneNumber) | 不正な電話番号形式 |

**処理フロー**:
```rust
1. 空白・ハイフン・括弧を除去
   - cleaned = phone_number.replace([' ', '-', '(', ')'], "")

2. E.164 変換
   a. 既に + で始まる場合
      - そのまま（E.164 形式と判定）
      - 次のステップ（形式チェック）へ

   b. 日本国内番号（0 で始まる）の場合
      - "+81" + &cleaned[1..] に変換
      - 次のステップ（形式チェック）へ

   c. その他の場合
      - Err(InvalidPhoneNumber) を返す

3. E.164 形式チェック
   a. + で始まること
   b. + を除く部分が 8〜16 文字であること
   c. + を除く部分が数字のみであること

   - 全てを満たす → Ok(normalized)
   - いずれか満たさない → Err(InvalidPhoneNumber)
```

**エラーケース**:
| エラー | 条件 | 対応 |
|--------|------|------|
| InvalidPhoneNumber | 数字以外の文字を含む（abc など） | Err を返す |
| InvalidPhoneNumber | 0 で始まらず、+ で始まらない | Err を返す |
| InvalidPhoneNumber | E.164 形式チェックに失敗 | Err を返す |

**実装例（Rust）**:
```rust
fn normalize_phone_number(&self, phone_number: &str) -> Result<String, RoutingError> {
    // 1. 空白・ハイフン・括弧を除去
    let cleaned = phone_number.replace([' ', '-', '(', ')'], "");

    // 2. E.164 変換
    let normalized = if cleaned.starts_with('+') {
        // 既に E.164 形式と判定
        cleaned
    } else if cleaned.starts_with('0') {
        // 日本国内番号（0 始まり）→ +81 + 先頭 0 除去
        format!("+81{}", &cleaned[1..])
    } else {
        // その他は変換不可
        return Err(RoutingError::InvalidPhoneNumber(format!("Cannot convert to E.164: {}", phone_number)));
    };

    // 3. E.164 形式チェック
    if !normalized.starts_with('+') {
        return Err(RoutingError::InvalidPhoneNumber(format!("Not starting with +: {}", phone_number)));
    }

    let digits = &normalized[1..];
    if digits.len() < 8 || digits.len() > 16 {
        return Err(RoutingError::InvalidPhoneNumber(format!("Invalid length (8-16): {}", phone_number)));
    }

    if !digits.chars().all(|c| c.is_ascii_digit()) {
        return Err(RoutingError::InvalidPhoneNumber(format!("Non-digit characters: {}", phone_number)));
    }

    Ok(normalized)
}
```

**トレース**:
- ← RD-004 FR-1.2
- → UT-004-TC-01〜05

---

#### DD-004-FN-02: evaluate() エラーハンドリング修正

**シグネチャ**:
```rust
pub async fn evaluate(&self, caller_id: &str, call_id: &str) -> Result<ActionConfig, RoutingError>
```

**修正内容**:

**現状（STEER-140）**:
```rust
// 1. 電話番号正規化
let normalized_caller_id = self.normalize_phone_number(caller_id)?;
// ↑ ここで Err が返ると関数が終了し、SessionCoordinator の Err 分岐で legacy IVR に落ちる
```

**修正後**:
```rust
// 1. 電話番号正規化（失敗時は defaultAction へフォールバック）
let normalized_caller_id = match normalize_phone_number_e164(caller_id) {
    Ok(normalized) => {
        info!("[RuleEvaluator] call_id={} Normalized: {} -> {}", call_id, caller_id, normalized);
        normalized
    }
    Err(e) => {
        warn!("[RuleEvaluator] call_id={} Phone normalization failed: {}, fallback to defaultAction", call_id, e);
        // defaultAction を返す（Ok を返すため、SessionCoordinator の Err 分岐には到達しない）
        return self.get_default_action(call_id).await;
    }
};
```

**修正理由**:
- 現状では正規化失敗時に `Err` が返り、SessionCoordinator の Err 分岐で `self.outbound_mode = false` のみ設定される
- その後、ACK 後分岐で既定の legacy IVR に落ちてしまう
- 正規化失敗は「不正な電話番号からの着信」として扱い、拒否ではなく defaultAction で処理するのが適切
- **フォールバック責務を RuleEvaluator 側に集約**し、SessionCoordinator は `evaluate()` の結果を信頼して実行するだけにする

**重要**:
- 正規化失敗時に `Ok(defaultAction)` を返すため、SessionCoordinator の Err 分岐には到達しない
- DB アクセスエラーなど、真の評価失敗時のみ `Err` を返す

**トレース**:
- ← RD-004 FR-1.1
- → UT-004-TC-06

---

#### DD-004-FN-03: 共通ユーティリティ関数の追加

**新規追加**: `src/service/routing/mod.rs`

**シグネチャ**:
```rust
/// 電話番号を E.164 形式に正規化する（共通ユーティリティ）
pub fn normalize_phone_number_e164(phone_number: &str) -> Result<String, RoutingError> {
    // DD-004-FN-01 と同じロジック
    // ...
}
```

**目的**:
- 電話番号正規化ロジックを RuleEvaluator から独立させ、他のモジュールからも利用可能にする
- call log 保存時など、ルール評価以外の場所でも E.164 正規化を統一的に適用する

**利用箇所**:
1. `RuleEvaluator::evaluate()` 内で使用（DD-004-FN-02）
2. call log 保存時に使用（SessionCoordinator 系、詳細調査中）

**実装**:
```rust
// src/service/routing/mod.rs

mod evaluator;
mod executor;

pub use evaluator::{ActionConfig, RuleEvaluator, RoutingError}; // RoutingError を追加
pub use executor::ActionExecutor;

/// 電話番号を E.164 形式に正規化する（共通ユーティリティ）
///
/// # Examples
/// ```
/// use crate::service::routing::normalize_phone_number_e164;
///
/// assert_eq!(normalize_phone_number_e164("09028894539")?, "+819028894539");
/// assert_eq!(normalize_phone_number_e164("+819028894539")?, "+819028894539");
/// ```
pub fn normalize_phone_number_e164(phone_number: &str) -> Result<String, RoutingError> {
    // 1. 空白・ハイフン・括弧を除去
    let cleaned = phone_number.replace([' ', '-', '(', ')'], "");

    // 2. E.164 変換
    let normalized = if cleaned.starts_with('+') {
        // 既に E.164 形式と判定
        cleaned
    } else if cleaned.starts_with('0') {
        // 日本国内番号（0 始まり）→ +81 + 先頭 0 除去
        format!("+81{}", &cleaned[1..])
    } else {
        // その他は変換不可
        return Err(RoutingError::InvalidPhoneNumber(format!("Cannot convert to E.164: {}", phone_number)));
    };

    // 3. E.164 形式チェック
    if !normalized.starts_with('+') {
        return Err(RoutingError::InvalidPhoneNumber(format!("Not starting with +: {}", phone_number)));
    }

    let digits = &normalized[1..];
    if digits.len() < 8 || digits.len() > 16 {
        return Err(RoutingError::InvalidPhoneNumber(format!("Invalid length (8-16): {}", phone_number)));
    }

    if !digits.chars().all(|c| c.is_ascii_digit()) {
        return Err(RoutingError::InvalidPhoneNumber(format!("Non-digit characters: {}", phone_number)));
    }

    Ok(normalized)
}
```

**修正理由**:
- 正規化ロジックの二重実装を避ける
- private メソッド（`RuleEvaluator::normalize_phone_number()`）を外部から呼び出せない問題を解決
- 共通ユーティリティとして公開することで、テストも一元化できる

**トレース**:
- ← RD-004 FR-1.2
- → UT-004-TC-01〜05, UT-004-TC-07

---

### 5.3 テストケース追加（UT-004-XX へマージ）

#### UT-004-TC-01: normalize_phone_number_e164() - 日本国内番号（090...）

**対象**: DD-004-FN-03（共通ユーティリティ）

**目的**: 日本国内番号（090...）が E.164 形式（+8190...）に正規化されることを検証

**入力**:
```rust
use crate::service::routing::normalize_phone_number_e164;

normalize_phone_number_e164("09028894539")
```

**期待結果**:
```rust
Ok("+819028894539")
```

**トレース**: ← DD-004-FN-03

---

#### UT-004-TC-02: normalize_phone_number_e164() - ハイフン付き日本国内番号

**対象**: DD-004-FN-03（共通ユーティリティ）

**目的**: ハイフン付き番号が正規化されることを検証

**入力**:
```rust
use crate::service::routing::normalize_phone_number_e164;

normalize_phone_number_e164("090-2889-4539")
```

**期待結果**:
```rust
Ok("+819028894539")
```

**トレース**: ← DD-004-FN-03

---

#### UT-004-TC-03: normalize_phone_number_e164() - 既に E.164 形式

**対象**: DD-004-FN-03（共通ユーティリティ）

**目的**: 既に E.164 形式の番号がそのまま返されることを検証

**入力**:
```rust
use crate::service::routing::normalize_phone_number_e164;

normalize_phone_number_e164("+819028894539")
```

**期待結果**:
```rust
Ok("+819028894539")
```

**トレース**: ← DD-004-FN-03

---

#### UT-004-TC-04: normalize_phone_number_e164() - 他国の E.164 形式

**対象**: DD-004-FN-03（共通ユーティリティ）

**目的**: 他国の E.164 形式番号がそのまま返されることを検証

**入力**:
```rust
use crate::service::routing::normalize_phone_number_e164;

normalize_phone_number_e164("+1234567890")
```

**期待結果**:
```rust
Ok("+1234567890")
```

**トレース**: ← DD-004-FN-03

---

#### UT-004-TC-05: normalize_phone_number_e164() - 不正文字列

**対象**: DD-004-FN-03（共通ユーティリティ）

**目的**: 不正な文字列が InvalidPhoneNumber エラーになることを検証

**入力**:
```rust
use crate::service::routing::normalize_phone_number_e164;

normalize_phone_number_e164("abc")
```

**期待結果**:
```rust
Err(RoutingError::InvalidPhoneNumber("Cannot convert to E.164: abc"))
```

**トレース**: ← DD-004-FN-03

---

#### UT-004-TC-06: evaluate() - 正規化失敗時のフォールバック

**対象**: DD-004-FN-02

**目的**: 正規化失敗時に defaultAction を返すこと（Ok を返すこと）を検証

**入力**:
```rust
evaluator.evaluate("abc", "call-123")
```

**期待結果**:
```rust
Ok(ActionConfig { action_code: "VR", ... }) // defaultAction（Err ではなく Ok）
```

**検証項目**:
- [ ] ログに `Phone normalization failed` が出力される
- [ ] ログに `fallback to defaultAction` が出力される
- [ ] 戻り値が `Ok(ActionConfig)` である（`Err` ではない）
- [ ] ActionConfig の action_code が system_settings.defaultAction の値である

**トレース**: ← DD-004-FN-02

---

#### UT-004-TC-07: normalize_phone_number_e164() - 共通ユーティリティ

**対象**: DD-004-FN-03

**目的**: normalize_phone_number_e164() が独立した関数として正しく動作することを検証

**入力**:
```rust
use crate::service::routing::normalize_phone_number_e164;

normalize_phone_number_e164("09028894539")
```

**期待結果**:
```rust
Ok("+819028894539")
```

**検証項目**:
- [ ] 日本国内番号（090...）が正規化される
- [ ] ハイフン付き番号が正規化される
- [ ] 既に E.164 形式の番号がそのまま返される
- [ ] 不正文字列が InvalidPhoneNumber エラーになる

**トレース**: ← DD-004-FN-03

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #183 | STEER-183 | 起票 |
| STEER-183 | RD-004-FR-1.2 | 要件修正 |
| RD-004-FR-1.2 | DD-004-FN-01 | 詳細設計 |
| DD-004-FN-01 | UT-004-TC-01〜05 | 単体テスト |
| DD-004-FN-02 | UT-004-TC-06 | 単体テスト |
| DD-004-FN-03 | UT-004-TC-07 | 単体テスト |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] 日本国内番号（090...）→ E.164（+8190...）変換ロジックが明確か
- [ ] 正規化失敗時のフォールバック先（defaultAction）が適切か
- [ ] 非通知（空文字/"anonymous"/"withheld"）の扱いが明確か
- [ ] 不正文字列（"abc" など）の扱いが明確か
- [ ] テストケースが網羅的か（正常系・異常系）
- [ ] 既存仕様（RD-004, STEER-140）との整合性があるか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] 実装が完了している
- [ ] コードレビューを受けている
- [ ] 単体テスト（UT-004-TC-01〜07）が PASS
- [ ] 結合テスト（09028894539 着信 → ルール評価 → 履歴保存）が PASS
- [ ] 結合テスト（不正 caller_id "abc" 着信 → evaluate() が Ok(defaultAction) を返す → legacy IVR 非遷移）が PASS
- [ ] 本体仕様書（RD-004, DD-004）への反映準備ができている

---

## 8. 未確定点・質問リスト（Open Questions）

全件 Resolved。

### ~~Q1: 日本以外の国内番号への対応~~ → **Resolved**

**決定**: 選択肢 3（MVP では日本のみ）
- 現時点では日本国内での使用を想定
- 将来的に選択肢 1（国別プレフィックステーブル）へ移行可能

---

### ~~Q2: 正規化失敗時のフォールバック先~~ → **Resolved**

**決定**: 選択肢 1（defaultAction）
- 不正な番号は「番号が読み取れなかった」のではなく「不正な形式の番号が来た」という状態
- 非通知（anonymousAction）とは区別すべき
- 専用 Action は過剰な複雑化

---

### ~~Q3: legacy IVR 直行の挙動~~ → **Resolved**

**調査結果**（Codex 提供）:
- SessionCoordinator の `handle_control_event()` 内で `evaluate()` を実行
- Err 分岐では `self.outbound_mode = false` のみ設定
- ACK 後分岐で announce_mode/voicebot_direct_mode が立っていないため、既定の legacy IVR に落ちる
- RuleEvaluator 内には defaultAction フォールバックがあるが、正規化失敗時には到達しない

**結論**:
フォールバック責務を **RuleEvaluator 側に集約**し、SessionCoordinator は `evaluate()` の結果を信頼して実行するだけにする。

**決定**:
- DD-004-FN-02 で `evaluate()` 内のエラーハンドリングを修正
- 正規化失敗時に `Ok(defaultAction)` を返すことで、SessionCoordinator の Err 分岐には到達しない
- SessionCoordinator 側の修正は不要
- DB アクセスエラーなど、真の評価失敗時のみ `Err` を返す

---

## 9. 備考

### 9.1 STEER-140 との関係

本ステアリングは STEER-140（Backend ルール評価エンジン実装）の修正版である。
- STEER-140 では E.164 形式のみ対応していた
- 本ステアリングで日本国内番号（090...）→ E.164 変換を追加

### 9.2 将来対応

以下の機能は将来対応として保留：
- 他国の国内番号への対応
- 国際プレフィックス（+, 010, 001 など）の正規化
- 市外局番（03, 06 など）の扱い

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-16 | 初版作成（Draft） | Claude Code (claude-sonnet-4-5) |
| 2026-02-16 | Q1, Q2, Q3 を Resolved に更新、DD-004-FN-04（SessionCoordinator 修正）追加、UT-004-TC-08 追加、影響範囲に mod.rs 追加 | Claude Code (claude-sonnet-4-5) |
| 2026-02-16 | Codex レビュー反映：フォールバック責務を RuleEvaluator 側に集約、DD-004-FN-04/UT-004-TC-08 削除、DD-004-FN-03 を共通ユーティリティ追加に変更、受入条件・Q3 修正 | Claude Code (claude-sonnet-4-5) |
| 2026-02-16 | Codex 再レビュー反映：§2.2 目的を「電話番号正規化失敗時」に限定、RoutingError の pub use を追加、UT-01〜05 を normalize_phone_number_e164() に統一、影響範囲の mod.rs を「修正」に変更 | Claude Code (claude-sonnet-4-5) |
| 2026-02-16 | 承認完了、ステータス → Approved：Codex レビュー2回実施（全指摘対応完了）、実装フェーズへ引き継ぎ | Claude Code (claude-sonnet-4-5) |
