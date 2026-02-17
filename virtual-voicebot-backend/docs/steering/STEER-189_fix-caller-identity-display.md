# STEER-189: B2BUA 転送時の発信者情報表示修正

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-189 |
| タイトル | B2BUA 転送時の発信者情報表示修正 |
| ステータス | Approved |
| 関連Issue | #189 |
| 優先度 | P1 |
| 作成日 | 2026-02-17 |

---

## 2. ストーリー（Why）

### 2.1 背景

**現象**:
- 着信転送（IVR Transfer または Outbound 発信）時、着信側の電話に表示される発信者名が常に "rustbot" になっている
- 期待動作では、実際の発信者番号（または anonymous）が表示されるべき

**問題**:
- B2BUA の B-leg 向け INVITE 生成時、From ヘッダーが実際の発信者番号ではなく固定値（rustbot）で組み立てられている
- 具体的には以下の箇所で問題が発生：
  - 転送（IVR Transfer）経路: `b2bua.rs` 220-222 行目
  - Outbound 経路: `b2bua.rs` 534-538 行目

**影響範囲**:
- 転送先の相手が「誰からの着信か」を判別できない（ユーザビリティ低下）
- セキュリティ観点で caller identity の透過性が保証されていない

### 2.2 目的

B2BUA 転送時の INVITE で、実際の発信者番号を From ヘッダー（および必要に応じて P-Asserted-Identity）に設定することで、着信側に正しい発信者情報を表示する。

### 2.3 ユーザーストーリー

```
As a 転送先の受話者（B-leg）
I want to 着信時に実際の発信者番号を確認したい
So that 誰からの電話かを判別して適切に対応できる

受入条件:
- [ ] 転送時の B-leg INVITE の From ヘッダーに実発信者番号が設定される
- [ ] 実発信者が非通知（anonymous）の場合、From は "anonymous" として扱われる
- [ ] 不正な caller_uri の場合、anonymous として処理される（エラーログ出力）

将来課題:
- P-Asserted-Identity（PAI）ヘッダーの設定（§8.1 Q1 参照）
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-17 |
| 起票理由 | Issue #189 報告 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-sonnet-4-5) |
| 作成日 | 2026-02-17 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Issue #189 を立てました。Codex の調査結果を基にステアリング Draft を作成してください" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
|   |           |      |      |         |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-17 |
| 承認コメント | レビュー完了、実装可 |

### 3.5 実装（該当する場合）

| 項目 | 値 |
|------|-----|
| 実装者 | |
| 実装日 | |
| 指示者 | |
| 指示内容 | |
| コードレビュー | |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | |
| マージ日 | |
| マージ先 | DD-008（新規、B2BUA モジュール詳細設計）, DD-005 |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| virtual-voicebot-backend/docs/design/detail/DD-005_session.md | 修正 | SessionCoordinator から B2BUA への caller identity 受け渡し仕様を追記 |
| virtual-voicebot-backend/docs/design/detail/DD-008_b2bua.md（新規） | 追加 | B2BUA モジュールの詳細設計（From/PAI 生成ロジック） |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| src/protocol/session/b2bua.rs | 修正 | spawn_transfer / spawn_outbound に caller_uri 引数を追加、From ヘッダー生成ロジックを修正 |
| src/protocol/session/handlers/mod.rs | 修正 | spawn_transfer / spawn_outbound 呼び出し時に from_uri を渡すように修正（140/833/910行目） |

---

## 5. 差分仕様（What / How）

### 5.1 要件追加（RD-005（新規、B2BUA 要件）へマージ）

> **注**: 既存の RD-001〜RD-004 に B2BUA 要件が存在しないため、RD-005 として新規作成する。
> DD-006, DD-007 が既存のため、詳細設計は DD-008 とする。

```markdown
## RD-005-FR-01: B2BUA 転送時の発信者情報透過

### 概要
B2BUA を用いた転送（IVR Transfer / Outbound 発信）時に、実際の発信者番号を B-leg の From ヘッダーに設定し、着信側に正しい caller identity を表示する。

### 入力
- A-leg の From ヘッダー（元発信者番号）
- 転送先（B-leg）の Request-URI

### 出力
- B-leg INVITE の From ヘッダー（実発信者番号 or anonymous）
- （必要に応じて）P-Asserted-Identity ヘッダー

