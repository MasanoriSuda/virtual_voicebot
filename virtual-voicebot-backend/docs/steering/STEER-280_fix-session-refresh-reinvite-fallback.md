# STEER-280: refresher=uas 時の session refresh を Allow ヘッダに従い re-INVITE へフォールバック

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-280 |
| タイトル | refresher=uas 時の session refresh を Allow ヘッダに従い re-INVITE へフォールバック |
| ステータス | Approved |
| 関連Issue | #280 |
| 優先度 | P0 |
| 作成日 | 2026-03-10 |

---

## 2. ストーリー（Why）

### 2.1 背景

着信呼で `refresher=uas`（Voicebot が session refresh 送信責任者）として OK DSP を返す場合、一定時間経過後に Voicebot は session refresh を送信する。
現在の実装は送信メソッドを `UPDATE` に固定しているため、相手の `Allow` ヘッダに `UPDATE` が含まれない場合でも `UPDATE` を送信してしまい、RFC 4028 準拠の `re-INVITE` フォールバックが行われない。

**再現条件**:
1. 着信で `refresher=uas` を含む `Session-Expires` を返す（200 OK DSP）
2. 相手の `INVITE` の `Allow` ヘッダに `UPDATE` が含まれない

**期待動作**: Session-Expires の 80% 経過後に `re-INVITE` を送信
**実動作**: Session-Expires の 80% 経過後に `UPDATE` を送信（相手が対応していない場合にも）

### 2.2 目的

RFC 4028 §8 の規定「refresher は相手が UPDATE をサポートしていない場合は re-INVITE を使用しなければならない」を満たし、UPDATE 未サポート相手との相互接続性を確保する。

### 2.3 ユーザーストーリー

```
As a Voicebot オペレーター
I want to UPDATE 未サポートの SIP 端末・PBX との通話が session timer によって強制切断されないこと
So that 長時間通話が正常に継続できる
```

受入条件:
- [ ] `Allow: UPDATE` が無い相手への session refresh は `re-INVITE` で送信される
- [ ] `Allow: UPDATE` がある相手への session refresh は従来通り `UPDATE` で送信される
- [ ] `Allow` ヘッダ自体が無い相手への session refresh は `re-INVITE` で送信される（安全側）
- [ ] `re-INVITE` の `2xx` 受信時に ACK が返送される
- [ ] `re-INVITE` の `2xx` 受信時に Session-Expires タイマがリセットされる
- [ ] `re-INVITE` が 408 / 481 の場合は BYE で通話終了する（transaction timeout は本 issue スコープ外）
- [ ] その他の非 `2xx` 応答では即時 BYE せず、session expiration は延長しない（`SessionTimerFired` 発火後 `AppSessionTimeout` 経由で終話）

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-03-10 |
| 起票理由 | UPDATE 未サポート端末と接続時に session timer 起因の意図せぬ切断が発生するバグ |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Sonnet 4.6 |
| 作成日 | 2026-03-10 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Issue #280 のバグ修正ステアリングファイルの作成（Codex 調査結果を踏まえて）" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | Codex 5.4 | 2026-03-10 | ok | ok |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 |@MasanoriSuda |
| 承認日 |2026-03-10  |
| 承認コメント | lgtm |

### 3.5 実装

