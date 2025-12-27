# uas-voice-bot: Design Document

## 1. 目的と概要

`uas-voice-bot` は、Rust で実装された SIP UAS ベースの音声対話ボットです。

- SIP UAS として INVITE を受け、RTP メディアセッションを確立する
- UAC の音声を RTP で受信し、音声認識（ASR）でテキスト化する
- LLM にテキストを渡して応答を生成する
- 応答テキストを TTS で音声化し、RTP で UAC に返す
- 必要に応じて SIP レベルの制御（切断、保留など）を行う

本ドキュメントでは、**責務分離**と**モジュール構成**を明確にし、実装・改修時に迷いが出ないことを目的とします。

---

## 2. 参照RFC とスコープ

### 2.1 参照RFC

#### Signaling (SIP)
- **RFC 3261**: SIP: Session Initiation Protocol
- **RFC 3262**: Reliability of Provisional Responses (100rel/PRACK)
- **RFC 3311**: The Session Initiation Protocol (SIP) UPDATE Method
- **RFC 4028**: Session Timers in the Session Initiation Protocol (SIP)
- **RFC 3263**: Session Initiation Protocol (SIP): Locating SIP Servers（DNSによるSIPサーバ探索）

#### Media / SDP
- **RFC 3264**: An Offer/Answer Model with the Session Description Protocol (SDP)（SDP Offer/Answer）
- **RFC 8866**: SDP: Session Description Protocol（SDP本体。RFC 4566 を obsolete）
- **RFC 3550**: RTP: A Transport Protocol for Real-Time Applications（RTP/RTCP）


### 2.2 段階的スコープ

**MVP（最初の実装）では以下に絞る：**

- RFC 3261 の基本的な呼制御
  - INVITE / 100 Trying / 180 Ringing / 200 OK / ACK / BYE
  - シンプルな SDP オファー/アンサー
- 単一通話・単一ダイアログ（B2BUA ではなくシンプルな UAS として）

**拡張段階：**

- RFC 3262: 100rel / PRACK 対応
- RFC 3311: UPDATE 対応
- RFC 4028: Session Timer 対応
- 複数セッション・高負荷環境でのスケール

### 2.3 未決事項（今後決める）

- UASのみで運用する前提で良いか、Registrar/Proxy機能を将来含めるか（RFC3261 10/16）
- SIPS/TLS を運用要件として想定するか（RFC3261 12.1.1, 18.2）
- Forking を扱う必要があるか（複数2xx/複数ダイアログ）（RFC3261 13.1）
- 認証（401/407）をスコープに入れるか（RFC3261 8.2/22系）
- OPTIONSの「INVITEと同等の可否」判定に使うアプリ状態（Busy等）の定義（RFC3261 11.2）

---

## 3. ディレクトリ構成

プロジェクト全体の構成は以下とする。
※以下は設計上の目安（将来的な分割を含む）で、MVPでは `session/session.rs` や `app/mod.rs`、`http/mod.rs` に集約される場合がある。必要なタイミングで段階的に分割する。

```text
virtual-voicebot-backend/
├─ Cargo.toml
├─ README.md              # 概要・ビルド/実行方法
├─ docs/
│  ├─ design.md           # 本ドキュメント（アーキテクチャ設計 / 神様）
│  ├─ contract.md         # Frontend ↔ Backend API 契約（MVP）
│  ├─ recording.md        # 録音保存・配信の設計（MVP）
│  ├─ sip.md              # （必要に応じて）SIP 詳細設計
│  ├─ rtp.md              # （必要に応じて）RTP/メディア詳細設計
│  └─ voice_bot_flow.md   # （必要に応じて）ASR/LLM/TTS 連携詳細
├─ storage/
│  └─ recordings/         # 録音の実体（callId配下）
│     └─ <callId>/
│        ├─ mixed.wav
│        └─ meta.json
└─ src/
   ├─ main.rs             # エントリポイント
   ├─ lib.rs              # 主要モジュール re-export
   ├─ config.rs           # 設定関連
   ├─ error.rs            # 共通エラー型
   ├─ logging.rs          # ログ初期化
   │
   ├─ transport/
   │  ├─ README.md
   │  ├─ mod.rs
   │  └─ packet.rs        # 生パケット I/O
   │
   ├─ sip/
   │  ├─ README.md
   │  ├─ mod.rs
   │  ├─ message.rs
   │  ├─ parser.rs
   │  ├─ builder.rs
   │  ├─ transaction.rs
   │  ├─ dialog.rs
   │  └─ timers.rs
   │
   ├─ rtp/
   │  ├─ README.md
   │  ├─ mod.rs
   │  ├─ packet.rs
   │  ├─ codec.rs
   │  ├─ stream.rs
   │  └─ jitter_buffer.rs
   │
   ├─ session/
   │  ├─ README.md
   │  ├─ mod.rs
   │  ├─ state.rs
   │  ├─ manager.rs
   │  └─ events.rs
   │
   ├─ app/
   │  ├─ README.md
   │  ├─ mod.rs
   │  ├─ dialog.rs
   │  ├─ policy.rs
   │  └─ events.rs
   │
   ├─ ai/
   │  ├─ README.md
   │  ├─ mod.rs
   │  ├─ asr.rs
   │  ├─ llm.rs
   │  └─ tts.rs
   │
   ├─ http/               # Frontend向け参照/録音配信（MVP: 録音配信のみ）
   │  ├─ README.md
   │  ├─ mod.rs
   │  ├─ routes.rs        # 参照系ルーティング（将来/必要時）
   │  └─ sse.rs           # SSEストリーム（将来/必要時、backend→frontend）
   │
   ├─ media/              # 録音生成・管理（保存/メタデータ）
   │  ├─ README.md
   │  ├─ mod.rs
   │  ├─ recorder.rs      # 録音の書き出し（mixed等）
   │  └─ meta.rs          # meta.json管理
   │
   └─ utils/
      ├─ mod.rs
      ├─ id.rs
      └─ time.rs

```

