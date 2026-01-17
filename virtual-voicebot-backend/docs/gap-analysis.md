<!-- SOURCE_OF_TRUTH: RFC準拠ギャップ分析・仕様 -->
# Gap Analysis: UAS優先のボイスボット向けSIPサーバー

| 項目 | 値 |
|------|-----|
| **Status** | Draft |
| **Owner** | TBD |
| **Last Updated** | 2025-12-27 |
| **SoT (Source of Truth)** | Yes - RFC準拠/仕様 |

---

本文書は、virtual-voicebot-backend の実装状況を分析し、**UAS（着信）機能を最優先**とした開発方針に基づくギャップを整理します。

**開発方針**: まずは UAS を完成させ、相互接続性を確保する。認証・暗号化・UAC・Proxy は将来実装として後工程に回す。

---

## 目次

1. [エグゼクティブサマリー](#1-エグゼクティブサマリー)
2. [RFC 3261 (SIP Core) ギャップ](#2-rfc-3261-sip-core-ギャップ)
3. [RFC 3262 (100rel/PRACK) ギャップ](#3-rfc-3262-100relprack-ギャップ)
4. [RFC 3311 (UPDATE) ギャップ](#4-rfc-3311-update-ギャップ)
5. [RFC 4028 (Session Timers) ギャップ](#5-rfc-4028-session-timers-ギャップ)
6. [RFC 3263 (DNS SRV/NAPTR) ギャップ](#6-rfc-3263-dns-srvnaptr-ギャップ)
7. [RFC 3264 (Offer/Answer SDP) ギャップ](#7-rfc-3264-offeranswer-sdp-ギャップ)
8. [RFC 3550 (RTP/RTCP) ギャップ](#8-rfc-3550-rtprtcp-ギャップ)
9. [RFC 8866 (SDP) ギャップ](#9-rfc-8866-sdp-ギャップ)
10. [優先度別ロードマップ](#10-優先度別ロードマップ)
11. [Acceptance Criteria (SIPp検証)](#11-acceptance-criteria-sipp検証)

---

## 1. エグゼクティブサマリー

### 1.1 UAS優先の現状評価

| カテゴリ | 実装状態 | テストカバレッジ | 優先度 | 次のアクション |
|---------|----------|-----------------|--------|----------------|
| **SIP UAS 基本** | Partial | Unit: ✓ / E2E: ✓ | P0 | CANCEL対応 |
| **SIP UAS 拡張** | Partial | Unit: ✓ | P1 | OPTIONS, 486 |
| **RTP/RTCP** | Partial | Unit: ✓ / E2E: Partial | P0 | SDES追加 |
| **SDP パース** | Partial | Unit: ✓ | P1 | rtpmap/fmtp |
| **DTMF 受信** | Not | - | P0 | Goertzel実装 |
| **E2E テスト** | Partial | E2E: Partial | P0 | シナリオ拡充 |
| トランスポート (UDP/TCP) | Implemented | Unit: ✓ / E2E: ✓ | - | 完了 |

### 1.2 Deferred（後工程）項目

以下は将来実装予定として優先度を下げますが、削除はしません。

| カテゴリ | 実装状態 | 備考 |
|---------|----------|------|
| 認証 (Digest/401/407) | Deferred | Spec策定後に実装 |
| 暗号化 (TLS) | Deferred | Spec策定後に実装 |
| 暗号化 (SRTP) | Deferred | Spec策定後に実装 |
| SIP UAC (発信) | Deferred | UAS完了後に着手 |
| Proxy機能 | Deferred | 現時点で不要 |
| DNS SRV/NAPTR | Deferred | UAC実装時に必要 |

### 1.3 凡例

| 実装状態 | 意味 |
|---------|------|
| Implemented | 完全実装済み |
| Partial | 部分実装（制限あり） |
| Not | 未実装（優先対象） |
| Deferred | 優先度を下げて後工程 |

### 1.4 総合評価

現在の実装は**ボイスボット専用のUAS**として機能している。UAS完成度を上げることで、SIPpおよび一般的なSIPクライアントとの相互接続性を確保することが最優先目標。

**P0 で解決すべきギャップ**:
1. CANCEL 受信処理の欠如
2. DTMF 受信機能の欠如
3. E2E テストシナリオの不足

---

## 2. RFC 3261 (SIP Core) ギャップ

### 2.1 メソッド対応（UAS観点）

| メソッド | RFC要件 | UAS実装状態 | テスト | 優先度 |
|---------|--------|-------------|--------|--------|
| INVITE | MUST | Implemented | E2E: ✓ | - |
| ACK | MUST | Implemented | E2E: ✓ | - |
| BYE | MUST | Implemented | E2E: ✓ | - |
| CANCEL | MUST | **Not** | - | **P0** |
| OPTIONS | SHOULD | Not | - | P1 |
| REGISTER | SHOULD | Partial | - | P2 |

### 2.2 トランザクション層（UAS）

| 要件 | RFC参照 | 実装状態 | テスト | 優先度 |
|-----|---------|----------|--------|--------|
| INVITE Server Transaction | §17.2.1 | Partial | Unit: ✓ | P1 (Timer厳密化) |
| Non-INVITE Server Transaction | §17.2.2 | Implemented | Unit: ✓ | - |

### 2.3 トランスポート層

| 要件 | RFC参照 | 実装状態 | テスト | 優先度 |
|-----|---------|----------|--------|--------|
| UDP | MUST | Implemented | E2E: ✓ | - |
| TCP | MUST | Implemented | E2E: ✓ | - |
| TLS | MUST (§26.3.1) | Deferred | - | Deferred |
| WebSocket | RFC 7118 | Deferred | - | Deferred |

### 2.4 ヘッダフィールド（UAS必須）

| ヘッダ | 要件 | 実装状態 | テスト | 優先度 |
|-------|-----|----------|--------|--------|
| Via | 必須 | Implemented | Unit: ✓ | - |
| From/To | 必須 | Implemented | Unit: ✓ | - |
| Call-ID | 必須 | Implemented | Unit: ✓ | - |
| CSeq | 必須 | Implemented | Unit: ✓ | - |
| Contact | 必須 | Partial | - | P1 |
| Authorization | 認証用 | Deferred | - | Deferred |

### 2.5 レスポンスコード（UAS）

| カテゴリ | 実装済み | 次に必要 | 優先度 |
|---------|---------|----------|--------|
| 1xx | 100, 180 | - | - |
| 2xx | 200 | - | - |
| 4xx | 400, 422, 481 | **486, 487** | **P0/P1** |
| 5xx | 504 | 500 | P2 |

---

## 3. RFC 3262 (100rel/PRACK) ギャップ

| 要件 | 実装状態 | テスト | 優先度 |
|-----|----------|--------|--------|
| RSeq ヘッダ生成 | Partial | Unit: ✓ | P1 (ランダム化) |
| PRACK受信処理 | Implemented | Unit: ✓ | - |
| 再送タイマ | Implemented | Unit: ✓ | - |
| タイムアウト | Implemented | Unit: ✓ | - |
| 複数Reliable Provisional | Not | - | P2 |

---

## 4. RFC 3311 (UPDATE) ギャップ

| 要件 | 実装状態 | テスト | 優先度 |
|-----|----------|--------|--------|
| UPDATE受信 (UAS) | Partial | Unit: ✓ | - |
| UPDATE送信 (UAC) | Deferred | - | Deferred |
| 491 Request Pending | Not | - | P2 |

---

## 5. RFC 4028 (Session Timers) ギャップ

| 要件 | 実装状態 | テスト | 優先度 |
|-----|----------|--------|--------|
| Session-Expires パース | Implemented | Unit: ✓ | - |
| Min-SE パース | Implemented | Unit: ✓ | - |
| 422応答 + Min-SE | Implemented | Unit: ✓ | - |
| re-INVITE送信 | Deferred | - | Deferred |
| UPDATE送信 | Deferred | - | Deferred |

---

## 6. RFC 3263 (DNS SRV/NAPTR) ギャップ

| 機能 | 実装状態 | 優先度 | 備考 |
|-----|----------|--------|------|
| SRV クエリ | Deferred | Deferred | UAC実装時に必要 |
| NAPTR クエリ | Deferred | Deferred | UAC実装時に必要 |
| A/AAAA クエリ | Deferred | Deferred | UAC実装時に必要 |

---

## 7. RFC 3264 (Offer/Answer SDP) ギャップ

| 要件 | 実装状態 | テスト | 優先度 |
|-----|----------|--------|--------|
| Offer受信 (INVITE) | Partial | Unit: ✓ | - |
| Answer生成 | Partial | - | P1 (動的化) |
| Re-INVITE offer | Deferred | - | Deferred |
| Hold/Resume | Deferred | - | Deferred |
| コーデック交渉 | Not | - | P2 |

---

## 8. RFC 3550 (RTP/RTCP) ギャップ

### 8.1 RTP（UAS）

| 要件 | 実装状態 | テスト | 優先度 |
|-----|----------|--------|--------|
| パケット構造 | Implemented | Unit: ✓ | - |
| SSRC管理 | Implemented | Unit: ✓ | - |
| Sequence番号 | Implemented | Unit: ✓ | - |
| Timestamp | Implemented | Unit: ✓ | - |

### 8.2 RTCP

| 要件 | 実装状態 | テスト | 優先度 |
|-----|----------|--------|--------|
| SR | Implemented | Unit: ✓ | - |
| RR | Implemented | Unit: ✓ | - |
| SDES (CNAME) | **Not** | - | **P1** |
| BYE | Not | - | P2 |

### 8.3 DTMF

| 機能 | 実装状態 | 優先度 | 備考 |
|-----|----------|--------|------|
| DTMF トーン検出 (Goertzel) | **Not** | **P0** | インバンド検出 |
| RFC 2833 DTMF | Not | P2 | アウトオブバンド |

---

## 9. RFC 8866 (SDP) ギャップ

| フィールド | 実装状態 | 優先度 | 備考 |
|-----------|----------|--------|------|
| c= (connection) | Partial | - | IPv6: P2 |
| m= (media) | Partial | - | - |
| a=rtpmap | **Not** | **P1** | コーデック識別 |
| a=fmtp | Not | P2 | パラメータ |
| a=sendrecv等 | Deferred | Deferred | Hold/Resume用 |

---

## 10. 優先度別ロードマップ

### 10.1 P0: UAS必須（即時対応）

| 項目 | RFC | 依存 | 備考 |
|------|-----|------|------|
| CANCEL 受信処理 | 3261 §9 | - | 487 応答 |
| DTMF トーン検出 | ITU-T Q.23 | - | Goertzel |
| E2E テスト拡充 | - | 上記2項目 | SIPp シナリオ |

### 10.2 P1: 相互接続性・RFC準拠

| 項目 | RFC | 依存 | 備考 |
|------|-----|------|------|
| OPTIONS 応答 | 3261 §11 | - | ケイパビリティ |
| RSeq ランダム化 | 3262 §3 | - | RFC準拠 |
| a=rtpmap パース | 8866 | - | コーデック識別 |
| RTCP SDES | 3550 §6.5 | - | CNAME必須 |
| 486 Busy Here | 3261 | - | 同時通話制限 |
| Timer G/H 厳密化 | 3261 §17.2.1 | - | トランザクション |

### 10.3 P2: 拡張

| 項目 | RFC | 備考 |
|------|-----|------|
| a=fmtp パース | 8866 | コーデックパラメータ |
| RFC 2833 DTMF | 2833/4733 | アウトオブバンド |
| RTCP BYE | 3550 | 終了通知 |
| 複数コーデック交渉 | 3264 | 相互接続性 |
| IPv6 対応 | 8866 | c=IN IP6 |

### 10.4 Deferred: 後工程

以下は P0〜P2 完了後に着手。Spec策定が必要な項目もあり。

| カテゴリ | 項目 | RFC | 必要な決定事項 |
|---------|------|-----|---------------|
| **認証** | Digest認証 | 3261 §22 | credentials設計 |
| **認証** | 401/407チャレンジ | 3261 | realm/qop方針 |
| **暗号化** | TLS | 3261 §26 | 証明書管理 |
| **暗号化** | SRTP | 3711 | キー交換方式 |
| **UAC** | INVITE送信 | 3261 §17.1.1 | 状態機械設計 |
| **UAC** | DNS SRV解決 | 3263 | resolverクレート |
| **Proxy** | 転送機能 | 3261 | フォーキング戦略 |
| **転送** | REFER | 3515 | NOTIFY送信 |

---

## 11. Acceptance Criteria (SIPp検証)

> **正本移行**: 受入条件（AC）の正本は [tests.md](tests.md) に移行しました（2025-12-27 確定、Refs Issue #7 CX-4）。
> 本セクションは参照用として残しますが、更新は tests.md で行ってください。

### AC-1: 基本着信フロー (実装済み)

**状態**: ✓ 動作確認済み

| # | シナリオ | 期待結果 |
|---|---------|---------|
| AC-1.1 | INVITE 受信 → 100/180/200 | 200 OK 受信 |
| AC-1.2 | ACK 受信 → セッション確立 | RTP 双方向 |
| AC-1.3 | BYE 受信 → 200 OK | 正常終了 |

---

### AC-2: 100rel/PRACK (実装済み)

**状態**: ✓ 動作確認済み

| # | シナリオ | 期待結果 |
|---|---------|---------|
| AC-2.1 | 183 Reliable送信 | 183 + RSeq 受信 |
| AC-2.2 | PRACK 送信 → 200 OK | 200 OK (PRACK) |
| AC-2.3 | 32秒タイムアウト | 504 受信 |

---

### AC-3: Session Timer (実装済み)

**状態**: ✓ 動作確認済み

| # | シナリオ | 期待結果 |
|---|---------|---------|
| AC-3.1 | Session-Expires 受信 | 200 OK + Session-Expires |
| AC-3.2 | Min-SE 下回り | 422 + Min-SE: 90 |

---

### AC-4: CANCEL 処理 (P0 - 実装対象)

**状態**: 未実装

| # | シナリオ | 期待結果 |
|---|---------|---------|
| AC-4.1 | CANCEL 受信 | 200 OK (CANCEL) |
| AC-4.2 | INVITE への応答 | 487 Request Terminated |

**SIPp シナリオ**: `test/sipp/sip/scenarios/cancel_uac.xml` (要作成)

---

### AC-5: DTMF トーン検出 (P0 - 実装対象)

**状態**: 未実装

| # | シナリオ | 期待結果 |
|---|---------|---------|
| AC-5.1 | DTMF "1" トーン受信 | SessionIn::Dtmf(1) 発火 |
| AC-5.2 | 全パターン (0-9,*,#) | 正常検出 |

**検証スクリプト**: `test/scripts/send_dtmf_tone.py` (要作成)

---

### AC-6: Digest 認証 (Deferred)

**状態**: Deferred - Spec策定後に実装

---

### AC-7: UAC 発信 (Deferred)

**状態**: Deferred - UAS完了後に着手

---

## 付録A: テスト要件

### A.1 SIPp シナリオ配置

```
test/sipp/sip/scenarios/
├── basic_uas.xml           # AC-1: 基本着信 ✓
├── basic_uas_100rel.xml    # AC-2: 100rel ✓
├── basic_uas_update.xml    # AC-3: Session Timer ✓
├── cancel_uac.xml          # AC-4: CANCEL (要作成)
└── ...
```

### A.2 相互接続テスト対象

| 製品 | 用途 | 優先度 |
|-----|------|-------|
| SIPp | 負荷・シナリオテスト | **必須** |
| FreeSWITCH | B2BUA連携 | P1 |
| Asterisk | 既存PBX連携 | P1 |

---

## 付録B: 参考資料

- [RFC 3261](../spec/rfc3261.txt) - SIP: Session Initiation Protocol
- [RFC 3262](../spec/rfc3262.txt) - Reliability of Provisional Responses (100rel)
- [RFC 3264](../spec/rfc3264.txt) - Offer/Answer Model with SDP
- [RFC 3550](../spec/rfc3550.txt) - RTP
- [RFC 4028](../spec/rfc4028.txt) - Session Timers
- [RFC 8866](../spec/rfc8866.txt) - SDP

---

## 変更履歴

| 日付 | バージョン | 変更内容 |
|------|-----------|---------|
| 2025-12-25 | 3.0 | UAS優先方針に改訂。認証/暗号化/UAC/ProxyをDeferredに変更 |
| 2025-12-25 | 2.0 | 仕様駆動形式に改訂。AC追加、成熟度%を廃止 |
| 2025-12-25 | 1.0 | 初版作成 |
