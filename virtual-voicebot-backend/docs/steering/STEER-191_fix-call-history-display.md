# STEER-191: 通話履歴の表示不整合修正

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-191 |
| タイトル | 通話履歴の表示不整合修正 |
| ステータス | Approved |
| 関連Issue | #191 |
| 優先度 | P1 |
| 作成日 | 2026-02-17 |

---

## 2. ストーリー（Why）

### 2.1 背景

通話履歴（call_log）の `callDisposition` と `finalAction` に関するBackend ロジックとFrontend 期待値の不整合により、以下2つの問題が発生している。

**現象1: IVR 選択時に実行アクションが IVR にならない**

| 想定動作 | 実際の動作 |
|---------|----------|
| IVR で選択肢を選んだ場合、`finalAction` は `"ivr"` となる | `finalAction` が `"normal_call"` などに上書きされる |

**現象2: アナウンス応答による拒否が着信応答で許可になる**

| ActionCode | 想定 callDisposition | 実際 | 影響 |
|-----------|---------------------|------|------|
| AN (Announce) | `"denied"` | `"allowed"` | Frontend で「応答あり」として表示される |
| AR (Announce+Record) | `"denied"` | `"allowed"` | Frontend で「応答あり」として表示される |

### 2.2 目的

Backend の `disposition_from_action_code` および `register_action_for_call_log` ロジックを修正し、Frontend の期待値と整合させる。

### 2.3 ユーザーストーリー

```
As a システム管理者
I want to 通話履歴で正確な応答状態と実行アクションを確認したい
So that 着信対応の品質を正しく把握できる

受入条件:
- [ ] AN (Announce) コードの通話が callDisposition="denied" になる
- [ ] IVR で選択された通話の finalAction が "ivr" のまま保持される
- [ ] 既存の BZ/RJ/NR の動作が影響を受けない
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-17 |
| 起票理由 | Codex による調査結果 #191 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Sonnet 4.5 |
| 作成日 | 2026-02-17 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "https://github.com/MasanoriSuda/virtual_voicebot/issues/191の対応をしたいのでステアリングファイルの作成お願いします" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|------------|
| 1 | | | | |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-17 |
| 承認コメント | AN のみ対象として統一、AR は将来対応として明確化 |

### 3.5 実装（該当する場合）

| 項目 | 値 |
|------|-----|
| 実装者 | Codex (GPT-5) |
| 実装日 | 2026-02-17 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "STEER-191 の実装をお願いします Refs #191" |
| コードレビュー | Codex self-check（`cargo test` 実行） |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | |
| マージ日 | |
| マージ先 | RD-004, DD-009 (新規) |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| virtual-voicebot-backend/docs/requirements/RD-004_call-routing-execution.md | 修正 | AN (Announce) の callDisposition 仕様を明記 |
| virtual-voicebot-backend/docs/design/detail/DD-009_call-log.md | 追加 | call_log 生成ロジックの詳細設計（新規） |
| virtual-voicebot-backend/docs/test/unit/UT-009_call-log.md | 追加 | 単体テストケース（新規） |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| src/protocol/session/coordinator.rs | 修正 | disposition_from_action_code 関数の修正 (line 723) |
| src/protocol/session/coordinator.rs | 修正 | register_action_for_call_log 関数の修正 (line 383) |

---

## 5. 差分仕様（What / How）

### 5.1 要件追加（RD-004 へマージ）

```markdown
## RD-004-FR-X: call_log の disposition/finalAction 生成ルール

### 概要

通話終了時に生成する call_log の `callDisposition` および `finalAction` フィールドは、以下のロジックで決定する。

### callDisposition の決定ロジック

`callDisposition` は最終的な ActionCode から以下のマッピングで決定する：

**MVP 対象**:

| ActionCode | callDisposition | 説明 |
|-----------|----------------|------|
| BZ (Busy) | `"denied"` | 話中応答 |
| RJ (Reject) | `"denied"` | 即時拒否 |
| AN (Announce) | `"denied"` | アナウンス再生（※着信応答なし、拒否扱い） |
| NR (No Response) | `"no_answer"` | 応答なし（コール音のみ） |
| VR, VB, VM, IV | `"allowed"` | 着信応答あり |
| その他 | `"allowed"` | デフォルト |

**将来対応** (MVP 外):

| ActionCode | callDisposition | 説明 |
|-----------|----------------|------|
| AR (Announce+Record) | `"denied"` | アナウンス再生（録音あり） |

### finalAction の決定ロジック

`finalAction` は **初回実行された ActionCode** から決定し、**途中遷移では上書きしない**。

| ActionCode | finalAction | 説明 |
|-----------|------------|------|
| VR (Voicebot+Record) | `"normal_call"` | AI応答（録音あり） |
| VB (Voicebot) | `"voicebot"` | AI応答（録音なし） |
| VM (Voicemail) | `"voicemail"` | 留守番電話 |
| IV (IVR) | `"ivr"` | IVR フロー実行 |
| AN (Announce) | `"announcement"` | アナウンス再生（※拒否扱い） |
| BZ (Busy) | `"busy"` | 話中 |
| RJ (Reject) | `"rejected"` | 拒否 |
| NR (No Response) | `null` | 応答なし |

**将来対応** (MVP 外):
| ActionCode | finalAction | 説明 |
|-----------|------------|------|
| AR (Announce+Record) | `"announcement_deny"` | アナウンス再生（録音） |

**途中遷移の扱い**:
- IVR → VR/VM などの遷移が発生した場合、`finalAction` は初回の `"ivr"` を保持する
- 例: IVR で選択肢1を選んで VR に遷移 → `finalAction = "ivr"`（`"normal_call"` にはならない）

### 入力

- `action_code`: 実行された ActionCode 文字列

### 出力

- `callDisposition`: String ("allowed" | "denied" | "no_answer")
- `finalAction`: Option<String> (上記マッピング参照)

### 受入条件

- [ ] AN (Announce) が callDisposition="denied" となる
- [ ] IVR → VR 遷移時に finalAction が "ivr" のまま保持される
- [ ] BZ/RJ/NR の既存動作が変わらない
- [ ] initial_action_code が正しく保持される

### 優先度

P1

### トレース

- → DD: DD-009-FN-01, DD-009-FN-02
- → UT: UT-009-TC-01 〜 UT-009-TC-06
```

---

### 5.2 詳細設計追加（DD-009 へマージ）

```markdown
# DD-009: call_log 生成ロジック詳細設計

## 1. 目的・スコープ

- 目的: 通話終了時の call_log データ生成ロジックを定義
- スコープ: callDisposition, finalAction の決定ロジック
- 非スコープ: call_log の DB 永続化、IVR イベント記録

## 2. 依存関係

- 依存するモジュール:
  - `protocol::session::coordinator` : SessionCoordinator 状態管理
  - `shared::ports::call_log_port` : EndedCallLog 構造体定義
- 依存されるモジュール:
  - `protocol::session::handlers::mod` : 通話終了時に call_log 生成を呼び出す

---

## DD-009-FN-01: disposition_from_action_code

### シグネチャ

```rust
fn disposition_from_action_code(action_code: &str) -> &'static str
```

### 入力

| パラメータ | 型 | 説明 |
|-----------|-----|------|
| action_code | &str | 正規化済み ActionCode ("VR", "IV", "AN", "AR", "BZ", "RJ", "NR" 等) |

### 出力

| 型 | 説明 |
|----|------|
| &'static str | "allowed", "denied", "no_answer" のいずれか |

### 処理フロー

1. action_code を match で分岐
2. "BZ" | "RJ" | "AN" → `"denied"` を返す
3. "NR" → `"no_answer"` を返す
4. その他 → `"allowed"` を返す

### エラーケース

なし（すべての入力に対してデフォルト値を返す）

