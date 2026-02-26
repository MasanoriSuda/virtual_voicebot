# ステアリングインデックス（Backend）

> Virtual Voicebot Backend のステアリング（差分仕様）一覧

| 項目 | 値 |
|------|-----|
| ステータス | Active |
| 作成日 | 2026-01-31 |

---

## 概要

本ディレクトリには、**Backend 固有**のステアリング（差分仕様）を格納する。

- ステアリングは「**イシュー単位の変更仕様**」を定義する
- 承認後に本体仕様書（RD/DD/UT等）へマージ

---

## 命名規則

```
STEER-{イシュー番号}_{slug}.md

例:
- STEER-085_clean-architecture.md
- STEER-080_cancel-handling.md
```

---

## ステアリング一覧

| ID | タイトル | ステータス | 関連Issue | 優先度 | 概要 |
|----|---------|----------|----------|--------|------|
| [STEER-085](STEER-085_clean-architecture.md) | クリーンアーキテクチャ移行（ISP準拠 + ファイル分割） | Draft | #52, #65, #85 | P0 | ISP準拠トレイト設計、エンティティ層新設、sip/mod.rs分割、Session分離 |
| [STEER-095](STEER-095_backend-refactoring.md) | Backend 磨き上げ（クリーンアーキテクチャ適合） | Draft | #95 | P1 | 現行実装と設計書の乖離を解消し、クリーンアーキテクチャに適合させる |
| [STEER-096](STEER-096_serversync.md) | Serversync実装（Backend-Frontend 同期機構） | Approved | #96 | P0 | Transactional Outbox Pattern、独立バイナリ、POST /api/ingest/sync + recording-file |
| [STEER-108](STEER-108_sip-core-engine-refactor.md) | 3層アーキテクチャへのリファクタリング | Draft | #108 | P1 | 全モジュールを Protocol/Service/Interface の3層構造に再構成し、依存方向を明確化 |
| [STEER-110](STEER-110_backend-db-design.md) | バックエンド側データベース設計 | Approved | #110 | P0 | PostgreSQL 統合 DB 設計（11テーブル、UUID v7、月次パーティション、Outbox 同期） |
| [STEER-123](STEER-123_recording-outbox-enqueue.md) | 録音データ sync_outbox エンキュー実装（Serversync バグフィックス） | Draft | #123 | P0 | 通話終了時に recording と recording_file を sync_outbox にエンキューするトランザクショナルライト実装 |
| [STEER-143](STEER-143_recording-enhancement.md) | Backend 録音実装強化（Phase 5） | Approved | #143 | P2 | 録音フラグ連動・録音メタデータ管理・Frontend 同期の整合実装 |
| [STEER-203](STEER-203_fix-bleg-rtp-timestamp-double-add.md) | B-leg → A-leg RTP タイムスタンプ二重加算バグ修正 | Approved | #203 | P0 | B2BUA 転送時に align_rtp_clock() と send_payload() でタイムスタンプが二重加算される問題を修正 |
| [STEER-206](STEER-206_fix-bye-during-announce-triggers-transfer.md) | アナウンス送信中の BYE 受信後に転送が誤起動するバグ修正 | Approved | #206 | P0 | cancel_playback() 経由で AppTransferRequest が誤 enqueue される問題を修正 |
| [STEER-224](STEER-224_weather-llm-local.md) | 天気要約 LLM をローカルサーバー設定対応に変更 | Approved | #224 | P1 | weather 要約の LLM 呼び出しを localhost 固定から LLM_LOCAL_SERVER_URL / LLM_LOCAL_MODEL / LLM_LOCAL_TIMEOUT_MS 設定に変更 |
| [STEER-227](STEER-227_announce-remote-audio-fetch.md) | 別マシン構成でのアナウンス音声 HTTP 取得対応 | Approved | #227 | P1 | `map_audio_file_url_to_local_path()` を `FRONTEND_BASE_URL` 経由の HTTP 取得に置き換え、Frontend=PC / Backend=ラズパイ 構成でアナウンス再生を実現する |
| [STEER-229](STEER-229_vb-recording-enabled.md) | VB 録音対応（recording_enabled フラグを尊重する） | Approved | #229 | P1 | `execute_vb()` の hardcoded `false` を `action.recording_enabled` に変更し、VB モードでもボイスボット会話の録音を可能にする |
| [STEER-231](STEER-231_openai-cloud-provider.md) | OpenAI クラウドプロバイダー追加（ASR / LLM / TTS / weather 要約） | Approved | #231 | P1 | OpenAI を cloud 最優先 provider として ASR/LLM/TTS/weather 要約に追加。既存フォールバック（AWS/Gemini/local/raspi）は維持。PoC = 軽量モデル固定 |
| [STEER-235](STEER-235_backend-systemd-service.md) | 通話本体の systemd 常駐化 | Approved | #235 | P1 | `virtual-voicebot-backend` を systemd unit で常駐管理。Rust コード無改修（`KillSignal=SIGINT` で graceful stop 維持）。unit ファイル + EnvironmentFile テンプレート追加 |
| [STEER-236](STEER-236_serversync-systemd-service.md) | serversync の systemd 常駐化 | Approved | #236 | P1 | `serversync` を systemd unit で常駐管理。STEER-235 の設計を踏襲（KillSignal=SIGINT / @@OS_USER@@ / EnvironmentFile）。通話本体と同一 OS ユーザーで録音ファイル権限を揃える |
| [STEER-241](STEER-241_intent-cloud-provider.md) | intent の OpenAI クラウドプロバイダー追加 | Approved | #241 | P1 | intent 分類に OpenAI Cloud ステージを追加（cloud → local → raspi）。STEER-231 の `openai_*_enabled` パターンを踏襲。local/raspi フォールバックは維持 |
| [STEER-246](STEER-246_whisper-docker-service.md) | Whisper Docker Compose 常駐化 | Approved | #246 | P1 | Whisper サーバーを Docker Compose サービスとして常駐化。GET /healthz 追加・model cache volume 永続化・compose ネットワーク内での接続先設定。#245 ダッシュボード死活監視の前提基盤 |
| [STEER-249](STEER-249_streaming-pipeline.md) | LLM ストリーミング ＋ 文単位 TTS 先行再生 | Draft | #249 | P1 | Ollama streaming 受信（stream:true）＋ 文単位 TTS キュー投入で first-audio latency を短縮。ASR 真のストリーミング化は別 Issue へ分割 |
| [STEER-250](STEER-250_asr-streaming.md) | 真の ASR ストリーミング化（第2段階） | Approved | #250 | P1 | 発話中に ASR WebSocket でリアルタイム転写し、発話終了→LLM開始のギャップをほぼゼロにする。#249 完了後に着手 |
| [STEER-251](STEER-251_tts-streaming.md) | 真の TTS ストリーミング化（第3段階） | Draft | #251 | P1 | TTS サービスの streaming API を利用して音声バイトを逐次受信し、文単位の first-audio latency をさらに削減する。#249 完了後に着手 |

