# STEER-181: B2BUA 切断処理の診断性改善

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-181 |
| タイトル | B2BUA 切断処理の診断性改善 |
| ステータス | Approved |
| 関連Issue | #181 |
| 優先度 | P0 |
| 作成日 | 2026-02-15 |

---

## 2. ストーリー（Why）

### 2.1 背景

**問題**: 通常着信（転送アクション）した際に、発信側が切断しても着信側が切断しない

- **期待動作**: 発信側切断契機で着信側が切断する
- **実際の動作**: 発信側が切断しても着信側が切断しない

**Codex 調査結果**:

1. **コード上では A-leg BYE 契機の B-leg 切断呼び出しは存在する**
   - `SessionControlIn::SipBye` → `shutdown_b_leg(true)` → `b_leg.send_bye()`

2. **しかし、実際に B-leg に BYE が送信されているかは不明**
   - `shutdown_b_leg` が `b_leg == None` の場合に **無言**（ログなし）
   - `SessionOut::SipSendBye200` の送信失敗を **捨てている**
   - B2BUA 送信失敗の原因が **潰れている**

3. **ログから判明した問題**
   - unknown BYE に 200 を返さず再送を誘発している（session_registry.get が None のとき）
   - 15:10:38, 15:10:42 で同一 BYE 再送が発生

### 2.2 目的

B2BUA 切断処理の診断性を改善し、以下を可視化する：
- `shutdown_b_leg` 呼び出しの有無
- `b_leg` の存在/不存在
- B-leg BYE 送信の成功/失敗
- エラー発生時の詳細情報

### 2.3 ユーザーストーリー