### 受入条件
- [ ] B2BUA 転送時、B-leg INVITE の From ヘッダーに A-leg の From URI が設定される
- [ ] A-leg が非通知（anonymous）の場合、From は `<sip:anonymous@anonymous.invalid>` となる
- [ ] From ヘッダーの tag パラメータは B2BUA 側で新規生成される（A-leg の tag は使用しない）
- [ ] 不正な caller_uri の場合、anonymous として処理される（エラーログ出力）

### スコープ外（将来課題）
- P-Asserted-Identity（PAI）ヘッダーの設定（§8.1 Q1 参照）

### 優先度
P1

### トレース
- → DD: DD-008-FN-01（B2BUA INVITE 生成）
- → UT: UT-008-TC-01（From ヘッダー設定テスト）
```

---

### 5.2 詳細設計追加（DD-008（新規、B2BUA モジュール）へマージ）

> **注**: DD-008 として B2BUA モジュールの詳細設計を新規作成する。

```markdown
## DD-008-FN-01: B2BUA INVITE 生成（From ヘッダー設定）

### シグネチャ
```rust
pub fn spawn_transfer(
    a_call_id: CallId,
    caller_uri: String,  // 追加
    control_tx: mpsc::Sender<SessionControlIn>,
    media_tx: mpsc::Sender<SessionMediaIn>,
    runtime_cfg: Arc<SessionRuntimeConfig>,
) -> tokio::sync::oneshot::Sender<()>

pub fn spawn_outbound(
    a_call_id: CallId,
    caller_uri: String,  // 追加
    number: String,
    control_tx: mpsc::Sender<SessionControlIn>,
    media_tx: mpsc::Sender<SessionMediaIn>,
    runtime_cfg: Arc<SessionRuntimeConfig>,
) -> tokio::sync::oneshot::Sender<()>
```

### 入力
| パラメータ | 型 | 説明 |
|-----------|-----|------|
| a_call_id | CallId | A-leg の Call-ID |
| caller_uri | String | 元発信者の URI（例: `sip:+819012345678@example.com`） |
| number | String | 転送先番号（Outbound のみ） |
| control_tx | mpsc::Sender | セッション制御チャネル |
| media_tx | mpsc::Sender | メディアチャネル |
| runtime_cfg | Arc<SessionRuntimeConfig> | ランタイム設定 |

### 出力
| 型 | 説明 |
|----|------|
| oneshot::Sender<()> | キャンセルシグナル送信用 |

### 処理フロー
1. caller_uri を解析し、ユーザー部分（電話番号）を抽出
   - name-addr 形式（例: `"User" <sip:+819012345678@host>`）に対応
   - tel: URI（例: `tel:+819012345678`）に対応
   - anonymous 判定は大文字小文字無視（`Anonymous` / `anonymous` / `ANONYMOUS`）
   - **b2bua.rs 内に helper 関数を新設**（`handlers/sip_handler.rs` の `extract_user_from_to` と同等仕様）
2. From ヘッダーを以下のフォーマットで生成：
   - 通常: `<sip:+819012345678@{advertised_ip}:{sip_port}>;tag={new_tag}`
   - 非通知: `<sip:anonymous@anonymous.invalid>;tag={new_tag}`
3. （将来対応）P-Asserted-Identity が必要な場合、PAI ヘッダーを追加
4. B-leg INVITE を生成・送信

### From ヘッダー生成ロジック

**caller_uri のパース**:
```rust
use crate::protocol::sip::{parse_name_addr, parse_uri};
use crate::shared::utils::mask_pii;

// b2bua.rs 内に helper 関数を新設（handlers/sip_handler.rs の extract_user_from_to と同等仕様）
// - name-addr 形式対応（例: "User" <sip:+819012345678@host>）
// - tel: URI 対応（例: tel:+819012345678）
// - parse_name_addr 失敗時は parse_uri にフォールバック
//
// 実装例:
// fn extract_caller_user(value: &str) -> Option<String> {
//     // 1. parse_name_addr を試す
//     if let Ok(name_addr) = parse_name_addr(value) {
//         if name_addr.uri.scheme.eq_ignore_ascii_case("tel") {
//             // tel: URI の場合、host（電話番号）を返す
//             if !name_addr.uri.host.trim().is_empty() {
//                 return Some(name_addr.uri.host);
//             }
//         }
//         // sip: URI の場合、user 部分を返す
//         if let Some(user) = name_addr.uri.user {
//             return Some(user);
//         }
//     }
//     // 2. parse_name_addr が失敗した場合、手動で URI を抽出して parse_uri を試す
//     let trimmed = value.trim();
//     let addr = if let Some(start) = trimmed.find('<') {
//         if let Some(end) = trimmed[start + 1..].find('>') {
//             &trimmed[start + 1..start + 1 + end]
//         } else {
//             trimmed
//         }
//     } else {
//         trimmed
//     };
//     let addr = addr.split(';').next().unwrap_or(addr).trim();
//     let uri = parse_uri(addr).ok()?;
//     if uri.scheme.eq_ignore_ascii_case("tel") {
//         if !uri.host.trim().is_empty() {
//             return Some(uri.host);
//         }
//     }
//     uri.user
// }

let caller_user = match extract_caller_user(&caller_uri) {
    Some(user) if !user.is_empty() && !user.eq_ignore_ascii_case("anonymous") => user,
    _ => {
        // PII 保護のため caller_uri をマスク化してログ出力
        warn!("[b2bua {}] Invalid or anonymous caller_uri: {}, using anonymous", a_call_id, mask_pii(&caller_uri));
        "anonymous".to_string()
    }
};
```