| 項目 | 値 |
|------|-----|
| 実装者 | Codex |
| 実装日 | 2026-03-10 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "本ステアリング §5 の差分仕様に従い最小差分で実装すること" |
| コードレビュー | 2026-03-10 ローカル品質ゲート通過（`cargo fmt --all -- --check` / `cargo clippy --all-targets --all-features -- -D warnings` / `cargo test --all --all-features`） |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | |
| マージ日 | |
| マージ先 | DD-005_session.md, ST-001_acceptance.md |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| docs/design/detail/DD-005_session.md | 修正 | §3.2 refresher 動作表・§7.1 実装状況を re-INVITE フォールバック対応に更新 |
| docs/test/system/ST-001_acceptance.md | 修正 | AC-3 Session Timer に UPDATE なし相手の re-INVITE ケースを追加 |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| `src/protocol/session/handlers/mod.rs` L662付近 | 修正 | `SessionRefreshDue` 処理から `update_session_expires` 呼び出しを削除。`SipSendSessionRefresh` を発行するだけにする |
| `src/protocol/session/handlers/timer_handler.rs` L32付近 | 修正 | `update_session_expires` の呼び出しタイミングを 2xx 受信時のみに変更（送信時点での呼び出し削除に伴う整合） |
| `src/main.rs` L601付近 | 修正 | `SessionOut::SipSendUpdate { expires }` を `SessionOut::SipSendSessionRefresh { expires, local_sdp }` に読み替え、`SipCommand::SendSessionRefresh { expires, local_sdp }` として SIP core へ転送する（`call_id` は既存どおり第1引数で渡す） |
| `src/protocol/sip/core.rs` L918付近 | 修正 | INVITE 受信時（`handle_invite`）に `Allow` ヘッダを解析し、`InviteContext` の `allow_update` フィールドに保持する |
| `src/protocol/sip/core.rs` L1695付近 | 修正/追加 | `SipSendSessionRefresh` 受信時に `allow_update` を参照し、UPDATE または `build_reinvite_request` を選択する |
| `src/protocol/sip/core.rs` L762付近 | 修正 | `handle_response` に outbound refresh の 2xx 受信処理（ACK 送信 + `SipEvent::SessionRefresh` emit）と 408/481 受信時の BYE + `SipEvent::SessionRefreshFailed` emit を追加 |
| `src/shared/ports/sip.rs` L38付近 | 追加 | `SipEvent::SessionRefreshFailed { call_id }` バリアントを追加 |
| `src/protocol/session/types.rs` L109付近 | 追加 | `SessionControlIn::SipSessionRefreshFailed { call_id }` バリアントを追加 |
| `src/main.rs` L487付近 | 修正 | `SipEvent::SessionRefreshFailed` を `SessionControlIn::SipSessionRefreshFailed` へルーティングする処理を追加 |

---

## 5. 差分仕様（What / How）

### 5.1 要件追加（DD-005_session.md §3.2 へマージ）

```markdown
### 3.2 refresher の動作（更新後）

| refresher | 責任 | Voicebot の動作 |
|-----------|------|----------------|
| `uas` | Voicebot | Session-Expires の 80% 経過時に session refresh を送信。相手の `Allow` に `UPDATE` が含まれる場合は `UPDATE`、それ以外（`Allow` 未受信・`UPDATE` 不在）は `re-INVITE` を送信する |
| `uac` | 相手側 | 相手からの re-INVITE/UPDATE を受信し 200 OK を返す（変更なし） |

#### 判定ロジック（refresher=uas 時）

1. **SIP core** は着信 INVITE 受信時（`handle_invite`）に相手の `Allow` ヘッダを解析し、`UPDATE` サポート有無を `InviteContext.allow_update: bool` として保持する（session 層は `allow_update` を持たない）
2. `Allow` ヘッダが存在しない、または `UPDATE` が含まれない場合は `allow_update = false` として扱う（安全側）
3. **session 層** は `SessionRefreshDue` 発火時に `SessionOut::SipSendSessionRefresh { expires, local_sdp: Option<Sdp> }` を発行するだけでよい（UPDATE か re-INVITE かは SIP core が決定する）
4. **SIP core** は `SipSendSessionRefresh` を受け取り、保持済みの `InviteContext.allow_update` に基づき送信方式を選択する:
   - `allow_update == true` → UPDATE を送信（従来動作）
   - `allow_update == false` → re-INVITE を送信（新規）
5. **タイマリセットは 2xx 受信時のみ行う**: `SessionRefreshDue` ハンドラ（`mod.rs` L669）の `update_session_expires` 呼び出しを削除する。SIP core が 2xx 受信後に既存の `SipEvent::SessionRefresh { call_id, timer }` を emit し、main.rs が既存の `SessionControlIn::SipSessionExpires { timer }` へ転送する（既存コードパスを再利用し、新規イベントは不要）
```

### 5.2 詳細設計追加（DD-005_session.md §7.1 更新 + core.rs）

#### (A) SIP core の `InviteContext` への `allow_update` フィールド追加