### 変更点（現行コードとの差分）

**現行**:
```rust
fn disposition_from_action_code(action_code: &str) -> &'static str {
    match action_code {
        "BZ" | "RJ" => "denied",
        "NR" => "no_answer",
        _ => "allowed",
    }
}
```

**修正後**:
```rust
fn disposition_from_action_code(action_code: &str) -> &'static str {
    match action_code {
        "BZ" | "RJ" | "AN" => "denied",
        "NR" => "no_answer",
        _ => "allowed",
    }
}
```

**備考**: AR (Announce+Record) は RD-004 で「将来対応」と定義されており、現在実装されていないため、本修正では対象外とする。将来 AR が実装される際は、同様に `"denied"` へ追加する。

### トレース

- ← RD: RD-004-FR-X
- → UT: UT-009-TC-01, UT-009-TC-02, UT-009-TC-03

---

## DD-009-FN-02: register_action_for_call_log

### シグネチャ

```rust
impl SessionCoordinator {
    pub(crate) fn register_action_for_call_log(&mut self, action_code: &str)
}
```

### 入力

| パラメータ | 型 | 説明 |
|-----------|-----|------|
| action_code | &str | 実行する ActionCode |

### 出力

なし（`self` の状態を更新）

### 処理フロー

1. action_code を normalize_action_code で正規化
2. **初回呼び出し判定**:
   - `self.initial_action_code.is_none()` なら、`initial_action_code` と `final_action` を設定
   - 2回目以降は `final_action` を上書きしない
3. `call_disposition` は毎回 `disposition_from_action_code` で更新
4. transfer_status の条件更新（既存ロジック維持）

### 変更点（現行コードとの差分）

**現行**:
```rust
pub(crate) fn register_action_for_call_log(&mut self, action_code: &str) {
    let normalized = normalize_action_code(action_code);
    if self.initial_action_code.is_none() {
        self.initial_action_code = Some(normalized.clone());
    }
    self.call_disposition = disposition_from_action_code(&normalized).to_string();
    self.final_action = final_action_from_action_code(&normalized).map(str::to_string); // ← 毎回上書き
    if matches!(normalized.as_str(), "IV" | "VR") && self.transfer_status == "no_transfer" {
        self.transfer_status = "none".to_string();
    }
}
```

**修正後**:
```rust
pub(crate) fn register_action_for_call_log(&mut self, action_code: &str) {
    let normalized = normalize_action_code(action_code);
    if self.initial_action_code.is_none() {
        self.initial_action_code = Some(normalized.clone());
        self.final_action = final_action_from_action_code(&normalized).map(str::to_string); // ← 初回のみ設定
    }
    self.call_disposition = disposition_from_action_code(&normalized).to_string();
    if matches!(normalized.as_str(), "IV" | "VR") && self.transfer_status == "no_transfer" {
        self.transfer_status = "none".to_string();
    }
}
```

### エラーケース

なし

### トレース

- ← RD: RD-004-FR-X
- → UT: UT-009-TC-04, UT-009-TC-05, UT-009-TC-06

---

## 3. final_action_from_action_code（既存関数、変更なし）

### シグネチャ

```rust
fn final_action_from_action_code(action_code: &str) -> Option<&'static str>
```

### 入力

| パラメータ | 型 | 説明 |
|-----------|-----|------|
| action_code | &str | 正規化済み ActionCode |

### 出力

