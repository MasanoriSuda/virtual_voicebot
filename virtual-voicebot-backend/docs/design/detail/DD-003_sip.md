# SIP モジュール詳細設計 (`src/protocol/sip`)

## 1. 目的・スコープ

- 目的:
  - RFC 3261 をベースにした UAS 側の SIP プロトコル処理を担当する。
  - メッセージのパース/ビルドと、トランザクション状態機械を提供する。
- スコープ:
  - UAS としての INVITE/ACK/BYE(＋その他リクエスト)の処理
  - RFC 3262 (100rel/PRACK)、RFC 3311 (UPDATE)、RFC 4028 (Session Timers) の **プロトコル部分**
- 非スコープ:
  - ダイアログ単位の業務ロジック（通話をどう扱うか）は `protocol/session` に委譲
  - AI 対話ロジック、RTP、ASR/LLM/TTS は扱わない

## 2. 依存関係

- 依存するモジュール:
  - `protocol::transport::packet` : 生の SIP テキスト送受信
  - `protocol::session` : SIP イベントを渡す先（通話セッション管理）
  - `shared::entities` : 共通エンティティ（CallId等）
- 依存されるモジュール:
  - `protocol::session` からレスポンス/リクエスト送信指示を受ける

### 優先ルール

- `docs/design.md` に記載された責務を優先し、矛盾する場合はそちらを正とする。
- 現行コードがこれと違う場合は、コードを修正する前提で TODO に載せる。

## 3. 主な責務

1. メッセージ表現
   - SIP リクエスト/レスポンスの構造体表現（メソッド、ステータス、ヘッダ、ボディ）
   - ヘッダ単位の内部表現（Via/From/To/Call-ID/CSeq/Contact/Require/Supported 等）

2. パーサ / ビルダ
   - テキスト → 構造体（リクエスト/レスポンス）
   - 構造体 → テキスト
   - 最低限の妥当性チェック（必須ヘッダ、フォーマット）

3. トランザクション管理 (RFC 3261)
   - INVITE サーバトランザクション状態機械
   - 非 INVITE サーバトランザクション状態機械
   - 再送制御とトランザクションタイマ (Timer A/B/E/F/G/H/I/J)

4. RFC 拡張のプロトコル処理
   - 3262: 100rel / PRACK（RSeq/RAck の管理）
   - 3311: UPDATE メソッドの扱い（トランザクションとして）
   - 4028: Session-Expires/Min-SE/refresher の解析・ヘッダ組み立て

## 4. トランザクション詳細設計（UAS）

### 4.1 INVITE サーバトランザクション
- RFC 範囲: RFC 3261 17.2.1（UDP/UAS）。MVP は UDP のみ。
  - INVITE に `Supported/Require: 100rel` がある場合は 180 に `Require: 100rel` と `RSeq: 1` を付与する。
  - PRACK は受信時に 200 OK を返す（RAck の検証/紐付けは未実装）。
- 状態: `Proceeding` / `Completed` / `Confirmed` / `Terminated`
- 主なイベント:
  - 受信: `INVITE`（新規/再送）、`ACK`
  - 送信: 1xx（100/180/183）、2xx、3xx–6xx
  - タイマ: Timer G/H/I 発火
- 状態 × イベント → 次状態/アクション（MVP で扱う経路に絞る）

| 現在 | イベント | 次状態 | アクション |
| --- | --- | --- | --- |
| (新規) | INVITE 受信 | Proceeding | 100/180 を送信（任意） |
| Proceeding | 再送 INVITE | Proceeding | 最新の 1xx を再送 |
| Proceeding | 2xx 送信 | Terminated | 2xx 送信。ACK 到着まで 2xx 再送を UAS コアで管理（トランザクションは即終了） |
| Proceeding | 3xx–6xx 送信 | Completed | 3xx–6xx 送信、Timer G/H 開始 |
| Completed | 再送 INVITE | Completed | 直近の最終応答を再送 |
| Completed | Timer G 発火 | Completed | 最終応答を再送（T1 倍増、上限 T2） |
| Completed | Timer H 発火 | Terminated | TransactionTimeout を session へ通知 |
| Completed | ACK 受信 | Confirmed | Timer I 開始、Timer G/H 停止 |
| Confirmed | Timer I 発火 | Terminated | 終了 |

