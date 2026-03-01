# STEER-273: 発着信判定バグ修正（From user チェック追加）

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-273 |
| タイトル | 発着信判定バグ修正（From user チェック追加） |
| ステータス | Approved |
| 関連Issue | #273 |
| 関連ステアリング | STEER-272（§5.1 決定① を本ステアリングで上書き） |
| 優先度 | P0 |
| 作成日 | 2026-03-01 |

---

## 2. ストーリー（Why）

### 2.1 背景

STEER-272 §5.1 決定① で定めた発着信判定規則（段階的フロー ①〜⑤）は、
**「To user ≠ REGISTER_USER」のみを発信の判定条件としていた**。

しかし、キャリア（電話会社）からの着信 INVITE では：
- From: `sip:+819012345678@carrier.example.com`（外部発信者）
- To: `sip:09012345678@domain`（着番号）

のように、To user の電話番号フォーマット（国際形式 vs 国内形式）が
`REGISTER_USER`（例: `09012345678`）と一致しない場合がある。

この場合、STEER-272 の規則ではステップ③「To user ≠ REGISTER_USER → 発信意図」
に分類されてしまい、**キャリアからの正規着信が誤って発信扱いされるバグが発生する**。

### 2.2 目的

発着信の判定規則に「From user が REGISTER_USER と一致する」条件を追加し、
SIP フォン（登録ユーザー）が発した INVITE のみを発信として扱うよう修正する。

### 2.3 ユーザーストーリー

```
As a システム管理者 / エンドユーザー
I want to キャリアからの着信が正しく「着信」として認識されてほしい
So that 着信時にラズパイ（AI パイプライン）が正常に応答できる

受入条件:
- [ ] キャリア等の外部から着信 INVITE が届いた場合、着信として処理される
- [ ] SIP フォン（REGISTER_USER）が発した INVITE のみ発信として処理される
- [ ] To user のフォーマット（E.164 vs 国内形式）の差異で誤分類が発生しない
- [ ] 発着信判定の変更により、既存の発信動作（STEER-272 で実装済み）に デグレが発生しない
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-03-01 |
| 起票理由 | STEER-272 実装後、通常着信が発信と誤認識されるバグを発見（Issue #273） |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-sonnet-4-6) |
| 作成日 | 2026-03-01 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "From user == REGISTER_USER を発信判定条件に追加。Codex との会話で確認済み。" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | @MasanoriSuda | 2026-03-01 | OK | lgtm |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-03-01 |
| 承認コメント | lgtm |

### 3.5 実装

| 項目 | 値 |
|------|-----|
| 実装者 | Codex |
| 実装日 | 2026-03-01 |
| 指示者 | @MasanoriSuda |
| 指示内容 | 「STEER-273 を承認しました、実装をお願いします、Refs #273」 |
| コードレビュー | 未実施（ローカルで fmt / clippy / test 実行済み） |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | |
| マージ日 | |
| マージ先 | STEER-272 §5.1 決定①（上書き）、`virtual-voicebot-backend/docs/design/detail/DD-003_sip.md` |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| `virtual-voicebot-backend/docs/steering/STEER-272_outbound-inbound-support.md` | 上書き | §5.1 決定① の判定フロー（①〜⑤）を本ステアリングの修正版（①〜⑥）で置換 |
| `virtual-voicebot-backend/docs/design/detail/DD-003_sip.md` | 修正 | 発着信判定フロー図・条件表を修正版で更新 |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| `src/protocol/session/handlers/mod.rs` | 修正 | 発着信判定ロジックに `from_user == REGISTER_USER` 条件を追加 |
| `src/protocol/sip/core.rs` | 修正 | `is_outbound_invite_intent`（L.627）に `from_user == REGISTER_USER` 条件を追加。同関数は L.900 の Busy 判定（`outbound_call_id` チェック）にも使用されており、修正しないと着信 INVITE が誤って Busy 判定される |

---

## 5. 差分仕様（What / How）

### 5.1 設計決定事項（STEER-272 §5.1 決定① の上書き）

#### 旧判定フロー（STEER-272 §5.1 決定①）

| ステップ | 条件 | 結果 |
|----------|------|------|
| ① | registrar が未設定 | 着信のみ → UAS 処理へ |
| ② | To user == REGISTER_USER | 着信 → UAS 処理へ |
| ③ | To user ≠ REGISTER_USER | 発信意図とみなす → ④へ |
| ④ | domain が空 または resolve_number(To user) が None | 503 Service Unavailable |
| ⑤ | それ以外 | 発信処理（B2BUA 転送） |

**問題点**: ③の条件「To user ≠ REGISTER_USER」だけでは、
キャリアからの着信（From user が外部番号）も発信意図とみなしてしまう。

#### 新判定フロー（本ステアリング）

| ステップ | 条件 | 結果 |
|----------|------|------|
| ① | registrar が未設定 | 着信のみ → UAS 処理へ |
| ② | To user == REGISTER_USER | 着信 → UAS 処理へ |
| ③ | **From user ≠ REGISTER_USER** | **着信（外部発信者からの着信）→ UAS 処理へ** |
| ④ | To user ≠ REGISTER_USER かつ From user == REGISTER_USER | 発信意図とみなす → ⑤へ |
| ⑤ | domain が空 または resolve_number(To user) が None | 503 Service Unavailable |
| ⑥ | それ以外 | 発信処理（B2BUA 転送） |

**追加ポイント（ステップ③）**:
From user が REGISTER_USER と一致しない場合は、外部から届いた着信 INVITE であると判断し、
To user の値にかかわらず UAS 処理へ渡す。

**発信と判定するための必要十分条件（ステップ④ 到達時点）**:
- To user ≠ REGISTER_USER（宛先が自分自身でない）
- From user == REGISTER_USER（INVITE を送ってきたのが登録済み SIP フォン）
- resolve_number(To user) が Some（ダイヤルプランで解決可能）

---

### 5.2 要件修正（RD-001 F-13 への追記）

```markdown
## F-13: UAC 発信（通常発信）—— 判定条件の修正（Issue #273 対応）

