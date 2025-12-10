# session モジュール詳細設計 (`src/session`)

## 1. 目的・スコープ
- 通話セッションのライフサイクル管理（生成/破棄/検索）と SIP/RTP 設定の橋渡しを行う。
- 呼状態（Idle/Early/Confirmed/Terminating/Terminated）の遷移と Session Timer/keepalive の保持。
- RTP 開始/停止と送信先設定のみを扱い、AI/アプリロジックは扱わない。

## 2. session manager API
- `create(call_id, media_cfg)` : 新規セッションを生成し内部マップに登録。ローカルSDP/RT P送信設定の初期化。
- `get(call_id)` / `remove(call_id)` : 参照/削除。BYE 受信や Session Timer 失効時に破棄。
- `list()` : デバッグ/メトリクス用。
- 保持情報: call_id、ダイアログ情報（From/To タグ、CSeq）、状態、タイマハンドル群、rtp 設定（ローカルIP/ポート/SSRC/PT）、sip/rtp へのチャネル。

## 3. Session Timer の状態管理
- 持つタイマ: keepalive/無音監視タイマ（MVP）、Session-Expires/Min-SE リフレッシュタイマ（拡張時）。
- ライフサイクル: Confirmed 遷移で開始。再INVITE/UPDATE 受信で残り時間を更新。BYE/エラー/終了で全タイマ停止。
- 発火時の通知: `SessionTimeout` を `session → app` に送る。必要に応じて `SessionAction::Bye` を自動生成して `session → sip` へ送る（ポリシー次第）。
- 簡略化（MVP）: Session-Expires/Min-SE は無効化可。keepaliveのみでタイムアウト検知、失効後は警告→BYE など単純ルールを適用。

## 4. keepalive / タイムアウト時の SessionOut
- keepalive運用: 20〜30ms 程度の周期 tick で無音フレーム送出やメトリクス更新を行う。送出失敗が連続する場合は `SessionOut::StopRtpTx` と `SessionOut::SendSipBye200` をトリガ。
- タイムアウト時のアクション: 原則 app に `SessionTimeout` を通知し方針を仰ぐ。即切断が必要な場合のみ直接 `SessionOut::StopRtpTx` / `SendSipBye200` を発火。
- 任意メトリクス: `SessionOut::Metrics { name: "session_timeout", value: 1 }` などを発行しても良い。

## 5. 他モジュールとの責務境界
- sip: 受信 SIP を `SessionIn::Invite/Ack/Bye/...` で通知。応答送信は `SessionOut::SendProvisionalResponse/SendFinalResponse` で依頼。
- rtp: I/O と SSRC/Seq 管理・簡易ジッタは rtp。session は開始/停止と送信先設定だけ伝える。
- app/ai: 対話・ASR/LLM/TTS は app/ai で完結。session はコール制御と PCM 経路制御のみ。

## 6. MVP と拡張
- MVP: keepaliveタイマを簡易 Session Timer として運用（失効で警告→BYE）。Session-Expires/Min-SE はオフでも可。
- NEXT: 4028 準拠の refresher 管理、再INVITE/UPDATE 送受、無音監視ポリシーの高度化。
