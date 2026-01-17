# AGENTS.md — Codex向け指示書（virtual-voicebot）

このリポジトリは Rust で実装された SIP UAS ベースの音声対話ボットです。
Codex（またはAI）による実装・改修では、以下の規則を**必ず**守ってください。

---

## 1. Single Source of Truth（必須）

- アーキテクチャ / 責務境界の正: `docs/design.md`
- Frontend ↔ Backend API 契約の正: `../docs/contract.md`
- 録音（保存/配信）設計の正: `docs/recording.md`

**コードと docs が矛盾する場合は docs が正**です。  
仕様や責務、フローが変わる修正では **先に docs を更新し、その内容に沿ってコードを変更**してください。

---

## 2. アーキテクチャの依存方向（必須）

依存は下向きのみ：

- `app → session → (sip, rtp) → transport`
- `app → ai`

### 禁止事項（必須）
- `session` から `ai` を直接呼ぶこと（必ず `app` を経由）
- `app/http/ai` が `sip/rtp/transport` を直接触ること（必ず `session` または Port 経由）
- `http` が通話制御（応答コード決定、BYE、RTP制御など）を行うこと
- `media` が配信を行うこと（生成・保存に限定）。配信は `http` の責務。

---

## 3. モジュール間インタフェース（必須）

- モジュール間通信は原則 **イベント駆動**：
  - 非同期チャネル + `enum` イベント + 境界DTO
- イベントには必ず相関IDを含める：
  - `call_id`（必須）
  - メディア系は `stream_id` も併用
- プロトコル内部詳細を上位へ漏らさない：
  - SIPヘッダ（Via/To/From等）
  - RTPのSSRC/Seq/Timestamp 等

迷ったら「イベント化」または「Port（trait）追加」で境界を守る。

---

## 4. 並行処理モデル（必須）

- 原則：**1セッション = 1タスク**（Tokio）
- 共有ロック（巨大Mutex）で全体の整合性を取らない。状態はセッションタスク内に閉じる。
- `tokio::spawn` を場当たり的に増やさず、設計されたタスク境界に従う。

---

## 5. タイムアウト/リトライ（必須）

- 外部I/O（ASR/LLM/TTS、HTTPクライアント等）は **必ず timeout** を設定する。
- リトライは `ai` の Adapter に閉じる（上位は「成功/失敗」だけ扱う）。
- 無限リトライは禁止。回数・バックオフは config 管理。
- ユーザ向けのフォールバック（謝罪・継続・終了判断）は `app` の責務。

---

## 6. Backpressure（必須：MVPデフォルト）

音声ボットは「遅延が伸び続ける」状態が最悪なので、以下をMVPのデフォルトとする。

- RTP → ASR：**最新優先**（古いPCMは破棄して遅延を抑える）
- TTS → RTP：**割り込み優先**（新しい発話が来たら古い送出を停止）
- LLM：**同一セッションで同時実行しない**（in-flightは1つ）

---

## 7. ログ/観測可能性（必須）

- 重要ログには必ず `call_id`（必要なら `session_id` も）を含める。
- 主要イベントは必ずログに残す：
  - CallStarted / CallEnded
  - ASR final
  - LLM 応答
  - TTS 開始/終了
  - BYE 送出
- PII（個人情報）になりうる本文（文字起こし/LLM入力/出力の全文）は、デフォルトでログに出さない。
  - 出す場合はデバッグフラグ等で明示的に制御する。

---

## 8. 変更手順（必須）

変更を実装する際は、必ず次の順序で進める。

1. 変更が影響する docs を特定する（`docs/design.md` など）
2. 仕様/責務/フローが変わる場合は **docs を先に更新**
3. コード変更は小さく、差分を最小化する
4. テストを追加/更新する
   - sip/rtp：ユニットテスト（パース/ビルド、ジッタ整列等）
   - app：状態機械テスト（入力イベント→期待する出力）
   - 必要なら統合テスト（擬似UAC：INVITE→RTP→BYE）
5. ビルド・フォーマット・静的解析が通ることを確認する

---

## 9. 出力要件（推奨）

- どのレイヤ/モジュールを変更したか、理由を docs に紐づけて説明する。
- 迷った場合は依存方向を破らず、イベント/Portの追加を提案する。
- “便利だから直呼び”は禁止（境界侵食を防ぐ）。

## 10. テスト方針

### ディレクトリ方針
- ユニットテスト：`src/**` に `#[cfg(test)]` で同居させる（純粋関数・パーサ・状態遷移など）
- E2E/統合テスト（HTTPサーバ起動、ファイルI/O、Rangeヘッダ等）：リポジトリ直下の `test/` 配下に置く
- 注意：Cargo は `test/` を自動実行しない。必ず `cargo test` で実行できるように **登録**すること。
  - 推奨：`Cargo.toml` の `[[test]]` で `test/*.rs` をテストターゲットとして登録する
  - 代替：`tests/` に薄いハーネスを置いて `include!` で読み込む

### 録音配信（MVP）E2E の目的
録音生成→録音配信（HTTP Range 対応含む）が、MVP前提で正しく動作することを E2E で担保する。

### 受け入れ条件（E2Eで必ず検証すること）
E2E では以下を満たすこと：
- 実サーバを `127.0.0.1:0`（エフェメラルポート）で起動し、`reqwest` でHTTPアクセスする
- temp dir に `storage/recordings/<callId>/mixed.wav` を作り、サーバがそれを配信できること
- 次の条件をすべて満たすこと
  1) HEAD（または GET）で `Accept-Ranges: bytes` が返る
  2) `Range: bytes=0-1023` で `206` / `Content-Range` / `Content-Length=1024`
  3) `Range: bytes=0-`（終端省略）が `206` または `200` で成立する（実装方針により許容）
  4) 不正Range（ファイルサイズ超）で `416`
  5) 存在しない callId で `404`
  6) mixed.wav 以外を許可しない方針の場合、`caller.wav` 等は `404`（または `403`）になることも検証する

### E2Eを成立させるための実装要件
- テストからHTTPサーバを起動できるように、`main.rs` に埋まっている Router/Server 組み立てをライブラリ側に切り出す
  - 例：`http::build_router(...) -> Router` を用意し、`main.rs` はそれを呼ぶだけにする
- テストで `recordings_dir` を tempdir に差し替えられるよう、`Config::for_test()` または builder を用意する
- design.md の依存方向・責務分離を破らない（httpがsession等に依存しない）
- dev-dependencies は最小限（tokio, reqwest, tempfile 程度）

### 完了条件
- `cargo test -q`（または `cargo test --test recording_http_e2e`）でE2Eが通る
- Range/404/416 などのケースがテストで担保される

### SIPp を用いたE2E（段階導入）

SIPp E2E テストの詳細は **[docs/tests_e2e_sipp.md](docs/tests_e2e_sipp.md)** を参照してください。

ここでは要点のみ記載します：

- **シナリオの正**: `test/sipp/sip/scenarios/`
- **compose ファイルの正**: `test/docker-compose.sipp.yml`
- **実行**: `docker compose -f test/docker-compose.sipp.yml up --build --abort-on-container-exit --exit-code-from sipp`
- **成功条件**: SIPp 終了コード 0