### 判定条件（修正後）
発信 INVITE として処理するには、以下の **すべて** を満たすこと：
- registrar が設定されている
- From user == REGISTER_USER（SIP フォン等の登録ユーザーからの発信）
- To user ≠ REGISTER_USER（宛先が自分自身でない）
- resolve_number(To user) が Some（ダイヤルプランで解決可能）

### 条件不足時の動作（フロー ①〜③ での早期脱出）
registrar 未設定 / To user == REGISTER_USER / From user ≠ REGISTER_USER
のいずれかに該当した場合は **着信として扱う**（正常な UAS 処理へ渡す）。
これらは「設定不備」ではなく「外部から届いた着信 INVITE」として正常処理する。

### 設定不備エラー（フロー ⑤）
発信意図（フロー ④）到達後、resolve_number(To user) が None の場合は
**503 Service Unavailable** を返す。
これは INVITE が登録 SIP フォンから届いているにもかかわらず、
ダイヤルプランが未設定で転送先を決定できない設定不備を示す。
```

---

### 5.3 詳細設計修正（DD-003_sip.md へマージ）

```markdown
## 発着信判定フロー（修正版）— STEER-273 適用

旧フロー（STEER-272 §5.1 決定①）の ③〜⑤ を以下で置き換える。

### 判定ロジック（handlers/mod.rs）

```rust
// ① registrar 未設定 → 着信
if config.registrar.is_none() {
    return handle_inbound(invite);
}

// ② To user == REGISTER_USER → 着信（voicebot 宛）
if to_user == register_user {
    return handle_inbound(invite);
}

// ③ From user ≠ REGISTER_USER → 着信（キャリア等外部からの着信）
if from_user != register_user {
    return handle_inbound(invite);
}

// ④ From user == REGISTER_USER かつ To user ≠ REGISTER_USER → 発信意図
// ⑤ resolve_number で解決できなければ 503
let Some(dest) = resolve_number(&to_user, &config.outbound) else {
    return reply_503(invite);
};

// ⑥ 発信処理
handle_outbound(invite, dest);
```