```markdown
`InviteContext`（`src/protocol/sip/core.rs` L918付近）に以下を追加する:

| フィールド | 型 | 初期値 | 説明 |
|-----------|-----|--------|------|
| `allow_update` | `bool` | `false` | ダイアログ相手が UPDATE をサポートするか。着信 INVITE の `Allow` ヘッダ解析結果を格納 |

設定タイミング:
- UAS 動作: `handle_invite` で着信 INVITE の `Allow` ヘッダを解析し、`InviteContext` 生成時に設定する
- UAC 動作（将来）: 2xx の `Allow` ヘッダを解析する（本 issue スコープ外）

session 層はこのフィールドを持たない。`allow_update` の参照と UPDATE/re-INVITE の選択は SIP core 内に閉じる。
```

#### (B) `SessionOut` バリアント変更

```markdown
`SessionOut::SipSendUpdate { expires }` を `SessionOut::SipSendSessionRefresh { expires, local_sdp: Option<Sdp> }` に変更する。

変更の意図:
- session 層は UPDATE か re-INVITE かを知る必要がなく、「session refresh を送信してほしい」という要求だけを出す
- SIP core が `InviteContext.allow_update` に基づき UPDATE か `build_reinvite_request` かを選択する
- re-INVITE の body SDP は `InviteContext.final_ok_payload`（SIPメッセージ全体の再送バッファ）では取得できないため、session 層が保持する `self.local_sdp`（`coordinator.rs` L94、型は `Option<Sdp>`）をそのまま `local_sdp` フィールドに格納して渡す
- `SessionRefreshDue` ハンドラは `self.local_sdp.clone()` を `local_sdp` として設定し、`SipSendSessionRefresh` を発行する

`local_sdp == None` の扱い:
- session 確立前（`local_sdp` が未設定）に `SessionRefreshDue` が発火するケースは通常ない（Confirmed 遷移後のみ timer が起動する）が、防御的に `None` の場合は SIP core がログ出力のみ行い送信をスキップする

`main.rs` L601付近の既存パターンに合わせて以下のように変更する:

```
// 変更前
SessionOut::SipSendUpdate { expires } => {
    sip_core.handle_sip_command(&call_id, SipCommand::SendUpdate { expires });
}
// 変更後
SessionOut::SipSendSessionRefresh { expires, local_sdp } => {
    sip_core.handle_sip_command(&call_id, SipCommand::SendSessionRefresh { expires, local_sdp });
}
```

`call_id` は `handle_sip_command` の第1引数として渡すため、`SipCommand::SendSessionRefresh` には含めない（既存パターンに準拠）。
```

#### (C) `SipCommand::SendSessionRefresh` 処理と re-INVITE 生成（sip/core.rs）

```markdown
SIP core の `SendSessionRefresh` ハンドラで以下を行う:

1. `call_id` から `InviteContext` を取得する
2. `InviteContext.allow_update` を参照する:
   - `true` → 既存の `build_update_request` を呼ぶ（従来動作）
   - `false` → `build_reinvite_request` を呼ぶ（新規）
3. **タイマリセットは行わない**（送信時点では `update_session_expires` を呼ばない）

`build_reinvite_request(ctx: &InviteContext, local_sdp: &Sdp, expires: Duration) -> SipMessage`

処理フロー:
1. `InviteContext` の From/To/Call-ID/Route を引き継ぎ INVITE リクエストを組み立てる
2. CSeq を `InviteContext.local_cseq + 1` にインクリメントする
3. `Session-Expires` ヘッダ（`refresher=uas`）を付与する
4. `local_sdp`（`SipSendSessionRefresh` で渡された session 層の `self.local_sdp`）を body に設定する
   ※ `final_ok_payload` は 200 OK 再送用のSIPメッセージ全体であり SDP body 取得には使わない
5. 送信後、`InviteContext` に「refresh 応答待ち」フラグを立てる（UPDATE / re-INVITE 共通）

エラーケース:
| エラー | 条件 | 対応 |
|--------|------|------|
| context 不存在 | `call_id` に対応する `InviteContext` が無い | ログ出力のみ |
| `local_sdp` 未設定 | `SipSendSessionRefresh` の `local_sdp` が `None` | ログ出力のみ・送信スキップ |
| CSeq オーバーフロー | CSeq が u32 上限 | ログ出力・BYE 送信 |
```

#### (D) outbound re-INVITE / UPDATE の応答処理（sip/core.rs handle_response）

