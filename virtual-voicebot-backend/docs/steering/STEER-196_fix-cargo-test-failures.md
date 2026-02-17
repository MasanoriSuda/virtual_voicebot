# STEER-196: Fix cargo test --all --all-features failures (doctest imports)

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-196 |
| タイトル | Fix cargo test --all --all-features failures (doctest imports) |
| ステータス | Approved |
| 関連Issue | #196 |
| 優先度 | P0 |
| 作成日 | 2026-02-17 |

---

## 2. ストーリー（Why）

### 2.1 背景

`cargo test --all --all-features` が失敗してバックエンドのCIが通らない状態。

調査の結果、以下が判明：
- ユニットテスト は全て PASS
- **doctests（ドキュメントコメント内のテスト）が 51件 FAIL**

#### 失敗の類型

**類型A: インポート不足（E0425, E0422）**
```
error[E0425]: cannot find function `parse_rtcp_packets` in this scope
error[E0422]: cannot find struct, variant or union type `VadConfig` in this scope
```
→ 必要な型・関数の `use` 文が欠けている

**類型B: 不正な crate パス（E0433, E0432, E0423）**
```
error[E0433]: failed to resolve: unresolved import
error[E0432]: unresolved import `crate::types`
error[E0423]: expected value, found crate `core`
```
→ doctest 内で `crate::` パスが不正、または外部クレート参照が誤っている

**類型C: 未定義変数（E0425 value not found）**
```
error[E0425]: cannot find value `worker` in this scope
error[E0425]: cannot find value `req` in this scope
```
→ doctest 内で使用する前提オブジェクトが定義されていない

**類型D: private item 参照**
→ doctest が private 関数・型を参照している場合の扱い

放置すると：
- CI が常に失敗し、PR マージができない
- 品質ゲートを通過できず、開発フローが停止する
- ドキュメントとコードの乖離が拡大する

### 2.2 目的

`cargo test --all --all-features` が全て PASS するようにして、CI を正常に通過させる。

具体的には：
- 51件の失敗している doctests に適切なインポート文を追加
- doctests がコンパイル・実行できることを確認

### 2.3 ユーザーストーリー（該当する場合）

```
As a 開発者
I want to CI が正常に通過するようにしたい
So that PR のマージがブロックされず、開発フローが円滑に回る

受入条件:
- [ ] `cargo test --all --all-features` が exit code 0 で終了
- [ ] テスト結果が `test result: ok. <N> passed; 0 failed; <M> ignored` となる（failed=0 が条件）
- [ ] 既存のユニットテストが引き続き PASS
- [ ] doctests が全て PASS（failed=0）
- [ ] GitHub Actions での CI ワークフローが成功
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-17 |
| 起票理由 | CI が通らないため |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (Sonnet 4.5) |
| 作成日 | 2026-02-17 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Run cargo test --all --all-featuresが失敗してバックエンドのCIが通らないので対処してほしいのでステアリングファイル作ってください、詳細はcodexにやってもらいます" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| - | - | - | - | - |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-17 |
| 承認コメント | Codex レビュー（第1回・第2回）を経て承認。21ファイル / 51件の doctest 修正方針（類型A/B/C/D 別）を確認。実装はCodexへ引き継ぎ。 |

### 3.5 実装（該当する場合）

| 項目 | 値 |
|------|-----|
| 実装者 | Codex (担当予定) |
| 実装日 | - |
| 指示者 | @MasanoriSuda |
| 指示内容 | "[Codexへ引き継ぎ]" |
| コードレビュー | - |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | - |
| マージ日 | - |
| マージ先 | （本体仕様書への反映は不要） |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| （なし） | - | doctest の修正はコード内コメントのため、本体ドキュメントへの影響なし |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| src/interface/http/mod.rs | 修正 | doctest修正（2件：handle_conn, write_response） |
| src/protocol/rtp/codec.rs | 修正 | **doctest修正（2件：alaw_to_linear16, linear16_to_alaw）** ← Codex指摘で追加 |
| src/protocol/rtp/rtcp.rs | 修正 | doctest修正（1件） |
| src/protocol/rtp/rx.rs | 修正 | doctest修正（1件） |
| src/protocol/session/b2bua.rs | 修正 | doctest修正（6件） |
| src/protocol/session/capture.rs | 修正 | doctest修正（1件） |
| src/protocol/session/coordinator.rs | 修正 | doctest修正（1件） |
| src/protocol/session/types.rs | 修正 | doctest修正（1件） |
| src/protocol/sip/auth.rs | 修正 | doctest修正（3件） |
| src/protocol/sip/core.rs | 修正 | doctest修正（7件） |
| src/protocol/sip/register.rs | 修正 | doctest修正（1件） |
| src/protocol/transport/packet.rs | 修正 | doctest修正（1件） |
| src/protocol/transport/tls.rs | 修正 | doctest修正（1件） |
| src/service/ai/llm.rs | 修正 | doctest修正（1件） |
| src/service/ai/mod.rs | 修正 | doctest修正（3件） |
| src/service/ai/ser.rs | 修正 | doctest修正（1件） |
| src/service/ai/weather.rs | 修正 | doctest修正（1件） |
| src/service/call_control/mod.rs | 修正 | doctest修正（7件） |
| src/service/call_control/router.rs | 修正 | doctest修正（2件） |
| src/shared/config/mod.rs | 修正 | doctest修正（6件） |
| src/shared/logging.rs | 修正 | doctest修正（1件） |

**計 21ファイル / 51件の doctest**

---

## 5. 差分仕様（What / How）

### 5.1 要件追加（RD-xxx へマージ）

**該当なし**（テストコード修正のため、要件仕様への追加は不要）

---

### 5.2 詳細設計追加（DD-xxx へマージ）

**該当なし**（テストコード修正のため、詳細設計への追加は不要）

---

### 5.3 テストケース追加（UT-xxx / ST-xxx へマージ）

**該当なし**（doctestの修正のみで、新規テストケースの追加は不要）

---

### 5.4 実装方針（Codex への引き継ぎ事項）

#### 5.4.1 対処方針

各 doctest コメント（`///` で始まるドキュメントコメント内の ` ```rust` ブロック）を、失敗類型に応じて修正する。