### 不変条件
- `from_user` は INVITE の `From` ヘッダーのユーザー部を抽出したもの。
- `register_user` は `REGISTER_USER` env var（`src/shared/config/mod.rs`）。
- From user / To user の比較は **文字列完全一致**（フォーマット正規化なし）。
  電話番号フォーマットの差異（E.164 vs 国内形式）による誤マッチは
  REGISTER_USER の値と SIP フォン設定を統一することで回避する。

---

## SipCore: `is_outbound_invite_intent` の修正（core.rs L.627）

`SipCore::is_outbound_invite_intent` は `handlers/mod.rs` の判定と同じ条件を
Busy 判定（`outbound_call_id` チェック）のために使用している（L.900）。
本修正で `handlers/mod.rs` 側に From user チェックを追加する場合、
`core.rs` 側も同様に修正しないと着信 INVITE が誤って Busy 判定され、
486 Busy Here で拒否されるデグレが発生する。

### 修正前
```rust
fn is_outbound_invite_intent(&self, to_header: &str) -> bool {
    let Some(registrar_user) = self.registrar_user.as_deref() else {
        return false;
    };
    let to_user = extract_user_from_to(to_header).unwrap_or_default();
    to_user != registrar_user
}
```

### 修正後（From ヘッダー引数を追加）
```rust
fn is_outbound_invite_intent(&self, to_header: &str, from_header: &str) -> bool {
    let Some(registrar_user) = self.registrar_user.as_deref() else {
        return false;
    };
    let to_user = extract_user_from_to(to_header).unwrap_or_default();
    let from_user = extract_user_from_from(from_header).unwrap_or_default();
    to_user != registrar_user && from_user == registrar_user
}
```

- 関数名・シグネチャは実装者（Codex）が調整可。ただし **To ≠ REGISTER_USER かつ From == REGISTER_USER** の両条件は変えないこと。
- L.900 の呼び出し側も `from` ヘッダーを渡すよう合わせて修正すること。
- 擬似コード中の `extract_user_from_from` は説明上の仮称。現コードは `extract_user_from_to` を From/To 両方に流用しているため、実装時は同関数を再利用するか、汎用 `extract_sip_user` に統一して命名の混乱を避けること。
```

---

### 5.4 テストケース追加（UT へマージ）

```markdown
## TC-273-01: キャリア着信 — To user が REGISTER_USER と E.164 形式で不一致でも着信と認識

### 目的
キャリアから届いた INVITE で To user が REGISTER_USER と異なる形式（E.164 国際形式）でも、
ステップ③の From user チェックにより着信として正しく処理されること

### 入力
- From: sip:+819012345678@carrier.example.com（From user = "+819012345678"）
- To: sip:+819012345678@carrier.example.com（To user = "+819012345678"）
- REGISTER_USER = "09012345678"

### 期待結果
- ステップ②: To user（"+819012345678"）≠ REGISTER_USER（"09012345678"）→ 通過
- ステップ③: From user（"+819012345678"）≠ REGISTER_USER（"09012345678"）→ **着信判定**
- UAS 処理（AI パイプライン）が起動する（発信処理は起動しない）

---

## TC-273-02: SIP フォン発信 — From user == REGISTER_USER かつ To user ≠ REGISTER_USER → 発信

### 目的
SIP フォン（REGISTER_USER）が発した INVITE が発信として処理されること

### 入力
- From: sip:09012345678@local（From user = "09012345678"）
- To: sip:09028894539@domain（To user = "09028894539"）
- REGISTER_USER = "09012345678"

### 期待結果
- ステップ②で To user ≠ REGISTER_USER を通過
- ステップ③で From user == REGISTER_USER を通過
- ステップ⑥で発信処理（B2BUA 転送）が起動する