```markdown
`handle_session_refresh_response(status: u16, ctx: &mut InviteContext)` として実装する
（UPDATE・re-INVITE 共通。既存の `handle_response` 内の「re-INVITE 応答待ちフラグ」確認後に呼ぶ）

処理フロー:
1. `InviteContext` の「refresh 応答待ちフラグ」を確認し、outbound refresh の応答として処理する
2. ステータスコードを確認する
3. `2xx` の場合（RFC 3261 §13.2.2.4 準拠）:
   a. re-INVITE の場合のみ ACK を生成・送信する（UPDATE は ACK 不要）
   b. `Session-Expires` ヘッダを解析しタイマ情報を取得する
   c. `InviteContext.allow_update` は**更新しない**（初期 INVITE 受信時に設定した値を使い続ける。refresh 応答の `Allow` 再評価は本 issue スコープ外）
   d. **既存の** `SipEvent::SessionRefresh { call_id, timer }` を emit する
      → main.rs の既存ルーティングにより `SessionControlIn::SipSessionExpires { timer }` として session 層へ転送される
      → session 層の `update_session_expires` はこの通知を受けて呼ぶ（送信時点では呼ばない）
4. `408` / `481` の場合（RFC 4028 §10 準拠。**transaction timeout は現行実装では観測不可のため対象外**）:
   - BYE を送信し通話を終了する
   - **新設** `SipEvent::SessionRefreshFailed { call_id }` を emit する
   - main.rs に新設ルーティングを追加し、**新設** `SessionControlIn::SipSessionRefreshFailed` として session 層へ転送する
   - session 層の `SipSessionRefreshFailed` ハンドラは BYE 送信後の後処理（リソース解放等）を行う
5. `422 Session Interval Too Small` の場合:
   - Min-SE を応答から更新して再送する（既存ロジックに準じる）
6. その他の非 `2xx` 応答の場合（RFC 4028 §10 準拠）:
   - 即時 BYE はしない
   - session expiration は延長しない（タイマリセットを行わない = `SipEvent` を emit しない）
   - 既存タイマの満了時に `SessionControlIn::SessionTimerFired` が発火し、session 層が `SessionOut::AppSessionTimeout` を emit する
   - `AppSessionTimeout` を受け取った app 層（service/call_control）が `HangupRequested` を返し、session 層が BYE を送信する（DD-005_session.md §3.5 準拠）
   - **実装変更不要**: 本 issue で追加する「非 2xx の場合は `SipEvent` を emit しない」ことで、既存の session timeout 終話フローがそのまま動作する

> **NOTE**: RFC 4028 §10 の「transaction timeout」は client transaction timer（Timer B/F）の発火に相当するが、
> 現行実装には outbound refresh 用の client transaction timeout 検知経路が存在しない。
> client transaction 実装の追加は本 issue のスコープ外とし、
> 「timeout」の実質的な救済は session 層の `SessionTimerFired` → `AppSessionTimeout` → app 層の `HangupRequested` → BYE（既存終話フロー）で代替する。
```

### 5.3 テストケース追加（ST-001_acceptance.md AC-3 へマージ）

