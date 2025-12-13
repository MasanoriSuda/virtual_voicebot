# docs/todo_rtp.md

## 挙動を変えないリファクタ
- [x] `transport::packet::run_rtp_udp_loop` 内のRTP処理と session 直行部分にコメントを追加し、rtpモジュールへの移行前提を明示する（挙動は維持）。
- [x] `session.rs` 内のRTP送信処理（RtpPacket生成・build・UdpSocket送信）を関数に分離し、将来rtpに移すフックを用意する（挙動は維持）。
- [x] rtpモジュールにストリーム管理用の構造体/インタフェースの枠だけ定義（SSRC/Seq/Timestamp管理のプレースホルダ）、現状ロジックは呼ばずに温存する。
- [x] RTCP用のI/F（受信通知/送信要求のスタブ）を定義し、未実装であることを明記したドキュメント/コメントを追加する。

## 挙動を変えるリファクタ（経路統一・ストリーム管理・RTCP入口）
- [x] 受信経路を transport→rtp→session/app に切り替え、SessionIn::RtpIn を rtp 経由で発火するよう変更する（直接parse→session送信を廃止）。
- [x] 送信経路を session→rtp→transport に切り替え、session内の build_rtp_packet/UdpSocket 使用を廃止して rtp がソケットと Seq/Timestamp/SSRC を管理する。
- [x] ストリーム管理を導入し、(セッションID/SSRC/リモートアドレス)をキーに Seq/Timestamp/SSRC を保持する（MVPは単一SSRC/PCMUのみ）。
- [x] payload type別処理の抽象を追加し、PCMU以外は未実装として分岐（将来拡張に備える）。
- [x] 簡易ジッタ/遅延廃棄ポリシーを rtp 内に実装し、ASRへのPCM通知を統一する。
- [x] RTCP入口を接続し、受信時にイベント/ログを出す。送信はI/Fを呼べる形にしてタイマ開始だけ実装（SR/RR生成は後続）。
- [x] 新経路で INVITE→ACK→RTP往復のスモークを通し、旧直結処理を削除する。