```
As a 開発者/運用者
I want to B2BUA 切断処理のログが詳細化されている
So that 切断バグの原因を迅速に特定できる

受入条件:
- [ ] shutdown_b_leg 呼び出し時に b_leg の有無がログに出る
- [ ] B-leg BYE 送信の成功/失敗がログに出る
- [ ] SessionOut::SipSendBye200 の送信失敗がログに出る
- [ ] B2BUA 送信失敗時に詳細なエラー情報がログに出る
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-15 |
| 起票理由 | 転送アクション時の切断バグ調査 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-sonnet-4-5) |
| 作成日 | 2026-02-15 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Issue #181: 診断性改善のステアリング作成" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | Codex | 2026-02-15 | NG | 初回: b2bua_bridge パス誤り、peer参照、見出し番号重複等 |
| 2 | Codex | 2026-02-15 | OK | 前回指摘全て解消。診断性改善スコープとして適切 |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-15 |
| 承認コメント | Codexレビュー通過、承認 |

### 3.5 実装（該当する場合）

| 項目 | 値 |
|------|-----|
| 実装者 | Codex (GPT-5) |
| 実装日 | 2026-02-14 |
| 指示者 | @MasanoriSuda |
| 指示内容 | Refs #181: B2BUA切断診断ログの実装と再現ログ調査 |
| コードレビュー | CodeRabbit |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | - |
| マージ日 | - |
| マージ先 | - |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| - | - | ログ強化のみ、仕様変更なし |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| src/main.rs | 修正 | unknown BYE 受信時のログ追加 |
| src/protocol/session/services/b2bua_service.rs | 修正 | shutdown_b_leg のログ強化 |
| src/protocol/session/handlers/mod.rs | 修正 | SipBye 処理のログ/エラーハンドリング強化 |
| src/protocol/session/b2bua.rs | 修正 | send_bye のログ強化 |
| src/protocol/sip/b2bua_bridge.rs | 修正 | B2BUA 送信失敗時のログ強化 |

---

## 5. 差分仕様（What / How）

### 5.1 修正内容

#### 5.1.1 unknown BYE 受信時のログ追加

**ファイル**: `src/main.rs`

**現状の問題**:
- session_registry.get が None のとき何もしない（200 OK を返さない）
- BYE 再送が発生している

**修正内容**:
```rust
// main.rs の BYE ハンドリング
if let Some(session_tx) = session_registry.get(&call_id).await {
    // 既存のセッション処理
} else {
    warn!("[main] received BYE for unknown call_id: {}", call_id);
    // TODO: 200 OK 応答（別チケットで対応）
}
```

**備考**:
- 診断目的でログを追加
- 200 OK 応答の実装は別チケットで対応（本ステアリングはログ強化のみ）

#### 5.1.2 shutdown_b_leg のログ強化

**ファイル**: `src/protocol/session/services/b2bua_service.rs`

**現状の問題**:
- `b_leg == None` の場合に無言（ログなし）
- B-leg BYE 送信の成功/失敗が分からない

**修正内容**:
```rust
pub(crate) async fn shutdown_b_leg(&mut self, send_bye: bool) {
    if let Some(mut b_leg) = self.b_leg.take() {
        info!(
            "[session {}] shutting down B-leg (send_bye={})",
            self.call_id, send_bye
        );
        if send_bye {
            match b_leg.send_bye().await {
                Ok(_) => {
                    info!("[session {}] B-leg BYE enqueued successfully", self.call_id);
                }
                Err(e) => {
                    warn!("[session {}] B-leg BYE enqueue failed: {:?}", self.call_id, e);
                }
            }
        }
        b_leg.shutdown();
        self.rtp.stop(&b_leg.rtp_key);
    } else {
        warn!(
            "[session {}] shutdown_b_leg called but b_leg is None (send_bye={})",
            self.call_id, send_bye
        );
    }
}
```

**備考**:
- `b_leg.sip_peer` は private のため参照できない（getter 追加は別スコープ）
- 「BYE sent successfully」→「BYE enqueued successfully」に変更（UDP 送信成功ではなく、キュー投入成功のみ保証）

#### 5.1.3 SipBye 処理のエラーハンドリング強化

**ファイル**: `src/protocol/session/handlers/mod.rs`

**現状の問題**:
- `SessionOut::SipSendBye200` の送信失敗を捨てている

**修正内容**:
```rust
(_, SessionControlIn::SipBye) => {
    info!("[session {}] A-leg BYE received", self.call_id);
    self.stop_ring_delay();
    self.cancel_transfer();
    self.shutdown_b_leg(true).await;
    self.cancel_playback();
    self.stop_keepalive_timer();
    self.stop_session_timer();
    self.stop_ivr_timeout();
    self.mark_transfer_ended();
    self.rtp.stop(self.call_id.as_str());
    let _ = self
        .session_out_tx
        .try_send((self.call_id.clone(), SessionOut::RtpStopTx));

    // 修正: try_send の結果をチェック
    if let Err(e) = self
        .session_out_tx
        .try_send((self.call_id.clone(), SessionOut::SipSendBye200))
    {
        warn!("[session {}] failed to send BYE 200 OK: {:?}", self.call_id, e);
    } else {
        info!("[session {}] BYE 200 OK sent to A-leg", self.call_id);
    }

    self.stop_recorders();
    self.send_ingest("ended").await;
    self.send_call_ended(EndReason::Bye);
}
```

#### 5.1.4 B-leg BYE 送信のログ強化

**ファイル**: `src/protocol/session/b2bua.rs`

**現状の問題**:
- 送信成功時のログがない

**修正内容**:
```rust
pub async fn send_bye(&mut self) -> Result<()> {
    self.cseq = self.cseq.saturating_add(1).max(2);
    let via = build_via(self.via_host.as_str(), self.via_port);
    let req = SipRequestBuilder::new(SipMethod::Bye, self.remote_uri.clone())
        .header("Via", via)
        .header("Max-Forwards", "70")
        .header("From", self.from_header.clone())
        .header("To", self.to_header.clone())
        .header("Call-ID", self.call_id.clone())
        .header("CSeq", format!("{} BYE", self.cseq))
        .build();

    info!(
        "[b2bua {}] sending BYE to {} (CSeq: {})",
        self.call_id, self.sip_peer, self.cseq
    );

    send_b2bua_payload(TransportPeer::Udp(self.sip_peer), req.to_bytes())?;

    info!(
        "[b2bua {}] BYE enqueued successfully to {}",
        self.call_id, self.sip_peer
    );

    Ok(())
}
```

#### 5.1.5 B2BUA 送信失敗のログ強化

**ファイル**: `src/protocol/sip/b2bua_bridge.rs`

**現状の問題**:
- `try_send(...).is_ok()` だけで失敗時の詳細が分からない

**修正内容**:
```rust
// send 関数内の try_send 部分
// 現状:
tx.try_send(SipTransportRequest {
    peer,
    src_port: sip_port,
    payload,
})
.is_ok()