| 型 | 説明 |
|----|------|
| Some(&'static str) | finalAction 値 ("normal_call", "ivr", "announcement" 等) |
| None | NR または未知の ActionCode |

### 処理フロー（変更なし）

```rust
fn final_action_from_action_code(action_code: &str) -> Option<&'static str> {
    match action_code {
        "VR" => Some("normal_call"),
        "VM" => Some("voicemail"),
        "VB" => Some("voicebot"),
        "IV" => Some("ivr"),
        "AN" => Some("announcement"),
        "BZ" => Some("busy"),
        "RJ" => Some("rejected"),
        "AR" => Some("announcement_deny"),
        "NR" => None,
        _ => None,
    }
}
```

### トレース

- ← RD: RD-004-FR-X
- → UT: UT-009-TC-04

```

---

### 5.3 テストケース追加（UT-009 へマージ）

```markdown
# UT-009: call_log 生成ロジック単体テスト

## UT-009-TC-01: AN コードが denied になる

### 対象

DD-009-FN-01 (disposition_from_action_code)

### 目的

AN (Announce) ActionCode が callDisposition="denied" にマッピングされることを検証

### 入力

```rust
disposition_from_action_code("AN")
```

### 期待結果

```rust
assert_eq!(disposition_from_action_code("AN"), "denied");
```

### トレース

← DD: DD-009-FN-01

---

## UT-009-TC-02: VB/VM/IV コードが allowed になる

### 対象

DD-009-FN-01 (disposition_from_action_code)

### 目的

VB, VM, IV ActionCode が callDisposition="allowed" にマッピングされることを検証（着信応答あり）

### 入力

```rust
disposition_from_action_code("VB")
disposition_from_action_code("VM")
disposition_from_action_code("IV")
```

### 期待結果

```rust
assert_eq!(disposition_from_action_code("VB"), "allowed");
assert_eq!(disposition_from_action_code("VM"), "allowed");
assert_eq!(disposition_from_action_code("IV"), "allowed");
```

### トレース

← DD: DD-009-FN-01

---

## UT-009-TC-03: 既存コード（BZ/RJ/NR）の動作が変わらない

### 対象

DD-009-FN-01 (disposition_from_action_code)

### 目的

既存の BZ, RJ, NR ActionCode の動作が修正後も変わらないことを検証

### 入力

```rust
disposition_from_action_code("BZ")
disposition_from_action_code("RJ")
disposition_from_action_code("NR")
disposition_from_action_code("VR")
```

### 期待結果

```rust
assert_eq!(disposition_from_action_code("BZ"), "denied");
assert_eq!(disposition_from_action_code("RJ"), "denied");
assert_eq!(disposition_from_action_code("NR"), "no_answer");
assert_eq!(disposition_from_action_code("VR"), "allowed");
```

### トレース

← DD: DD-009-FN-01

---

## UT-009-TC-04: IVR → VR 遷移時に finalAction が ivr のまま保持される

### 対象

DD-009-FN-02 (register_action_for_call_log)

### 目的

IVR 実行後に VR へ遷移した場合、finalAction が "ivr" のまま保持されることを検証

### 入力

```rust
let mut coordinator = SessionCoordinator::new(...);
coordinator.register_action_for_call_log("IV");
coordinator.register_action_for_call_log("VR");
```

### 期待結果

```rust
assert_eq!(coordinator.initial_action_code, Some("IV".to_string()));
assert_eq!(coordinator.final_action, Some("ivr".to_string())); // "normal_call" ではない
assert_eq!(coordinator.call_disposition, "allowed"); // 最新の disposition
```

### トレース

← DD: DD-009-FN-02

---

## UT-009-TC-05: IV → VM 遷移時に finalAction が ivr のまま保持される

### 対象

DD-009-FN-02 (register_action_for_call_log)

### 目的

IVR 実行後に VM (留守番電話) へ遷移した場合、finalAction が "ivr" のまま保持されることを検証（IVR 内での選択肢として VM に遷移する実経路を想定）

### 入力

```rust
let mut coordinator = SessionCoordinator::new(...);
coordinator.register_action_for_call_log("IV");
coordinator.register_action_for_call_log("VM");
```

### 期待結果

```rust
assert_eq!(coordinator.initial_action_code, Some("IV".to_string()));
assert_eq!(coordinator.final_action, Some("ivr".to_string())); // "voicemail" ではない
assert_eq!(coordinator.call_disposition, "allowed"); // VM は allowed
```

### トレース

← DD: DD-009-FN-02

---

## UT-009-TC-06: 初回 VR 実行で finalAction が normal_call になる

### 対象

DD-009-FN-02 (register_action_for_call_log)

### 目的

遷移がない単純な VR 実行時に finalAction が "normal_call" になることを検証

### 入力

```rust
let mut coordinator = SessionCoordinator::new(...);
coordinator.register_action_for_call_log("VR");
```

### 期待結果

```rust
assert_eq!(coordinator.initial_action_code, Some("VR".to_string()));
assert_eq!(coordinator.final_action, Some("normal_call".to_string()));
assert_eq!(coordinator.call_disposition, "allowed");
```

### トレース

← DD: DD-009-FN-02

```

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #191 | STEER-191 | 起票 |
| STEER-191 | RD-004-FR-X | 要件追加 |
| RD-004-FR-X | DD-009-FN-01 | 設計 |
| RD-004-FR-X | DD-009-FN-02 | 設計 |
| DD-009-FN-01 | UT-009-TC-01 | 単体テスト |
| DD-009-FN-01 | UT-009-TC-02 | 単体テスト |
| DD-009-FN-01 | UT-009-TC-03 | 単体テスト |
| DD-009-FN-02 | UT-009-TC-04 | 単体テスト |
| DD-009-FN-02 | UT-009-TC-05 | 単体テスト |
| DD-009-FN-02 | UT-009-TC-06 | 単体テスト |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] AN (Announce) の callDisposition 仕様が明確か
- [ ] finalAction 保持ロジックが実装者に伝わるか
- [ ] 既存の BZ/RJ/NR 動作への影響がないか
- [ ] テストケースが網羅的か（遷移あり/なし、各 ActionCode）
- [ ] RD-004 との整合性があるか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] 実装が完了している
- [ ] コードレビューを受けている
- [ ] UT-009 のテストがすべて PASS
- [ ] 本体仕様書（RD-004, DD-009）への反映準備ができている

---

## 8. 備考

### 8.1 スコープ外

以下は本ステアリングのスコープ外とし、将来対応とする：

- **AR (Announce+Record) ActionCode**: RD-004 で「将来対応」と定義されており、現在実装されていない。将来 AR が実装される際は、disposition を `"denied"` へ追加する必要がある
- **RJ (Reject) ActionCode の実装**: RD-004 で「将来対応」と定義されている
- **AN の finalAction 値の見直し**: 現状は `"announcement"` だが、拒否扱いなので `"announcement_deny"` の方が適切かもしれない。ただし、Issue #191 では言及されていないため、本修正では対象外とする
- **Frontend の callDisposition 表示ロジック**: Backend の修正のみを対象とする

### 8.2 参考: Codex 調査結果（Issue #191）

**現象1の根本原因**:
- `register_action_for_call_log` が毎回呼ばれるたびに `final_action` を上書き
- IVR → VR 遷移時に `final_action` が `"ivr"` → `"normal_call"` に変わる

**現象2の根本原因**:
- `disposition_from_action_code` の match 文で AN がデフォルトケース `_` に該当
- デフォルトが `"allowed"` のため、AN が `"denied"` にならない

**修正方針**:
1. `disposition_from_action_code` に `"AN"` を追加（AR は未実装のため対象外）
2. `register_action_for_call_log` で `final_action` を初回のみ設定するよう変更

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-17 | 初版作成 | Claude Sonnet 4.5 |
| 2026-02-17 | レビュー指摘対応 (1回目): VB finalAction 修正 (voicebot), AR スコープ外明記, UT-009-TC-02/05 修正 | Claude Sonnet 4.5 |
| 2026-02-17 | レビュー指摘対応 (2回目): AN/AR 混在を AN のみに統一（AR は将来対応として完全除外） | Claude Sonnet 4.5 |
| 2026-02-17 | 承認（ステータス: Approved） | @MasanoriSuda |