## 4. レイヤ構成と全体アーキテクチャ

### 4.1 レイヤ構造（論理）

上から下に向かって抽象度が下がる。

アプリケーション / AI レイヤ

app：対話フロー・業務ロジック

ai::{asr, llm, tts}：外部 AI サービスのクライアント

セッションレイヤ

session：SIP ダイアログ＋RTP セッションの統合管理

プロトコルレイヤ

sip：SIP メッセージ、トランザクション、タイマ

rtp：RTP/RTCP、コーデック、ストリーム

トランスポートレイヤ

transport::packet：UDP/TCP ソケット、生パケット I/O（送信指示型 TransportSendRequest を提供し、sip/session 型に依存しない）

テキスト図：

```text
+-------------------------+
|      app (dialog)       |  AI 対話フロー・業務ロジック
+-------------------------+
|        ai (asr/llm/tts) |  外部 AI サービスクライアント
+-------------------------+
|         session         |  呼セッション管理 (SIP+RTP)
+-------------------------+
| sip (SIP)   |   rtp     |  プロトコル処理
+-------------------------+
|   transport::packet     |  ネットワーク I/O
+-------------------------+
|        Network          |
+-------------------------+
```

### 4.2 依存関係のルール

下方向への依存のみ許可（上位レイヤは下位レイヤを知るが、その逆は知らない）

特に禁止したい依存:

ai から sip / rtp / transport への直接依存

session から ai への直接依存（必ず app を経由）

モジュール間は基本的に「イベント/メッセージ」とチャンネルで接続する

#### 4.2.1 依存関係（上下方向と禁止事項の明文化）

- 下方向のみ参照可: app → ai / session → (sip, rtp) → transport。逆方向の直接参照は禁止。
- app から transport/sip/rtp への直接依存は禁止（必ず session を経由）。
- ai から sip/rtp/transport への直接依存は禁止（必ず app を経由）。
- session から ai への直接依存は禁止（必ず app を経由）。
- http から sip/rtp/transport への直接依存は禁止（プロトコル処理を混ぜない）。
- session/app/ai から http への直接依存は禁止（HTTPに引きずられない）。
- media は録音生成・保存のみに責務を限定し、http は配信に限定する（生成と配信を混ぜない）。


#### 4.2.2 依存関係の簡易図

```text
app  ──→ session ──→ sip ──→ transport
  │         │         │
  │         └─→ rtp ──┘
  └─→ ai (asr/llm/tts)
```

※矢印は「知ってよい方向（依存の向き）」を示す。逆方向はイベント/メッセージでのみ疎結合に通知する。

### 4.3 Frontend 連携（HTTP/SSE）レイヤ

本プロジェクトは SIP/RTP（Zoiper）向けのネットワーク入口に加えて、
Frontend 向けの HTTP 入口（録音配信）を持つ。SSE は backend→frontend の通知用途として将来導入する前提とする。

- SIP/RTP 経路: `transport` → (`sip` / `rtp`) → `session` → `app` → `ai`
- Frontend 経路（MVP）: backend → frontend（Call Events の push/emit。frontend は受信して取り込む。詳細は `docs/contract.md`）
- 録音再生: frontend → backend（`Call.recordingUrl` の GET）

MVP の http は `Call.recordingUrl` の GET（録音配信）のみを必須とする。
参照系 REST API / SSE は将来の拡張であり、追加する場合は `docs/contract.md` の合意を正とする。

Frontend ↔ Backend の API 契約は `docs/contract.md` を正とする。
録音の保存と配信に関する内部設計は `docs/recording.md` を正とする。

※ 重要: `http` は通話制御や SIP/RTP の内部状態機械には立ち入らず、
参照/配信用に整形されたデータを提供することに徹する。Frontend から通話制御や入力を行う契約は持たない。

Read Model 方針（MVP）：
- Call read model store は未実装とする（`recordingUrl` は純関数で算出可能なため）。
- 将来の参照API/SSE導入時に、http から参照する専用ストアを追加する。
- app/session はストアを直接持たず、イベントで更新する。

## 5. 各モジュールの責務

### 5.1 transport モジュール (transport::packet)

目的：
SIP/RTP を含むすべてのネットワークトラフィックに対する、生パケット送受信の専任レイヤ。

責務（やること）：

UDP/TCP ソケットの初期化・bind・listen

受信ループを持ち、ネットワークから生パケットを受け取る

受信パケットを「プロトコル種別 + アドレス情報 + バイト列」で上位に通知

上位レイヤからの送信依頼（宛先アドレス + バイト列）をネットワークに送る

非責務（やらないこと）：

SIP/RTP のパース、Call-ID や SSRC などアプリケーションレベルの概念を持たない

セッションやトランザクション状態を持たない

RFC レベルのタイマ・再送制御を行わない

### 5.2 sip モジュール

目的：
SIP プロトコルのテキスト/構造化表現、トランザクション、RFC 拡張のロジックを担当する。

責務：

SIP メッセージの表現

リクエスト/レスポンス、ヘッダ、ボディ（SDP 等）の構造体