// 修正後:
match tx.try_send(SipTransportRequest {
    peer,
    src_port: sip_port,
    payload,
}) {
    Ok(_) => true,
    Err(e) => {
        warn!(
            "[b2bua bridge] failed to send SIP message: peer={:?} error={:?}",
            peer, e
        );
        false
    }
}
```

---

### 5.2 受入条件

#### 5.2.1 ログ出力の確認

- [ ] A-leg BYE 受信時に `A-leg BYE received` ログが出る
- [ ] `shutdown_b_leg` 呼び出し時に以下が出る:
  - b_leg が存在する場合: `shutting down B-leg (send_bye=true)`
  - b_leg が存在しない場合: `shutdown_b_leg called but b_leg is None`
- [ ] B-leg BYE 送信時に以下が出る:
  - 送信前: `sending BYE to ...`
  - 送信成功: `BYE enqueued successfully to ...`
  - 送信失敗: `B-leg BYE enqueue failed: ...`
- [ ] `SessionOut::SipSendBye200` 送信時に以下が出る:
  - 送信成功: `BYE 200 OK sent to A-leg`
  - 送信失敗: `failed to send BYE 200 OK: ...`

#### 5.2.2 エラーハンドリング

- [ ] `try_send` の結果を捨てずにログ出力する
- [ ] B2BUA 送信失敗時に詳細なエラー情報がログに出る

#### 5.2.3 ユニットテスト

- [ ] `SessionOut::SipSendBye200` の `try_send` 失敗時のログ出力を確認
- [ ] `shutdown_b_leg` で `b_leg = None` の場合のログ出力を確認
- [ ] `b2bua_bridge` の `try_send` 失敗時のログ出力を確認

#### 5.2.4 既存機能への影響

- [ ] ログ追加以外の動作変更がない
- [ ] 既存の切断処理が正常に動作する

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #181 | STEER-181 | 起票 |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] ログレベルが適切か（info/warn）
- [ ] ログ出力のパフォーマンス影響が許容範囲か
- [ ] 既存の動作を変更していないか
- [ ] Codex 調査結果と矛盾していないか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] 実装が完了している
- [ ] コードレビューを受けている
- [ ] 実際のログを確認して診断性が向上していることを確認

---

## 8. 備考

### 8.1 次のステップ（本修正後）

このログ強化により、以下の情報が可視化される：
1. `shutdown_b_leg` が呼ばれているか
2. `b_leg` が存在するか
3. B-leg BYE が送信されているか
4. 送信失敗の詳細

ログ強化後、再度バグを再現して以下を確認：
- A-leg BYE 受信時に `shutdown_b_leg` が呼ばれているか
- `b_leg` が None になっている場合、どのタイミングで None になったか
- B-leg BYE が送信されているが、相手側が受信していない場合、SIP トレースで確認

### 8.2 技術的注意点

- **ログレベル**: info と warn を適切に使い分ける
  - 正常な動作: info
  - 異常な動作（b_leg None、送信失敗）: warn

- **パフォーマンス**: ログ出力の頻度は高くない（通話終了時のみ）ため、影響は軽微

---

## 9. 未確定点・質問

なし

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-15 | 初版作成 | Claude Code (claude-sonnet-4-5) |
| 2026-02-15 | Codexレビュー指摘対応（パス修正、.await追加、見出し番号修正等） | Claude Code (claude-sonnet-4-5) |
| 2026-02-15 | Codexレビュー通過、承認 (Status: Draft → Approved) | @MasanoriSuda |
