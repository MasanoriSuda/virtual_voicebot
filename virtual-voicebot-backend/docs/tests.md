# テスト計画

## テストレベル
- ユニット: sip/rtp/session/app/ai 各モジュールのロジック単位（必須）。パーサ/ビルダ、状態機械、ジッタ処理、AIクライアントのエラー伝搬など。
- モジュール間結合: sip↔session、session↔rtp、app↔ai のイベント疎通（必須）。実パケットやPCMを使わずにモックで確認。
- E2E: SIP+RTP+AI の通し（必須）。INVITE→ACK→RTP往復、AIフォールバック、タイマ挙動を通しで検証。
- 負荷/長時間（任意・後回し可）: 長時間通話や多数同時呼の安定性。

## E2E: 基本呼シナリオ (INVITE→ACK→RTP)
- 前提: UAC が INVITE（SDP PCMU/8000）送出、UAS は 100/180/200 返却、UAC が ACK 返送。
- ケース:
  - 正常系: INVITE→100/180/200→ACK、RTP 双方向が 10 秒継続し重大エラーなし。
  - SDPバリエーション: コーデック1種類 (PCMU)、sendrecv/recvonly など方向が合致。
  - RTP観察: 20ms 間隔程度でRTPが到達すること、SSRC/Seq/Timestamp が単調進行すること。
- 入力条件: 正常な SIP/SDP、連続した RTP 送出（テスト用ジェネレータ可）。
- 期待: SIPレスポンスシーケンスが規定通り、RTPが途切れず届き、session/app/ai に重大エラーが出ない。

## SIP トランザクションタイマ
- ACK欠落: INVITE に 3xx–6xx を返した後 ACK を止める。Timer G/H で再送し、Timer H 発火で Terminated＋TransactionTimeout 通知。
- 再送 INVITE: Proceeding/Completed 状態で INVITE 再送を受けると最新応答を再送すること。
- 非INVITEタイムアウト: 最終応答後 Timer J で Terminated になること（再送リクエストには最終応答を再送）。
- 期待: 状態遷移が docs/sip.md の表通り、session への通知（TransactionTimeout）が行われ、不要な通話継続が残らない。

## AI 失敗フォールバック
- ASR失敗: ai::asr が `AsrError` を返す。app が謝罪定型を返し通話継続（1回目）、連続失敗で BYE を指示できること。
- LLM失敗/タイムアウト: `LlmError` を返し、app がフォールバック応答または終了判断。謝罪後に継続、連続で終了のポリシーを確認。
- TTS失敗: `TtsError` を返し、該当発話をスキップまたは謝罪短縮発話を試み、必要なら BYE。RTP送出が止まる/止まらないを確認。
- 期待: design.md のエラーポリシーに沿った SessionOut/SIP動作（必要なら BYE）と、ログ/メトリクスの出力。

## RTP / メディアタイムアウト・品質
- 無着信: 一定時間 RTP が来ない（RtpTimeout）。session が警告ログのみ→2回目で BYE などのポリシーを満たすこと。
- 軽微ロス/ジッタ: 低率のロスや遅延で ASR が動作継続すること（重大エラーにならない）。遅延パケット破棄が想定どおり。
- 期待: RtpTimeout イベントが適切に発火し、通話継続/終了の判断がポリシー通り。ログで損失・タイムアウトが確認できる。

## ログ / メトリクス検証
- SIP: 受信/送信メッセージ、トランザクション遷移、Timer 発火のログ。メトリクスは応答コード数、タイムアウト件数。
- RTP: 受信/送信パケット数、SSRC/Seq/Timestamp の進行、RtpTimeout/ロス警告のログ。
- Session: SessionTimeout/keepalive 発火、SessionOut 実行のログ。メトリクスはセッション数、終了理由。
- AI: AsrResult/AsrError、LlmResponse/LlmError、TtsPcmChunk/TtsError のログ。メトリクスは成功/失敗回数、遅延。
- 代表ケースで、上記ログ/メトリクスが出ることを確認する（具体的な文字列は問わないが粒度を確認）。