### 4.2 非 INVITE サーバトランザクション
- RFC 範囲: RFC 3261 17.2.2（UDP/UAS）。
- 状態: `Trying` / `Proceeding` / `Completed` / `Terminated`
- 主なイベント:
  - 受信: 非 INVITE リクエスト（BYE/CANCEL/OPTIONS 等）、再送リクエスト
  - 送信: 1xx、最終応答（2xx–6xx）
  - タイマ: Timer J 発火
- 状態 × イベント → 次状態/アクション（MVP パターン）

| 現在 | イベント | 次状態 | アクション |
| --- | --- | --- | --- |
| (新規) | 非 INVITE 受信 | Trying | 100 Trying（任意） |
| Trying | 1xx 送信 | Proceeding | 1xx 送信 |
| Proceeding | 再送リクエスト | Proceeding | 最新の 1xx を再送 |
| Trying/Proceeding | 最終応答 2xx–6xx 送信 | Completed | 最終応答送信、Timer J 開始 |
| Completed | 再送リクエスト | Completed | 最終応答を再送 |
| Completed | Timer J 発火 | Terminated | 終了 |

## 5. protocol/session モジュールとのインタフェース

### 5.1 protocol/sip → protocol/session（入力イベント）

- `IncomingInvite`
  - フィールド例: Call-ID, From/To, CSeq, SDP, 送信元アドレス 等
- `IncomingAck`
- `IncomingBye`
- `IncomingCancel`
- `IncomingUpdate`
- `TransactionTimeout`
  - どのトランザクション/ダイアログに紐付くタイマかを示す

※ここでは「イベントの名前と意味」だけを書き、具体的な型定義はコード側に任せる。

### 5.2 protocol/session → protocol/sip（出力アクション）

- `SendProvisionalResponse`
  - 100/180/183 などを送る指示
- `SendFinalResponse`
  - 200 OK / 4xx など
- `SendPrack`
- `SendUpdate`
- （必要なら）`SendBye` などの UAS 発呼リクエスト

各アクションで必要な情報（ステータスコード、理由句、ヘッダ、SDP 等）を箇条書きにしておく。

## 6. タイマと送信キュー

### 6.1 トランザクションタイマ（UAS で使うもの）
- 使用するもの: Timer G/H/I（INVITE）と Timer J（非 INVITE）。
- 使用しないもの（MVP/UAS）: A/B/E/F/D（UAC 側）、100rel 再送タイマ（PRACK 未対応のため）。
- セット/開始:
  - Timer G/H: INVITE で 3xx–6xx を送信したとき `sip` トランザクションが開始。
  - Timer I: INVITE の ACK 受信で Confirmed 遷移時に開始。
  - Timer J: 非 INVITE の最終応答送信時に開始。
- 停止:
  - Timer G/H: ACK 受信で停止。
  - Timer I/J: 発火で Terminated へ遷移し停止。
- 発火時の通知:
  - Timer H/J 発火時に `TransactionTimeout(call_id, tx_type, method)` を `protocol/sip → protocol/session` へ通知（セッション側で通話維持/終了を判断）。

### 6.2 送信キュー I/F（protocol/sip → protocol/transport）
- 目的: トランザクションロジックは「構造化メッセージ＋宛先」をキューへ渡すだけとし、テキスト生成と送信は transport に委譲する。
- キューに載せる情報案:
  - 宛先: 受信元アドレスを逆向きに使うか、明示的な送信先 SocketAddr
  - 識別子: transaction ID（Via branch + CSeq）、dialog 情報（Call-ID + From/To tag）
  - 種別: レスポンスコード＋理由句、またはリクエストメソッド
  - ヘッダ: 必須ヘッダ（Via/To/From/Call-ID/CSeq/Contact/Content-Type 等）、必要に応じて Require/Supported/Record-Route
  - ボディ: SDP などの payload（長さ情報含む）
- sip 側アクション: session からの `SendProvisionalResponse`/`SendFinalResponse` 等を構造体で受け取り、送信キューへ push。再送もキュー経由で transport に依頼する。

## 7. MVP と拡張範囲

- MVP でサポートする:
  - UDP 上の INVITE/ACK/BYE のみ
  - 100/180/200 のみ（INVITE が 100rel 対応なら 180 に Require/RSeq を付与、PRACK は受信時に 200 OK）
- NEXT で追加する:
  - PRACK の RAck 検証/紐付け
  - 100rel の再送タイマ/再送制御
  - UPDATE
  - Session Timer (4028)
