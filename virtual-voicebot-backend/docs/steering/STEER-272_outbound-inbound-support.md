# STEER-272: 発着信同時サポート（通常発信 UAC + 多重着信）

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-272 |
| タイトル | 発着信同時サポート（通常発信 UAC + 多重着信） |
| ステータス | Approved |
| 関連Issue | #272 |
| 優先度 | P1 |
| 作成日 | 2026-03-01 |

---

## 2. ストーリー（Why）

### 2.1 背景

現在のバックエンドは着信専用（UAS のみ）として設計されており、発信を有効にするには
`OUTBOUND_ENABLED=true` 等の env 操作が必要である。
また `SipCore.active_call_id: Option<CallId>` により同時通話が 1本に制限されているため、
発信セッションと着信セッションを並走させることができない。

RD-001 には F-13「UAC 発信」が既に定義されているが、ステータスは Deferred のまま。

### 2.2 目的

- **`OUTBOUND_ENABLED` 不要（registrar 設定済み環境では発着信がデフォルト有効）**
- SIPフォン（Zoiper 等）から発信 INVITE を受けて相手先へ転送できるようにする
- 発信セッションと着信セッションを別セッションとして並走させる

### 2.3 ユーザーストーリー

```
As a SIPフォン利用者（Zoiper 等）
I want to バックエンドを介して発信しながら、着信も別セッションで受け付けてほしい
So that 発着信を柔軟に扱える SIPフォン的な運用ができる

受入条件:
- [ ] `OUTBOUND_ENABLED` 不要（registrar 設定済み環境では発着信がデフォルトで動作する）
- [ ] SIPフォンから送られた発信 INVITE を受けてバックエンドが相手先へ転送できる
- [ ] 発信中に着信 INVITE が届いた場合、別セッションとして処理される（拒否しない）
- [ ] 発信セッションでは AI パイプライン（ASR/LLM/TTS）が起動しない
- [ ] 発信の同時上限は 1本（着信は制限しない）
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-03-01 |
| 起票理由 | 発着信同時サポートの要望。既存 F-13（Deferred）の具体化。 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-sonnet-4-6) |
| 作成日 | 2026-03-01 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "発着信同時サポート。発信は通常発信のみ、env 操作不要、発信中の着信は別セッション並走、発信時 AI パイプライン不使用" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | @MasanoriSuda |  2026-03-01  | OK | lgtm |

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
| 実装日 | |
| 指示者 | |
| 指示内容 | |
| コードレビュー | |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | |
| マージ日 | |
| マージ先 | virtual-voicebot-backend/docs/requirements/RD-001_product.md (F-13), virtual-voicebot-backend/docs/design/detail/DD-003_sip.md |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| `virtual-voicebot-backend/docs/requirements/RD-001_product.md` | 修正 | F-13「UAC 発信」を Deferred → P1 に昇格、受入条件を具体化 |
| `virtual-voicebot-backend/docs/design/detail/DD-003_sip.md` | 追記 | UAS のみの記述に UAC（通常発信）の設計を追加 |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| `src/protocol/sip/core.rs` | 修正 | `active_call_id: Option<CallId>` を廃止し `outbound_call_id: Option<CallId>` へ置換、発着信の Busy 判定を分離 |
| `src/shared/config/mod.rs` | 修正 | `OUTBOUND_ENABLED` フラグを廃止。registrar 設定で発信判定を自動化 |
| `src/protocol/session/handlers/mod.rs` | 修正 | `outbound.enabled` チェックを廃止し、registrar + To user 判定に一本化 |
| `src/protocol/session/b2bua.rs` | 修正 | AI パイプラインなしで動作する通常発信用 UAC 関数の切り出し |

---

## 5. 差分仕様（What / How）

### 5.1 設計決定事項

#### 決定①【Q1】発信 INVITE と着信 INVITE の判定規則

**採用: 案A — `To user ≠ REGISTER_USER`（registrar 必須）**

判定は以下の順序で評価する（上位が優先）。

| ステップ | 条件 | 結果 |
|----------|------|------|
| ① | registrar が未設定 | 着信のみ（発信不可）→ 通常の UAS 処理へ |
| ② | To user == REGISTER_USER | 着信（voicebot が応答）→ 通常の UAS 処理へ |
| ③ | To user ≠ REGISTER_USER | 発信意図とみなす → ④へ |
| ④ | domain が空 または resolve_number(To user) が None | 503 Service Unavailable（設定不備） |
| ⑤ | それ以外 | 発信処理（B2BUA 転送）|

- `resolve_number` のロジック（dial_plan → phone number 判定 → default_number）は既存のまま流用。
- ③で発信意図とみなした後、④のエラーチェックに進む。④に引っかかった場合は 503 を返して終了（着信扱いにはしない）。

#### 決定②【Q2】`OUTBOUND_ENABLED` の廃止・統合

**採用: 方針Y — `OUTBOUND_ENABLED` を廃止し、registrar 設定で自動判定に統合**

| 変更前 | 変更後 |
|--------|--------|
| `OUTBOUND_ENABLED=true` が必要 | registrar 設定のみで自動的にアウトバウンド判定が有効 |
| `OUTBOUND_ENABLED` フラグで ON/OFF | フラグ廃止。registrar 未設定 = 発信無効、設定あり = 発信有効 |

- `OUTBOUND_DOMAIN` と `OUTBOUND_DEFAULT_NUMBER` 等の補助 env は引き続き有効。
- 移行上の破壊的変更: `OUTBOUND_ENABLED=true` を設定中の既存環境は、同フラグを削除して registrar 設定のみにすることで同等の動作になる。

---

### 5.2 要件追加（RD-001 へマージ）

```markdown
## F-13: UAC 発信（通常発信）

