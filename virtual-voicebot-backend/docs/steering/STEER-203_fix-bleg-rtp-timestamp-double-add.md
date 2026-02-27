# STEER-203: B-leg → A-leg RTP タイムスタンプ二重加算バグ修正

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-203 |
| タイトル | B-leg → A-leg RTP タイムスタンプ二重加算バグ修正 |
| ステータス | Approved |
| 関連Issue | #203 |
| 優先度 | P0 |
| 作成日 | 2026-02-20 |

---

## 2. ストーリー（Why）

### 2.1 背景

通常着信時、着信側から発信側への通話音声品質が非常に悪い（パケットキャプチャで聴取済み）。
Codex による静的解析（#203）により、B-leg → A-leg 転送時に RTP タイムスタンプが実質二重加算されていることが判明した。

放置すると:
- 受信側 RTP スタックがタイムスタンプの飛びをパケット損失と誤認し、音声復元が失敗し続ける
- 片方向（着信側 → 発信側）のみ音声が劣化し続け、通常着信の用途で製品として使用不能

### 2.2 目的

B-leg → A-leg RTP 転送時のタイムスタンプ管理を修正し、片方向の音声劣化を解消する。
併せて、B-leg 受信パケット破棄の可視化を追加し、負荷時の障害追跡を容易にする。

### 2.3 ユーザーストーリー

```text
As a 通話ユーザー（着信側）
I want to 発信者の声を聞き取れるレベルの品質で聞きたい
So that 通常着信の通話が成立する

受入条件:
- [ ] 着信側から発信側への音声が聞き取れるレベルになる
- [ ] パケットキャプチャで B-leg → A-leg 転送の RTP タイムスタンプ連続性が保たれている（20ms = +160 サンプル/パケット）
- [ ] try_send 失敗時に warn（Full）または error（Closed）ログが出力される
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-20 |
| 起票理由 | 通常着信で着信側 → 発信側音声が聞き取り不能（パケットキャプチャ確認済み） |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-sonnet-4-6) |
| 作成日 | 2026-02-20 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Issue #203 のステアリング作成。Codex 調査結果（タイムスタンプ二重加算）を元に仕様化" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | Codex | 2026-02-20 | NG（要修正） | 中×2（不変条件矛盾・try_send Full/Closed 未分岐）、軽×1（align 加算値の固定値表現） |
| 2 | Codex | 2026-02-20 | OK | 指摘3点すべて解消確認。残リスク2点を §8 に補足 |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-20 |
| 承認コメント | 承認 |

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
| マージ先 | DD-004_rtp.md, DD-005_session.md |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| `docs/design/detail/DD-004_rtp.md` | 修正 | B2BUA 転送時のタイムスタンプ管理仕様を追記 |
| `docs/design/detail/DD-005_session.md` | 修正 | B2BUA モード遷移時の RTP クロック処理を明確化 |

### 4.2 影響するコード

| ファイル | 行 | 変更種別 | 概要 |
|---------|-----|---------|------|
| `src/protocol/session/handlers/mod.rs` | 890 | 修正 | B2BUA 転送時の `align_rtp_clock()` 呼び出しを除去 |
| `src/protocol/session/b2bua.rs` | 1087 | 修正 | `try_send` 失敗時に `warn!` ログを追加 |

---

## 5. 差分仕様（What / How）

### 5.1 バグの根本原因

B2BUA モード（`IvrState::B2buaMode`）で B-leg から受信した RTP パケットを A-leg に転送する際、以下の二段階でタイムスタンプが加算される。

**現状の処理フロー（`mod.rs` 890–893行）**:

```text
1. align_rtp_clock() を呼ぶ
   └─ rtp_handler.rs の align_rtp_clock() が実行される（48–54行）
      └─ gap_samples = 最後の送信からの経過時間 × 8000.0
      └─ adjust_timestamp(gap_samples) → stream.ts += gap_samples
2. rtp.send_payload() を呼ぶ
   └─ tx.rs 内部で送信後に stream.ts += pcm_len (= 160)