---

## ステータス定義

| ステータス | 説明 |
|-----------|------|
| Draft | 作成中 |
| Review | レビュー中 |
| Approved | 承認済み（実装待ち） |
| Merged | 本体仕様書へマージ完了 |

---

## 運用ガイド

ステアリングの作成・運用手順は [GUIDE.md](GUIDE.md) を参照すること。

---

## テンプレート

新規作成時は [TEMPLATE.md](TEMPLATE.md) を使用すること。

---

## 参照

- [プロセス定義書](../process/v-model.md) - §5 ステアリング運用
- [要件仕様インデックス](../requirements/index.md) - RD 一覧
- [詳細設計インデックス](../design/detail/index.md) - DD 一覧（予定）

---

## 変更履歴

| 日付 | バージョン | 変更内容 | 作成者 |
|------|-----------|---------|--------|
| 2026-01-31 | 1.0 | 初版作成 | Claude Code |
| 2026-02-06 | 1.1 | STEER-108 追加 | Claude Code |
| 2026-02-07 | 1.2 | STEER-095, STEER-110 追加 | Claude Code |
| 2026-02-07 | 1.3 | STEER-096, STEER-123 追加（Serversync 実装とバグフィックス） | Claude Code |
| 2026-02-11 | 1.4 | STEER-143 追加（Phase 5: 録音実装強化） | Codex |
| 2026-02-20 | 1.5 | STEER-203 追加（B-leg RTP タイムスタンプ二重加算バグ修正） | Claude Code |
| 2026-02-21 | 1.6 | STEER-206 追加（アナウンス中 BYE 後の転送誤起動バグ修正） | Claude Code |
| 2026-02-21 | 1.7 | STEER-206 ステータス Draft → Approved | Claude Code |
| 2026-02-23 | 1.8 | STEER-224 追加（天気要約 LLM ローカルサーバー設定対応） | Claude Code |
| 2026-02-23 | 1.9 | STEER-224 ステータス Draft → Approved | @MasanoriSuda |
| 2026-02-23 | 2.0 | STEER-227 追加（別マシン構成アナウンス音声 HTTP 取得対応） | Claude Code |
| 2026-02-24 | 2.1 | STEER-229 追加（VB 録音対応）、STEER-227 ステータス Approved に更新 | Claude Code |
| 2026-02-24 | 2.2 | STEER-229 ステータス Draft → Approved | @MasanoriSuda |
| 2026-02-24 | 2.3 | STEER-231 追加（OpenAI クラウドプロバイダー追加） | Claude Sonnet 4.6 |
| 2026-02-24 | 2.4 | STEER-231 ステータス Draft → Approved | @MasanoriSuda |
| 2026-02-24 | 2.5 | STEER-235 追加（通話本体 systemd 常駐化） | Claude Sonnet 4.6 |
| 2026-02-24 | 2.6 | STEER-235 ステータス Draft → Approved | @MasanoriSuda |
| 2026-02-24 | 2.7 | STEER-236 追加（serversync systemd 常駐化） | Claude Sonnet 4.6 |
| 2026-02-24 | 2.8 | STEER-236 ステータス Draft → Approved | @MasanoriSuda |
| 2026-02-24 | 2.9 | STEER-241 追加（intent OpenAI クラウドプロバイダー追加） | Claude Sonnet 4.6 |
| 2026-02-24 | 3.0 | STEER-241 ステータス Draft → Approved | @MasanoriSuda |
| 2026-02-24 | 3.1 | STEER-246 追加（Whisper Docker Compose 常駐化） | Claude Sonnet 4.6 |
| 2026-02-24 | 3.2 | STEER-246 ステータス Draft → Approved | @MasanoriSuda |
| 2026-02-25 | 3.3 | STEER-249 追加（LLM ストリーミング ＋ 文単位 TTS 先行再生） | Claude Sonnet 4.6 |
| 2026-02-26 | 3.4 | STEER-250 追加（真の ASR ストリーミング化・第2段階） | Claude Sonnet 4.6 |
| 2026-02-26 | 3.5 | STEER-250 ステータス Draft → Approved | @MasanoriSuda |
| 2026-02-26 | 3.6 | STEER-251 追加（真の TTS ストリーミング化・第3段階） | Claude Sonnet 4.6 |