#### 5.4.2 類型別修正パターン

**類型A: インポート不足**

コンパイラの提案（`help: consider importing`）に従って `use` 文を追加：

```rust
// 修正前
/// ```
/// let empty = parse_rtcp_packets(&[]);
/// assert_eq!(empty.len(), 0);
/// ```

// 修正後
/// ```
/// use virtual_voicebot_backend::protocol::rtp::rtcp::parse_rtcp_packets;
/// let empty = parse_rtcp_packets(&[]);
/// assert_eq!(empty.len(), 0);
/// ```
```

**類型B: 不正な crate パス**

`crate::` パスを正しいモジュールパスに修正、または絶対パスに変更：

```rust
// 修正前（不正な crate:: パス）
/// ```
/// use crate::types::Foo;  // doctest では不正
/// ```

// 修正後（絶対パス）
/// ```
/// use virtual_voicebot_backend::protocol::types::Foo;
/// ```
```

**類型C: 未定義変数**

doctest 内で前提オブジェクトを定義、または `# ` プレフィックスで隠し行を追加：

```rust
// 修正前
/// ```
/// worker.notify_ringing(...);  // worker が未定義
/// ```

// 修正後（前提オブジェクトを定義）
/// ```
/// # use virtual_voicebot_backend::service::call_control::AppWorker;
/// # let worker = AppWorker::new(...);  // 隠し行（ドキュメントには表示されない）
/// worker.notify_ringing(...);
/// ```
```

**類型D: private item 参照**

private な関数・型を doctest で参照している場合の対処順序：

1. **優先**: 公開APIを使う例に書き換え（プロダクションコード変更なし）
   ```rust
   // private関数の例をやめて、公開APIの例に書き換える
   /// ```
   /// use virtual_voicebot_backend::public_module::PublicApi;
   /// let result = PublicApi::new().public_method();
   /// ```
   ```

2. **代替1**: doctest を `no_run` に変更（コンパイルのみ検証）
   ```rust
   /// ```no_run
   /// // private item を使う例だが、コンパイルのみ確認
   /// ```
   ```

3. **代替2**: doctest を `ignore` に変更（最小限に）
   ```rust
   /// ```ignore
   /// // private item を使う例（実行しない）
   /// ```
   ```

4. **最終手段**: `pub` に変更（プロダクションコードへの影響を慎重に検討）
   - 公開APIを広げることの副作用を評価
   - 必要最小限に留める

5. **例外**: doctest を削除（ドキュメント価値が低い場合のみ）

#### 5.4.3 注意事項

- **既存のユニットテストは全て PASS しているため、変更しない**
- doctest コメント内のみを修正（プロダクションコードの変更は最小限）
- コンパイラの提案（`help: consider importing`）に従う
- **51件全てを修正する** ← @MasanoriSuda 決定
  - `ignore` / `no_run` 属性は private item 参照の場合のみ許容（最小限）
- `cargo test --doc --all-features` が全て PASS することを確認
- **GitHub Actions での CI PASS が必須条件** ← @MasanoriSuda 決定
- 類型別の修正優先度：
  1. 類型A（インポート不足） → 確実に修正
  2. 類型B（不正パス） → 確実に修正
  3. 類型C（未定義変数） → doctest を充実させる機会として前提定義を追加
  4. 類型D（private item） → **公開API使用例へ書き換え優先**、難しければ `no_run` / `ignore`、`pub` 化は最終手段

#### 5.4.4 確認コマンド

**doctest のみ確認**
```bash
cd virtual-voicebot-backend
cargo test --doc --all-features
```

**全テスト確認（CI と同等）**
```bash
cd virtual-voicebot-backend
cargo test --all --all-features
```

期待結果：
```
test result: ok. <N> passed; 0 failed; <M> ignored; 0 measured; 0 filtered out
（exit code 0）
```
※ 件数は固定せず、`failed=0` と `exit code 0` が条件

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #196 | STEER-196 | 起票 |
| STEER-196 | （コード修正のみ） | doctest修正 |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] 背景・目的が明確か
- [ ] 影響範囲（21ファイル / 51件のdoctest項目）が網羅されているか
- [ ] 実装方針が具体的か
- [ ] 既存機能への影響がないか（ユニットテストは全てPASS維持）

### 7.2 マージ前チェック（Approved → Merged）

- [ ] `cargo test --all --all-features` が exit code 0 で終了
- [ ] テスト結果が `test result: ok. <N> passed; 0 failed; <M> ignored`（**failed=0 が条件**）
- [ ] 既存のユニットテストが引き続き PASS
- [ ] doctests が全て PASS（**原則 `ignore` 属性不使用**、private item のみ許容）
- [ ] **GitHub Actions での CI ワークフローが成功**（必須条件）

---

## 8. 備考

### 8.1 失敗している doctest の一覧（51件）

以下のファイルの doctest が失敗している（`cargo test --doc --all-features 2>&1` の出力より）:

1. src/interface/http/mod.rs - interface::http::handle_conn (line 73)
2. src/interface/http/mod.rs - interface::http::write_response (line 469)
3. src/protocol/rtp/codec.rs - protocol::rtp::codec::alaw_to_linear16 (line 82)
4. src/protocol/rtp/codec.rs - protocol::rtp::codec::linear16_to_alaw (line 114)
5. src/protocol/rtp/rtcp.rs - protocol::rtp::rtcp::parse_rtcp_packets (line 78)
6. src/protocol/rtp/rx.rs - protocol::rtp::rx::RtpReceiver::handle_raw (line 74)
7. src/protocol/session/b2bua.rs - protocol::session::b2bua::build_via (line 1335)
8. src/protocol/session/b2bua.rs - protocol::session::b2bua::extract_contact_uri (line 1430)
9. src/protocol/session/b2bua.rs - protocol::session::b2bua::format_response_dump (line 1501)
10. src/protocol/session/b2bua.rs - protocol::session::b2bua::generate_branch (line 1351)
11. src/protocol/session/b2bua.rs - protocol::session::b2bua::resolve_rtp_addr (line 1198)
12. src/protocol/session/b2bua.rs - protocol::session::b2bua::resolve_target_addr (line 1168)
13. src/protocol/session/capture.rs - protocol::session::capture::AudioCapture::ingest (line 72)
14. src/protocol/session/coordinator.rs - protocol::session::coordinator::SessionCoordinator::run (line 270)
15. src/protocol/session/types.rs - protocol::session::types::next_session_state (line 259)
16. src/protocol/sip/auth.rs - protocol::sip::auth::build_authorization_header (line 68)
17. src/protocol/sip/auth.rs - protocol::sip::auth::build_authorization_header_with_cnonce (line 107)
18. src/protocol/sip/auth.rs - protocol::sip::auth::md5_bytes (line 229)
19. src/protocol/sip/core.rs - protocol::sip::core::SipCore::handle_cancel (line 1060)
20. src/protocol/sip/core.rs - protocol::sip::core::SipCore::handle_reinvite (line 955)
21. src/protocol/sip/core.rs - protocol::sip::core::SipCore::handle_response (line 738)
22. src/protocol/sip/core.rs - protocol::sip::core::SipCore::handle_sip_command (line 1516)
23. src/protocol/sip/core.rs - protocol::sip::core::SipCore::handle_update (line 1276)
24. src/protocol/sip/core.rs - protocol::sip::core::contact_scheme_from_uri (line 470)
25. src/protocol/sip/core.rs - protocol::sip::core::extract_contact_uri (line 443)
26. src/protocol/sip/register.rs - protocol::sip::register::RegisterClient::build_request_with_expires (line 79)
27. src/protocol/transport/packet.rs - protocol::transport::packet::run_sip_tcp_accept_loop (line 249)
28. src/protocol/transport/tls.rs - protocol::transport::tls::load_cert_chain (line 37)
29. src/service/ai/llm.rs - service::ai::llm::system_prompt (line 26)
30. src/service/ai/mod.rs - service::ai::DefaultAiPort::transcribe_chunks (line 278)
31. src/service/ai/mod.rs - service::ai::call_gemini (line 401)
32. src/service/ai/mod.rs - service::ai::transcribe_with_aws (line 492)
33. src/service/ai/ser.rs - service::ai::ser::analyze (line 35)
34. src/service/ai/weather.rs - service::ai::weather::fetch_weather_report (line 110)
35. src/service/call_control/mod.rs - service::call_control::AppWorker::handle_audio_buffer (line 335)
36. src/service/call_control/mod.rs - service::call_control::AppWorker::handle_phone_lookup (line 707)
37. src/service/call_control/mod.rs - service::call_control::AppWorker::new (line 165)
38. src/service/call_control/mod.rs - service::call_control::AppWorker::notify_ended (line 624)
39. src/service/call_control/mod.rs - service::call_control::AppWorker::notify_ringing (line 588)
40. src/service/call_control/mod.rs - service::call_control::is_spec_question (line 762)
41. src/service/call_control/mod.rs - service::call_control::spawn_app_worker (line 79)
42. src/service/call_control/router.rs - service::call_control::router::Router::resolve_transfer_person (line 308)
43. src/service/call_control/router.rs - service::call_control::router::normalize_person (line 346)
44. src/shared/config/mod.rs - shared::config::AiConfig::from_env (line 878)
45. src/shared/config/mod.rs - shared::config::Config::from_env (line 80)
46. src/shared/config/mod.rs - shared::config::LineNotifyConfig::from_env (line 542)
47. src/shared/config/mod.rs - shared::config::RegistrarConfig::from_env (line 321)
48. src/shared/config/mod.rs - shared::config::database_url (line 484)
49. src/shared/config/mod.rs - shared::config::line_notify_config (line 581)
50. src/shared/config/mod.rs - shared::config::registrar_config (line 387)
51. src/shared/logging.rs - shared::logging::init (line 20)

**ユニークファイル数**: 21ファイル
- interface/http/mod.rs (2件)
- protocol/rtp/codec.rs (2件) ← **Codex 指摘で追加**
- protocol/rtp/rtcp.rs (1件)
- protocol/rtp/rx.rs (1件)
- protocol/session/b2bua.rs (6件)
- protocol/session/capture.rs (1件)
- protocol/session/coordinator.rs (1件)
- protocol/session/types.rs (1件)
- protocol/sip/auth.rs (3件)
- protocol/sip/core.rs (7件)
- protocol/sip/register.rs (1件)
- protocol/transport/packet.rs (1件)
- protocol/transport/tls.rs (1件)
- service/ai/llm.rs (1件)
- service/ai/mod.rs (3件)
- service/ai/ser.rs (1件)
- service/ai/weather.rs (1件)
- service/call_control/mod.rs (7件)
- service/call_control/router.rs (2件)
- shared/config/mod.rs (6件)
- shared/logging.rs (1件)

### 8.2 参考：実際のエラー例

**類型A: インポート不足（E0425, E0422）**
```
error[E0425]: cannot find function `parse_rtcp_packets` in this scope
 --> src/protocol/rtp/rtcp.rs:78:13
  |