パーサ / ビルダ

テキスト ⇔ 構造体の変換

基本的な妥当性チェック（文法、必須ヘッダなど）

トランザクション処理（主に UAS 側）

INVITE / 非 INVITE のトランザクション状態機械

再送制御

トランザクションタイマ（Timer A/B/E/F/G/H/I/J）

RFC 拡張のプロトコルロジック

RFC 3262: 100rel / PRACK（RSeq / RAck、信頼性付き 1xx）

RFC 3311: UPDATE メソッド

RFC 4028: Session-Expires / Min-SE / refresher パラメータ処理

session との役割分担：

sip → session

受信した SIP メッセージをイベントとして通知（例：IncomingInvite, IncomingAck, IncomingBye）

トランザクションタイムアウト発生などのイベント

session → sip

返したいレスポンス内容（ステータスコード、ヘッダ、SDP 等）を「構造体」として渡す

sip 側が適切な Via/To/From/CSeq 等を補完し、テキスト化・送信依頼を行う

非責務：

「どのステータスコードを返すか」「いつ 180/183/200 を返すか」などのビジネス判断

セッション固有の業務ロジック（それは session / app の仕事）

補足（トランザクション詳細の要約、詳細は docs/sip.md）：

- INVITE サーバトランザクション: Proceeding → Completed → Confirmed → Terminated。2xx 送信時はトランザクション自体は Terminated としつつ、ACK 到着まで 2xx を再送する（UASコアで管理）。3xx–6xx 送信時は Timer G/H、ACK 受信で Confirmed→Timer I→Terminated。
- 非 INVITE サーバトランザクション: Trying → Proceeding → Completed → Terminated。最終応答送信で Timer J、発火で Terminated。
- UAS で使うタイマは G/H/I（INVITE）と J（非 INVITE）。発火時は TransactionTimeout を session へ通知。
- sip→transport 送信は「構造化メッセージ＋宛先」を送信キューへ渡し、テキスト化と送信は transport 層が担当。

### 5.3 rtp モジュール

目的：
RTP/RTCP パケット処理と、音声ストリームの管理を担当する。

責務：

RTP/RTCP ヘッダの構造化（SSRC, Seq, Timestamp, PayloadType など）

バイト列 ⇔ RTP/RTCP 構造体の変換

ストリーム管理

SSRC / ポート単位の送受信ストリーム

Seq/Timestamp の管理

再生順序の整列、必要に応じたジッタバッファ

コーデック抽象

最低限の G.711 (PCMU/PCMA) など

PCM ⇔ RTP ペイロード変換

上位レイヤとのインタフェース

受信側：PCM フレームを session に渡す（session → app → ai::asr の経路で ASR に到達）

送信側：session から受け取った PCM フレームを RTP に載せて送信（app → session → rtp の経路で TTS 出力が到達）

**注意**: rtp から ai への直接依存は禁止（必ず session → app を経由）

非責務：

音声認識（ASR）や音声合成（TTS）を直接扱わない

通話終了などのビジネス判断をしない（タイムアウト検知は行っても、判断は上位）

補足（ストリーム管理の要約、詳細は docs/rtp.md）：

- SSRC/Seq/Timestamp は rtp 内で生成・管理（1セッション1 SSRC、Seq/Timestamp は乱数初期化＋単純インクリメント）。
- 簡易ジッタ対応として約100msの小バッファで整列し、古い/遅延パケットは破棄。PCMのみを asr/tts に受け渡す。
- RTCP は SR/RR のインタフェースのみ定義（実装は後続スプリント）。品質通知は将来的にイベント化。
- 上位（session/app/ai）は SSRC/Seq/Timestamp/ジッタを意識せず、PCMイベントだけ扱う。

補足（app/ai I/F の要約、詳細は docs/voice_bot_flow.md）：

- ASR/TTS はチャネルベースのストリーミング、LLM は 1リクエスト1レスポンスの Future を基本とするハイブリッド。
- app→ai: AsrInputPcm / LlmRequest / TtsRequest、ai→app: AsrResult/AsrError / LlmResponse/LlmError / TtsPcmChunk/TtsError。
- 必須フィールドは session_id/stream_id とテキスト/PCM、終端フラグや理由（エラー）を含め、エラーポリシーに従い謝罪継続・連続失敗で終了が判断できる。

### 5.4 session モジュール

目的：
SIP ダイアログと RTP セッションをまとめた「1通話」のライフサイクル管理を行う。

責務：

セッション（呼）状態管理

early / confirmed / terminated など

Call-ID, From/To タグ, CSeq 等、ダイアログ識別情報の保持

SDP に基づくメディア設定

ローカル/リモート IP/Port

コーデック、direction (sendrecv/sendonly/recvonly)

rtp モジュールへの設定の伝達

UAS としてのコール制御

INVITE をトリガにセッションを生成

100/180/183/200 等のレスポンスを出すタイミングは app の方針に従う

Session Timer（RFC 4028）対応（拡張段階）

refresher の管理

Timer の起動・キャンセル

Timer 切れ時の動作（BYE 送信など）のトリガ

app との役割分担：

session → app

通話開始/終了/エラー等のイベント通知

app → session

「ここで応答を 180 にして」

「ここで 200 + SDP を返す」

「ここで BYE を送って通話を切る」
といった高レベルの指示

非責務：

音声認識/LLM/TTS の呼び出し

ユーザ発話の内容に応じた業務ロジックの判断（これは app の仕事）