> **注**:
> - モジュール可視性の制約により、`handlers/sip_handler.rs` の `extract_user_from_to` は `b2bua.rs` から直接参照できない（`handlers` は private module）
> - そのため、b2bua.rs 内に同等仕様の helper 関数 `extract_caller_user` を新設する
> - 実装は `parse_name_addr` を試し、失敗時に `parse_uri` にフォールバックする（既存実装と同等）
> - tel: URI の場合は `uri.host`（電話番号）を、sip: URI の場合は `uri.user` を返す
> - PII 保護のため、WARN ログでは `mask_pii` ユーティリティ（`shared/utils/mod.rs`）を使用する

**From ヘッダー生成（経路別）**:

**Transfer 経路**: advertised_ip:sip_port を使用
```rust
let from_header = if caller_user == "anonymous" {
    format!("<sip:anonymous@anonymous.invalid>;tag={}", generate_tag())
} else {
    format!("<sip:{}@{}:{}>;tag={}", caller_user, runtime_cfg.advertised_ip, sip_port, generate_tag())
};
```

**Outbound 経路**: registrar.domain を使用（既存の実装に合わせる）
```rust
let from_header = if caller_user == "anonymous" {
    format!("<sip:anonymous@anonymous.invalid>;tag={}", generate_tag())
} else {
    format!("<sip:{}@{}>;tag={}", caller_user, registrar.domain, generate_tag())
};
```

### エラーケース
| エラー | 条件 | 対応 |
|--------|------|------|
| InvalidUri | caller_uri が不正な SIP URI | WARN ログ出力（PII マスク済み）、caller_user = "anonymous" として処理継続 |

### 注記
- **Transfer 経路と Outbound 経路で From の host/domain が異なる**:
  - Transfer: `advertised_ip:sip_port` を使用（SIP クライアント向け）
  - Outbound: `registrar.domain` を使用（既存実装との整合性を保つため）

### トレース
- ← RD: RD-005-FR-01
- → UT: UT-008-TC-01, UT-008-TC-02, UT-008-TC-03, UT-008-TC-04
```

---

### 5.3 DD-005 への差分追加

```markdown
## DD-005-FN-XX: SessionCoordinator イベントハンドラの修正

### 変更内容
`handlers/mod.rs` の spawn_transfer / spawn_outbound 呼び出し箇所で `self.from_uri` を渡す。

**変更箇所**:
- `handlers/mod.rs:140` - Outbound 発信時（IVR 経由）
- `handlers/mod.rs:833` - Transfer 実行時（メインフロー）
- `handlers/mod.rs:910` - Transfer 実行時（アナウンス後転送）

**変更例（spawn_transfer）**:
```rust
// 変更前
self.transfer_cancel = Some(b2bua::spawn_transfer(
    self.call_id.clone(),
    self.control_tx.clone(),
    self.media_tx.clone(),
    self.runtime_cfg.clone(),
));

// 変更後
self.transfer_cancel = Some(b2bua::spawn_transfer(
    self.call_id.clone(),
    self.from_uri.clone(),  // 追加
    self.control_tx.clone(),
    self.media_tx.clone(),
    self.runtime_cfg.clone(),
));
```

