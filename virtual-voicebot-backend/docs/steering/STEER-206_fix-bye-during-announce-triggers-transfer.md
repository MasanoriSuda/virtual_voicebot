# STEER-206: アナウンス送信中の BYE 受信後に転送が誤起動するバグ修正

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-206 |
| タイトル | アナウンス送信中の BYE 受信後に転送が誤起動するバグ修正 |
| ステータス | Approved |
| 関連Issue | #206 |
| 優先度 | P0 |
| 作成日 | 2026-02-21 |

---

## 2. ストーリー（Why）

### 2.1 背景

着信中、相手にアナウンス（録音告知）を送信しているタイミングで A-leg から BYE を受信した場合、呼び出し（B-leg への転送）が誤って開始される。
その後オンフックすると大量の warning ログが連発する。

放置すると:
- 切断済みセッションで B-leg の転送・接続が試みられ、資源リークや予期しない SIP メッセージが発生する
- B-leg が生きている間、チャネル満杯 warn が連発してログが汚染される
- 障害時の調査が困難になる

### 2.2 目的

`SipBye` 受信後は録音告知転送（AppTransferRequest）を発火させない状態遷移ガードを追加し、BYE 受信後の誤転送と warning 連発を解消する。

### 2.3 ユーザーストーリー

```text
As a 通話オペレーター
I want to 着信中にアナウンス送信中に相手が切断した場合、転送が起動されないようにしたい
So that 切断後に不要な警告ログや B-leg 呼び出しが発生しない

受入条件:
- [ ] 録音告知アナウンス中に BYE を受信した場合、B-leg への転送（AppTransferRequest）が起動されない
- [ ] オンフック後に warning ログが連発しない
- [ ] 正常な録音告知完了時（BYE なし）は従来通り転送が起動される
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-21 |
| 起票理由 | 着信中アナウンス送信中に切断されたにもかかわらず呼び出しが行われ、大量 warn ログが発生 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-sonnet-4-6) |
| 作成日 | 2026-02-21 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Issue #206 のステアリング作成。Codex 調査結果（BYE後のAppTransferRequest誤起動）を元に仕様化" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | Codex | 2026-02-21 | OK | 重大①②・中・軽 の4件を NG 後修正し再レビューで合格。残リスク2点を §8 に追記 |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-21 |
| 承認コメント | |

### 3.5 実装

| 項目 | 値 |
|------|-----|
| 実装者 | Codex |
| 実装日 | |
| 指示者 | @MasanoriSuda |
| 指示内容 | "§5 差分仕様に従い最小差分で修正" |
| コードレビュー | |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | |
| マージ日 | |
| マージ先 | DD-005_session.md |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| `docs/design/detail/DD-005_session.md` | 修正 | セッション状態遷移ガード（Terminated 時の AppTransferRequest 無効化）を追記 |

### 4.2 影響するコード

| ファイル | 行 | 変更種別 | 概要 |
|---------|-----|---------|------|
| `src/protocol/session/services/playback_service.rs` | 120–127 | 修正 | `cancel_playback()` を `clear_playback_state()` のみ呼ぶ形に変更、`clear_playback_state()` を新規追加 |
| `src/protocol/session/handlers/mod.rs` | 609–612 | 修正 | `AppTransferRequest` ハンドラに `Established` 状態ガードを追加 |
| `src/protocol/session/services/b2bua_service.rs` | 62–66 | 修正 | `b_leg = None` 時のログを条件付き `warn` / `debug` に変更 |

---

## 5. 差分仕様（What / How）

### 5.1 バグの根本原因

BYE 受信時のイベント処理順序が以下の問題を引き起こしている。

```text
SipBye 受信
  └─ mod.rs:548   SipBye ハンドラ
       ├─ mod.rs:552   shutdown_b_leg(true)   → B-leg を終了
       └─ mod.rs:553   cancel_playback()      → ★ ここが問題の起点
            └─ playback_service.rs:126   finish_playback(false)
                 └─ playback_service.rs:84  announce_mode && recording_notice_pending が true なら
                      └─ control_tx.try_send(AppTransferRequest { "recording_notice" })  ← 誤って enqueue

  types.rs:268   SipBye → SessState::Terminated への遷移

  その後キューを処理:
  └─ mod.rs:609   AppTransferRequest ハンドラ（状態チェックなし）
       └─ start_b2bua_transfer() 実行  ← Terminated 状態で B-leg 転送が起動してしまう