補足（session 詳細の要約、詳細は docs/session.md）：
- manager が call_id をキーにセッション生成/破棄/検索を一元管理し、タイマハンドルや rtp 設定を保持する。
- Session Timer/keepalive を保持し、発火時に app へ `SessionTimeout` を通知。必要に応じて `SessionOut::StopRtpTx` / `SendSipBye200` を発火する。
- rtp には送信開始/停止と送信先設定のみ伝え、AI/対話ロジックは持たない。

### 5.5 app モジュール（対話アプリケーション）

目的：
1通話単位の対話状態と業務ロジックを管理し、ASR/LLM/TTS をオーケストレーションする。

責務：

会話状態管理（dialog）

ユーザ発話履歴

LLM に渡すコンテキスト

進行フェーズ（認証中、案内中、終了フェーズなど）

イベントフロー

ASR から認識結果（テキスト）を受け取る

必要なコンテキストを含めて LLM に問い合わせる

LLM の応答を解釈し、

UAC に返すテキスト（発話内容）

セッション操作（切断、保留など）
を決定

発話内容を TTS に渡し、生成された PCM を rtp へ送るよう依頼

業務ポリシー (policy)

対話フローの分岐条件（「○○と言われたら終了」など）

LLM 応答の post-process（NG ワード除去など）

依存関係：

下方向

session：通話開始/終了の制御

ai::asr / ai::llm / ai::tts：対話に必要な AI 機能

非責務：

SIP/RTP プロトコルの詳細（ヘッダ、トランザクション等）を扱わない

生パケットやソケットを触らない

### 5.6 ai モジュール（ASR / LLM / TTS クライアント）

目的：
ASR / LLM / TTS など外部 AI サービスへのアクセスをカプセル化し、app からはシンプルな API として見えるようにする。

責務：

asr：

PCM フレームを受け取り、ストリーミング/チャンク単位で音声認識を行う

発話単位でテキスト結果を app に返す

llm：

テキスト + コンテキストを受け取り、応答テキスト/アクションを返す

LLM のプロンプト構築は基本 app 側で行い、llm は純粋なクライアントに留めてもよい

tts：

テキストを受け取り、PCM 音声フレームをストリーミングで返す

非責務：

SIP/RTP/セッションを直接操作しない

通話の開始・終了を決定しない（app / session 側の責務）

### 5.7 http モジュール（録音配信/参照API）

目的：
Frontend 向けに録音再生を提供し、参照系 API / SSE は将来/必要時に提供する。

責務：
- `docs/contract.md` の `recordingUrl` が参照する録音配信（MVP 必須）
- （将来/必要時）参照系の REST API を read-only で提供
- （将来/必要時）backend→frontend の通知に限定した SSE を提供
- （推奨）ブラウザ再生のための Range 対応（可能な範囲で）

非責務：
- Frontend からの通話制御/入力の受け口を持たない（契約外）
- SIP/RTP のパースや RFC ロジックを扱わない（sip/rtp の責務）
- セッション状態機械やコール制御判断をしない（session/app の責務）
- 録音ファイルの生成・ミックス・エンコードをしない（media の責務）

MVP 方針：
- http は録音配信（recordingUrl の GET）のみ提供する。
- 参照系 REST API / SSE は実装しない（必要になった時点で contract/design を更新して追加する）。

### 5.8 media モジュール（録音生成・保存）

目的：
通話中の音声データ（PCM等）を録音として保存し、Frontend で再生できる形を用意する。

責務：
- 録音の開始/停止のライフサイクルを受け取り、録音ファイルを生成する
- `storage/recordings/<callId>/` 配下に録音実体を保存する（例: mixed.wav）
- 録音メタデータ（meta.json）を生成・更新する
- 録音の 0秒基準（recording timeline）を確立し、将来 `Utterance.startSec/endSec` と同期できるようにする
- 将来拡張: mixed/caller/bot の複数トラック、mp3/opus などへのエンコード、外部ストレージ（S3 等）へのアップロード

非責務：
- HTTP 配信をしない（http の責務）
- SIP/RTP のプロトコル判断をしない（sip/rtp の責務）
- 通話の維持/終了の最終判断をしない（session/app の責務）
- AI 呼び出しをしない（app/ai の責務）

詳細：
- 録音設計は `docs/recording.md` を正とする。


## 6. モジュール間インタフェース（イベント指向）

実装上は、モジュール間のやりとりは 構造体/enum ベースのイベント と 非同期チャネル を基本とする。
※Frontend 連携（backend→frontend の ingest/push）は外部経路（contract）であり、本章の内部モジュールイベント（sip/rtp/session/app/ai）には含めない。

### 6.1 主なイベントの流れ（例）

transport::packet → sip / rtp

RawPacketEvent（プロトコル種別、アドレス情報、バイト列）

sip → session

IncomingInvite / IncomingAck / IncomingBye / IncomingUpdate など

TransactionTimeout など

session → sip

SendProvisionalResponse / SendFinalResponse / SendPrack / SendUpdate など

rtp → session

PcmInputChunk（セッションID/ストリームID + PCM データ）→ session が app へ転送

session → app

PcmReceived（PCM チャンク）+ CallStarted/Ended 等の通話イベント

app → ai::asr

AsrInputPcm（app が session から受け取った PCM を ASR へ送信）

ai::asr → app

AsrResult（セッションID + テキスト + メタ情報）

app → ai::llm

LlmRequest（履歴 + ユーザ発話など）

ai::llm → app

LlmResponse（テキスト応答 + アクション）