```markdown
### AC-3: Session Timer（更新後）

| # | シナリオ | 期待結果 | SIPp |
|---|---------|---------|------|
| AC-3.1 | Session-Expires 受信（Allow: UPDATE あり）・UPDATE 2xx 受信 | UPDATE で refresh → 2xx 受信後にタイマがリセットされる | basic_uas_update.xml（既存） |
| AC-3.2 | Min-SE 下回り | 422 + Min-SE: 90 | basic_uas_update.xml（既存） |
| AC-3.3 | Allow: UPDATE なし（refresher=uas） | Session-Expires 80% 経過後に re-INVITE が送信される | basic_uas_reinvite_refresh.xml（要作成） |
| AC-3.4 | Allow ヘッダなし（refresher=uas） | Session-Expires 80% 経過後に re-INVITE が送信される | basic_uas_reinvite_refresh.xml（共用可） |
| AC-3.5 | re-INVITE 2xx 受信 | ACK 送信 + タイマリセット（SIP core が `SipEvent::SessionRefresh` を emit し session 層が `SipSessionExpires` 経由で `update_session_expires` を呼ぶ） | basic_uas_reinvite_refresh.xml |
| AC-3.6 | re-INVITE 408 / 481 受信 | BYE を送信し通話終了する | basic_uas_reinvite_refresh.xml（エラーシナリオ） |
| AC-3.7 | re-INVITE その他非 2xx（例: 500） | 即時 BYE しない・session expiration は延長しない・`SessionTimerFired` 発火後 `AppSessionTimeout` 経由で app 層が終話する | basic_uas_reinvite_refresh.xml（エラーシナリオ） |

> **スコープ外**: transaction timeout（Timer B/F 発火）の検知は client transaction 実装が必要なため本 issue のスコープ外。
> timeout の実質的な救済は `SessionTimerFired` → `AppSessionTimeout` → app 層の `HangupRequested` → BYE（既存終話フロー）による AC-3.7 の挙動で代替する。
>
> SIPp シナリオ `basic_uas_reinvite_refresh.xml` は Codex が Approved 後に作成する。
> エラーシナリオは同ファイル内のバリアントとして追加してよい。
```

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #280 | STEER-280 | 起票 |
| STEER-280 | DD-005_session.md §3.2, §7.1 | 設計修正 |
| STEER-280 | ST-001_acceptance.md AC-3.3〜3.7 | 受入条件追加 |
| DD-005_session.md §3.2 | src/protocol/session/handlers/mod.rs | 実装 |
| DD-005_session.md §3.2 | src/protocol/sip/core.rs | 実装 |
| ST-001_acceptance.md AC-3.3〜3.7 | basic_uas_reinvite_refresh.xml | E2E テスト（Codex が作成） |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] `Allow` なし相手を `allow_update = false`（安全側）として扱う方針に合意している（→ Q1 解決済み）
- [ ] `allow_update` が SIP core の `InviteContext` に置かれ、session 層には存在しないことを確認した
- [ ] outbound re-INVITE の SDP 元データが session 層の `local_sdp`（`coordinator.rs` L94）経由で `SipSendSessionRefresh.local_sdp` として渡されることを確認した（`final_ok_payload` は使わない）
- [ ] `local_sdp == None` の場合に SIP core が送信スキップ+ログ出力することを確認した
- [ ] re-INVITE の `2xx` に対する ACK 生成ロジックが RFC 3261 §13.2.2.4 を満たすか（UPDATE には ACK 不要であることを確認）
- [ ] タイマリセット（`update_session_expires`）が `SessionRefreshDue` ハンドラから削除され、2xx 受信後に SIP core が既存 `SipEvent::SessionRefresh` を emit することで session 層が `update_session_expires` を呼ぶことを確認した
- [ ] 408/481 の場合に新設 `SipEvent::SessionRefreshFailed` が emit され、main.rs で `SessionControlIn::SipSessionRefreshFailed` へルーティングされることを確認した
- [ ] その他非 2xx の場合はイベントを emit せず、既存の `SessionTimerFired` → `AppSessionTimeout` → `HangupRequested` → BYE のフローで終話することを確認した（→ Q2 解決済み）
- [ ] transaction timeout のスコープ外処理が AC-3.7 の `SessionTimerFired` で代替されることに合意しているか
- [ ] UAC 動作（Voicebot 発信）側への影響がないか確認した
- [ ] 既存 `UPDATE` パスを壊していないこと（AC-3.1 が引き続き通ること）
- [ ] トレーサビリティが維持されているか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] 実装が完了している
- [ ] AC-3.1〜3.7 がすべて PASS
- [ ] `basic_uas_reinvite_refresh.xml` シナリオファイルが Codex により作成されている
- [ ] CodeRabbit レビューを受けている
- [ ] DD-005_session.md / ST-001_acceptance.md への本体反映準備ができている

---

## 8. 備考

### 解決済みクエスチョン

| # | 質問 | 決定者 | 回答 |
|---|------|--------|------|
| Q1 | `Allow` ヘッダなし相手を `allow_update = false` として扱う（安全側）でよいか？ | @MasanoriSuda | **解決**: Yes。RFC 3311 は UPDATE サポートの判断材料を `Allow: UPDATE` または mid-dialog OPTIONS とする。`Allow` なしは「未確認」のため安全側（`re-INVITE`）に倒す |
| Q2 | re-INVITE が非 2xx で失敗した場合の動作は？ | @MasanoriSuda | **解決**: RFC 4028 §10 準拠。408/481 は即 BYE。その他非 2xx は即 BYE せず session expiration を延長しない。期限到来時は `SessionTimerFired` → `AppSessionTimeout` → app 層の `HangupRequested` → BYE（既存フロー）。transaction timeout（Timer B/F）は client transaction 未実装のため本 issue スコープ外（`SessionTimerFired` 経由の既存終話フローで代替） |
| Q3 | SIPp テストシナリオ `basic_uas_reinvite_refresh.xml` の作成担当は？ | @MasanoriSuda | **解決**: Codex 担当。docs ではなく backend テスト資産のため Approved 後に Codex が作成する |

