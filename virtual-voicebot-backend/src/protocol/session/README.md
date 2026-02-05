# session モジュール

目的・責務
- 通話セッションのライフサイクル管理（生成/検索/破棄）
- 呼状態遷移と Session Timer/keepalive の管理
- SIP/rtp/app 間で SessionIn/SessionOut を介したコール制御の橋渡し
- RTP の開始/停止と送信先設定のみ扱い、メディア処理や AI ロジックは持たない

他モジュールとの関係
- sip: 受信イベント（Invite/Ack/Bye 等）を SessionIn で受け、レスポンス送出を SessionOut で指示
- rtp: 送受信の開始/停止と送信先設定を SessionOut で指示、RTP入力を SessionIn で受ける
- app/ai: ASR/LLM/TTS など対話ロジックは app/ai が担当、session はタイミングをイベントで通知するのみ
- transport: 直接依存しない（sip/rtp 経由）

注意事項
- SIP プロトコル詳細は sip、メディア処理は rtp、AI 呼び出しは app/ai が担当する
- SessionOut を経由して外部へ指示し、直接他モジュールの実装に依存しない
- Call-ID などの識別情報を manager 経由で一元管理し、状態をばらばらに持たない