app → ai::tts

TtsRequest（読み上げテキスト + オプション）

ai::tts → app

TtsPcmChunk（セッションID/ストリームID + PCM データ）

app → session

PcmOutputChunk（app が TTS から受け取った PCM を session 経由で rtp へ）

app → session

SessionAction（応答コード、SDP 付き応答指示、BYE 指示など）

※具体的な型名/フィールドは実装時に調整するが、このレベルの分解を維持する。

### 6.2 イベント一覧と向き・役割（今回の設計タスク対象）

```text
[transport → sip]   RawPacketEvent (src/dst, bytes)     : SIPポートで受信した生データを渡す
[transport → rtp]   RawRtpPacket (src/dst, bytes)       : RTPポートで受信した生データを渡す

[sip → session]     IncomingInvite/IncomingAck/IncomingBye/IncomingUpdate,
                    TransactionTimeout                  : SIP受信/タイマをセッションへ通知
[session → sip]     SendProvisionalResponse (100/180等), SendFinalResponse (200/4xx/5xx),
                    SendPrack, SendUpdate               : 応答/再送/UPDATE送信の指示（構造体ベース）

[rtp → session]     RtpIn (ts, payload, ssrc/pt/seq)    : RTP受信のペイロード通知
[session → rtp]     StartRtpTx/StopRtpTx, RtpOutFrame   : 送信開始/停止とPCM→RTP化の指示

[app → session]     SessionAction (例: 180/200+SDP/Bye) : 高レベルなコール制御指示
[session → app]     CallStarted/CallEnded/Error, MediaReady : 通話状態/エラーの通知

[rtp → session]     PcmInputChunk                       : PCM入力をsessionへ（session→app→ai::asrの経路）
[session → app]     PcmReceived                         : PCMをappへ通知
[app → ai::asr]     AsrInputPcm                         : appがPCMをASRへ送信
[ai::asr → app]     AsrResult                           : 認識結果をappへ
[app → ai::llm]     LlmRequest                          : コンテキスト付き質問をLLMへ
[ai::llm → app]     LlmResponse                         : 応答テキスト/アクションをappへ
[app → ai::tts]     TtsRequest                          : 読み上げテキストをTTSへ
[ai::tts → app]     TtsPcmChunk                         : 生成PCMをappへ
[app → session]     PcmOutputChunk                      : appがPCMをsessionへ渡す（session→rtpで送出）
```

### 6.3 イベント方向の簡易図

```text
Network
  │
  │ RawPacketEvent / RawRtpPacket
  ▼
transport
  │            ┌──────────────┐
  │            │              │
  │        sip │              │ rtp
  │            │              │
  ▼            ▼              ▼
 session  ←────┴──────────────┘
  ▲   │
  │   │ PcmInputChunk / PcmOutputChunk
  │   │ CallStarted / CallEnded / SessionTimeout
  │   ▼
 app ←──────────────────────────────→ ai (asr/llm/tts)
      AsrInputPcm / TtsRequest / LlmRequest
      AsrResult / TtsPcmChunk / LlmResponse
```

**注意**: rtp↔ai の直接通信は禁止。PCM は必ず session→app を経由する（2025-12-27 確定、Refs Issue #7）

## 7. 並行処理モデル（Tokio）

Tokio を用いた非同期/並行実行の基本方針：

transport::packet

ソケットごとに 1 タスク（例：SIP 受信、RTP 受信）

sip

受信した SIP メッセージを処理し、トランザクションを管理するディスパッチタスク

トランザクションタイマはこのタスクから管理（必要なら追加のタスクを spawn）

session

「1セッション = 1タスク」が基本方針

各セッションタスクが、sip/app/rtp からのイベントを受け取り select 的に処理

rtp

受信処理タスク（RTP → PCM）

送信処理はキューを持つタスクで一括管理するか、セッション毎に持つかはスケール要件に応じて決定

ai（asr/llm/tts）

サービス側の特性に応じて、リクエスト単位/セッション単位で非同期関数 or タスクを生成

原則：

- 各モジュールがバラバラに tokio::spawn するのではなく、「どの単位でタスクを切るか」を設計通りに守る

- 間のやりとりは非同期チャネルで行い、極力共有可変状態を避ける

## 8. エラー・タイムアウト処理の責務分担

### 8.1 sip

トランザクションタイマ発火時：

対応するトランザクションを終了

必要に応じて TransactionTimeout イベントを session に通知

パース不能なメッセージ：

ログ出力

ポリシーに応じて 400 Bad Request を返すか、静かに破棄

### 8.2 session

TransactionTimeout を受けた際：

セッション状態を更新（エラー終了など）

必要であれば app に通知

Session Timer（RFC 4028）：

Timer 発火時に BYE/再INVITE を行うかなどのポリシー判断

SIP レベルの実際のメッセージ送信は sip に依頼

### 8.3 rtp

一定時間 RTP が来ない場合：

RtpTimeout イベントを session や app に通知

パケット異常（SSRC 不一致など）：

ログ出力＋場合によってはストリームエラーイベントを発行

### 8.4 ai（asr/llm/tts）

API エラー・タイムアウト：

リトライポリシー（回数、間隔）は ai モジュール内で処理

一定回数失敗した場合は app にエラーイベントを通知し、app がユーザ向けの応答（「ただいま混み合っています」など）や通話終了判断を行う

### 8.5 エラー・タイムアウトポリシー（詳細）

SIP / RTP / AI の代表的なエラー検知からユーザ影響までを明文化する。

