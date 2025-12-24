# Gap Analysis: 汎用SIPサーバー（Asterisk相当）を目指すための機能ギャップ

本文書は、現在の virtual-voicebot-backend の実装と、RFC仕様および Asterisk のような汎用SIPサーバーとして必要な機能との差分を分析します。

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
10. [汎用SIPサーバー必須機能](#10-汎用sipサーバー必須機能)
11. [優先度別実装ロードマップ](#11-優先度別実装ロードマップ)

---

## 1. エグゼクティブサマリー

### 現状評価

| カテゴリ | 現在の成熟度 | 汎用SIP要件 | ギャップ |
|---------|-------------|------------|---------|
| SIP UAS (着信) | 60% | 100% | **中** |
| SIP UAC (発信) | 0% | 100% | **致命的** |
| Proxy機能 | 0% | 100% | **致命的** |
| 認証 | 0% | 100% | **致命的** |
| トランスポート | 50% | 100% | **高** |
| RTP/RTCP | 40% | 100% | **高** |
| SDP/Offer-Answer | 30% | 100% | **高** |
| 追加機能 (DTMF等) | 0% | 100% | **高** |

### 総合評価

現在の実装は**ボイスボット専用のUAS**として機能しているが、汎用SIPサーバーとしては以下の致命的なギャップがある：

1. **UAC機能の完全欠如** - 発信ができない
2. **Proxy機能なし** - ルーティング/フォワードができない
3. **認証機構なし** - セキュリティが確保できない
4. **DNS解決なし** - SIP URI解決ができない

---

## 2. RFC 3261 (SIP Core) ギャップ

### 2.1 メソッド対応

| メソッド | RFC要件 | 現状 | ギャップ | 優先度 |
|---------|--------|------|---------|-------|
| INVITE | MUST (UAS/UAC) | UAS のみ | UAC未実装 | **P0** |
| ACK | MUST (UAS/UAC) | UAS のみ | UAC未実装 | **P0** |
| BYE | MUST (UAS/UAC) | UAS のみ | UAC未実装 | **P0** |
| CANCEL | MUST | 未実装 | 完全欠如 | **P0** |
| REGISTER | SHOULD | 200 OK即時返信 | 認証・バインディング管理なし | **P1** |
| OPTIONS | SHOULD | 未実装 | ケイパビリティ応答なし | **P2** |

### 2.2 トランザクション層

| 要件 | RFC参照 | 現状 | ギャップ |
|-----|---------|------|---------|
| INVITE Client Transaction | §17.1.1 | 未実装 | UAC全体が欠如 |
| Non-INVITE Client Transaction | §17.1.2 | 未実装 | UAC全体が欠如 |
| INVITE Server Transaction | §17.2.1 | 簡易実装 | Timer G/H の厳密な実装なし |
| Non-INVITE Server Transaction | §17.2.2 | 実装済み | Timer J相当は動作 |

**RFC 3261 §17.2.1 INVITE Server Transaction 状態遷移**:
```
現状の問題点:
- Timer G (再送間隔) の正確な実装なし
- Timer H (最終応答タイムアウト) の厳密な管理なし
- Timer I (ACK待機後のConfirmed→Terminated) は即時遷移で代用
```

### 2.3 トランスポート層

| 要件 | RFC参照 | 現状 | ギャップ |
|-----|---------|------|---------|
| UDP | MUST | 実装済み | - |
| TCP | MUST | 実装済み | - |
| TLS | MUST (§26.3.1) | 未実装 | **セキュリティ要件** |
| SCTP | MAY | 未実装 | 低優先度 |
| WebSocket (RFC 7118) | - | 未実装 | WebRTC連携に必要 |

### 2.4 ヘッダフィールド

| ヘッダ | 要件 | 現状 | ギャップ |
|-------|-----|------|---------|
| Via | 必須 | パース/生成済み | branch計算の厳密性要確認 |
| From/To | 必須 | パース/生成済み | tag生成は実装済み |
| Call-ID | 必須 | 実装済み | - |
| CSeq | 必須 | 実装済み | - |
| Max-Forwards | 必須 | パースのみ | デクリメント/転送なし |
| Contact | 必須 | 簡易実装 | URI完全パースなし |
| Record-Route/Route | Proxy必須 | 未実装 | Proxy機能欠如 |
| Authorization/WWW-Authenticate | 認証必須 | 未実装 | 認証欠如 |
| Proxy-Authorization/Proxy-Authenticate | Proxy認証 | 未実装 | Proxy欠如 |

### 2.5 レスポンスコード

| カテゴリ | 実装済み | 未実装（必須） |
|---------|---------|---------------|
| 1xx | 100, 180 | 181, 182, 183 |
| 2xx | 200 | 202 |
| 3xx | なし | 300, 301, 302, 305, 380 (リダイレクト必須) |
| 4xx | 400, 422, 481 | **401, 403, 404, 405, 406, 407, 408, 415, 420, 480, 486, 487, 488** |
| 5xx | 504 | 500, 501, 502, 503, 505, 513 |
| 6xx | なし | 600, 603, 604, 606 |

**致命的な欠如**: 401/407 (認証チャレンジ)、408 (タイムアウト)、486 (Busy)、487 (Terminated)

---

## 3. RFC 3262 (100rel/PRACK) ギャップ

### 現状実装

| 要件 | 現状 | 評価 |
|-----|------|------|
| RSeq ヘッダ生成 | 固定値 "1" | 初期値のランダム化必要 |
| PRACK受信処理 | RAck検証実装済み | 準拠 |
| 再送タイマ (T1開始, 指数バックオフ) | 500ms開始, 4秒上限 | T1=500ms でRFC準拠 |
| タイムアウト (64*T1 = 32秒) | 32秒で504返信 | 準拠 |
| 複数Reliable Provisional | 未対応 | RSeq連番管理必要 |

### ギャップ詳細

```
RFC 3262 §3 UAS Behavior:
"The value of the header field for the first reliable provisional
response in a transaction MUST be between 1 and 2**31 - 1.
It is RECOMMENDED that it be chosen uniformly in this range."

現状: 固定値 1 を使用
対応: 乱数生成による初期値設定が推奨
```

---

## 4. RFC 3311 (UPDATE) ギャップ

### 現状実装

| 要件 | 現状 | 評価 |
|-----|------|------|
| UPDATE受信 (UAS) | Session-Expires更新のみ | SDP offer/answer未対応 |
| UPDATE送信 (UAC) | 未実装 | セッション更新発信不可 |
| 491 Request Pending | 未実装 | offer衝突時の処理なし |
| Early Dialog対応 | 未確認 | テスト必要 |

### RFC要件との差分

```
RFC 3311 §5.2:
"If an UPDATE is received that contains an offer, and the UAS has
generated an offer (in an UPDATE, PRACK or INVITE) to which it has
not yet received an answer, the UAS MUST reject the UPDATE with a
491 response."

現状: 491応答未実装、offer状態管理なし
```

---

## 5. RFC 4028 (Session Timers) ギャップ

### 現状実装

| 要件 | 現状 | 評価 |
|-----|------|------|
| Session-Expires パース | 実装済み | 準拠 |
| Min-SE パース | 実装済み | 準拠 |
| refresher パラメータ | パース済み | 準拠 |
| 422応答 + Min-SE | 実装済み | 準拠 |
| re-INVITE送信 (refresher=uas時) | 未実装 | **リフレッシュ発信不可** |
| UPDATE送信 (refresher=uas時) | 未実装 | **リフレッシュ発信不可** |
| Supported: timer | 200 OK に付与 | 準拠 |

### 致命的ギャップ

```
RFC 4028 §10:
"If the refresher never gets a response to that session refresh
request, it sends a BYE to terminate the session."

現状: refresher=uas の場合、リフレッシュを送信できないため
      セッションがタイムアウトする可能性あり
```

---

## 6. RFC 3263 (DNS SRV/NAPTR) ギャップ

### 現状

**完全未実装**

### RFC要件

| 機能 | 要件 | 現状 |
|-----|------|------|
| NAPTR クエリ | SHOULD | 未実装 |
| SRV クエリ | MUST | 未実装 |
| A/AAAA クエリ | MUST | 未実装 |
| トランスポート選択 | MUST | 未実装 |
| フェイルオーバー | SHOULD | 未実装 |

### 影響

```
UAC機能（発信）には DNS 解決が必須:
- sip:user@domain.com → IP:port:transport への解決
- SRV による負荷分散・冗長化
- NAPTR によるトランスポート自動選択

現状は発信機能自体がないため未対応
```

---

## 7. RFC 3264 (Offer/Answer SDP) ギャップ

### 現状実装

| 要件 | 現状 | 評価 |
|-----|------|------|
| Offer受信 (INVITE) | c=/m= パースのみ | **最小限** |
| Answer生成 | 固定PCMU/8000 | **コーデックネゴシエーションなし** |
| Re-INVITE offer | 未対応 | セッション変更不可 |
| Hold/Resume (a=sendonly等) | 未対応 | 保留操作不可 |
| 複数メディアストリーム | 未対応 | audioのみ |

### SDPパース/生成の問題

```
現状のSDPパーサ (sip/mod.rs:135-159):
- c=IN IP4 のみ対応 (IPv6未対応)
- m=audio のみ対応 (video等未対応)
- rtpmap 属性未パース
- fmtp 属性未パース
- 複数コーデック提示時の選択ロジックなし

現状のSDP生成:
- 固定テンプレート（PCMU/8000のみ）
- 動的コーデック選択なし
```

### RFC 3264 必須機能

| 機能 | RFC参照 | 現状 |
|-----|---------|------|
| コーデック交渉 | §5, §6 | 未実装 |
| ポート0によるストリーム拒否 | §6 | 未実装 |
| a=inactive/sendonly/recvonly/sendrecv | §8.4 | 未実装 |
| IP変更 (re-INVITE) | §8.3.1 | 未実装 |

---

## 8. RFC 3550 (RTP/RTCP) ギャップ

### 8.1 RTP

| 要件 | 現状 | 評価 |
|-----|------|------|
| パケット構造 | 実装済み | 準拠 |
| SSRC管理 | 実装済み | 準拠 |
| Sequence番号 | 実装済み | 準拠 |
| Timestamp | 実装済み | 準拠 |
| CSRC (mixer) | 未実装 | 会議機能用 |
| Padding | 未実装 | 必要時に追加 |
| Extension Header | 未実装 | RTP拡張用 |

### 8.2 RTCP

| 要件 | 現状 | 評価 |
|-----|------|------|
| SR (Sender Report) | パース/生成実装 | 準拠 |
| RR (Receiver Report) | パース/生成実装 | 準拠 |
| SDES | 未実装 | **CNAME必須** |
| BYE | 未実装 | セッション終了通知 |
| APP | 未実装 | アプリ定義用 |
| 送信間隔アルゴリズム (§6.2) | 固定5秒 | 動的計算なし |

### 致命的ギャップ

```
RFC 3550 §6.5.1 CNAME:
"The CNAME identifier MUST be included in each compound RTCP packet."

現状: SDES未実装のためCNAME送信なし
      → 厳密なRTCP準拠には必須
```

### 8.3 追加RTP機能

| 機能 | 現状 | 必要性 |
|-----|------|--------|
| RFC 2833 DTMF | 未実装 | **汎用SIPでは必須** |
| RFC 4733 (DTMF更新) | 未実装 | 推奨 |
| RFC 3389 Comfort Noise | 未実装 | 推奨 |
| RFC 2198 Redundant Audio | 未実装 | オプション |

---

## 9. RFC 8866 (SDP) ギャップ

### SDPフィールド対応

| フィールド | 現状 | 必要性 |
|-----------|------|--------|
| v= (version) | 生成のみ | OK |
| o= (origin) | 生成のみ | セッションバージョン管理なし |
| s= (session name) | 生成のみ | OK |
| c= (connection) | パース/生成 | IPv6未対応 |
| t= (timing) | 生成のみ | OK |
| m= (media) | 簡易パース | 複数メディア未対応 |
| a=rtpmap | 未パース | **コーデック識別に必須** |
| a=fmtp | 未パース | コーデックパラメータ必須 |
| a=ptime | 未パース | パケット化間隔 |
| a=sendrecv等 | 未パース | Hold/Resume必須 |

### 必須パース拡張

```
現状のm=行パース (最初のPT値のみ取得):
m=audio 49170 RTP/AVP 0 8 101

RFC要件:
- 全PT値をパースしてコーデックリスト構築
- rtpmapとの照合でコーデック特定
- 優先順位に基づく選択

例:
m=audio 49170 RTP/AVP 0 8 101
a=rtpmap:0 PCMU/8000
a=rtpmap:8 PCMA/8000
a=rtpmap:101 telephone-event/8000
a=fmtp:101 0-16
```

---

## 10. 汎用SIPサーバー必須機能

### 10.1 Asterisk相当に必要な機能

| カテゴリ | 機能 | 現状 | 優先度 |
|---------|------|------|-------|
| **発信** | UAC INVITE/ACK/BYE | 未実装 | **P0** |
| **発信** | DNS SRV解決 | 未実装 | **P0** |
| **転送** | REFER | 未実装 | **P1** |
| **転送** | Blind Transfer | 未実装 | **P1** |
| **転送** | Attended Transfer | 未実装 | **P2** |
| **認証** | Digest認証 (MD5) | 未実装 | **P0** |
| **認証** | 401/407チャレンジ | 未実装 | **P0** |
| **登録** | REGISTER処理 | 200即時返信のみ | **P1** |
| **登録** | バインディング管理 | 未実装 | **P1** |
| **Proxy** | Request転送 | 未実装 | **P2** |
| **Proxy** | Record-Route/Route | 未実装 | **P2** |
| **セキュリティ** | TLS | 未実装 | **P1** |
| **セキュリティ** | SRTP | 未実装 | **P1** |
| **DTMF** | RFC 2833 | 未実装 | **P0** |
| **コーデック** | 複数コーデック交渉 | 未実装 | **P1** |
| **コーデック** | G.729, Opus等 | 未実装 | **P2** |
| **保留** | Hold/Resume | 未実装 | **P1** |
| **会議** | 3者以上会議 | 未実装 | **P2** |
| **録音** | MixMonitor相当 | 部分実装 | **P2** |

### 10.2 機能別の影響度

```
P0 (致命的 - 汎用SIPとして機能しない):
├── UAC機能 → 発信ができない
├── 認証 → セキュリティが確保できない
└── DTMF → IVR/音声ガイダンスが使えない

P1 (重要 - 基本機能が制限される):
├── TLS/SRTP → セキュア通信ができない
├── Hold/Resume → 保留操作ができない
├── コーデック交渉 → 相互接続性が低い
└── REGISTER管理 → 端末登録ができない

P2 (標準機能 - 完全なPBX機能に必要):
├── Proxy機能 → ルーティングができない
├── 転送 → コール転送ができない
├── 会議 → 会議通話ができない
└── 追加コーデック → 特殊用途に対応できない
```

---

## 11. 優先度別実装ロードマップ

### Phase 1: 最小限のUAC機能 (P0)

```
目標: 発信ができる状態にする

1. INVITE Client Transaction (RFC 3261 §17.1.1)
   - Timer A/B/D の実装
   - 状態遷移: Calling → Proceeding → Completed/Terminated

2. Non-INVITE Client Transaction (RFC 3261 §17.1.2)
   - Timer E/F/K の実装
   - BYE/CANCEL送信対応

3. DNS SRV解決 (RFC 3263)
   - SRV クエリによるtarget/port/priority取得
   - A/AAAA クエリへのフォールバック

4. CANCEL (RFC 3261 §9)
   - INVITE中断機能
```

### Phase 2: 認証とDTMF (P0)

```
目標: セキュアな通信とIVR対応

1. Digest認証 (RFC 3261 §22)
   - 401/407 チャレンジ生成
   - Authorization/Proxy-Authorization検証
   - nonce管理

2. RFC 2833 DTMF
   - RTPペイロードタイプ検出
   - DTMF イベントパケット生成/受信
   - duration/volume処理
```

### Phase 3: セキュリティとコーデック (P1)

```
目標: セキュア通信と相互接続性向上

1. TLS対応
   - SIP over TLS (SIPS URI)
   - 証明書検証

2. SRTP (RFC 3711)
   - キー交換 (SDES or DTLS-SRTP)
   - 暗号化/復号処理

3. コーデック交渉
   - rtpmap パース
   - 優先順位に基づく選択
   - 動的PT対応
```

### Phase 4: 完全なPBX機能 (P2)

```
目標: Asterisk相当の機能

1. REGISTER管理
   - バインディングDB
   - Expires管理
   - 複数Contact対応

2. 転送機能
   - REFER (RFC 3515)
   - Replaces (RFC 3891)

3. 会議機能
   - RTPミキシング
   - CSRC管理

4. Proxy機能
   - Record-Route/Route処理
   - フォーキング
```

---

## 付録A: テスト要件

### A.1 相互接続テスト対象

| 製品 | 用途 | 優先度 |
|-----|------|-------|
| SIPp | 負荷・シナリオテスト | **必須** |
| FreeSWITCH | B2BUA連携 | 高 |
| Asterisk | 既存PBX連携 | 高 |
| Ohmybox | PSTN接続 | 中 |
| Ohmybox | クラウドPBX | 中 |

### A.2 RFC準拠テスト

- RFC 4475 (SIP Torture Tests)
- RFC 3261 Appendix B (State Machines)

---

## 付録B: 参考資料

- [RFC 3261](../spec/rfc3261.txt) - SIP: Session Initiation Protocol
- [RFC 3262](../spec/rfc3262.txt) - Reliability of Provisional Responses (100rel)
- [RFC 3263](../spec/rfc3263.txt) - Locating SIP Servers (DNS)
- [RFC 3264](../spec/rfc3264.txt) - Offer/Answer Model with SDP
- [RFC 3311](../spec/rfc3311.txt) - UPDATE Method
- [RFC 3550](../spec/rfc3550.txt) - RTP
- [RFC 4028](../spec/rfc4028.txt) - Session Timers
- [RFC 8866](../spec/rfc8866.txt) - SDP

---

*文書バージョン: 1.0*
*作成日: 2025-12-25*
*次回レビュー: 実装進捗に応じて更新*
