# transport モジュール

目的: SIP/RTP を含むネットワークトラフィックの「生パケット I/O 専任」レイヤ。ソケットの bind / 受信 / 送信と、上位への配送だけを担当する。

主な責務
- UDP/TCP ソケットの初期化・bind と受信ループの維持
- 受信パケットを「送信元アドレス + 受信ポート + ペイロード（バイト列）」として上位に通知する
  - SIP 現状: `SipInput { src: SocketAddr, data: Vec<u8> }`（受信ポートはソケットから取得可能）
  - RTP 現状: `RawPacket { src: SocketAddr, dst_port: u16, data: Vec<u8> }`
- 上位からの送信指示（宛先アドレス、送信元ポート、バイト列）をそのままネットワークに送る
  - 送信指示型は transport 側で `TransportSendRequest { dst, src_port, payload }` として定義し、sip/session 依存を避ける

上位モジュールとの関係
- SIP のパース、応答コードの決定、レスポンス組み立ては `sip` / `session` が行い、送信指示として渡す
- RTP の解析やセッション状態は `rtp` / `session` が持ち、transport は配送のみ行う

してはいけないこと
- transport 内で SIP 応答（100/180/200/BYE/REGISTER など）を決定・生成しない
- トランザクションやセッションの状態、Call-ID/SDP といった上位の概念を保持しない
- `sip` や `session` の型に直接依存しない（イベント/チャネル経由で疎結合にやり取りする）

補足（現状のSIP即時返信が参照するメタ情報）
- `local_ip` / `sip_port` / `advertised_rtp_port` を SDP 生成・Contact ヘッダに埋め込んでいる
- 今後は sip/session 側で必要な形に渡し、transport では保持しない方針