- SIP トランザクションタイマ発火時
  - 検知レイヤ: sip（トランザクション状態機械が Timer A/B/E/F… を管理）
  - 通知イベント: `sip → session` に `TransactionTimeout(call_id, tx_type, method)`
  - 最終判断レイヤ: session（通話維持/再送終了/切断を決定。UASとしては再送を諦め終了へ寄せる）
  - UAC への振る舞い: 200 送信前の INVITE なら応答なしでタイムアウト終了（UAC 側が再INVITEを期待）。確立後の再送失敗は session が BYE を送出し切断。

- RTP 無着信（一定時間メディアが来ない）
  - 検知レイヤ: rtp（ストリーム単位の無着信タイマ）
  - 通知イベント: `rtp → session` に `RtpTimeout(call_id, stream_id, elapsed_ms)`
  - 最終判断レイヤ: session（MVP ポリシー: 1回目は警告ログで継続、連続発生または一定回数超過で BYE 送出）
  - UAC への振る舞い: BYE 時は即切断。警告のみの段階では応答なしで通話継続。

- AI 失敗（ASR / LLM / TTS エラー）
  - 検知レイヤ: ai::asr / ai::llm / ai::tts（各クライアントがリトライ後に失敗判定）
  - 通知イベント: `ai::asr → app` に `AsrError`、`ai::llm → app` に `LlmError`、`ai::tts → app` に `TtsError`（call_id/理由付き）
  - 最終判断レイヤ: app（フォールバック方針を決める）
    - 基本方針: 初回失敗は謝罪定型を生成し `app → ai::tts → app → session → rtp` で返して継続。同一フェーズで連続失敗（回数は config 管理、既定: 2回）で `app → session` に `SessionAction::Bye` を送り終了。
  - UAC への振る舞い: 初回は謝罪音声を返して継続。連続失敗時は謝罪音声の後に BYE（もしくは即 BYE）で切断。

## 9. 音声対話フロー（概要）

UAC からの INVITE を sip が受信し、session が新しいセッションを生成

SDP 交渉完了後、session が rtp を設定し、app に「AI 対話セッション開始」を通知

UAC 音声（受信フロー）:

transport::packet → rtp → PCM フレーム → session → app → ai::asr

ai::asr が発話テキストを app に通知

app:

LLM に問い合わせ → 応答テキスト/アクション

応答テキストを ai::tts へ

ai::tts:

PCM フレームを生成し、app へ渡す

app → session → rtp:

app が PCM を session 経由で rtp に渡し、RTP パケットにエンコードして transport::packet 経由で UAC に送信

**注意**: rtp↔ai の直接通信は禁止。PCM は必ず session→app を経由する（2025-12-27 確定、Refs Issue #7 CX-1）

必要なら:

LLM 応答が「終了」指示を含む → app → session に BYE 通知 → sip 経由で BYE 送信

## 10. 拡張・今後の方針

複数コーデック対応（Opus 等）

B2BUA 化（別の宛先への転送）

マルチターンでのコンテキスト管理強化（LLM プロンプト設計の拡張）

ログ・トレース・メトリクスの充実

本 docs/design.md を 設計の唯一のソース とし、仕様変更時はここを更新してから各モジュールの README.md に必要な要約を反映する運用を前提とする。

## 11. 運用メモ（神様ドキュメントと Codex への認識手順）

### 11.1 神様（Single Source of Truth）
- アーキテクチャと責務境界の神様: `docs/design.md`
- Frontend ↔ Backend API 契約の神様: `docs/contract.md`
- 録音（保存/配信）設計の神様: `docs/recording.md`

※ 仕様変更や判断は、該当する「神様ドキュメント」を先に更新し、他ドキュメントやコードを追従させる。

### 11.2 Codex / AI への認識手順（毎回のプロンプトに含める）
- 「設計の正: docs/design.md」「API契約の正: docs/contract.md」「録音設計の正: docs/recording.md」を明示する。
- 依存関係ルール（下方向のみ、禁止事項）を必ず守るよう指示する。
- 矛盾があれば「docs が正、コードは追従」と明記する。
- 変更依頼では「まず該当 docs を更新 → その内容に沿ってコードを変更」と指示する。
- レビュー依頼では「docs に整合しているか」を観点に含める。

## 12. アーキテクチャ原則（実装ガイド）

本プロジェクトは「イベント駆動 + レイヤ分離」によって、SIP/RTP（プロトコル）と対話（アプリ）とAI（外部統合）を疎結合に保つ。
実装・改修・コード生成（Codex含む）では、以下の原則を**必ず**守る。

### 12.1 クリーンアーキテクチャ（依存方向の厳守）
- 依存は下向きのみ（`app → session → (sip, rtp) → transport`、および `app → ai`）。
- 逆方向の直接参照は禁止。逆方向の通知は **イベント/メッセージ** に限定する。
- 迷ったら「責務を上に上げる（app/sessionへ寄せる）」か「イベント化」する。
- `http` は参照提供に徹し、通話制御や状態機械（sip/session）に立ち入らない。
- `media` は生成/保存に徹し、配信（http）と混ぜない。

### 12.2 Ports & Adapters（差し替え可能性の担保）
- `ai::{asr,llm,tts}` は実装差し替えが前提（ローカル/クラウド、ベンダ差し替え）。
- app から見えるI/Fは **Port（trait）** として定義し、具体実装は **Adapter** として ai 内に閉じる。
- session から ai を直接呼ばない（必ず app を経由）。AIの成否による判断は app の責務。
- 外部I/O（HTTPクライアント、APIキー、リトライ等）は Adapter 側に閉じ込め、上位は純粋な「要求/応答」を扱う。

