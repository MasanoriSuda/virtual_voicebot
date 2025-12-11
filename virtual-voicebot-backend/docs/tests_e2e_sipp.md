# sipp を用いた E2E スモークテスト

## 前提
- サーバ起動先: `localhost`（デフォルト） / 環境変数で SIP/RTP ポートを指定可能。
- ツール: `sipp` がインストール済みであること。
- ネットワーク: ローカルで UAC (sipp) → UAS (本サーバ) の UDP が通ること。
- コーデック/SDP: PCMU/8000 のみを想定。

## サーバの起動方法（例）
1. 必要な環境変数を設定（例: `SIP_PORT=5060`, `RTP_PORT=10000`, `ADVERTISED_IP=127.0.0.1`）。
2. 本サーバを起動（例: `cargo run` または既存の起動スクリプト）。
3. ログに `Listening SIP on ...` が出ることを確認。

## sipp シナリオと実行例
- シナリオ例: `test/scenarios/invite_basic.xml`
  - 流れ: INVITE 送出（SDP: PCMU/8000） → 100/180/200 を受信 → ACK 送信 → 約10秒 RTP 送出。
  - RTP: sipp の `-mp` オプション等で固定ポートから PCMU パケットを送出。
- 実行例（UAC として動作）:
  - `sipp 127.0.0.1:5060 -sf test/scenarios/invite_basic.xml -m 1 -s 1000 -trace_msg -trace_err -l 1 -r 1`
  - 必要に応じて `-mp 40000` などで RTP ポートを指定。

## 期待される挙動
- SIP: sipp 側で 100/180/200 を受信し、ACK を送出してシナリオが成功終了。
- RTP: sipp の送出が本サーバに届き、サーバ側ログに RTP 受信の概要が出る（Seq/Timestamp 進行など）。
- サーバログ例（モジュール別の目安）:
  - transport: 受信/送信した SIP 行の概要、RTP 受信ポートとバイト数。
  - sip: トランザクション遷移（INVITE Proceeding→Completed 等）とタイマ発火。
  - session: セッション生成/確立/終了、SessionOut 実行（StartRtpTx/StopRtpTx など）。
  - rtp: RTP ストリーム開始/終了、受信フレーム数や SSRC/Seq/Timestamp の進行。
  - app/ai: （有効なら）ASR/LLM/TTS 呼び出しの開始/成功/失敗ログ。

## 簡易チェックリスト
- [ ] サーバ起動ログが出ている。
- [ ] sipp シナリオが 0 エラーで終了。
- [ ] SIP 応答シーケンスが正しい（100/180/200 → ACK）。
- [ ] RTP 受信がログで確認できる（一定期間連続）。
- [ ] 重大エラーログが出ていない。