3 | let empty = parse_rtcp_packets(&[]);
  |             ^^^^^^^^^^^^^^^^^^ not found in this scope
  |
help: consider importing this function
  |
2 + use virtual_voicebot_backend::protocol::rtp::rtcp::parse_rtcp_packets;
  |
```

**類型B: 不正な crate パス（E0433, E0432）**
```
error[E0432]: unresolved import `crate::types`
 --> src/protocol/session/types.rs:259:5
  |
1 | use crate::types::SessionState;
  |     ^^^^^^^^^^^^ could not find `types` in the crate root
```

**類型C: 未定義変数（E0425 value not found）**
```
error[E0425]: cannot find value `worker` in this scope
 --> src/service/call_control/mod.rs:592:1
  |
6 | worker.notify_ringing(...);
  | ^^^^^^ not found in this scope
```

コンパイラが適切な修正提案をしている場合（`help: consider importing`）はそれに従う。
提案がない場合（未定義変数等）は、doctest内で前提オブジェクトを定義する。

### 8.3 質問への回答（2026-02-17 @MasanoriSuda）

**Q1. doctestの修正方針は適切か？**
- **A1**: 暫定それでお願いします
- **決定**: コンパイラの提案（`help: consider importing`）に従う方針で進める

**Q2. 51件全てを修正するか、一部を `ignore` にするか？**
- **A2**: 原則品質守りたいので全修正でお願いします
- **決定**: 51件全てを修正する（`ignore` 属性は private item の場合のみ許容、最小限）

**Q3. 他のCI環境（GitHub Actions等）での確認が必要か？**
- **A3**: github action pass が前提です、しなかったらこちらから指摘あるいは新規イシュー立てます
- **決定**: GitHub Actions での PASS が必須条件。ローカルで PASS 後、PR で CI 確認

---

### 8.4 Codex レビュー対応（2026-02-17）

#### 第1回レビュー対応

**重大な指摘**
1. ✅ 原因分析が「import不足のみ」に寄りすぎている
   → §2.1 と §5.4 を失敗類型別（A/B/C/D）に全面改訂

**中程度の指摘**
2. ✅ 失敗一覧が48件で3件不足（codec.rs 等が欠けていた）
   → §8.1 を正確な51件リストに更新、§4.2 に codec.rs 追加
3. ✅ 受入条件がテスト件数固定で誤検知しやすい
   → §2.3, §5.4.4, §7.2 を「failed=0 / exit code 0」基準に変更

**軽微な指摘**
4. ✅ 取得コマンドが `--all-features` 条件とズレている
   → §8.1, §5.4.4 に `cargo test --doc --all-features` を明記

#### 第2回レビュー対応（再レビュー）

**中程度の指摘**
1. ✅ ファイル数が20と表記されているが実際は21
   → §4.2, §7.1, §8.1 を 20ファイル → 21ファイル に修正
2. ✅ private item 対応方針で「pub 化優先」はリスク（プロダクションコード変更最小限と緊張）
   → §5.4.2 類型D を「公開API使用例へ書き換え優先、pub化は最終手段」に変更

**軽微な指摘**
3. ✅ レビューチェックリストの文言が不正確（「51件のdoctestファイル」）
   → §7.1 を「21ファイル / 51件のdoctest項目」に修正

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-17 | 初版作成 | Claude Code (Sonnet 4.5) |
| 2026-02-17 | 質問回答を記録（§8.3）、注意事項・マージ前チェックを更新 | Claude Code (Sonnet 4.5) |
| 2026-02-17 | **Codex レビュー対応（第1回）**（§8.4）：失敗類型別に全面改訂（§2.1, §5.4）、失敗一覧を51件に修正（§8.1, §4.2）、受入条件を exit code 0 基準に変更（§2.3, §7.2） | Claude Code (Sonnet 4.5) |
| 2026-02-17 | **Codex 再レビュー対応（第2回）**（§8.4）：ファイル数を21に修正（§4.2, §7.1, §8.1）、private item 対応方針を安全な順序に変更（§5.4.2, §5.4.3）、レビューチェックリストの文言を修正（§7.1） | Claude Code (Sonnet 4.5) |
| 2026-02-17 | **承認**：Status を Draft → Approved に変更（§1, §3.4）。@MasanoriSuda による承認。実装フェーズへ移行。 | Claude Code (Sonnet 4.5) |
