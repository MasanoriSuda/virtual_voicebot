# 2025-12-15 MVP 再掲（現行実装ベース）

## 最小動作の定義（「ここまでをMVP」とする）
- SIP: INVITE → 200 OK → ACK が通る。BYE 受信時に 200 を返す。再送/タイマは簡易で可。
- RTP: 片方向でも OK（PCMU/8000 固定）。音声を受信し、録音（mixed.wav＋meta.json）を生成できる。
- 録音配信: `/recordings/<日時_callId>/mixed.wav` を HTTP で配信し、`recordingUrl` をフロントに渡す。
- 履歴連携: 通話終了時に ingest で Call 情報（from/to/開始/終了/recordingUrl）をフロントへ送る。
- これ以上のSIP拡張（PRACK/UPDATE/Session-Expires 等）、RTCP、本格的なエラーポリシーは後続。

## 全体アーキテクチャ（現行コードが正）
```
app (dialog)  ─┐
               │  ingest/recording連携 (reqwest)
session ───────┼─→ sip ─→ transport(UDP)
       └─→ rtp ─┘
          └→ media (録音生成)
http (録音配信のみ /recordings/…)
```
- transport: UDP入出力とバイト配送のみ。
- sip: メッセージ/トランザクション処理（タイマ簡易版）。
- rtp: RTP受信パースと jitter 簡易廃棄。
- session: コール制御＋録音開始/停止＋ingest送信（現状ここに集約）。
- media: 録音生成と meta.json 出力。
- http: 録音ファイルの静的配信（/recordings）。

## ディレクトリ構成（主要）
- src/transport/: UDP I/O（SIP/RTP振り分け）
- src/sip/: SIP パース・トランザクション
- src/rtp/: RTP 受信/送信、stream管理
- src/session/: コール制御、録音/ingestを呼び出し
- src/media/: 録音生成（mixed.wav/meta.json）
- src/http/: 録音静的配信サーバ
- docs/design.md: レイヤと責務の設計
- docs/mvp.md: 本ファイル（MVP再掲）

## イベント流れ（現行）
1. transport が SIP UDP を受信 → sip へ `SipInput`
2. sip が INVITE をパースし SessionOut で 180/200 を送出 → transport 経由で送信
3. main が SipEvent を受けて session を生成（rtp/recording 配線）
4. RTP 受信: transport → rtp → session に `PcmInputChunk`（旧名 MediaRtpIn は廃止）
5. session が録音開始、音声を media に書き込み
6. BYE/タイムアウトで録音停止し ingest に通話情報＋recordingUrl を POST
7. http モジュールが `/recordings/<日時_callId>/mixed.wav` を配信、フロントは recordingUrl を再生

## 設定（主な環境変数）
- SIP_BIND_IP / SIP_PORT / ADVERTISED_IP / ADVERTISED_RTP_PORT
- RECORDING_HTTP_ADDR（録音配信サーバのbind、デフォルト 0.0.0.0:18080）
- RECORDING_BASE_URL（フロントに渡す録音URLのベース、未指定なら RECORDING_HTTP_ADDR を http:// で利用）
- INGEST_CALL_URL（フロントの `/api/ingest/call` へのURL）

## これ以降の拡張（後続タスク）
- 正式な SIP トランザクションタイマ（A/B/E/F/H/J）と状態機械の実装
- RTCP SR/RR、ジッタバッファ、Codec拡張
- 録音の時間同期（送受ミックスを実時間ベースにする or トラック分離）
- http レイヤでの REST/SSE 提供と認証
