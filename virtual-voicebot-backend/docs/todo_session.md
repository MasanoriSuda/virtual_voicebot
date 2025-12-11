# docs/todo_session.md

## 挙動を変えないリファクタ
- [ ] `writing.rs` の SessionOut 配線スタブにコメントを足し、sip/rtp/app への出口の責務を明示したまま現挙動を維持する。
- [ ] session manager の骨組み（create/get/destroy/list）を types 近辺に定義し、現行マップ操作をラップするだけの薄いAPIを導入する（挙動は同じ）。
- [ ] `SessionIn`/`SessionOut`/状態遷移のコメントを docs/session.md と一致させ、命名・説明の整合を取る（動作は変えない）。
- [ ] keepaliveタイマ開始・停止の責務を関数に分離し、現状の TimerTick ループの挙動を変えずに整理する。
- [ ] `handle_bot_pipeline` を分離して「ここは app/ai に移す予定」と明記し、呼び出し元に薄いラッパを挟む（挙動は変えない）。

## 挙動を変えるリファクタ（SessionOut 実配線・Session Timer・AI 移譲）
- [ ] session manager を本格化し、main からのセッション生成/破棄をすべて manager 経由に切り替え、Call-ID マップを一本化する。
- [ ] SessionOut を sip 送信キューに繋ぎ、180/200/Bye200 送出を sip 経由で実送信するようにする（transport への直接依存を排除）。
- [ ] SessionOut を rtp 制御（開始/停止/送信先設定）に繋ぎ、ソケット管理を rtp 側へ移す。
- [ ] Session Timer（簡易keepaliveタイマを含む）をセッション状態に統合し、発火時に SessionOut/SessionTimeout を出す実装に変更する。
- [ ] AI 呼び出しを session から削除し、app へのイベント（音声解釈依頼/応答受信）に置き換える。app→session で BotAudio/終了指示などを戻す経路を整備する。
- [ ] `SessionIn/Out` を app/ai イベントと整合する形に拡張/改名し、RTP/PCM 経路とコール制御を分離する。
- [ ] 新フローで INVITE→ACK→RTP→ASR/LLM/TTS（app経由）→応答送出→BYE のスモークを通し、旧直呼びを排除する。
- [ ] 不要になった `handle_bot_pipeline`、wav一時ファイル処理、直接の ai 依存を削除する。
