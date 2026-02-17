# sip モジュール

目的・責務
- SIP メッセージの表現（Request/Response/ヘッダ/SDP ペイロード）
- テキストと構造体の相互変換（パーサ/ビルダ）
- UAS 側のトランザクション状態機械とタイマ管理（INVITE/非 INVITE）
- transport からの受信を session イベントへ変換し、session からの送信指示をレスポンス/リクエストに組み立てて transport へ渡す

他モジュールとの関係
- 入力: transport からの生 SIP データ、トランザクションタイマ発火
- 出力: session への受信イベント/タイムアウト通知、transport への送信指示
- コール制御（どのステータスを返すか）は session が判断し、sip はプロトコル処理に専念する
- AI/音声処理は app/ai/rtp が担当し、sip から直接呼ばない

注意事項
- トランザクション/タイマ管理は sip に集約し、transport では行わない
- Call-ID/タグ/CSeq などのプロトコル識別情報を sip 内で一貫して扱う
- ビジネスロジックや対話フローの判断は持たず、session/app に委譲する