```

**核心**: `cancel_playback()` は「再生のキャンセル」を意図しているが、内部で `finish_playback()` を経由するため「再生完了後アクション（転送・ハングアップ）」まで発火してしまう。また `AppTransferRequest` ハンドラに状態ガードがない。

**warning 連発の主因**:
- 終話後に `shutdown_b_leg(true)` が複数経路から呼ばれ、`b_leg = None` の場合に `warn` が出力される（`b2bua_service.rs:62`）
- 誤起動した B-leg 側が生きていると `BLegRtp` 投入で channel full `warn` が連発する（`b2bua.rs:1094`）

---

### 5.2 修正① `cancel_playback()` でアクションを発火させない（主要因修正）

**対象**: `src/protocol/session/services/playback_service.rs` 120–127行

**現状**:
```rust
pub(crate) fn cancel_playback(&mut self) {
    if self.playback.is_none() {
        self.sending_audio = false;
        return;
    }
    info!("[session {}] playback cancelled", self.call_id);
    self.finish_playback(false);   // ← finish_playback を経由するためアクションが発火する
}
```

**修正後（Q1 解決済み: 直接クリア方式を採用）**:

`clear_playback_state()` を新規追加し、**再生状態（`playback` / `sending_audio`）のみ**をクリアする。
`announce_mode` / `recording_notice_pending` は `clear_playback_state()` では触らない。
`cancel_playback()` は `clear_playback_state()` 後に**明示的に**両フラグをクリアする。
`finish_playback()` は現行の判定フロー（`announce_mode` → Branch 分岐）を維持したまま、先頭で `clear_playback_state()` を呼ぶ形にリファクタリングする。

```rust
/// 再生状態（playback / sending_audio）のみをクリアする。
/// announce_mode / recording_notice_pending には触れない。
fn clear_playback_state(&mut self) {
    self.playback = None;
    self.sending_audio = false;
}

pub(crate) fn cancel_playback(&mut self) {
    if self.playback.is_none() {
        self.sending_audio = false;
        return;
    }
    info!("[session {}] playback cancelled", self.call_id);
    self.clear_playback_state();
    // キャンセル時はアクション（Transfer/Hangup）を発火しないため
    // announce_mode / recording_notice_pending を明示的にクリアする。
    // finish_playback() は呼ばない。
    self.announce_mode = false;
    self.recording_notice_pending = false;
}

