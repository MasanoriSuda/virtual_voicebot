# はじめに
このファイルは「設計/実装タスクと優先度」を管理する。  
- 設計の詳細は docs/design.md を正とし、ここには書かない。  
- 制限事項・非目標は docs/constraints.md に書く。  
- チェックボックス表記: `[ ]` 未着手 / `[x]` 完了。カテゴリは `[設計]` `[MVP]` `[NEXT]` を先頭に付ける。  
- 設計タスクと実装タスクを分けて管理する（設計を先に完了させる）。

# 設計タスク
- [x] [設計][レイヤ配線] transportは入出力＋配送のみ、SIP応答はsip/sessionで組み立てる流れを明文化・図示。
- [x] [設計][イベント設計] SessionIn/SessionOut と app↔ai のイベント種別・チャネル方向を確定。
- [x] [設計][エラーポリシー] SIPトランザクションタイマ、RTP無着信、AI失敗時の動作（再送/謝罪/終了）を決定。
- [x] [設計][sipトランザクション詳細] 状態遷移表と Timer A/B/E/F… の扱い、送信キューI/Fを具体化。
- [x] [設計][rtpストリーム詳細] SSRC/Seq/Timestamp管理と簡易ジッタポリシー、RTCP送受のシグネチャ定義。
- [x] [設計][session詳細] manager API、Session Timerの状態持ち、keepalive/タイムアウト時の SessionOut 定義。
- [x] [設計][app/ai I/F] asr/llm/tts のAPI型（チャネルor Future）、ストリーミングI/O形を決める。
- [x] [設計][テスト計画] INVITE→ACK→RTP往復、トランザクションタイマ、AI失敗フォールバック等のケース列挙。

# 実装タスク

## [MVP]
- [ ] [MVP][transport] SIP応答生成をsip/sessionに委譲し、UDPの受信/配送に専念（100/180/200/BYE/REGISTER即時返信を撤去）。
- [ ] [MVP][sip] レスポンス組み立て＋送信指示の経路を用意し、INVITE/非INVITEトランザクション状態機械とタイマを実装。SessionOutを受けて送信まで繋ぐ。
- [ ] [MVP][session] SessionOutの実配線（SIP送出、RTP開始/停止）を実装し、managerでセッション生成/破棄を一元化。Session Timerの基本処理を追加し、ASR/LLM/TTS処理はapp/aiに移す。
- [ ] [MVP][rtp] 送受信をrtpモジュール経由に統一し、簡易ストリーム管理とpayload type別処理を追加。RTCP用の入口を用意。
- [ ] [MVP][app/ai] appレイヤを新設して対話状態・イベント分配を担当。botロジックをai::{asr,llm,tts}に分割し、チャネル経由でsession↔app↔aiを接続。
- [ ] [MVP][tests/ops] 簡易E2E（INVITE→ACK→RTP往復）のスモークを追加し、基本ログ/メトリクス出力を整備。

## [NEXT]
- [ ] [NEXT][transport] SIP TCPリスナ対応とポートマッピングの期限管理。
- [ ] [NEXT][sip] 100rel/PRACK, UPDATE, Session-Expires/Min-SE（refresher含む）対応、エラーハンドリング強化。
- [ ] [NEXT][rtp] ジッタバッファと再送整列、RTCP SR/RR送受信、PCMU以外のコーデック抽象を拡張。
- [ ] [NEXT][session] 保留/再開、無音・RTP無着信のタイムアウト検知とBYE発火、Keepalive戦略の改善。
- [ ] [NEXT][app/ai] ファイル経由をやめストリーミングI/O化、プロンプト/ポリシー管理の強化、フェイルセーフ応答ポリシー整備。
- [ ] [NEXT][ops] メトリクス/トレースのモジュール別計測、設定バリデーション、グレースフルシャットダウン対応。

