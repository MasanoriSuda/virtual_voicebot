# rtp モジュール

目的・責務
- RTP/RTCP パケット処理と音声ストリーム管理を担当する
- PCM と RTP ペイロードの相互変換を行い、ASR/TTS と連携する
- SSRC/Seq/Timestamp の生成・管理、簡易ジッタバッファによる整列

他モジュールとの関係
- transport: RTP/RTCP 生パケットの送受信
- session: SDP で決定されたメディア設定を受け取り、送信先を設定
- ai::asr: PCM を ASR に供給
- ai::tts: TTS から PCM を受け取り RTP にエンコード

注意事項
- SSRC/Seq/Timestamp は rtp 内で生成・管理し、上位へ漏らさない
- コーデックは MVP では PCMU (G.711 μ-law) のみ対応
- RTCP は SR/RR の基本実装あり、SDES(CNAME) は実装予定

詳細設計
- 正本: [DD-004_rtp.md](../../docs/design/detail/DD-004_rtp.md)
