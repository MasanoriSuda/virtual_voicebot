# はじめに
このファイルは「設計/実装タスクと優先度」を管理する。  
- 設計の詳細は docs/design.md を正とし、ここには書かない。  
- 制約/非目標は docs/design.md と docs/contract.md に集約する。  
- チェックボックス表記: `[ ]` 未着手 / `[x]` 完了。カテゴリは `[設計]` `[MVP]` `[NEXT]` を先頭に付ける。  
- 設計タスクと実装タスクを分けて管理する（設計を先に完了させる）。

# ドキュメントの役割
- `README.md`: 各ディレクトリの現行仕様/使い方の入口（`Readme.md` は使わない）。
- `docs/todo.md`: 全体の TODO と優先度。
- `docs/todo_*.md`: モジュール別の TODO（詳細タスク）。

# 設計タスク
る。
# 実装タスク

## [NEXT]
- [ ] [NEXT][session] 保留/再開、無音・RTP無着信のタイムアウト検知とBYE発火、Keepalive戦略の改善。
- [ ] [NEXT][app/ai] ファイル経由をやめストリーミングI/O化、プロンプト/ポリシー管理の強化、フェイルセーフ応答ポリシー整備。
- [ ] [NEXT][app/ai] チャンク/ストリーミングI/Oの実装（ASR/LLM/TTSをリアルタイム呼び出しに置き換え、既存バッチI/Fを置き換える）
- [ ] [NEXT][ops] メトリクス/トレースのモジュール別計測、設定バリデーション、グレースフルシャットダウン対応。
- [ ] [NEXT][tests/ops] RTP往復のE2E（SIP→RTP→BYE）を段階導入し、実運用に近いシナリオで検証。
- [ ] [NEXT][recording] caller/bot 分離トラック（caller.wav, bot.wav）とメタ同期の精度向上（startSec/endSec の運用を本格化）。
- [ ] [NEXT][storage] 録音を外部ストレージ（S3/MinIO等）へ移行し、recordingUrl を署名付きURLにする。
- [ ] [NEXT][http] 認証/認可（Bearer等）導入、録音URLの寿命運用、CORS/CSRF方針整備。
- [ ] [NEXT][ops] 録音容量の保持期間・削除ポリシー・圧縮/エンコード（mp3/opus）戦略。

## スプリント計画（優先順）
- Sprint 1（配線と基盤を固める）
  1. [x] [MVP][transport] SIP応答をsip/sessionへ委譲し、UDP受信/配送のみとする。
  2. [x] [MVP][sip] レスポンス組み立て・送信指示経路、INVITE/非INVITEトランザクションとタイマの骨組み実装。
  3. [x] [MVP][session] SessionOut配線（SIP送出・RTP開始/停止）、managerによる生成/破棄一元化、Session Timerの基本処理。
  4. [x] [MVP][tests/ops] SIPのみE2E（SIPp: INVITE→ACK→BYE）のスモークと基本ログ/メトリクス整備。
- Sprint 2（メディアと対話の分離）
  1. [x] [MVP][rtp] 送受信をrtpモジュール経由に統一し、簡易ストリーム管理とRTCP入口を追加。
  2. [x] [MVP][app/ai] appレイヤ新設、botロジックをai::{asr,llm,tts}に分割しチャネル接続、sessionからAI呼び出しを排除。
  3. [x] [MVP][session] app/aiイベントとの結線を反映（BotAudio/ASR結果の経路を整理）。
  4. [x] [MVP][tests/ops] recordingUrl の Range 対応E2E（HTTP）のリグレッション整備。
- Sprint 3（拡張・運用強化）
  1. [x] [NEXT][transport] SIP TCPリスナと接続（peer/conn）のidle timeout管理。
  2. [ ] [NEXT][sip] 100rel/PRACK, UPDATE, Session-Expires/Min-SE（refresher）の対応とエラーハンドリング強化。
  3. [ ] [NEXT][rtp] ジッタバッファ、RTCP SR/RR、PCMU以外のコーデック抽象拡張。
  4. [ ] [NEXT][session] 保留/再開、無音・RTP無着信タイムアウトからのBYE発火、Keepalive改善。
  5. [ ] [NEXT][app/ai] ストリーミングI/O化、プロンプト/ポリシー強化、フェイルセーフ応答ポリシー整備。
  6. [ ] [NEXT][ops] メトリクス/トレース充実、設定バリデーション、グレースフルシャットダウン対応。
  7. [ ] [NEXT][tests/ops] RTP往復のE2E（SIP→RTP→BYE）の段階導入。

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