### 概要
SIPフォン（Zoiper 等）からの発信 INVITE を受け、バックエンドが UAC として
相手先へ INVITE を転送する通常発信機能。
registrar 設定のみで自動有効（`OUTBOUND_ENABLED` フラグ不要）。

### 前提・制約
- 発信の同時上限: 1本
- 着信は発信と独立した別セッションとして並走する（上限なし）
- 発信セッションでは ASR / LLM / TTS は起動しない
- `OUTBOUND_ENABLED` env var は廃止（決定②）
- registrar 未設定の環境では発信は無効（着信のみ動作）

### 入力
- SIPフォンからの発信 INVITE（To ユーザー = ダイヤル先電話番号）

### 出力
- 相手先への INVITE 転送
- 発信セッションの確立（発信側 ↔ バックエンド ↔ 着信側）

### 受入条件
- [ ] registrar 設定のみで発信機能が有効になる（`OUTBOUND_ENABLED` 不要）
- [ ] 発信 INVITE 受信時、AI パイプラインが起動しない
- [ ] 発信中に着信 INVITE が届いた場合、着信は別セッションとして処理される
- [ ] 発信が既に 1本アクティブな状態で 2本目の発信 INVITE が来た場合、486 Busy Here を返す
- [ ] 着信は発信セッション数に関わらず受け付ける

### 優先度
P1

### トレース
- → DD: virtual-voicebot-backend/docs/design/detail/DD-003_sip.md（UAC セクション追加）
```

---

### 5.3 詳細設計追加（DD-003_sip.md へマージ）

```markdown
## SipCore: セッション管理モデルの変更

### 現状
`active_call_id: Option<CallId>` が 2本目以降の INVITE を無条件に 486 で拒否する。
発着信の区別がなく、1本でも通話があると新規 INVITE を全拒否する。

### 変更後（統一モデル）

```rust
pub struct SipCore {
    // 発信セッション専用（上限 1本）。BYE/CANCEL/timeout で None に戻る。
    outbound_call_id: Option<CallId>,
    // active_call_id は廃止。着信セッションは invites HashMap で管理する。
    // ...既存フィールド（invites, non_invites, register, ...）
}
```

### Busy 判定ルール（変更後）

SipCore は「Busy かどうか」の判定のみを行う。最終的な応答（100/180/200 等）は後段のセッション処理が決定する。

| INVITE 種別 | 判定条件 | SipCore の動作 |
|-------------|----------|----------------|
| 着信 INVITE | 常に | Busy では拒否しない（後段セッション判定へ渡す） |
| 発信 INVITE | `outbound_call_id` が None | Busy では拒否しない（後段セッション判定へ渡す） |
| 発信 INVITE | `outbound_call_id` が Some | 486 Busy Here を返して終了 |

発信 / 着信の区別: 決定① に従い `To user ≠ REGISTER_USER` で判定。

### outbound_call_id の解放ルール

**前提: 以下はすべて `event.call_id == outbound_call_id` の場合のみ `None` に戻す。
無関係な着信セッションのイベントでは解放しない。**

| イベント | 処理 |
|----------|------|
| BYE 受信（発信側 A-leg / 着信側 B-leg いずれか） | `outbound_call_id = None`（当該発信 call_id に限る） |
| CANCEL 受信 | `outbound_call_id = None`（当該発信 call_id に限る） |
| タイムアウト（B-leg 無応答） | `outbound_call_id = None`（当該発信 call_id に限る） |
| B-leg エラー（4xx/5xx/6xx） | `outbound_call_id = None`（当該発信 call_id に限る） |

### 通常発信 UAC の切り出し

`b2bua.rs` の `spawn_outbound` をベースに、AI パイプラインを起動しない独立 UAC 関数として切り出す。

```rust
/// 通常発信（AI パイプラインなし）。SIPフォンからの発信 INVITE を相手先へ転送する。
pub fn spawn_plain_outbound(
    call_id: CallId,
    to_number: String,
    control_tx: mpsc::Sender<SessionControlIn>,
    media_tx: mpsc::Sender<SessionMediaIn>,
    runtime_cfg: Arc<SessionRuntimeConfig>,
) -> tokio::sync::oneshot::Sender<()>
```