pub(crate) fn finish_playback(&mut self, restart_ivr_timeout: bool) {
    self.clear_playback_state();   // ← 新規呼び出し（既存の playback=None, sending_audio=false を置換）
    // announce_mode / recording_notice_pending はここで判定する（変更なし）
    if self.announce_mode {
        // ... 既存の Branch A / B / C 分岐は変更なし ...
    }
    // ... 既存の IVR タイムアウト処理は変更なし ...
}
```

**不変条件（修正後に成り立つべき前提）**:
- `cancel_playback()` は「再生を中断する」のみであり、`AppTransferRequest` や `AppHangup` を発火しない。
- `finish_playback()` は `announce_mode` 判定時点でフラグが残っているため、正常完了時の転送判定は成立する。
- `from_cancel: bool` 引数は追加しない（関数境界で責務を分離する方針）。

---

### 5.3 修正② `AppTransferRequest` ハンドラへの状態ガード追加（防御的修正）

**対象**: `src/protocol/session/handlers/mod.rs` 609–612行

`handle_control_event()` は `current_state: SessState` を引数に取り `match (current_state, ev)` でパターンマッチする（`coordinator.rs:293–300`）。
`self.state` フィールドは存在せず、`self.state_machine.state()` で取得するが、ハンドラ内では `current_state` 引数を使うのが正しい実装パターン（`SipReInvite`, `RtpPayload` ハンドラと同様）。

**現状**:
```rust
// current_state をワイルドカードで無視している
(_, SessionControlIn::AppTransferRequest { person }) => {
    if !self.start_b2bua_transfer(person.as_str()) {
        return false;
    }
}
```

**修正後**:
```rust
// Established 状態のみ転送を実行
(SessState::Established, SessionControlIn::AppTransferRequest { person }) => {
    if !self.start_b2bua_transfer(person.as_str()) {
        return false;
    }
}
// それ以外の状態では無視
(state, SessionControlIn::AppTransferRequest { person }) => {
    warn!(
        "[session {}] AppTransferRequest ignored in state {:?} (person={})",
        self.call_id, state, person
    );
}
```

**目的**: 修正①で `cancel_playback()` からの誤 enqueue を防ぐが、他経路で `AppTransferRequest` が `Terminated` 状態で届いた場合の防御として、ハンドラ側でも状態チェックを追加する。
`current_state` を match パターンに含める方式は既存の `SipReInvite` / `RtpPayload` ハンドラと一致し、実装者が迷わない。

---

### 5.4 修正③ `shutdown_b_leg` の `b_leg = None` 時ログ格下げ（Q2 解決済み）

**対象**: `src/protocol/session/services/b2bua_service.rs` 62–66行

`b_leg = None` での `shutdown_b_leg` 呼び出しは、着信中断や転送未確立など通常の状態遷移でも発生するため、一律 `warn` はノイズになる。

**修正後**:
```rust
} else {
    if self.ivr_state == IvrState::B2buaMode {
        // B2BUA 確立済みのはずなのに b_leg が None → 不整合として warn
        warn!(
            "[session {}] shutdown_b_leg called but b_leg is None (send_bye={}) in B2buaMode",
            self.call_id, send_bye
        );
    } else {
        // B2BUA 未確立での呼び出しは通常起こりうる → debug
        debug!(
            "[session {}] shutdown_b_leg called but b_leg is None (send_bye={})",
            self.call_id, send_bye
        );
    }
}
```

**目的**: `IvrState::B2buaMode`（B2BUA 確立済み）で `b_leg = None` の場合のみ `warn` を残し、それ以外は `debug` に格下げして warning ノイズを削減する。

---

### 5.5 スコープ外

`shutdown_b_leg(true)` の複数経路呼び出しによる `warn` 連発は、修正①②で B-leg の誤起動を防いだ後は自然に減少する見込み。
`b_leg = None` 時の `warn` → `debug` 格下げは修正③（§5.4）本チケットで実施する。

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #206 | STEER-206 | 起票 |
| STEER-206 | playback_service.rs:120 | 修正①対象 |
| STEER-206 | mod.rs:609 | 修正②対象 |
| STEER-206 | b2bua_service.rs:62 | 修正③対象 |
| STEER-206 | DD-005_session.md | 設計書更新 |

---

## 7. 未確定点・質問リスト（Open Questions）

| # | 質問 | 状態 | 回答 |
|---|------|------|------|
| Q1 | `cancel_playback()` の実装方針: 直接クリア vs `from_cancel: bool` 引数追加 | **解決済み** | `clear_playback_state()` を共通ヘルパーとして新規追加し、`finish_playback()` は通常完了専用、`cancel_playback()` は状態クリアのみに責務を分離する（§5.2 参照）。`from_cancel: bool` 引数は追加しない。 |
| Q2 | `shutdown_b_leg` の `b_leg = None` 時 `warn` を `debug` に格下げすべきか？ | **解決済み** | `IvrState::B2buaMode` かつ `b_leg = None` の場合のみ `warn`（不整合として検知）、それ以外は `debug` に格下げ（§5.4 参照）。 |

---

## 8. リスク / ロールバック観点

| リスク | 対処 |
|--------|------|
| 修正①で正常な録音告知完了ケースが壊れる場合 | 正常完了は `finish_playback()` 経由（`cancel_playback()` は通らない）なので影響なし。既存テスト `finish_playback_requests_transfer_after_recording_notice`（`coordinator.rs:974`）が継続 PASS することで確認する |
| 修正②の状態ガードが早すぎる遷移で正常転送をブロックする場合 | `AppTransferRequest` は `start_b2bua_transfer()` を呼ぶ直前のみ有効であり、`Established` でない場合はそもそも転送を行う意図がない |
| 修正② 実装時の match 順序ミス（意図せずワイルドカードに吸われる） | `(SessState::Established, ...)` アームを既存の `(_, ...)` ワイルドカードアームより**前に**配置すること。Rust のパターンマッチは上から評価されるため、逆順では Established でも常に warn ログが出力される |
| `cancel_playback_does_not_trigger_transfer` テスト未追加 | 受入条件に追加済み（§9 最後から2行目）。実装者は BYE 受信 → `cancel_playback()` → `AppTransferRequest` 未発行を検証する新規テストを必ず追加すること |

---

## 9. 受入条件（実装確認用）

- [ ] 録音告知アナウンス送信中に A-leg から BYE を受信した場合、B-leg への転送が起動されない
- [ ] BYE 受信後、オンフック時に `shutdown_b_leg` の多重呼び出しによる `warn` が連発しない
- [ ] 正常な録音告知完了時（BYE なし）は従来通り B-leg への転送が起動される
- [ ] `AppTransferRequest` を `Terminated` 状態で受け取った場合、`warn` ログを出力して無視する
- [ ] `AppTransferRequest` の match パターンで `(SessState::Established, ...)` が `(_, ...)` ワイルドカードより前に配置されている
- [ ] `IvrState::B2buaMode` かつ `b_leg = None` の場合のみ `warn` ログが出力され、それ以外は `debug` になる（修正③）
- [ ] 既存テスト `finish_playback_requests_transfer_after_recording_notice` が PASS する
- [ ] 既存テスト `cancel_playback_clears_state` が PASS する
- [ ] BYE 中断ケースの新規テスト（`cancel_playback_does_not_trigger_transfer`）が PASS する
- [ ] `cargo test` が pass する

---

## 10. レビューチェックリスト

### 10.1 仕様レビュー（Review → Approved）

- [ ] バグの根本原因の説明が正確か
- [ ] 修正①と修正②が最小差分か
- [ ] 不変条件が明確か
- [ ] Q1〜Q2 の回答方針が決まっているか
- [ ] 正常な録音告知完了ケースへの影響がないことが説明されているか

### 10.2 マージ前チェック（Approved → Merged）

- [ ] 実装が完了している
- [ ] CodeRabbit レビューを受けている
- [ ] 関連テストが PASS
- [ ] DD-005_session.md の更新準備ができている

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-21 | 初版作成 | Claude Code (claude-sonnet-4-6) |
| 2026-02-21 | Q1・Q2 回答を反映（§5.2 に clear_playback_state() 追加、§5.3 に b_leg ログ格下げ追加、OQ 全件解決済み） | Claude Code (claude-sonnet-4-6) |
| 2026-02-21 | Codex NG 修正: 重大①（self.state → current_state match パターン）、重大②（clear_playback_state() スコープ制限）、中（§5.4→§5.5 スコープ外を §5.4 参照に整合）、軽（§5.3 見出し重複解消 → fix③ を §5.4 に繰り上げ） | Claude Code (claude-sonnet-4-6) |
| 2026-02-21 | Codex レビュー OK 受領: §3.3 に結果記録、残リスク2点（match 順序・テスト必須）を §8 に追記 | Claude Code (claude-sonnet-4-6) |
| 2026-02-21 | @MasanoriSuda 承認: ステータス Draft → Approved、§3.4 記入、index.md 更新 | Claude Code (claude-sonnet-4-6) |
| 2026-02-21 | CodeRabbit 指摘反映: §9 に match パターン順序の受入条件を追加 | Claude Code (claude-sonnet-4-6) |
