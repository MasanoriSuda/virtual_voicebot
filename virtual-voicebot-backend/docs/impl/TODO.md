<!-- SOURCE_OF_TRUTH: 実装バックログ -->
# Implementation Backlog (TODO.md)

| 項目 | 値 |
|------|-----|
| **Status** | Active |
| **Owner** | TBD |
| **Last Updated** | 2025-12-27 |
| **SoT (Source of Truth)** | Yes - 実装バックログ |
| **上流ドキュメント** | [gap-analysis.md](../gap-analysis.md), [PLAN.md](PLAN.md) |

---

## 概要

gap-analysis.md から抽出した実装項目のバックログです。

**分類**:
- **P0**: 現行 UAS 機能に必須（ボイスボット運用に必要）
- **P1**: 相互接続性・RFC 準拠に重要
- **P2**: 汎用 SIP サーバー向け拡張
- **Deferred**: UAS 完了後に着手（設計決定/仕様策定が必要）

---

## P0: 必須（現行 UAS）

| ID | 項目 | RFC | 状態 | PLAN Step |
|----|------|-----|------|-----------|
| P0-01 | CANCEL 受信処理 | 3261 §9 | 未着手 | [Step-01](PLAN.md#step-01-cancel-受信処理) |
| P0-02 | DTMF トーン検出 (Goertzel) | ITU-T Q.23 | 未着手 | [Step-02](PLAN.md#step-02-dtmf-トーン検出-goertzel) |
| P0-03 | SIPp CANCEL シナリオ | - | 未着手 | [Step-03](PLAN.md#step-03-sipp-cancel-シナリオ) |
| P0-04 | DTMF トーン検出 E2E 検証 | - | 未着手 | [Step-04](PLAN.md#step-04-dtmf-トーン検出-e2e-検証) |

---

## P1: 重要（RFC 準拠・相互接続性）

| ID | 項目 | RFC | 状態 | PLAN Step |
|----|------|-----|------|-----------|
| P1-01 | RSeq ランダム化 | 3262 §3 | 未着手 | [Step-05](PLAN.md#step-05-rseq-ランダム化) |
| P1-02 | OPTIONS 応答 | 3261 §11 | 未着手 | [Step-06](PLAN.md#step-06-options-応答) |
| P1-03 | a=rtpmap パース | 8866 | 未着手 | [Step-07](PLAN.md#step-07-artpmap-パース) |
| P1-04 | RTCP SDES (CNAME) | 3550 §6.5 | 未着手 | [Step-08](PLAN.md#step-08-rtcp-sdes-cname) |
| P1-05 | 486 Busy Here | 3261 | 未着手 | [Step-09](PLAN.md#step-09-486-busy-here) |
| P1-06 | 183 Session Progress | 3261 | 実装済み | - |
| P1-07 | 複数 Reliable Provisional | 3262 | 未着手 | - |
| P1-08 | Timer G/H 厳密化 | 3261 §17.2.1 | 未着手 | - |
| P1-09 | Contact URI 完全パース | 3261 | 未着手 | - |
| P1-10 | IPv6 対応 (c=IN IP6) | 8866 | 未着手 | - |

---

## P2: 拡張（汎用 SIP）

| ID | 項目 | RFC | 状態 | PLAN Step |
|----|------|-----|------|-----------|
| P2-01 | a=fmtp パース | 8866 | 未着手 | [Step-10](PLAN.md#step-10-afmtp-パース) |
| P2-02 | RFC 2833 DTMF 受信 | 2833/4733 | 未着手 | [Step-11](PLAN.md#step-11-rfc-2833-dtmf-受信) |
| P2-03 | a=ptime パース | 8866 | 未着手 | - |
| P2-04 | RTCP BYE | 3550 | 未着手 | - |
| P2-05 | RTCP 動的送信間隔 | 3550 §6.2 | 未着手 | - |
| P2-06 | RFC 3389 Comfort Noise | 3389 | 未着手 | - |
| P2-07 | 5xx サーバーエラー応答 | 3261 | 未着手 | - |

---

## Deferred（UAS 完了後に着手）

以下の項目は UAS 完了後に着手します。実装前に仕様/設計の決定が必要です。

### UAC 機能（発信）

| ID | 項目 | RFC | 必要な決定事項 |
|----|------|-----|---------------|
| SPEC-01 | UAC INVITE 送信 | 3261 §17.1.1 | UAC トランザクション状態機械の設計 |
| SPEC-02 | UAC ACK/BYE 送信 | 3261 | ダイアログ管理の設計 |
| SPEC-03 | DNS SRV 解決 | 3263 | resolver クレート選定、キャッシュ戦略 |
| SPEC-04 | DNS NAPTR 解決 | 3263 | トランスポート自動選択ロジック |

### 認証

| ID | 項目 | RFC | 必要な決定事項 |
|----|------|-----|---------------|
| SPEC-05 | Digest 認証 (UAS) | 3261 §22 | credentials ストア設計、nonce 管理 |
| SPEC-06 | 401/407 チャレンジ | 3261 | realm/qop 設定方針 |
| SPEC-07 | 403 Forbidden | 3261 | 認証失敗時のポリシー |

### セキュリティ

| ID | 項目 | RFC | 必要な決定事項 |
|----|------|-----|---------------|
| SPEC-08 | TLS トランスポート | 3261 §26 | 証明書管理、SIPS URI 対応 |
| SPEC-09 | SRTP | 3711 | キー交換方式 (SDES vs DTLS-SRTP) |

### セッション管理

| ID | 項目 | RFC | 必要な決定事項 |
|----|------|-----|---------------|
| SPEC-10 | re-INVITE 送信 | 4028 | refresher=uas 時のタイマー設計 |
| SPEC-11 | UPDATE 送信 | 3311 | セッション更新トリガー設計 |
| SPEC-12 | Hold/Resume | 3264 | a=sendonly/recvonly 切り替え設計 |
| SPEC-13 | 複数コーデック交渉 | 3264 | コーデック優先度、動的 PT 管理 |

### Proxy 機能

| ID | 項目 | RFC | 必要な決定事項 |
|----|------|-----|---------------|
| SPEC-14 | Proxy 機能 | 3261 | Stateful/Stateless、フォーキング戦略 |
| SPEC-15 | Record-Route/Route | 3261 | ルーティングテーブル設計 |
| SPEC-16 | REGISTER バインディング | 3261 | バインディング DB、Expires 管理 |

### 転送

| ID | 項目 | RFC | 必要な決定事項 |
|----|------|-----|---------------|
| SPEC-17 | REFER | 3515 | Refer-To 処理、NOTIFY 送信 |
| SPEC-18 | Replaces | 3891 | ダイアログ置換ロジック |

---

## 完了済み

| ID | 項目 | 完了日 | PR |
|----|------|--------|-----|
| - | (なし) | - | - |

---

## 凡例

| 状態 | 意味 |
|------|------|
| 未着手 | 作業開始前 |
| 進行中 | 作業中 |
| レビュー中 | PR レビュー待ち |
| 完了 | マージ済み |
| 実装済み | 既に実装されている |

---

## 変更履歴

| 日付 | バージョン | 変更内容 |
|------|-----------|---------|
| 2025-12-27 | 1.2 | UAS 優先構成に変更、Spec待ち → Deferred、PLAN Step 参照を更新 |
| 2025-12-25 | 1.1 | RFC 2833 を P2 に移動、DTMF トーン検出 (Goertzel) を P0 に追加 |
| 2025-12-25 | 1.0 | 初版作成 |