- ASR / LLM / TTS は呼び出さない
- 関数名・シグネチャは実装者（Codex）が調整可。ただし AI パイプライン非起動の制約は変えないこと。
```

---

### 5.4 テストケース追加（UT へマージ）

```markdown
## TC-272-01: 発信 INVITE 受信 → 相手先へ転送

### 目的
SIPフォンからの発信 INVITE が相手先へ正しく転送されること

### 入力
- SIPフォンから INVITE（To: sip:09012345678@domain）

### 期待結果
- 相手先への INVITE が生成される
- AI パイプライン（ASR/LLM/TTS）が起動しない

---

## TC-272-02: 発信中の着信受け付け

### 目的
発信セッションがアクティブな状態で着信 INVITE が届いた場合、別セッションとして処理されること

### 入力
1. 発信セッション確立済み
2. 着信 INVITE 受信

### 期待結果
- 着信 INVITE に対して 486 を返さない
- 着信セッションが独立して起動する

---

## TC-272-03: 発信 2本目の拒否

### 目的
発信が 1本アクティブな状態で 2本目の発信 INVITE が来た場合、486 を返すこと

### 入力
1. 発信セッション確立済み
2. 2本目の発信 INVITE 受信

### 期待結果
- 486 Busy Here を返す

---

## TC-272-04: registrar 設定あり・To user ≠ REGISTER_USER → 発信動作

### 目的
`OUTBOUND_ENABLED` フラグなし・registrar 設定のみで、従来の B2BUA 転送が起動すること

### 入力
- registrar 設定あり・`OUTBOUND_ENABLED` は未設定
- 着信 INVITE（To user ≠ REGISTER_USER）受信

### 期待結果
- B2BUA 転送が起動する（`OUTBOUND_ENABLED` フラグなしで動作）
- `outbound_call_id` に当該 call_id がセットされる

---

## TC-272-05: 発信失敗後の再発信（ロック解放確認）

### 目的
B-leg エラー（4xx/5xx）後に `outbound_call_id` が解放され、再発信できること

### 入力
1. 発信 INVITE → B-leg 送信 → 486 応答（B-leg 拒否）
2. 再発信 INVITE 受信

### 期待結果
- `outbound_call_id` が None に戻る
- 2本目の発信 INVITE が受け付けられる

---

## TC-272-06: CANCEL / タイムアウト時のロック解放

### 目的
発信中に CANCEL または B-leg タイムアウトが発生した場合、`outbound_call_id` が解放されること

### 入力
- 発信中に CANCEL 受信 または B-leg 無応答タイムアウト

### 期待結果
- `outbound_call_id` が None に戻る
- 以降の発信 INVITE が受け付けられる
```

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #272 | STEER-272 | 起票 |
| STEER-272 | RD-001 F-13 | 要件昇格（Deferred → P1） |
| RD-001 F-13 | DD-003_sip.md | 設計追加（UAC / 多重セッション） |
| DD-003_sip.md | TC-272-01〜06 | 単体テスト |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [x] 判定規則（To user ≠ REGISTER_USER）が確定している（決定①）
- [x] `OUTBOUND_ENABLED` 廃止・registrar 自動判定への統合が確定している（決定②）
- [ ] 詳細設計で実装者が迷わないか（特に outbound_call_id 解放ルール表）
- [ ] テストケース（TC-272-01〜06）が網羅的か
- [ ] `OUTBOUND_ENABLED` 廃止による既存環境への移行影響が整理されているか
- [ ] 既存仕様（DD-003_sip.md、RD-001）との整合性があるか
- [ ] トレーサビリティが維持されているか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] 実装が完了している
- [ ] コードレビューを受けている
- [ ] TC-272-01〜06 が PASS
- [ ] 既存の UAS 着信テストが PASS（デグレなし）
- [ ] 本体仕様書（RD-001, DD-003）への反映準備ができている

---

## 8. 備考

- 発信上限「1本」は暫定。将来的な N 本化は別 Issue で対応。
- `OUTBOUND_ENABLED` env var は本変更で廃止（破壊的変更）。既存環境では同フラグを削除し、registrar 設定のみにすれば同等の動作になる。README / .env.example の更新も合わせて行うこと。
- `OUTBOUND_DOMAIN`・`OUTBOUND_DEFAULT_NUMBER` 等の補助 env は引き続き有効。

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-03-01 | 初版作成 | Claude Code (claude-sonnet-4-6) |
| 2026-03-01 | レビュー指摘対応（判定テーブル追加・セッションモデル統一・コード参照修正・テストケース追加） | Claude Code (claude-sonnet-4-6) |
| 2026-03-01 | レビュー指摘対応（Q1=案A・Q2=方針Y 確定反映、解放条件に call_id 一致条件明記、Busy 判定表の過剰規定修正） | Claude Code (claude-sonnet-4-6) |
| 2026-03-01 | レビュー指摘対応（判定フローを段階化、「env設定なし」表現を「OUTBOUND_ENABLED不要」に統一） | Claude Code (claude-sonnet-4-6) |
