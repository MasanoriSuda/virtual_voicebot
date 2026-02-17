<!-- SOURCE_OF_TRUTH: SIPp E2Eテスト -->
# sipp を用いた E2E スモークテスト

## 前提
- SIPp シナリオの正は `test/sipp/sip/scenarios/`（例: `test/sipp/sip/scenarios/basic_uas.xml`）。
- ツール: `sipp` がインストール済みであること。
- 推奨: docker compose 経由で実行（CI と同じ前提）。
- コーデック/SDP: PCMU/8000 のみを想定。

## docker compose（推奨・CIと同じ）
```
docker compose -f test/docker-compose.sipp.yml up --build --abort-on-container-exit --exit-code-from sipp
docker compose -f test/docker-compose.sipp.yml down -v
```
compose 実行時は `UAS_SIP_HOST=uas` を固定とする（127.0.0.1 は使わない）。

## sipp シナリオと実行例
- シナリオ例: `test/sipp/sip/scenarios/basic_uas.xml`
  - 流れ: INVITE 送出（SDP: PCMU/8000） → 100/180/200 を受信 → ACK 送信 → 約10秒 RTP 送出。
  - RTP: sipp の `-mp` オプション等で固定ポートから PCMU パケットを送出。

### ローカル実行（補助）
- サーバを別途起動している場合:
  - `sipp <server_ip>:<sip_port> -sf test/sipp/sip/scenarios/basic_uas.xml -m 1 -trace_err -trace_msg`
  - `UAS_SIP_HOST=127.0.0.1 UAS_SIP_PORT=5060 sipp $UAS_SIP_HOST:$UAS_SIP_PORT -sf test/sipp/sip/scenarios/basic_uas.xml -m 1 -trace_err -trace_msg`

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
