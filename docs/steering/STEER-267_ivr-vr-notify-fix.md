# STEER-267: DB IVR VR ルートの転送通知欠落バグ修正

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-267 |
| タイトル | DB IVR VR ルートの転送通知欠落バグ修正 |
| ステータス | Approved |
| 関連Issue | #267 |
| 優先度 | P1 |
| 作成日 | 2026-02-28 |

---

## 2. ストーリー（Why）

### 2.1 背景

STEER-266 で着信ポップアップ通知機能を実装したが、IVR 転送の通知発火点を `IvrAction::Transfer`（handlers/mod.rs L869）のみと規定した。

実際の DB IVR 転送は `"VR" =>` ブランチ（handlers/mod.rs L1304）経由で `start_b2bua_transfer("ivr_vr")` が呼ばれる。このブランチに `notify_ivr_transfer_if_needed()` の呼び出しが存在しないため、IVR 転送時に Frontend へのポップアップ通知が発火しない。

通話転送（B2BUA）自体は正常に動作している。通知処理の仕様漏れによるバグ。

### 2.2 目的

DB IVR VR ルートで転送が開始されたとき、Frontend にポップアップ通知が発火するよう修正する。

### 2.3 ユーザーストーリー

```
As a オペレーター
I want to IVR で転送選択したときにポップアップが表示される
So that 着信に気づいて対応できる

受入条件:
- [ ] IVR で転送（action_code="VR"）が選択されたとき、Frontend にポップアップが表示される
- [ ] ポップアップには callerNumber・IVR 滞留時間・DTMF 押下履歴が表示される
- [ ] 直接着信通知と重複して表示されない（notification_sent による idempotency が機能する）
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-28 |
| 起票理由 | STEER-266 実装後の動作確認で IVR 転送時にポップアップが出ないことを確認 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Sonnet 4.6 |
| 作成日 | 2026-02-28 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Refs #267 としてステアリングを切ってほしい" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | @MasanoriSuda | 2026-02-28 | ok | |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-28 |
| 承認コメント | Codex 合意取得済み |

### 3.5 実装

| 項目 | 値 |
|------|-----|
| 実装者 | Codex |
| 実装日 | - |
| 指示者 | @MasanoriSuda |
| 指示内容 | "STEER-267 に従い handlers/mod.rs L1304 に notify 呼び出しを追加" |
| コードレビュー | - |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | - |
| マージ日 | - |
| マージ先 | STEER-266（補完） |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| docs/steering/STEER-266_incoming-call-popup.md | 修正 | §5.4 通知発火条件表に `"VR" =>` ルート行を追加（マージ後） |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| virtual-voicebot-backend/src/protocol/session/handlers/mod.rs | 修正 | L1304 `"VR" =>` ブランチに `notify_ivr_transfer_if_needed()` 追加 |

---

## 5. 差分仕様（What / How）

### 5.1 要件（STEER-266 既存要件の補完）

新規要件なし。STEER-266 §5.1 の IVR 転送通知要件（`RD-266-FR-02` 相当）を DB IVR VR ルートにも適用する。

### 5.2 詳細設計追加（handlers/mod.rs L1304 `"VR" =>` ブランチ修正）

#### 修正前

```rust
"VR" => {
    self.record_ivr_event(
        "exit",
        None,
        None,
        None,
        Some("VR"),
        Some("transfer_initiated"),
    );
    self.set_transfer_after_answer_pending(false);
    self.start_b2bua_transfer("ivr_vr");
}
```

#### 修正後

```rust
"VR" => {
    self.record_ivr_event(
        "exit",
        None,
        None,
        None,
        Some("VR"),
        Some("transfer_initiated"),
    );
    self.set_transfer_after_answer_pending(false);
    self.notify_ivr_transfer_if_needed(); // 追加: IVR 転送通知（STEER-267）
    self.start_b2bua_transfer("ivr_vr");
}
```

#### 設計根拠

- `notify_ivr_transfer_if_needed()`（coordinator.rs L501）は以下を行う：
  - `notification_sent` が true の場合は即 return（idempotency 保証）
  - `ivr_started_at` から IVR 滞留時間を算出
  - `dtmf_history` を取得
  - `write_incoming_call_notification()` に `trigger: "ivr_transfer"` で書き込み
- `start_b2bua_transfer("ivr_vr")` の**前**に呼び出すことで、転送前に通知を確実に発火させる
- `IvrAction::Transfer`（L869）と同じ呼び出しパターンに統一する

#### 実装上の確認事項

DB IVR コールで 180 Ringing 時点に `is_ivr_call = true` が設定されていることを実装者が確認すること。これにより直接着信通知（`notify_direct_incoming_if_needed()`）との二重発火が防止される（STEER-266 §5.4 の idempotency 仕様）。

### 5.3 テストケース追加

```markdown
## TC-267-01: DB IVR VR ルートで転送通知が発火すること

### 対象
handlers/mod.rs L1304 `"VR" =>` ブランチ

### 目的
`notify_ivr_transfer_if_needed()` が呼ばれ、pending.jsonl に通知エントリが書き込まれることを確認

### 入力
- action_code = "VR"
- ivr_started_at が設定済み（例: 2秒前）
- dtmf_history = ['1', '3']
- notification_sent = false

### 期待結果
- pending.jsonl に trigger="ivr_transfer" のエントリが 1 件書き込まれる
- payload.ivrData.dwellTimeSec ≥ 2
- payload.ivrData.dtmfHistory = ["1", "3"]

## TC-267-02: 二重発火しないこと（notification_sent idempotency）

### 目的
notification_sent = true の状態で呼び出しても追記されないことを確認

### 入力
TC-267-01 と同じ条件 + notification_sent = true

### 期待結果
- pending.jsonl への追記なし
```

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #267 | STEER-267 | 起票 |
| STEER-266 | STEER-267 | 仕様漏れ補完（IVR VR ルート未カバー） |
| STEER-267 | handlers/mod.rs L1304 | 修正箇所 |
| STEER-267 | TC-267-01, TC-267-02 | テスト |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] `notify_ivr_transfer_if_needed()` の呼び出し位置が `start_b2bua_transfer` の前であることを確認
- [ ] `IvrAction::Transfer`（L869）パターンとの対称性が確保されているか
- [ ] `is_ivr_call` の設定タイミングが DB IVR フローで正しいか（実装者確認事項として明記されているか）
- [ ] idempotency（notification_sent）が既存実装で機能することを確認
- [ ] テストケースが網羅的か（正常系・idempotency）

### 7.2 マージ前チェック（Approved → Merged）

- [ ] handlers/mod.rs L1304 への修正が完了している
- [ ] TC-267-01, TC-267-02 が PASS
- [ ] STEER-266 §5.4 発火条件表の更新準備ができている

---

## 8. 備考

- STEER-266 承認後の実装で発覚した仕様漏れ。責任所在は仕様作成時のコード探索不足（Claude Code）
- `notify_ivr_transfer_if_needed()` の実装自体は coordinator.rs L501 に存在し、変更不要
- レガシー IVR パス（L869）は正常に動作している。本 STEER は DB IVR パスのみを対象とする

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-28 | 初版作成 | Claude Sonnet 4.6 |