例（概念）:
- `trait AsrPort { async fn push_pcm(...); }`
- `trait LlmPort { async fn complete(...); }`
- `trait TtsPort { async fn synthesize_stream(...); }`

### 12.3 イベント駆動（結合度を上げない接続）
- モジュール間は原則 **非同期チャネル + enumイベント** で接続する。
- 共有可変状態より、イベントで状態遷移させる（「通知」と「判断」を分離）。
- イベントは以下を必須とする：
  - 相関ID（`call_id` / `session_id` / `stream_id` のいずれか）
  - 発生源（どのレイヤから来たか分かること）
  - 終端（`End/Done`）やエラー理由（失敗時）
- イベントは「重複して届いても壊れない（冪等）」設計を基本とする（SIP再送・ネットワーク重複を前提）。

### 12.4 状態機械ファースト（State Machine First）
- SIP（トランザクション/ダイアログ）、セッション、対話（dialog）は **状態が本体**。
- if文を散らさず、`enum` と遷移関数で状態を表現し、遷移を1箇所に集約する。
- 「許可されないイベント」を受けた場合の方針を明示する（ログ＋無視 / エラー終了など）。
- 重要な状態（early/confirmed/terminated 等）と遷移トリガ（INVITE/ACK/BYE、タイムアウト等）を docs に残す。

### 12.5 並行処理モデル（Actor/タスク設計の統一）
- 原則：`1セッション = 1タスク`（sessionタスクがイベントを `select!` で処理）。
- タスク境界を跨ぐ共有ロック（巨大Mutex）で整合性を取らない。状態はセッションタスク内に閉じる。
- `tokio::spawn` は方針に従って最小限にし、「どの単位でタスクを切るか」を設計通りに守る。
- rtp送信/受信は（スケール要件に応じて）専用タスク＋キューで吸収し、上位はPCMイベントだけを見る。

### 12.6 タイムアウトとキャンセル（リアルタイム会話の前提）
- ASR/LLM/TTS は遅延・失敗・キャンセルが起きる前提で設計する。
- すべての外部呼び出しにタイムアウトを設定し、失敗時のフォールバック方針を app の policy に集約する。
- 「割り込み（ユーザが話し始めた等）」が起きた場合の扱い（TTS停止、LLM中断など）を app の状態機械で定義する。
- “無言になる”状態を避けるため、AI失敗時の定型応答（謝罪など）を用意する（継続/終了判断は app）。

### 12.7 観測可能性（Observability）を仕様として扱う
- ログは必ず相関ID（`call_id`/`session_id`）付きで出す（後追い解析できることが最重要）。
- 最低限、次のレイテンシを計測できる構造にする：
  - RTP受信 → ASR確定
  - ASR確定 → LLM応答
  - LLM応答 → TTS最初のPCM送出
- 重要イベント（CallStarted/Ended、ASR final、LLM応答、TTS開始/終了、BYE送出等）は構造化ログで残す。

### 12.8 アンチパターン（禁止事項）
- app/http/ai が sip/rtp の内部構造（Via/To/From、SSRC/Seq/Timestamp 等）を直接触ること。
- session が ai を直接呼ぶこと（必ず app 経由）。
- http が通話制御（応答コード決定、BYE、RTP制御）を行うこと。
- “便利だから” を理由に utils に何でも詰めること（肥大化の温床）。
- 例外的に境界を跨ぐ必要が出た場合は、まず docs（本design）を更新し、イベント/Portを追加して解決する。

## 13. プロジェクト規約

この章は「設計の一貫性を保つための規則」を定める。
例外が必要な場合は、必ず本docsを先に更新し、レビュー観点に含める。

### 13.1 命名とID（相関の規則）
- すべての主要ログ/イベントに `call_id` を付与する（必須）。
- メディア（RTP/PCM）単位は `stream_id` を併用する（SSRC等のプロトコル内部IDは上位へ漏らさない）。
- **MVP**: `call_id == session_id` として統一する（2025-12-27 確定、Refs Issue #7）。
  - ai.md/app.md の DTO では `session_id` フィールドを使用するが、値は `call_id` と同一。
  - 将来的に分離する場合は本 docs を先に更新する。
- 文字列IDの形式（ULID/UUID等）を統一し、生成箇所は `utils::id` に集約する。

### 13.2 データモデル境界（DTOの規則）
- レイヤを跨ぐデータは「境界用DTO」として定義し、プロトコル内部構造（SIPヘッダ、RTP seq等）を混ぜない。
- DTOは以下のどちらかに分類する：
  - (A) 内部イベント用（非同期チャネルで流す）
  - (B) 外部公開用（httpが返すレスポンス）
- 内部イベントDTOと外部公開DTOを混ぜない（変換は http 側の整形層で行う）。

### 13.3 エラー分類（失敗の扱いの規則）
- エラーは「どの層で検知されたか」で分類し、上位に上げるほど抽象化する。
  - sip/rtp: プロトコル/メディアの異常（入力不正、タイムアウト等）
  - ai: 外部依存の失敗（タイムアウト、認証、レート制限等）
  - app/session: ポリシー判断（継続/終了、謝罪返答等）
- 原則として「検知した層が復旧（リトライ等）まで実施」し、「継続/終了の最終判断は app/session」が行う。
- パニック（panic）はバグとして扱い、回復手段にしない（落とすより、エラーイベントで上げる）。