**変更例（spawn_outbound）**:
```rust
// 変更前
self.transfer_cancel = Some(b2bua::spawn_outbound(
    self.call_id.clone(),
    number,
    self.control_tx.clone(),
    self.media_tx.clone(),
    self.runtime_cfg.clone(),
));

// 変更後
self.transfer_cancel = Some(b2bua::spawn_outbound(
    self.call_id.clone(),
    self.from_uri.clone(),  // 追加
    number,
    self.control_tx.clone(),
    self.media_tx.clone(),
    self.runtime_cfg.clone(),
));
```

### トレース
- ← RD: RD-005-FR-01
- → UT: UT-005-TC-XX
```

---

### 5.4 テストケース追加（UT-008 / IT-008 へマージ）

```markdown
## UT-008-TC-01: B2BUA INVITE の From ヘッダー設定（Transfer 経路・通常ケース）

### 対象
DD-008-FN-01

### 目的
B2BUA 転送時、From ヘッダーに実発信者番号が正しく設定されることを検証

### 入力
- caller_uri: `"sip:+819012345678@10.0.0.1"`
- advertised_ip: `"192.168.1.100"`
- sip_port: `5060`
- 経路: Transfer

### 期待結果
- From ヘッダー: `<sip:+819012345678@192.168.1.100:5060>;tag={tag}`
- tag パラメータが存在し、`b2bua` プレフィックスで始まる（例: `b2bua123456789`）

### トレース
← DD: DD-008-FN-01

---

## UT-008-TC-02: B2BUA INVITE の From ヘッダー設定（非通知ケース）

### 対象
DD-008-FN-01

### 目的
非通知着信の B2BUA 転送時、From ヘッダーが anonymous になることを検証

### 入力
- caller_uri: `"sip:anonymous@anonymous.invalid"`
- advertised_ip: `"192.168.1.100"`
- sip_port: `5060`

### 期待結果
- From ヘッダー: `<sip:anonymous@anonymous.invalid>;tag={tag}`
- tag パラメータが存在する

### トレース
← DD: DD-008-FN-01

---

## UT-008-TC-03: B2BUA INVITE の From ヘッダー設定（Outbound 経路）

### 対象
DD-008-FN-01

### 目的
Outbound 経路で From ヘッダーが registrar.domain を使用することを検証

### 入力
- caller_uri: `"sip:+819012345678@10.0.0.1"`
- registrar.domain: `"sip.example.com"`
- 経路: Outbound

### 期待結果
- From ヘッダー: `<sip:+819012345678@sip.example.com>;tag={tag}`
- tag パラメータが存在する

### トレース
← DD: DD-008-FN-01

---

## UT-008-TC-04: B2BUA INVITE の From ヘッダー設定（不正 URI フォールバック）

### 対象
DD-008-FN-01

### 目的
caller_uri が不正な場合、anonymous として処理されることを検証

### 入力
- caller_uri: `"invalid-uri-format"`（不正な SIP URI）
- advertised_ip: `"192.168.1.100"`
- sip_port: `5060`
- 経路: Transfer

### 期待結果
- From ヘッダー: `<sip:anonymous@anonymous.invalid>;tag={tag}`
- WARN ログ出力: `"[b2bua xxx] Invalid or anonymous caller_uri: <マスク済み文字列>, using anonymous"`（caller_uri は `mask_pii` でマスクされる）

### トレース
← DD: DD-008-FN-01

---

## IT-008-TC-01: B2BUA 転送の E2E テスト（発信者表示）

### 対象
B2BUA 転送フロー全体

### 目的
A-leg → B2BUA → B-leg の転送シナリオで、B-leg に発信者番号が正しく表示されることを検証

### 入力
- A-leg: Zoiper から `sip:bot@server` へ発信（From: `+819012345678`）
- IVR で転送先番号を選択（例: DTMF "1" で `+819087654321` へ転送）

### 期待結果
- B-leg の Zoiper に `+819012345678`（A-leg の番号）が発信者として表示される
- B-leg の SIP ログに From: `<sip:+819012345678@...>` が記録される

