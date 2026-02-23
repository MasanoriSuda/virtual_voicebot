# STEER-229: VB 録音対応（recording_enabled フラグを尊重する）

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-229 |
| タイトル | VB 録音対応（recording_enabled フラグを尊重する） |
| ステータス | Approved |
| 関連Issue | #229 |
| 優先度 | P1 |
| 作成日 | 2026-02-24 |

---

## 2. ストーリー（Why）

### 2.1 背景

VB（Voicebot）モードの実行時、`execute_vb()` が `action.recording_enabled` フラグを無視して録音を常に `false` に固定している。

**現状の問題:**

| 問題 | 詳細 |
|------|------|
| 録音フラグの無視 | `src/service/routing/executor.rs` L123: `session.set_recording_enabled(false)` が hardcoded であり、`action.recording_enabled=true` を設定しても録音されない |
| ログ文言の不正確 | 同 L118: `"recording_enabled=false"` がリテラルで埋め込まれており、実際の設定値を反映しない |
| 仕様記述の齟齬 | `STEER-141_actioncode-phase3.md` L147 に「VB: `recording_enabled=false` で呼ばれる」と記載されているが、これは当時の実装前提であり現在のシステム要件を反映していない |

対照的に VR（B2BUA 転送）は `executor.rs` L81 で `session.set_recording_enabled(action.recording_enabled)` を使っており、フラグを正しく尊重している。

### 2.2 目的

`execute_vb()` で `action.recording_enabled` フラグを尊重するよう修正し、
VB モードでもボイスボット会話の録音を可能にする。

変更は最小差分（2行変更 + テスト追加）で実施する。

### 2.3 ユーザーストーリー

```text
As a オペレーター
I want to VB（Voicebot）モードで録音設定を有効にしたい
So that ボイスボットとの会話を録音・確認できる

受入条件:
- [ ] `recording_enabled=true` を設定した VB アクションで着信すると録音が行われる
- [ ] `recording_enabled=false` の VB アクションでは従来通り録音されない
- [ ] VR アクションの録音挙動は変わらない
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-24 |
| 起票理由 | VB モードで録音設定を有効にしてもボイスボット会話が録音されない |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Sonnet 4.6 |
| 作成日 | 2026-02-24 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "VB 実行時に set_recording_enabled(false) をやめて action.recording_enabled を使う、ログ文言修正、テスト追加（VB + recording_enabled=true で録音されること）" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | Codex | 2026-02-24 | NG | §7.2 L234 「またはデフォルト」はデフォルト=`true` のため不正確。`recording_enabled=false` 明示時と未指定（デフォルト `true`）時を分離して記述すること |
| 2 | Codex | 2026-02-24 | OK | 前回残件解消。§7.2 が `false` 明示と未指定（デフォルト `true`）に分離され §8 Q1・§9 と整合 |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-24 |
| 承認コメント | 承認 |

### 3.5 実装

| 項目 | 値 |
|------|-----|
| 実装者 | Codex |
| 実装日 | 2026-02-24 |
| 指示者 | @MasanoriSuda |
| 指示内容 | #229 承認済みステアリングに従い、VB 実行時の `recording_enabled` 尊重・ログ修正・回帰テスト追加を実装 |
| コードレビュー | ローカル検証実施（`cargo fmt --check` / `cargo test --lib` / `cargo clippy --lib -- -D warnings` / `cargo build --lib`） |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | - |
| マージ日 | - |
| マージ先 | `src/service/routing/executor.rs`、テストファイル |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| `docs/steering/STEER-141_actioncode-phase3.md` | 参照のみ | L147「VB: `recording_enabled=false` で呼ばれる」は本修正後に古くなる。本 STEER が前提となる |

> **Note:** STEER-141 は Merged 済みのため記述が古くなるが、本 Issue のスコープ外とし別 Issue で対処する。

### 4.2 影響するコード

| ファイル | 変更種別 | 概要 |
|---------|---------|------|
| `src/service/routing/executor.rs` | 修正 | `execute_vb()` の `set_recording_enabled(false)` を `set_recording_enabled(action.recording_enabled)` に変更（L123）、ログ文言を動的値に修正（L118） |
| テストファイル（executor または integration） | 追加 | VB + `recording_enabled=true` で `set_recording_enabled(true)` が呼ばれることを検証するテスト |

---

## 5. 差分仕様（What / How）

### 5.1 設計方針

最小差分での修正。`execute_vb()` 内の hardcoded `false` を `action.recording_enabled` に置き換えるだけ。
VR との対称性を回復する。

| 変更点 | 変更前 | 変更後 |
|--------|--------|--------|
| 録音フラグ設定（L123） | `session.set_recording_enabled(false)` | `session.set_recording_enabled(action.recording_enabled)` |
| ログ文言（L118） | `"recording_enabled=false"`（リテラル） | `"recording_enabled={}"`, `action.recording_enabled` |

### 5.2 executor.rs の変更（execute_vb）

```rust
// 変更前（L117-123）:
info!(
    "[ActionExecutor] call_id={} executing VB (voicebot mode, recording_enabled=false)",
    call_id
);
session.set_outbound_mode(false);
session.set_voicebot_direct_mode(true);
session.set_recording_enabled(false);  // ← hardcoded false