### RFC 参照

- RFC 4028 §8: Behavior of a Refresher — "If the remote side does not support the UPDATE method... the refresher MUST use re-INVITE."
- RFC 4028 §10: Handling of Session Expiration — timeout/408/481 は BYE。その他の非 2xx は即 BYE しない。
- RFC 3311: UPDATE method — `Allow` ヘッダまたは mid-dialog OPTIONS を UPDATE サポートの判断材料とする。
- RFC 3261 §13.2.2.4: 2xx への ACK は UAC コア層（ダイアログ）の責務。

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-03-10 | 初版作成 | Claude Sonnet 4.6 |
| 2026-03-10 | Q1〜Q3 解決反映: §5.2(D) を RFC 4028 §10 準拠エラー処理に更新、AC-3.6〜3.7 追加、§8 クエスチョン解決 | Claude Sonnet 4.6 |
| 2026-03-10 | Codex 指摘反映: ①`allow_update` を SIP core の `InviteContext` に移設（session 境界整理）、②タイマリセットを 2xx 受信時のみに変更（`SessionRefreshDue` からの `update_session_expires` 削除）、③ transaction timeout をスコープ外とし AC-3.6 を 408/481 に限定 | Claude Sonnet 4.6 |
| 2026-03-10 | Codex 指摘反映（第2回）: ①SDP 元データを `final_ok_payload` → session 層の `local_sdp` に修正し `SipSendSessionRefresh` に追加、②`SessionIn` を実際の型（`SipEvent`/`SessionControlIn`）に置換・2xx 成功は既存 `SipEvent::SessionRefresh` 再利用・失敗は新設 `SipEvent::SessionRefreshFailed`、③AC-3.1 に UPDATE 2xx 後タイマリセット検証を追加 | Claude Sonnet 4.6 |
| 2026-03-10 | Codex 指摘反映（第3回）: ①受入条件 L50 を `408/481`（transaction timeout 除外）に揃え、②`local_sdp` 型を `Option<Sdp>` に統一し `None` 時の挙動を明記、③§7.1 チェックリストを `local_sdp`・`SipEvent::SessionRefresh` 再利用・`SipEvent::SessionRefreshFailed` ルーティングに更新 | Claude Sonnet 4.6 |
| 2026-03-10 | Codex 指摘反映（第4回）: ①§5.1 判定ロジックの `SipSendSessionRefresh { call_id }` を `{ expires, local_sdp }` に揃え、②`SipCommand::SendSessionRefresh` から `call_id` を除外（既存パターン準拠・第1引数渡し）、③AC-3.5 の `SessionRefreshed` を `SipEvent::SessionRefresh` / `SipSessionExpires` 経由に更新 | Claude Sonnet 4.6 |
| 2026-03-10 | Codex 指摘反映（第5回）: §5.2(D) の refresh 2xx 処理で `allow_update` を再評価しないことを明記（初期 INVITE 受信時の値を維持。§5.2(A) スコープ外宣言と整合） | Claude Sonnet 4.6 |
| 2026-03-10 | Codex 指摘反映（第6回）: `SessionTimerFired` が直接 BYE しないことを実装確認し、「AppSessionTimeout → HangupRequested → BYE」の実際のフローに修正（受入条件 L51・AC-3.7・Q2・§5.2(D) step 6・§7.1 チェックリスト）。実装変更不要であることも明記 | Claude Sonnet 4.6 |
| 2026-03-10 | Codex 指摘反映（第7回）: §5.2(D) NOTE および AC-3.7 スコープ外 NOTE の `SessionTimerFired`「期限切れ BYE」文言を `SessionTimerFired → AppSessionTimeout → HangupRequested → BYE（既存終話フロー）` に修正。Codex レビュー OK 確認 | Claude Sonnet 4.6 |
