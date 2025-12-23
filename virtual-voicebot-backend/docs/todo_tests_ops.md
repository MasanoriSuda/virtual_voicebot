# docs/todo_tests_ops.md

全体のTODOは `docs/todo.md` を参照。

## 挙動を変えないテスト/ログ整備
- [ ] sipp 用基本シナリオ（INVITE→100/180/200→ACK→短時間RTP送出）のひな型を追加する（例: `test/scenarios/invite_basic.xml`）。
- [ ] ローカル実行手順（サーバ起動→sipp実行）を `docs/tests_e2e_sipp.md` にまとめる（前提/コマンド/期待ログの記載）。
- [ ] SIPp/E2E 手順の正本を決め、`docs/tests_e2e_sipp.md` と `test/README.md` / `AGENTS.md` の参照先・前提（シナリオ配置や実行方法）を整合させる。
- [ ] 既存ログで不足する粒度を洗い出し、モジュール別に最低限のログ行を追加する（transport/sip/session/rtp/app/ai の開始・終了・エラー）。
- [ ] sipp 実行用の簡易スクリプト（例: `scripts/run_sipp_smoke.sh`）を追加し、環境変数でIP/ポートを切り替えられるようにする。
- [ ] テスト資材の配置を整理（`test/scenarios/`, `test/pcap/` など）し、README に参照先を追記する。

## 挙動に影響しうる ops 改修
- [ ] メトリクス枠組みを決め、主要カウンタ/ゲージ/タイマ（セッション数、RTP受信数、AI呼び出し回数/失敗など）の出力ポイントを仕込む。
- [ ] ログレベル/フォーマットのデフォルトを見直し、スモーク時に必要十分な情報が出るよう調整する（冗長すぎないよう確認）。
- [ ] CI/自動化で sipp スモークを回す場合の起動オプション/ポート設定を検討し、必要ならスクリプトにフラグを追加する。