### 13.4 タイムアウト規約（リアルタイムの規則）
- 外部I/O（ASR/LLM/TTS、録音書き出し、HTTPクライアント）は必ず timeout を持つ。
- タイムアウト値は config で一元管理し、ハードコードしない。
- 「無音が続いた」「相手が話し始めた」など、会話上の割り込み条件は app の状態機械で定義する。

### 13.5 リトライ規約（再試行の規則）
- リトライは ai モジュールに閉じる（上位は「成功/失敗」だけ扱う）。
- リトライ回数とバックオフは config 管理とする。
- リトライ可能/不可能（例：認証失敗は不可）を明示し、無限リトライは禁止。

### 13.6 設定（config）の規則
- ポート、IP、コーデック許可、ASR/LLM/TTSのエンドポイント、タイムアウト等は config に集約する。
- config は「起動時に読み、以降は不変」を原則にする（ホットリロードはMVPではやらない）。
- 機密情報（APIキー等）は環境変数/secret管理とし、ログに出さない。

### 13.7 ログ規約（運用の規則）
- ログは構造化ログを推奨し、最低限 `call_id`/`session_id` を含める。
- PII（個人情報）になりうる文字起こし/LLM入力/出力は、保存の可否とマスキング方針を決める。
- “音声ボットが無言になった”を追えるよう、重要イベントは必ずログに残す（CallStarted/Ended、ASR final、LLM応答、TTS開始/終了、BYE送出など）。

### 13.8 テスト方針（品質の規則）
- プロトコル層（sip/rtp）はユニットテストを優先（パース/ビルド、ジッタ整列、デコード等）。
- app は状態機械のテストを優先（入力イベント→期待する出力イベント/SessionAction）。
- 統合テストは「擬似UAC（INVITE→RTP→BYE）」の最小シナリオを用意し、回帰を防ぐ。

### 13.9 変更手順（ドキュメント駆動の規則）
- 仕様/責務/依存方向に関わる変更は、必ず docs/design.md を先に更新する（Single Source of Truth）。
- コードが docs と矛盾する場合、正は docs とし、コードを追従させる。
- Codex/AIに依頼する場合も同様に「まず docs 更新 → その内容に沿って実装」を徹底する。

## 14. 追加で採用するソフトウェア原則（このプロジェクト向け）

### 14.1 Separation of Concerns（関心の分離）
- 「通話制御（SIP）」「メディア（RTP）」「対話（app）」「AI統合（ai）」「録音（media）」「配信（http）」を混ぜない。
- 1つの変更理由（変更要因）につき、影響範囲が1レイヤ/1モジュールに収まることを目指す。

### 14.2 Information Hiding（情報隠蔽）
- 下位の詳細（SIPヘッダ、RTPのSeq/Timestamp/SSRC、ジッタ等）を上位に漏らさない。
- 上位は「意味のあるイベント（CallStarted/AsrFinal/TtsChunk等）」だけを扱う。

### 14.3 Principle of Least Knowledge（Law of Demeter）
- 上位は下位の“内部構造”をたどらない。欲しい結果はイベント/Port経由で取得する。
- 例：appがsip::messageのヘッダを直接編集しない、sessionがaiのHTTP層を触らない。

### 14.4 Design by Contract（契約による設計）
- レイヤ境界のDTO/イベントは「入力条件」「出力保証」「例外（エラー）」をdocsで明文化する。
- 重要なフィールド（call_id/stream_id、終端フラグ等）は必須とし、欠落は即エラーにする。

### 14.5 Robustness Principle（堅牢性：受信に寛容、送信に厳格）
- 受信（特にSIP/RTP）は不正・欠落・順序乱れを前提に“壊れない”設計にする。
- 送信（SIP応答、RTP送出、httpレスポンス）は仕様に厳密に従う（曖昧なデータを出さない）。
- ただし「受信に寛容」は無制限ではなく、危険な入力は拒否する（ログ＋400/破棄など）。

### 14.6 Backpressure（逆流圧：詰まりを伝播させる）
- RTP→ASR、LLM→TTS→RTPは詰まりやすい。無制限キューは禁止。
- チャネル容量を制限し、詰まったら「間引き」「最新優先」「切断」などの方針を app policy に置く。
- “遅延が伸び続ける”より“少し欠ける”ほうが会話品質は良い、を原則とする。
MVP のデフォルト方針：
- RTP→ASR：最新優先（遅延を増やす古いPCMは破棄）
- TTS→RTP：割り込み優先（新しい発話で古い送出を停止）
- LLM：同一セッションで in-flight は1つ（同時実行しない）

### 14.7 Idempotency / Determinism（冪等性 / 決定性）
- 同じイベントを複数回処理しても破綻しない（SIP再送、二重BYE、重複RTPなど）。
- 状態機械は「同じ入力→同じ出力」になりやすい形に寄せ、ログ解析可能にする。

### 14.8 Simplicity First（KISS / YAGNI）
- MVPでは必要最小限のRFC・機能に絞る（“将来のため”の複雑化をしない）。
- 追加機能は「イベント追加」「Port追加」「状態遷移追加」で段階的に導入する。

### 14.9 Security & Privacy by Design（最小限）
- 音声・文字起こし・LLM入出力はPIIになりうる前提で扱う。
- 保存するなら期間/権限/マスキングを決め、外部送信（クラウドASR/LLM）可否を明示する。
- ログに原文（全文）を出さない方針を推奨（必要ならデバッグフラグで限定的に）。
