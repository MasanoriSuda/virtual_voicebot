# SIP モジュール詳細設計 (`src/sip`)

## 1. 目的・スコープ

- 目的:
  - RFC 3261 をベースにした UAS 側の SIP プロトコル処理を担当する。
  - メッセージのパース/ビルドと、トランザクション状態機械を提供する。
- スコープ:
  - UAS としての INVITE/ACK/BYE(＋その他リクエスト)の処理
  - RFC 3262 (100rel/PRACK)、RFC 3311 (UPDATE)、RFC 4028 (Session Timers) の **プロトコル部分**
- 非スコープ:
  - ダイアログ単位の業務ロジック（通話をどう扱うか）は `session` に委譲
  - AI 対話ロジック、RTP、ASR/LLM/TTS は扱わない

## 2. 依存関係

- 依存するモジュール:
  - `transport::packet` : 生の SIP テキスト送受信
  - `session` : SIP イベントを渡す先（通話セッション管理）
- 依存されるモジュール:
  - `session` からレスポンス/リクエスト送信指示を受ける

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

## 4. 状態機械

### 4.1 INVITE サーバトランザクション

- 状態:
  - `Proceeding`
  - `Completed`
  - `Confirmed`
  - `Terminated`
- イベント例:
  - 受信: `INVITE`, `ACK`
  - 送信: 1xx, 2xx, 3xx–6xx
  - タイマ: Timer G/H/I の発火
- 状態遷移表:
  - 後で「イベント × 現在状態 → 次状態/アクション」を表にする（TODO）

※ここは簡単な表でも、箇条書きでも OK。  
　実装前に「どのパターンをサポートするか（MVP範囲）」を書いておく。

### 4.2 非 INVITE サーバトランザクション

- 状態:
  - `Trying`
  - `Proceeding`
  - `Completed`
  - `Terminated`
- 同様にイベントと遷移を整理する。

## 5. session モジュールとのインタフェース

### 5.1 sip → session（入力イベント）

- `IncomingInvite`
  - フィールド例: Call-ID, From/To, CSeq, SDP, 送信元アドレス 等
- `IncomingAck`
- `IncomingBye`
- `IncomingCancel`
- `IncomingUpdate`
- `TransactionTimeout`
  - どのトランザクション/ダイアログに紐付くタイマかを示す

※ここでは「イベントの名前と意味」だけを書き、具体的な型定義はコード側に任せる。

### 5.2 session → sip（出力アクション）

- `SendProvisionalResponse`
  - 100/180/183 などを送る指示
- `SendFinalResponse`
  - 200 OK / 4xx など
- `SendPrack`
- `SendUpdate`
- （必要なら）`SendBye` などの UAS 発呼リクエスト

各アクションで必要な情報（ステータスコード、理由句、ヘッダ、SDP 等）を箇条書きにしておく。

## 6. タイマとエラー処理

- 管理するタイマ:
  - INVITE トランザクション用 Timer A/B/D/E/F/G/H/I/J
  - 100rel 関連の再送タイマ（必要なら）
- エラー時の扱い:
  - パース不能メッセージ → ログ + 400 Bad Request or 破棄（ポリシーを書く）
  - タイムアウト → `TransactionTimeout` イベントで session に通知（その後どうするかは session 側）

## 7. MVP と拡張範囲

- MVP でサポートする:
  - UDP 上の INVITE/ACK/BYE のみ
  - 100/180/200 のみ（PRACK/UPDATE/Session Timer は無効）
- NEXT で追加する:
  - 100rel/PRACK
  - UPDATE
  - Session Timer (4028)
