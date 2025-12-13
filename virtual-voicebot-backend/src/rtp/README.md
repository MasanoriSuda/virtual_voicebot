# rtp module notes

- 本モジュールの責務は RTP/RTCP の構造化、ストリーム管理、将来のジッタバッファ/RTCP 対応。
- 現状のコードは transport→session の直結を維持しており、rtp モジュールへの委譲は未着手。
- `stream.rs` に Seq/Timestamp/SSRC 管理のプレースホルダを置き、今後の移行先を示す。
- `rtcp.rs` に RTCP 受信/送信のスタブ I/F を定義し、未実装であることを明記している。