---

## TC-273-03: To user == REGISTER_USER の着信（ステップ② で早期着信）

### 目的
To user が REGISTER_USER と一致する着信が、From user にかかわらず着信として処理されること

### 入力
- From: sip:+819099998888@carrier.example.com
- To: sip:09012345678@domain（To user = REGISTER_USER）

### 期待結果
- ステップ②で即座に着信判定
- UAS 処理（AI パイプライン）が起動する

---

## TC-273-04: registrar 未設定 → 常に着信

### 目的
registrar が未設定の場合、From/To にかかわらず着信として処理されること

### 入力
- registrar 未設定
- From: sip:09012345678@local（From user == REGISTER_USER の値と同一）
- To: sip:09028894539@domain（To user ≠ REGISTER_USER）

### 期待結果
- ステップ①で着信判定
- 発信処理は起動しない

---

## TC-273-05: 既存発信（STEER-272）デグレなし確認

### 目的
STEER-272 で実装済みの発信動作（TC-272-01〜06）が本修正後も正常に動作すること

### 入力
- TC-272-01〜06 の各シナリオ

### 期待結果
- TC-272-01〜06 全ケースが PASS（デグレなし）
```

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #273 | STEER-273 | 起票 |
| STEER-272 §5.1 決定① | STEER-273 §5.1 | バグ起因（判定条件の不足） |
| STEER-273 | RD-001 F-13 | 判定条件の修正追記 |
| STEER-273 | DD-003_sip.md | 判定フロー修正 |
| STEER-273 | TC-273-01〜05 | テストケース追加 |
| TC-273-01〜05 | DD-003_sip.md | ← 設計トレース |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] 修正後の判定フロー（①〜⑥）が旧フロー（①〜⑤）の問題を正しく解決しているか
- [ ] TC-273-01〜04 がバグシナリオを網羅しているか
- [ ] TC-273-05（デグレ確認）が STEER-272 TC-272-01〜06 との整合性を保持しているか
- [ ] STEER-272 §5.1 決定① の上書き範囲が明確か（ステップ③〜⑤ → ③〜⑥）
- [ ] 既存 UAS 処理（着信側）に影響がないか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] 実装が完了している
- [ ] コードレビューを受けている
- [ ] TC-273-01〜05 が PASS
- [ ] TC-272-01〜06（STEER-272 テスト）がデグレなく PASS
- [ ] STEER-272 §5.1 決定① に「STEER-273 により上書き」旨を追記する

---

## 8. 備考

### From user 比較の注意点

From user（`REGISTER_USER`）と INVITE の From ヘッダーのユーザー部を**文字列完全一致**で比較する（確定）。
電話番号のフォーマット（例: `09012345678` と `+819012345678`）が異なる場合は一致しないため、
`REGISTER_USER` の値と SIP フォン（Zoiper 等）の `From` ユーザー設定を統一すること。
フォーマット正規化（E.164 ↔ 国内形式の変換）は本修正スコープ外とする。

### 「認証済み内線」の扱い

発信判定の対象は `REGISTER_USER` のみとする（確定）。
複数内線番号のサポートは本修正スコープ外とし、必要であれば別 Issue で対応する。

### STEER-272 との関係

- 本ステアリングは STEER-272 §5.1 決定①（旧ステップ①〜⑤）を**完全に置き換える**。
- STEER-272 §5.2〜5.4（要件・設計・テストケース TC-272-01〜06）は本修正後も有効。
- マージ後、STEER-272 §5.1 決定① に「STEER-273 により更新済み」と注記すること。

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-03-01 | 初版作成 | Claude Code (claude-sonnet-4-6) |
| 2026-03-01 | レビュー指摘対応（core.rs の `is_outbound_invite_intent` を影響範囲に追加、§5.2 の着信扱い/503 の区別を明確化、TC-273-01 の入力値を E.164 形式に修正） | Claude Code (claude-sonnet-4-6) |