## スプリント計画（優先順）
- Sprint 1（配線と基盤を固める）
  1. [ ] [MVP][transport] SIP応答をsip/sessionへ委譲し、UDP受信/配送のみとする。
  2. [ ] [MVP][sip] レスポンス組み立て・送信指示経路、INVITE/非INVITEトランザクションとタイマの骨組み実装。
  3. [ ] [MVP][session] SessionOut配線（SIP送出・RTP開始/停止）、managerによる生成/破棄一元化、Session Timerの基本処理。
  4. [ ] [MVP][tests/ops] INVITE→ACK→RTP往復のスモークと基本ログ/メトリクス整備。
- Sprint 2（メディアと対話の分離）
  1. [ ] [MVP][rtp] 送受信をrtpモジュール経由に統一し、簡易ストリーム管理とRTCP入口を追加。
  2. [ ] [MVP][app/ai] appレイヤ新設、botロジックをai::{asr,llm,tts}に分割しチャネル接続、sessionからAI呼び出しを排除。
  3. [ ] [MVP][session] app/aiイベントとの結線を反映（BotAudio/ASR結果の経路を整理）。
  4. [ ] [MVP][tests/ops] 音声→ASR→LLM→TTS→RTPの1ターンE2E確認。
- Sprint 3（拡張・運用強化）
  1. [ ] [NEXT][transport] SIP TCPリスナとポートマッピング期限管理。
  2. [ ] [NEXT][sip] 100rel/PRACK, UPDATE, Session-Expires/Min-SE（refresher）の対応とエラーハンドリング強化。
  3. [ ] [NEXT][rtp] ジッタバッファ、RTCP SR/RR、PCMU以外のコーデック抽象拡張。
  4. [ ] [NEXT][session] 保留/再開、無音・RTP無着信タイムアウトからのBYE発火、Keepalive改善。
  5. [ ] [NEXT][app/ai] ストリーミングI/O化、プロンプト/ポリシー強化、フェイルセーフ応答ポリシー整備。
  6. [ ] [NEXT][ops] メトリクス/トレース充実、設定バリデーション、グレースフルシャットダウン対応。

## フェーズ別の進め方

### 要件定義（なにを満たすか）
- [SIP/RTP最小要件] INVITE/ACK/BYEとPCMU/8000の単一通話を確実に処理し、RTP往復が10秒以上安定すること。理由: MVPで検証すべきコア通信要件。
- [対話要件] 音声→テキスト→LLM→TTSの1ターン応答を完了できること（失敗時は謝罪定型でフォールバック）。理由: ユースケースの価値を最低限成立させるため。
- [運用要件] ログ/メトリクスがモジュール別に出ること、環境変数でバインドIP/ポートが設定できること。理由: 検証・デプロイを迅速にするため。

### 基本設計（構成・責務を固める）
- [レイヤ配線] transportは入出力とチャネル配送のみ、SIP応答はsip/sessionで組み立てる流れを図示。理由: 責務混在を防ぎ後工程の実装指針にする。
- [イベント設計] SessionIn/SessionOut、app↔aiのイベント種別とチャネル方向を決める。理由: 非同期タスク分割の境界を明確化する。
- [エラーポリシー] SIPトランザクションタイマ、RTP無着信、AI失敗時の動作（再送/謝罪/終了）を選択。理由: 実装優先度とテスト観点を揃える。

### 詳細設計（実装手順・データ構造）
- [sipトランザクション] 状態遷移表とTimer A/B/E/F…の扱い、レスポンス生成の関数群、送信キューのI/Fを具体化。理由: コード化時の迷いを減らす。
- [rtpストリーム] SSRC/Seq/Timestamp管理、ジッタバッファの簡易ポリシー（スキップ/バッファ長）、RTCPの送受信シグネチャを決める。理由: 後からの拡張を容易にする。
- [session] manager API（生成/破棄/検索）とSession Timerの状態持ち方、keepalive/タイムアウト時のSessionOutを定義。理由: main/packetからの直接アクセスを排除するため。
- [app/ai] asr/llm/ttsのI/F（リクエスト/レスポンス型、チャネルかFutureか）、ストリーミングI/O化のデータ形を決める。理由: botモジュール分割をスムーズにする。
- [テスト計画] INVITE→ACK→RTP往復のスモーク、トランザクションタイマ発火、AI失敗時のフォールバックなどのテストケースを列挙。理由: MVP完了の受け入れ基準を明確にする。
