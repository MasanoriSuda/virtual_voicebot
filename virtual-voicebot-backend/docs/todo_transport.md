# docs/todo_transport.md

## [MVP][transport] SIP応答をsip/sessionへ委譲するタスク

### 挙動を変えないリファクタ
- [x] `run_sip_udp_loop` の即時返信ブロックを識別しやすい形に分離し、現在の責務過多をコメントやドキュメントで明記する。
- [x] 受信イベントの情報粒度（送信元アドレス、受信ポート、ペイロード）を transport README に記述する。
- [x] SIP 即時返信で使っているメタ情報（local_ip、sip_port、advertised_rtp_port）を洗い出し、sip 側へ渡す必要があるデータ項目を一覧化する。

### 委譲を伴う本番リファクタ
- [x] transport→sip の受信イベントを「バイト配送のみ」に変更し、`parse_sip_message` など sip 依存を除去する。
- [x] `build_provisional_response` / `build_final_response` / `build_simple_response` と即時送信処理を `run_sip_udp_loop` から削除する。
- [x] sip/session→transport の送信指示チャネルを追加し、送信指示に含める情報（宛先 IP/ポート、送信元ポート、バイト列）を定義する。
- [x] sip/session 側に応答組み立て・送信指示の経路を実装し、INVITE/BYE/REGISTER の応答を新経路に切り替える。
- [x] transport が sip 型に依存しない形にモジュール境界を整理し、README と設計ドキュメントに反映する。
- [ ] 新フローで既存スモークを確認し、必要に応じて transport 単体の I/O スモーク（受信→上位配送、送信指示→UDP 送信）を追加する。
