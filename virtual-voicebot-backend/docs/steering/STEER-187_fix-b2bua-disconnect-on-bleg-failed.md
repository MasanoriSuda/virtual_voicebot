# STEER-187: 転送中のB-leg切断時にA-legも切断する

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-187 |
| タイトル | 転送中のB-leg切断時にA-legも切断する |
| ステータス | Approved |
| 関連Issue | #187 |
| 優先度 | P1 |
| 作成日 | 2026-02-16 |

---

## 2. ストーリー（Why）

### 2.1 背景

現在、着信時の転送中（B2BUA による B-leg 確立中）に着信側（B-leg）が切断した場合、発信側（A-leg）が切断されない問題が発生している。

**現状のコード仕様**：
- **BLegBye（B-leg確立後の切断）**： [mod.rs:410-427](virtual-voicebot-backend/src/protocol/session/handlers/mod.rs#L410-L427)
  - B-legからBYEを受信 → A-legにもBYEを送る（正常動作）
- **B2buaFailed（B-leg確立前の失敗）**： [mod.rs:376-408](virtual-voicebot-backend/src/protocol/session/handlers/mod.rs#L376-L408)
  - outbound_mode：A-legにエラー応答（503 Service Unavailable）を返す
  - IVR mode：IVRメニューに戻す（**A-legは切断しない**）← 問題箇所

つまり、B-leg確立前に相手側が切断した場合は `B2buaFailed` 扱いになり、IVR mode では A-leg が切断されない仕様になっている。

**追加の問題**：
- イベント伝達に `try_send` を使用しているため、チャネルが満杯の場合にイベントが取りこぼされる可能性がある
  - [b2bua.rs:819](virtual-voicebot-backend/src/protocol/session/b2bua.rs#L819) - BLegBye
  - [b2bua.rs:107](virtual-voicebot-backend/src/protocol/session/b2bua.rs#L107) - B2buaFailed
  - [b2bua.rs:143](virtual-voicebot-backend/src/protocol/session/b2bua.rs#L143) - B2buaFailed

### 2.2 目的

転送中に着信側（B-leg）が切断した場合、確立前/確立後に関わらず、発信側（A-leg）も確実に切断する。

### 2.3 ユーザーストーリー

```
As a 発信者（A-leg）
I want 転送先（B-leg）が切断したら自分も切断される
So that いつまでも電話が繋がったままにならない

受入条件:
- [ ] IVR mode で B-leg確立前に切断された場合、A-legにBYEが送られる
- [ ] IVR mode で B-leg確立後に切断された場合も、引き続きA-legにBYEが送られる（現状維持）
- [ ] outbound mode では B2buaFailed 時に 503 エラー応答が返される（現状維持）
- [ ] イベント送信失敗を検知し、エラーログを出力できる
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-16 |
| 起票理由 | 転送中にB-legが切断してもA-legが切断されない不具合 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-sonnet-4-5) |
| 作成日 | 2026-02-16 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Issue #187のステアリングファイルを作成" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | @MasanoriSuda | 2026-02-16 | 要修正 | 重大2件、中2件の指摘 |
| 2 | @MasanoriSuda | 2026-02-16 | 要修正 | 重大1件、中1件、軽1件の指摘 |
| 3 | @MasanoriSuda | 2026-02-16 | 要修正 | 重大1件、中1件、軽1件の指摘 |
| 4 | @MasanoriSuda | 2026-02-16 | 要修正 | 重大1件、中1件の指摘 |
| 5 | @MasanoriSuda | 2026-02-16 | 要修正 | 重大1件、中1件の指摘 |
| 6 | @MasanoriSuda | 2026-02-16 | OK | レビュー通過 |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-16 |
| 承認コメント | レビュー#6 で OK 判定。5回のレビューを経て仕様が明確化された。 |

### 3.5 実装（該当する場合）

| 項目 | 値 |
|------|-----|
| 実装者 | Codex (GPT-5) |
| 実装日 | 2026-02-16 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "/home/msuda/workspace/virtual_voicebot_4/virtual_voicebot/virtual-voicebot-backend/docs/steering/STEER-187_fix-b2bua-disconnect-on-bleg-failed.mdに対する実装をお願いします" |
| コードレビュー | 未実施（CodeRabbit想定） |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | |
| マージ日 | |
| マージ先 | RD-xxx, DD-xxx, UT-xxx |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| docs/design/detail/DD-xxx.md | 修正 | B2buaFailed時の切断処理を追加 |
| docs/test/unit/UT-xxx.md | 追加 | B2buaFailed時のA-leg切断テスト |
| docs/test/integration/IT-xxx.md | 追加 | 転送中切断のE2Eテスト |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| src/protocol/session/handlers/mod.rs | 修正 | B2buaFailed時の処理変更 |
| src/protocol/session/b2bua.rs | 修正 | イベント送信の信頼性向上（try_send + リトライ） |
| src/protocol/session/handlers/sip_handler.rs | 修正 | send_bye_to_a_leg のエラーハンドリング強化 |
| src/protocol/sip/core.rs | 修正 | transport_tx.try_send のエラーハンドリング強化 |

---

## 5. 差分仕様（What / How）

### 5.1 要件追加（RD-xxx へマージ）

```markdown
## RD-xxx-FR-xx: 転送中のB-leg切断時にA-legも切断する（IVR mode）

### 概要
IVR mode で転送中に着信側（B-leg）が切断した場合、確立前/確立後に関わらず、発信側（A-leg）にBYEを送って切断する。

**注**: outbound mode は現状維持（B2buaFailed 時に 503 Service Unavailable を返し、A-leg は切断しない）

### 入力
- B2buaFailed イベント（B-leg確立前の失敗、IVR mode のみ）
- BLegBye イベント（B-leg確立後の切断、現状維持）

### 出力
- A-legへのBYE送信（IVR mode のみ）
- セッション終了（IVR mode のみ）

### 受入条件
- [ ] IVR mode で B-leg確立前に切断された場合、A-legにBYEが送られる
- [ ] IVR mode で B-leg確立後に切断された場合も、引き続きA-legにBYEが送られる（現状維持）
- [ ] outbound mode では B2buaFailed 時に 503 エラー応答が返される（現状維持）
- [ ] イベント送信失敗を検知し、エラーログを出力できる

### 優先度
P1

### トレース
- → DD: DD-xxx-FN-xx
- → IT: IT-xxx-TC-xx
```

---

### 5.2 詳細設計追加（DD-xxx へマージ）

```markdown
## DD-xxx-FN-01: B2buaFailed時の切断処理

### 対象
SessionHandler::handle_control_in における B2buaFailed 処理
[mod.rs:376-408](virtual-voicebot-backend/src/protocol/session/handlers/mod.rs#L376-L408)

### 変更内容

**現状**：
```rust
(_, SessionControlIn::B2buaFailed { reason, status }) => {
    warn!("[session {}] transfer failed: {}", self.call_id, reason);
    self.transfer_cancel = None;
    self.stop_transfer_announce();
    self.mark_transfer_failed();
    if self.outbound_mode {
        // エラー応答を返す
        let code = status.unwrap_or(503);
        let _ = self.session_out_tx.try_send(...);
        self.outbound_mode = false;
        self.invite_rejected = true;
    } else {
        // IVRメニューに戻す（A-legは切断しない）← 問題
        self.ivr_state = IvrState::IvrMenuWaiting;
        self.b_leg = None;
        if let Err(e) = self.start_playback(...).await { ... }
    }
}
```

**変更後**：
```rust
(_, SessionControlIn::B2buaFailed { reason, status }) => {
    warn!("[session {}] transfer failed: {}", self.call_id, reason);
    self.transfer_cancel = None;
    self.stop_transfer_announce();
    self.mark_transfer_failed();
    if self.outbound_mode {
        // エラー応答を返す（現状維持）
        let code = status.unwrap_or(503);
        let _ = self.session_out_tx.try_send(...);
        self.outbound_mode = false;
        self.invite_rejected = true;
    } else {
        // IVR mode でも A-leg を切断する
        self.cancel_transfer();
        self.shutdown_b_leg(false).await;
        self.cancel_playback();
        self.stop_keepalive_timer();
        self.stop_session_timer();
        self.stop_ivr_timeout();
        self.mark_transfer_ended();
        self.send_bye_to_a_leg();  // 追加
        self.stop_recorders();
        self.send_ingest("ended").await;
        self.rtp.stop(self.call_id.as_str());
        let _ = self.session_out_tx.send((self.call_id.clone(), SessionOut::RtpStopTx)).await;
        self.send_call_ended(EndReason::Error);  // 追加（転送失敗はエラー扱い）
    }
}
```

### 処理フロー
1. B2buaFailed イベントを受信
2. 転送キャンセル処理
3. B-leg シャットダウン（存在する場合）
4. A-leg に BYE を送信（**IVR mode でも実施**）
5. セッション終了

### トレース
- ← RD: RD-xxx-FR-xx
- → UT: UT-xxx-TC-01
```

```markdown
## DD-xxx-FN-02: イベント送信の信頼性向上

### 対象
B2BUA イベント送信処理
- [b2bua.rs:819](virtual-voicebot-backend/src/protocol/session/b2bua.rs#L819) - BLegBye
- [b2bua.rs:107](virtual-voicebot-backend/src/protocol/session/b2bua.rs#L107) - B2buaFailed
- [b2bua.rs:143](virtual-voicebot-backend/src/protocol/session/b2bua.rs#L143) - B2buaFailed
- [sip_handler.rs:79-89](virtual-voicebot-backend/src/protocol/session/handlers/sip_handler.rs#L79-L89) - send_bye_to_a_leg
- [core.rs:1740](virtual-voicebot-backend/src/protocol/sip/core.rs#L1740) - transport_tx.try_send

### 変更内容

**現状**：
```rust
// b2bua.rs
let _ = control_tx.try_send(SessionControlIn::BLegBye);
let _ = control_tx.try_send(SessionControlIn::B2buaFailed { ... });

// sip_handler.rs
pub(crate) fn send_bye_to_a_leg(&self) {
    if let Err(err) = self.session_out_tx.try_send(...) {
        log::warn!("[session {}] dropped SipSendBye (channel full): {:?}", ...);
    }
}

// core.rs
let _ = self.transport_tx.try_send(SipTransportRequest { ... });
```

**変更後（try_send + リトライ）**：
```rust
// b2bua.rs - SessionControlIn イベント送信
// ※SessionControlIn は Clone ではないため、イベントごとに再送信を実装

// BLegBye の送信（リトライあり）
for attempt in 0..3 {
    match control_tx.try_send(SessionControlIn::BLegBye) {
        Ok(_) => break,
        Err(mpsc::error::TrySendError::Full(_)) => {
            if attempt < 2 {
                warn!("[b2bua] BLegBye event channel full, retrying ({}/3)", attempt + 1);
                tokio::time::sleep(Duration::from_millis(10)).await;
            } else {
                error!("[b2bua] failed to send BLegBye event after 3 retries");
            }
        }
        Err(mpsc::error::TrySendError::Closed(_)) => {
            error!("[b2bua] BLegBye event channel closed");
            break;
        }
    }
}

// B2buaFailed の送信（リトライあり）

// ① run_transfer（IVR mode）の場合
// ※status: None 固定
let reason_str = err.to_string();  // err は外側の match Err(err) から
for attempt in 0..3 {
    match control_tx.try_send(SessionControlIn::B2buaFailed {
        reason: reason_str.clone(),
        status: None,
    }) {
        Ok(_) => break,
        Err(mpsc::error::TrySendError::Full(_)) => {
            if attempt < 2 {
                warn!("[b2bua] B2buaFailed event channel full, retrying ({}/3)", attempt + 1);
                tokio::time::sleep(Duration::from_millis(10)).await;
            } else {
                error!("[b2bua] failed to send B2buaFailed event after 3 retries");
            }
        }
        Err(mpsc::error::TrySendError::Closed(_)) => {
            error!("[b2bua] B2buaFailed event channel closed");
            break;
        }
    }
}

// ② run_outbound（outbound mode）の場合
// ※status を伝搬（err.downcast_ref::<OutboundError>().map(|e| e.status)）
let reason_str = err.to_string();
let status_code = err.downcast_ref::<OutboundError>().map(|e| e.status);
for attempt in 0..3 {
    match control_tx.try_send(SessionControlIn::B2buaFailed {
        reason: reason_str.clone(),
        status: status_code,
    }) {
        Ok(_) => break,
        Err(mpsc::error::TrySendError::Full(_)) => {
            if attempt < 2 {
                warn!("[b2bua] B2buaFailed event channel full, retrying ({}/3)", attempt + 1);
                tokio::time::sleep(Duration::from_millis(10)).await;
            } else {
                error!("[b2bua] failed to send B2buaFailed event after 3 retries");
            }
        }
        Err(mpsc::error::TrySendError::Closed(_)) => {
            error!("[b2bua] B2buaFailed event channel closed");
            break;
        }
    }
}

// sip_handler.rs - SessionOut イベント送信（現状維持、ログ出力のみ）
// ※既にログ出力している

// core.rs - SipTransportRequest 送信
// ※ネットワーク送信層のため、リトライよりもログ強化が適切
if let Err(e) = self.transport_tx.try_send(SipTransportRequest { ... }) {
    error!("[sip_core] failed to send SIP message: {:?}", e);
}
```

### 処理フロー
1. イベント送信を試行（try_send）
2. Full エラーの場合、最大3回まで 10ms 間隔でリトライ
3. 3回失敗した場合、エラーログを出力
4. Closed エラーの場合、即座にエラーログを出力して終了

### トレース
- ← RD: RD-xxx-FR-xx
- → UT: UT-xxx-TC-02
```

---

### 5.3 テストケース追加（UT-xxx / IT-xxx へマージ）

```markdown
## UT-xxx-TC-01: B2buaFailed時のA-leg切断（IVR mode）

### 対象
DD-xxx-FN-01

### 目的
B-leg確立前に切断された場合（IVR mode）、A-legにBYEが送られることを検証する

### 入力
- outbound_mode = false（IVR mode）
- SessionControlIn::B2buaFailed { reason: "transfer failed status 486", status: None }

### 期待結果
- A-legにBYEが送信される
- セッションが終了する（EndReason::Error）

### トレース
← DD: DD-xxx-FN-01
```

```markdown
## UT-xxx-TC-01b: B2buaFailed時のエラー応答（outbound mode）

### 対象
DD-xxx-FN-01

### 目的
B-leg確立前に切断された場合（outbound mode）、A-legにエラー応答が返されることを検証する（現状維持）

### 入力
- outbound_mode = true
- SessionControlIn::B2buaFailed { reason: "connection refused", status: Some(503) }

### 期待結果
- A-legに 503 Service Unavailable が送信される
- セッションが継続される（invite_rejected = true）

### トレース
← DD: DD-xxx-FN-01
```

```markdown
## UT-xxx-TC-02: イベント送信のリトライ

### 対象
DD-xxx-FN-02

### 目的
チャネルが満杯の場合、イベント送信がリトライされることを検証する

### 入力
- チャネルが満杯（bounded channel の容量上限）
- BLegBye/B2buaFailed イベントを送信

### 期待結果
- 最大3回までリトライされる
- リトライ間隔は 10ms
- 3回失敗した場合、エラーログが出力される

### トレース
← DD: DD-xxx-FN-02
```

```markdown
## UT-xxx-TC-02b: イベント送信のチャネルクローズ検知

### 対象
DD-xxx-FN-02

### 目的
チャネルがクローズされた場合、即座にエラーログが出力されることを検証する

### 入力
- チャネルがクローズ（受信側が drop された状態）
- BLegBye/B2buaFailed イベントを送信

### 期待結果
- リトライせずに即座にエラーログが出力される

### トレース
← DD: DD-xxx-FN-02
```

```markdown
## IT-xxx-TC-01: 転送中のB-leg切断E2Eテスト

### 目的
転送中に着信側が切断した場合、発信側も切断されることをE2Eで検証する

### テストシナリオ
1. A-leg（発信側）が着信
2. IVR で転送を選択
3. B-leg（着信側）への転送を開始
4. B-leg が確立前に切断（INVITE に対して 486 Busy Here を返す）
5. A-leg にBYEが送られることを確認
6. セッションが終了することを確認

### 期待結果
- A-legにBYEが送信される
- A-legのセッションが終了する

### トレース
← RD: RD-xxx-FR-xx
```

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #187 | STEER-187 | 起票 |
| STEER-187 | RD-xxx-FR-xx | 要件追加 |
| RD-xxx-FR-xx | DD-xxx-FN-01 | 詳細設計（B2buaFailed処理） |
| RD-xxx-FR-xx | DD-xxx-FN-02 | 詳細設計（イベント送信） |
| DD-xxx-FN-01 | UT-xxx-TC-01 | 単体テスト（IVR mode） |
| DD-xxx-FN-01 | UT-xxx-TC-01b | 単体テスト（outbound mode） |
| DD-xxx-FN-02 | UT-xxx-TC-02 | 単体テスト（リトライ） |
| DD-xxx-FN-02 | UT-xxx-TC-02b | 単体テスト（チャネルクローズ） |
| RD-xxx-FR-xx | IT-xxx-TC-01 | 統合テスト |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] 要件の記述が明確か
- [ ] 詳細設計で実装者が迷わないか
- [ ] テストケースが網羅的か
- [ ] 既存仕様との整合性があるか
- [ ] トレーサビリティが維持されているか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] 実装が完了している（該当する場合）
- [ ] コードレビューを受けている（該当する場合）
- [ ] 関連テストがPASS（該当する場合）
- [ ] 本体仕様書への反映準備ができている

---

## 8. 備考

### 8.1 決定済み方針の要約

Codex による調査結果と §8.2 の決定を踏まえ、以下の方針で実装する：

1. **B2buaFailed時の処理（IVR mode のみ変更）**
   - 決定：IVR mode では B2buaFailed 時に A-leg を切断する
   - outbound mode：現状維持（エラー応答 503 を返す、A-leg は切断しない）
   - IVR mode の変更点：現状は IVR メニューに戻す実装だが、今後は A-leg に BYE を送信してセッション終了

2. **BYE連鎖系イベントを try_send + リトライで信頼性向上**
   - 決定：try_send + リトライ（最大3回、10ms間隔）
   - チャネルクローズ時は即座にエラーログを出力

3. **切断フェーズ（確立前/確立後）ごとの動作（IVR mode）**
   - 確立後（BLegBye）：A-leg を切断する（現状維持）
   - 確立前（B2buaFailed）：A-leg を切断する（**新規対応**、IVR mode のみ）

### 8.2 決定事項一覧

| # | 質問 | 選択肢 | 推奨 | 決定 |
|---|------|--------|------|------|
| 1 | B2buaFailed時、IVR mode でもA-legを切断するか？ | A) IVR mode のみ切断<br>B) 条件付き切断（B-leg明示的切断時のみ）<br>C) 現状維持（IVR に戻す）<br>D) すべてのモードで切断 | A | **A) IVR mode のみ切断**<br>（outbound mode は現状維持：503 応答） |
| 2 | イベント送信を信頼送達化するか？ | A) send に変更<br>B) try_send + ログ<br>C) try_send + リトライ<br>D) 現状維持 | B or C | **C) try_send + リトライ** |
| 3 | 切断理由の判定基準は？ | A) ステータスコード（4xx/5xx）<br>B) 理由文字列<br>C) すべて同じ扱い | C | **C) すべて同じ扱い** |

### 8.3 レビュー指摘対応履歴

#### 2026-02-16 レビュー #1（@MasanoriSuda）

**重大指摘**：
1. ✅ 実装方針が未確定のまま受入条件だけ確定 → §8.2 で Q1/Q2 を決定済みに変更、§5.2 から Option 分岐を削除
2. ✅ 「イベント取りこぼしなし」の受入条件と提案オプションが矛盾 → 受入条件を「検知・ログ出力」に緩和

**中指摘**：
3. ✅ 影響範囲が不十分（sip_handler.rs と core.rs の try_send が漏れている） → §4.2 と §5.2 DD-xxx-FN-02 に追加
4. ✅ UT入力例が実装実態とズレている（status: Some(503) vs None） → UT-xxx-TC-01 を IVR mode（status: None）と outbound mode（status: Some(503)）に分離

#### 2026-02-16 レビュー #2（@MasanoriSuda）

**重大指摘**：
1. ✅ EndReason::TransferFailed が存在しない、影響範囲も不足 → EndReason::Error に変更（TransferFailed は追加しない方針）

**中指摘**：
2. ✅ event.clone() 前提だが SessionControlIn は Clone ではない → clone を使わず、イベントごとにリトライコードを記述

**軽指摘**：
3. ✅ §8.1 の「[要確認]」表現が「未確定点は解消済み」と矛盾 → §8.1 を「決定済み方針の要約」に更新

#### 2026-02-16 レビュー #3（@MasanoriSuda）

**重大指摘**：
1. ✅ 仕様が内部で矛盾（outbound_mode の扱い） → §8.1 と §8.2 を修正：IVR mode のみ切断、outbound mode は現状維持（503 応答）

**中指摘**：
2. ✅ 実装例コードで send を await していない → line 222 に `.await` を追加

**軽指摘**：
3. ✅ §8.2 の見出しが「Open Questions」のまま → 「決定事項一覧」に変更

#### 2026-02-16 レビュー #4（@MasanoriSuda）

**重大指摘**：
1. ✅ 要件定義とテスト定義が矛盾（IVR mode のみ vs すべてのモード） → §5.1 の入力/出力/受入条件に「IVR mode のみ」を明記、outbound mode は現状維持を明示

**中指摘**：
2. ✅ 実装例コードの変数スコープが不明確（err が未定義） → line 295 に err のコンテキスト（外側の match Err(err) から束縛）を明記

#### 2026-02-16 レビュー #5（@MasanoriSuda）

**重大指摘**：
1. ✅ B2buaFailed 再送仕様が outbound mode 現状維持と矛盾（status: None 固定） → run_transfer と run_outbound を分けて記述、run_outbound は status 伝搬（err.downcast_ref）を維持

**中指摘**：
2. ✅ §2.3 の受入条件がモード非限定で §5.1/§8.1 と不整合 → §2.3 も「IVR mode で〜」に揃え、outbound は 503 応答維持を追記

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-16 | 初版作成（Draft） | Claude Code |
| 2026-02-16 | レビュー#1指摘対応（重大2件、中2件） | Claude Code |
| 2026-02-16 | レビュー#2指摘対応（重大1件、中1件、軽1件） | Claude Code |
| 2026-02-16 | レビュー#3指摘対応（重大1件、中1件、軽1件） | Claude Code |
| 2026-02-16 | レビュー#4指摘対応（重大1件、中1件） | Claude Code |
| 2026-02-16 | レビュー#5指摘対応（重大1件、中1件） | Claude Code |
| 2026-02-16 | レビュー#6 OK、承認（Approved） | @MasanoriSuda |