### トレース
← DD: DD-008-FN-01
```

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #189 | STEER-189 | 起票 |
| STEER-189 | RD-005-FR-01 | 要件追加 |
| RD-005-FR-01 | DD-008-FN-01 | 詳細設計 |
| RD-005-FR-01 | DD-005-FN-XX | 既存設計修正 |
| DD-008-FN-01 | UT-008-TC-01, UT-008-TC-02, UT-008-TC-03, UT-008-TC-04 | 単体テスト |
| DD-008-FN-01 | IT-008-TC-01 | 統合テスト |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] 要件の記述が明確か
- [ ] 詳細設計で実装者が迷わないか
- [ ] テストケースが網羅的か（通常ケース・非通知ケース）
- [ ] 既存仕様（DD-003, DD-005）との整合性があるか
- [ ] トレーサビリティが維持されているか
- [ ] SIP RFC 3261 §20.20（From ヘッダー）に準拠しているか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] 実装が完了している
- [ ] コードレビューを受けている
- [ ] 単体テスト・統合テストが PASS している
- [ ] 本体仕様書（RD-005, DD-008, DD-005）への反映準備ができている

---

## 8. 備考

### 8.1 未確定点・質問リスト（Open Questions）

#### Q1: P-Asserted-Identity（PAI）ヘッダーの必要性
- **質問**: SIP キャリアによっては、From ヘッダーだけでなく P-Asserted-Identity（RFC 3325）ヘッダーが必要か？
- **調査事項**:
  - MVP では PAI は不要と仮定する
  - Zoiper 同士の通話では From ヘッダーのみで発信者番号が表示されることを確認
  - 将来、SIP トランク（キャリア）との連携時に PAI が必要になる可能性がある
- **判断基準**: 実環境での動作確認後、必要に応じて PAI を追加

#### Q2: Privacy ヘッダーの扱い
- **質問**: 非通知着信の場合、Privacy: id ヘッダー（RFC 3323）を付与すべきか？
- **仮決定**: MVP では Privacy ヘッダーなしで From を anonymous とする
- **将来対応**: SIP トランク連携時に Privacy ヘッダーを追加

#### Q3: From ヘッダーの Display Name
- **質問**: From ヘッダーに Display Name（表示名）を含めるべきか？
  - 例: `"山田太郎" <sip:+819012345678@...>`
- **仮決定**: MVP では Display Name なし（URI のみ）
- **将来対応**: registered_numbers テーブルの `name` フィールドを Display Name として使用

### 8.2 リスク・ロールバック観点

**リスク**:
- From ヘッダーの変更が既存の SIP クライアントとの互換性に影響する可能性
- URI パース処理で例外が発生した場合の fallback が必要

**ロールバック方針**:
- 問題発生時は From を固定値（rustbot）に戻すことで即座にロールバック可能
- ただし、本修正は SIP RFC 準拠の正しい実装のため、問題があれば根本原因を調査して修正すべき

**モニタリング**:
- B2BUA INVITE 生成時に From ヘッダーの内容をログ出力（INFO レベル）
- URI パースエラー時は WARN ログ出力（`mask_pii` でマスク済み、匿名として fallback）
- デバッグ環境では環境変数などで詳細ログを有効化する余地を残す

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-17 | 初版作成（Draft） | Claude Code (claude-sonnet-4-5) |
| 2026-02-17 | レビュー指摘反映（第1回）：DD番号を DD-008 へ変更、修正対象ファイルを handlers/mod.rs へ修正、URI パース fallback 明示化、tag 条件を現実装に合わせる、Transfer/Outbound 経路の From ドメイン方針を分離、Draft 段階の実装者欄を空欄に修正 | Claude Code (claude-sonnet-4-5) |
| 2026-02-17 | レビュー指摘反映（第2回）：PAI を受入条件からスコープ外に移動、WARN ログの PII マスク化、caller_uri 解析仕様の明記（name-addr/tel URI/大文字小文字対応）、既存ロジック再利用方針の追記、UT-008-TC-04 追加（不正 URI フォールバックテスト） | Claude Code (claude-sonnet-4-5) |
| 2026-02-17 | レビュー指摘反映（第3回）：§2.3 の PAI 受入条件を削除し将来課題へ統一、関数名を extract_user_from_to に修正（実在する関数名に合わせる）、mask_pii を既存ユーティリティとして明記（将来導入 → 既存利用） | Claude Code (claude-sonnet-4-5) |
| 2026-02-17 | レビュー指摘反映（第4回）：extract_user_from_to の再利用方針をモジュール可視性に合わせて修正（b2bua.rs 内に helper 新設、parse_name_addr/parse_uri を使用） | Claude Code (claude-sonnet-4-5) |
| 2026-02-17 | レビュー指摘反映（第5回）：影響ドキュメントのパスを実在パスに修正（virtual-voicebot-backend/docs/... に統一）、helper 実装例を parse_name_addr + parse_uri フォールバックありに修正（既存実装と同等の構造に修正） | Claude Code (claude-sonnet-4-5) |