```

IVR メッセージ再生では、`align_rtp_clock()` はメッセージ間の無音ギャップを補正するために意図的に設計されている。
しかし B2BUA 転送では B-leg からリアルタイムで 20ms ごとにパケットが到着するため、`align_rtp_clock()` が「最後の IVR 送信からの時間ギャップ」を計算してタイムスタンプを先送りし、その後 `send_payload()` がさらに +160 を加算する。

結果: 20ms パケットに対して、`align` 側で経過時間依存の加算（20ms 周期なら概ね +160 サンプル）が行われ、さらに `send_payload()` 内で +160 が加算される。
合計で概ね +320/パケット となり、受信側がタイムスタンプの飛びをパケット損失と誤認して音声が劣化する。

---

### 5.2 修正① B-leg 転送時の `align_rtp_clock()` 除去（主要因修正）

**対象**: `src/protocol/session/handlers/mod.rs` 890行付近

**現状**:
```rust
if self.ivr_state == IvrState::B2buaMode {
    self.align_rtp_clock();          // ← ここが問題
    self.recording.push_tx(&payload);
    self.recording.push_b_leg_rx(&payload);
    self.rtp.send_payload(self.call_id.as_str(), payload);
    self.rtp_last_sent = Some(Instant::now());
}
```

**修正後**:
```rust
if self.ivr_state == IvrState::B2buaMode {
    // B-leg からリアルタイム転送時は align_rtp_clock() を呼ばない。
    // タイムスタンプは send_payload() 内部で +pcm_len される（これで十分）。
    self.recording.push_tx(&payload);
    self.recording.push_b_leg_rx(&payload);
    self.rtp.send_payload(self.call_id.as_str(), payload);
    self.rtp_last_sent = Some(Instant::now());
}
```

**不変条件（修正後に成り立つべき前提）**:
- B2BUA 転送中、A-leg に送出される RTP タイムスタンプは 20ms ごとに +160 だけ単調増加する。
- IVR 再生からの遷移直後（最初の B-leg 受信時）は `rtp_last_sent` はリセットされない（Q1 解決済み）。
  `align_rtp_clock()` を除去した後は `send_payload()` 内の `+pcm_len` のみが走るため、二重加算は生じない。
  ただし遷移直後の挙動は実機確認を必須とする。

---

### 5.3 修正② B-leg `try_send` 失敗の可視化（副因修正）

**対象**: `src/protocol/session/b2bua.rs` 1087行付近

**現状**:
```rust
let _ = media_tx.try_send(SessionMediaIn::BLegRtp {
    call_id: a_call_id.clone(),
    stream_id: "b-leg".to_string(),
    payload,
});
```

**現在のコード（コード確認済み）**: `match` による `Full`/`Closed` 分岐は実装済みだが、`Closed` 時に `break` がなくループが継続する。

```rust
// b2bua.rs 1099–1104 現状（break なし）
Err(mpsc::error::TrySendError::Closed(_)) => {
    error!(
        "[b2bua {} stream=b-leg] B-leg RTP drop: channel closed",
        a_call_id
    );
    // ← ここで break が必要
}
```

**修正後**:
```rust
Err(mpsc::error::TrySendError::Full(_)) => {
    warn!(
        "[b2bua {} stream=b-leg] B-leg RTP drop: channel full",
        a_call_id
    );
}
Err(mpsc::error::TrySendError::Closed(_)) => {
    error!(
        "[b2bua {} stream=b-leg] B-leg RTP drop: channel closed",
        a_call_id
    );
    break;  // セッション終了済み。これ以上ループを継続しない
}
```

**目的**: チャネル満杯（`Full`）とチャネル閉塞（`Closed`）を区別して可視化する。
- `Full`: 高負荷によるバックプレッシャー。`warn` で検知し、頻度次第でキャパシティ調整を検討。ループは継続。
- `Closed`: セッション終了後にチャネルが閉じられた状態。`error` で検知後、`break` でループを終了する。
  `break` 後は既存の後続処理（`info!("[b2bua {}] rtp listener ended", ...)` ログ）が実行される。

---

### 5.4 スコープ外（P1 別チケット）

B-leg 受信側のジッター再整列（reorder バッファ）は現状未実装（Q2 回答により本チケットから分離）。

- A-leg 受信（`rx.rs` 151行）では reorder 実装済み
- B-leg 受信は `b2bua.rs` 1073行以降で parse → decode → `try_send` 直送（seq/ts 保持なし）
- reorder 導入は seq/ts 伝搬とバッファ構造の変更を伴い修正範囲が広がるため、P0 主因修正（修正①②）とは別チケットで対処する

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #203 | STEER-203 | 起票 |
| STEER-203 | mod.rs:890 | 修正①対象 |
| STEER-203 | b2bua.rs:1087 | 修正②対象 |
| STEER-203 | DD-004_rtp.md | 設計書更新 |
| STEER-203 | DD-005_session.md | 設計書更新 |

---

## 7. 未確定点・質問リスト（Open Questions）

| # | 質問 | 状態 | 回答 |
|---|------|------|------|
| Q1 | IVR → B2BUA 遷移直後、`rtp_last_sent` はリセットされるか？ | **解決済み** | `mod.rs` 332行付近の `B2buaEstablished` 処理に `rtp_last_sent = None` が無い（リセットなし）。`align_rtp_clock()` が B2BUA 遷移後も「前回 IVR 送信からのギャップ」を加算し続けるため、B-leg 転送経路での除去は妥当。 |
| Q2 | B-leg 受信の jitter reorder を本チケットに含めるか？ | **解決済み** | P1 別チケットに分離。reorder 追加は seq/ts 伝搬・バッファ構造変更を伴い修正範囲が広がるため、P0 主因修正と切り分ける（§5.4 参照）。 |
| Q3 | `media_tx` チャネルのキャパシティは？ | **解決済み** | 容量 64（`coordinator.rs` 41行 `SESSION_MEDIA_CHANNEL_CAPACITY`、171行で生成）。20ms/packet 前提で約 1.28 秒分。`try_send` 失敗の黙捨ては可視化（修正②）が先決で、キャパシティ調整は warn ログ確認後に判断する。 |

---

## 8. リスク / ロールバック観点

| リスク | 対処 |
|--------|------|
| 修正①適用後に IVR → B2BUA 遷移直後に音飛びが発生する場合 | Q1 回答により `rtp_last_sent` はリセットされないため、遷移直後の最初のパケットで `send_payload()` 内の `+pcm_len` のみが走る（問題なし）。ただし **実機での RTP タイムスタンプ連続性確認（20ms ごとに +160）は必須**。 |
| 修正②で warn ログが大量出力されてログが埋もれる場合 | `media_tx` 容量は 64（約 1.28 秒分）。短時間で 64 超えは高負荷状態を示すため、まず warn で可視化し、頻度が高い場合は rate-limit ログへの変更を検討する。 |
| `Closed` ログが実運用で発生した場合の調査観点 | `Closed` 検出時に `break` でループを終了する（修正②）。`break` 後は既存の `rtp listener ended` ログでクリーンアップ完了を確認できる。 |

---

## 9. 受入条件（実装確認用）

- [ ] B-leg → A-leg 転送中の RTP タイムスタンプが 20ms ごとに +160 で単調増加している（パケットキャプチャで確認）
- [ ] 着信側から発信側への音声が聞き取れるレベルになる
- [ ] `media_tx.try_send` が `Full` の場合に `warn` ログが出力される
- [ ] `media_tx.try_send` が `Closed` の場合に `error` ログを出力後、RTP リスナーループが `break` で終了する
- [ ] `Closed` で `break` 後、`info!("[b2bua {}] rtp listener ended", ...)` ログが出力される（クリーンアップ確認）
- [ ] IVR 再生モード（`B2buaMode` 以外）の動作に影響がないこと（既存テスト通過）
- [ ] `cargo test` が pass する

---

## 10. レビューチェックリスト

### 10.1 仕様レビュー（Review → Approved）

- [ ] バグの根本原因の説明が正確か
- [ ] 修正箇所が最小差分か
- [ ] 不変条件が明確か
- [ ] Q1〜Q3 の回答方針が決まっているか
- [ ] 既存の IVR 再生への影響がないことが説明されているか

### 10.2 マージ前チェック（Approved → Merged）

- [ ] 実装が完了している
- [ ] CodeRabbit レビューを受けている
- [ ] 関連テストが PASS
- [ ] DD-004_rtp.md / DD-005_session.md の更新準備ができている

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-20 | 初版作成 | Claude Code (claude-sonnet-4-6) |
| 2026-02-20 | Q1〜Q3 回答をステアリングへ反映（全 OQ 解決済み） | Claude Code (claude-sonnet-4-6) |
| 2026-02-20 | Codex レビュー指摘3点を修正（不変条件の矛盾解消・try_send Full/Closed 分岐・align 加算値の表現修正） | Claude Code (claude-sonnet-4-6) |
| 2026-02-20 | Codex レビュー OK 確認。§3.3 にレビュー結果記録、§8 に残リスク2点補足 | Claude Code (claude-sonnet-4-6) |
| 2026-02-20 | @MasanoriSuda 承認。Status: Approved | Claude Code (claude-sonnet-4-6) |
| 2026-02-20 | §5.3 に Closed 時 break 追加（コード確認済み: 現状 break なし）、§9 受入条件に Closed ループ終了を追記 | Claude Code (claude-sonnet-4-6) |