// 変更後:
info!(
    "[ActionExecutor] call_id={} executing VB (voicebot mode, recording_enabled={})",
    call_id, action.recording_enabled   // ← 実際の値を表示
);
session.set_outbound_mode(false);
session.set_voicebot_direct_mode(true);
session.set_recording_enabled(action.recording_enabled);  // ← フラグを尊重
```

**VR との対称性（L76-81、変更なし）:**

```rust
info!(
    "[ActionExecutor] call_id={} executing VR (B2BUA transfer mode, recording_enabled={})",
    call_id, action.recording_enabled
);
// ...
session.set_recording_enabled(action.recording_enabled);  // 既にフラグを尊重
```

### 5.3 テストケース追加

**テスト名:** `test_execute_vb_recording_enabled_true`（および対称テスト `_false`）

**目的:** `recording_enabled=true/false` の VB アクションで `set_recording_enabled()` に正しい値が渡されることを検証

**検証方針（疑似コード）:**

```rust
#[tokio::test]
async fn test_execute_vb_recording_enabled_true() {
    let action = ActionConfig {
        action_code: "VB".to_string(),
        recording_enabled: true,
        announce_enabled: false,
        ..ActionConfig::default_vr()  // またはテスト用ヘルパー
    };

    let mut session = MockSessionCoordinator::new();
    executor.execute_vb(&action, "test-call-id", &mut session).await.unwrap();

    // recording_enabled=true が session に伝わること
    assert!(session.recording_enabled());
}

#[tokio::test]
async fn test_execute_vb_recording_enabled_false() {
    let action = ActionConfig {
        action_code: "VB".to_string(),
        recording_enabled: false,
        ..
    };
    // ...
    assert!(!session.recording_enabled());
}
```

> **Note:** 既存テストの MockSessionCoordinator / ActionConfig の構成に合わせて Codex が具体化する。

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #229 | STEER-229 | 起票 |
| STEER-141 L147 | STEER-229 | VB recording_enabled=false 前提の旧記述（本 STEER で上書き） |
| STEER-229 | `executor.rs` L118/L123 | コード修正 |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] `action.recording_enabled` のデフォルト値（`ActionConfigDto.recording_enabled` は `default = "default_true"` → `true`）が既存 VB 運用に与える影響を確認しているか（§8 Q1 参照）
- [ ] VR の録音挙動と対称性が保たれているか（VR は既に `action.recording_enabled` を使用）
- [ ] テストが「VB + recording_enabled=true で録音される」「VB + recording_enabled=false で録音されない」の両方を網羅しているか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] `recording_enabled=true` を設定した VB アクションで着信したとき録音ファイルが生成されることを確認している
- [ ] `recording_enabled=false` を**明示した** VB アクションで従来通り録音されないことを確認している
- [ ] `recording_enabled` 未指定（手動 API / 旧データ等）の VB アクションはデフォルト `true` により録音が開始されることを確認/仕様として合意している（§8 Q1・§9 参照）
- [ ] VR アクションの録音挙動が変わっていないことを確認している
- [ ] 既存のテストがすべて PASS している

---

## 8. 未確定点・質問

| # | 質問 | 選択肢 | 推奨 | オーナー回答 |
|---|------|--------|------|-------------|
| Q1 | `ActionConfigDto.recording_enabled` は `#[serde(default = "default_true")]` でデフォルト `true`。Frontend が VB アクションに対して常に `recording_enabled` を明示的に送信しているか、それとも省略（= デフォルト `true`）のケースがあるか | 常に明示送信 / 省略あり（デフォルト true が適用される） | 確認が必要。省略ありの場合、本修正後に意図せず録音が開始するケースが発生しうる | **Codex 調査結果（2026-02-24）:** 公式 Frontend UI 経由では `recordingEnabled: boolean` は必須フィールド（`call-actions.ts:38`）かつ初期値 `true`（`call-actions.ts:118`）で、保存時に `JSON.stringify` で送信（`call-actions-content.tsx:534,539`）→ **常に明示送信**。ただし PUT parser は未指定時に `true` 補完（`call-actions.ts:332,337`）し、読み込み正規化も同様（`call-actions.ts:85,90`）→ **手動 API / 旧データ / 外部入力 では省略ケースがあり得る（その場合は `true` になる）**。Backend `ActionConfigDto` も未指定時 `true`（`evaluator.rs:65,426`）。|

---

## 9. リスク・ロールバック観点

| リスク | 影響 | 緩和策 |
|--------|------|--------|
| 手動 API / 旧データ / 外部入力 での省略ケース（Q1 確認済み） | Frontend・Backend 両方がデフォルト `true` で補完するため、省略ケースでは修正後に録音が開始する。修正前は hardcoded `false` で録音されなかった | **軽微**。公式 Frontend UI 経由では必ず明示送信される。省略ケースは手動 API 等に限定されており、意図的に `recording_enabled=false` を設定した運用には影響しない |
| `recording_enabled=false` を明示した既存 VB アクション | Frontend が `recording_enabled=false` を明示設定していれば変更前後で挙動は同一 | DB 値をそのまま渡すため後方互換 |

**ロールバック手順:** PR を revert。変更は `executor.rs` 2行のみ（テスト除く）。影響は最小。

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-24 | 初版作成（Codex 調査結果・コード確認を元に差分仕様を記述） | Claude Sonnet 4.6 |
| 2026-02-24 | §8 Q1 オーナー回答記録、§9 リスク評価更新（Codex 調査結果を反映） | Claude Sonnet 4.6 |
| 2026-02-24 | §3.3 Round 1 NG 記録、§7.2 修正（「またはデフォルト」削除・未指定ケースを独立項目に分離） | Claude Sonnet 4.6 |
| 2026-02-24 | §3.3 Round 2 OK 記録 | Claude Sonnet 4.6 |
| 2026-02-24 | §1 ステータス Draft → Approved、§3.4 承認者記録 | @MasanoriSuda |
