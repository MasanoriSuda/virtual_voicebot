# STEER-235: 通話本体の systemd 常駐化

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-235 |
| タイトル | 通話本体の systemd 常駐化 |
| ステータス | Approved |
| 関連Issue | #235 |
| 優先度 | P1 |
| 作成日 | 2026-02-24 |

---

## 2. ストーリー（Why）

### 2.1 背景

通話本体バイナリ（`virtual-voicebot-backend`）は現状 `run_register.sh` による手動起動を前提としており、サーバー再起動後の自動復旧や常駐監視の仕組みがない。

| 課題 | 詳細 |
|------|------|
| サーバー再起動後の手動復旧 | OS 再起動やプロセスクラッシュ後に手動で起動し直す必要がある |
| 起動スクリプト依存 | `run_register.sh` は `cargo run` 前提で release binary 直接起動に非対応 |
| graceful shutdown の保証なし | `systemd` デフォルトの `SIGTERM` では `main.rs` の `ctrl_c()` ハンドラを通らず REGISTER unregister が実行されない恐れがある（`src/main.rs:212`/`220`） |

### 2.2 目的

`systemd` unit ファイルを追加し、通話本体バイナリを常駐プロセスとして管理する。

- **Rust コード無改修**で実現する（`KillSignal=SIGINT` で既存 graceful stop を活用）
- `EnvironmentFile` 方式で `.env` 管理を systemd と統一する
- `WorkingDirectory` を明示し、録音配信パスと prompt override 読み込みの相対パス依存を保証する
- `serversync` の常駐化は本 Issue スコープ外とする

### 2.3 ユーザーストーリー

```text
As a 運用者
I want to ラズパイ再起動後に通話本体が自動で起動してほしい
So that 手動操作なしで VoIP サービスが復旧する

受入条件:
- [ ] OS 起動後に通話本体プロセスが自動起動する
- [ ] `systemctl stop virtual-voicebot-backend` で graceful stop（REGISTER unregister）が実行される
- [ ] プロセスクラッシュ後に自動再起動する
- [ ] 環境変数は EnvironmentFile から読み込まれる
- [ ] `storage/recordings` への録音配信パスが正しく解決される
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-24 |
| 起票理由 | 通話本体を systemd で常駐化し、再起動後の自動復旧と安定稼働を実現したい |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Sonnet 4.6 |
| 作成日 | 2026-02-24 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "通話本体側の常駐処理を追加する。systemd で実施する。" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | Codex | 2026-02-24 | NG | ①`DATABASE_URL` の SQLite DSN は backend 実装が `PgPool`/`PostgresAdapter` のため不整合（`routing/call_log` 系が無効化される恐れ） ②`RECORDING_HTTP_ADDR` のポートがコード既定値 `18080` とズレ（`src/shared/config/mod.rs:104`） |
| 2 | Codex | 2026-02-24 | OK | 前回指摘解消。`DATABASE_URL` が PostgreSQL DSN に修正、`RECORDING_HTTP_ADDR` が既定値 `18080` に揃い、Q1/Q2 も確認済み |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-24 |
| 承認コメント | lgtm |

### 3.5 実装

| 項目 | 値 |
|------|-----|
| 実装者 | Codex |
| 実装日 | 2026-02-24 |
| 指示者 | @MasanoriSuda |
| 指示内容 | 承認済み STEER-235 に基づき、通話本体の systemd unit テンプレートと `.env.systemd.example` を追加する |
| コードレビュー | 未実施（systemd unit / env テンプレート追加のみ、動作確認は §7.2 で実機確認） |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | - |
| マージ日 | - |
| マージ先 | `virtual-voicebot-backend/systemd/virtual-voicebot-backend.service`、`virtual-voicebot-backend/.env.systemd.example` |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| `docs/steering/STEER-235_backend-systemd-service.md`（本ファイル） | 新規 | 本 Issue の差分仕様 |

> **参考:** `STEER-096_serversync.md:448` に serversync の systemd 記述があるが `WorkingDirectory` / `EnvironmentFile` / `KillSignal` が省略されており、本 STEER では全項目を明示する。

### 4.2 影響するコード

| ファイル | 変更種別 | 概要 |
|---------|---------|------|
| `virtual-voicebot-backend/systemd/virtual-voicebot-backend.service` | 新規追加 | 通話本体 systemd unit ファイル |
| `virtual-voicebot-backend/.env.systemd.example` | 新規追加 | systemd 用 EnvironmentFile テンプレート（shell 構文なし） |
| Rust コード（`src/main.rs` 等） | **変更なし** | `KillSignal=SIGINT` で既存 `ctrl_c()` ハンドラを活用するため Rust 改修は不要 |

---

## 5. 差分仕様（What / How）

### 5.1 設計方針

| 決定事項 | 採用方針 | 根拠 |
|---------|---------|------|
| Rust コード改修 | **しない**（`KillSignal=SIGINT` で対応） | `main.rs:212` は `tokio::signal::ctrl_c()` = SIGINT 待ち。SIGINT を送れば既存 graceful stop（`sip_core.shutdown()` → unregister）が動作する |
| env ファイル形式 | `EnvironmentFile=`（shell 構文なし） | `run_register.sh` は `export VAR=value` 形式だが `systemd` は `VAR=value` のみを解釈する。専用 example ファイルを用意する |
| `WorkingDirectory` | `virtual-voicebot-backend` ディレクトリに固定 | 録音静的配信のパス（`src/main.rs:125`）、prompt override 読み込み（`src/service/ai/llm.rs:39`、`intent.rs:58`、`weather.rs:364`）が `current_dir()` 相対のため |
| OS 実行ユーザー | **PoC: 既存ログインユーザー**（`User=msuda`/`Group=msuda`）。本番は専用サービスユーザー（例: `voicebot`）を推奨（CodeRabbit 指摘） | `SIP_PORT(5060)` / `RTP_PORT(10000)` はいずれも 1024 超のため root 権限不要。最小権限の原則により非 root ユーザーで実行する。テンプレートでは `@@OS_USER@@` プレースホルダーで吸収する |
| serversync | **スコープ外** | 本 Issue は通話本体のみ |

### 5.2 systemd unit ファイル（`virtual-voicebot-backend/systemd/virtual-voicebot-backend.service`）

```ini
[Unit]
Description=Virtual VoiceBot Backend (call service)
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
# @@INSTALL_DIR@@ / @@ENV_FILE@@ / @@OS_USER@@ を実際の値に置換してから使用すること（§5.4 の sed コマンド参照）
# PoC: 既存ログインユーザーを使用。本番は専用ユーザー（voicebot 等）を推奨
User=@@OS_USER@@
Group=@@OS_USER@@
ExecStart=@@INSTALL_DIR@@/target/release/virtual-voicebot-backend
WorkingDirectory=@@INSTALL_DIR@@
EnvironmentFile=@@ENV_FILE@@
Restart=always
RestartSec=2
# SIGTERM ではなく SIGINT を送ることで main.rs の ctrl_c() ハンドラ経由の graceful stop を保証する
KillSignal=SIGINT
TimeoutStopSec=10

[Install]
WantedBy=multi-user.target
```

> **Note:** `@@INSTALL_DIR@@` は backend ディレクトリの絶対パス、`@@ENV_FILE@@` は EnvironmentFile の絶対パス、`@@OS_USER@@` は実行 OS ユーザー名に置換する（§5.4 参照）。PoC 環境では既存ログインユーザー（例: `msuda`）をそのまま使用可能。本番環境では専用サービスユーザーを作成して使用することを推奨（`systemd-analyze security` の「Service runs as root user」警告を回避）。

### 5.3 EnvironmentFile テンプレート（`virtual-voicebot-backend/.env.systemd.example`）

```dotenv
# systemd EnvironmentFile（shell 構文不可: export / $VAR 参照 不可）
# 実際の値を設定した .env.systemd として配置すること
# 変数名は run_register.sh および src/shared/config/mod.rs を参照（Codex 調査 2026-02-24）

# ネットワーク（run_register.sh 対応）
SIP_BIND_IP=192.168.x.x
SIP_PORT=5060
RTP_PORT=10000
LOCAL_IP=192.168.x.x
ADVERTISED_IP=192.168.x.x

# REGISTER（src/shared/config/mod.rs:336-356 対応）
REGISTRAR_HOST=192.168.x.x
REGISTRAR_PORT=5060
REGISTRAR_TRANSPORT=UDP
REGISTER_USER=xxxx
REGISTER_DOMAIN=192.168.x.x
REGISTER_EXPIRES=3600
REGISTER_AUTH_USER=xxxx
REGISTER_AUTH_PASSWORD=xxxx
# 任意: contact ヘッダのオーバーライド（mod.rs:345-349）
# REGISTER_CONTACT_HOST=192.168.x.x
# REGISTER_CONTACT_PORT=5060

# 録音 HTTP 配信（既定値 18080、src/shared/config/mod.rs:104）
RECORDING_HTTP_ADDR=0.0.0.0:18080

# ログ
RUST_LOG=info

# DB（routing / call_log 系。backend は PgPool/PostgresAdapter を使用。未設定時は無効化）
# src/interface/db/postgres.rs:6,42,48 / src/main.rs:73 参照
DATABASE_URL=postgres://voicebot:voicebot_dev@localhost:5432/voicebot

# Frontend 連携（STEER-227: アナウンス音声 HTTP 取得が必要な場合のみ設定）
# FRONTEND_BASE_URL=http://192.168.x.x:3000

# OpenAI（STEER-231 対応の場合のみ設定）
# OPENAI_API_KEY=sk-...
# OPENAI_BASE_URL=https://api.openai.com/v1
# OPENAI_ASR_ENABLED=true
# OPENAI_LLM_ENABLED=true
# OPENAI_TTS_ENABLED=true
# TTS_CLOUD_TIMEOUT_MS=10000

# LLM（ローカルサーバー経由）
# LLM_LOCAL_SERVER_URL=http://localhost:11434
# LLM_LOCAL_MODEL=llama3
# LLM_LOCAL_TIMEOUT_MS=30000
```

> **Note:** `run_register.sh` で使用している変数のうち通話本体に必要なものを網羅すること。`export` や `$VAR` 参照構文は systemd の `EnvironmentFile` では解釈されないため使用不可。

### 5.4 インストール・運用手順（README / 手順書への追記案）

```bash
# ビルド
cd virtual-voicebot-backend
cargo build --release

# EnvironmentFile の準備
cp .env.systemd.example .env.systemd
vi .env.systemd  # 実際の値を設定

# OS ユーザーの決定
# PoC: 既存ログインユーザーをそのまま使用
OS_USER=$(whoami)
# 本番推奨: 専用サービスユーザーを作成（CodeRabbit 指摘: 最小権限の原則）
# sudo useradd -r -s /sbin/nologin voicebot
# sudo chown -R voicebot:voicebot ${INSTALL_DIR}
# OS_USER=voicebot

# unit ファイルのプレースホルダーを実機パスで置換してインストール
INSTALL_DIR=$(pwd)
ENV_FILE="${INSTALL_DIR}/.env.systemd"
sed -e "s|@@INSTALL_DIR@@|${INSTALL_DIR}|g" \
    -e "s|@@ENV_FILE@@|${ENV_FILE}|g" \
    -e "s|@@OS_USER@@|${OS_USER}|g" \
    systemd/virtual-voicebot-backend.service \
  | sudo tee /etc/systemd/system/virtual-voicebot-backend.service > /dev/null
sudo systemctl daemon-reload

# 起動
sudo systemctl enable virtual-voicebot-backend
sudo systemctl start virtual-voicebot-backend

# 状態確認
sudo systemctl status virtual-voicebot-backend
journalctl -u virtual-voicebot-backend -f

# 停止（graceful: SIGINT → ctrl_c() ハンドラ → unregister）
sudo systemctl stop virtual-voicebot-backend
```

### 5.5 SIGTERM 対応の将来方針（本 Issue スコープ外）

現状は `KillSignal=SIGINT` で対応するが、将来的には `main.rs` に `SIGTERM` ハンドラを追加して `systemd` 標準の停止シグナルに対応することを推奨する。対応時は別 Issue で実施する。

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #235 | STEER-235 | 起票 |
| STEER-096:448 | STEER-235 | serversync 側の systemd 記述を参考・本 STEER で不足項目を補完 |
| STEER-235 | `systemd/virtual-voicebot-backend.service` | unit ファイル追加 |
| STEER-235 | `.env.systemd.example` | EnvironmentFile テンプレート追加 |
| STEER-235 | `src/main.rs:212` | `ctrl_c()` = SIGINT 待ちの確認（Rust 無改修の根拠） |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] `KillSignal=SIGINT` で `main.rs:212` の `ctrl_c()` ハンドラが正常に動作し graceful stop できることの根拠が明確か
- [ ] `WorkingDirectory` 依存の全パス（録音配信・prompt override）が unit ファイルと整合しているか
- [ ] `EnvironmentFile` が shell 構文を排除しており、`systemd` で正しく読み込まれることを確認しているか
- [x] `run_register.sh` で使用している必要な環境変数が `.env.systemd.example` に漏れなく記載されているか（§8 Q1 確認済み・§5.3 修正済み）
- [ ] `Restart=always` / `RestartSec=2` / `TimeoutStopSec=10` が運用要件に合っているか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] `systemctl start` で通話本体が正常起動することを確認している
- [ ] `systemctl stop` で REGISTER unregister ログが出力されることを確認している（graceful stop 確認）
- [ ] OS 再起動後に自動起動することを確認している（`systemctl enable` 設定確認）
- [ ] プロセスクラッシュ後に `Restart=always` で自動再起動することを確認している
- [ ] `storage/recordings` への録音配信が unit 経由起動でも正常に動作することを確認している

---

## 8. 未確定点・質問

| # | 質問 | 選択肢 | 推奨 | オーナー回答 |
|---|------|--------|------|-------------|
| Q1 | `run_register.sh` で使用している環境変数のうち、通話本体（`virtual-voicebot-backend`）が必要とするものを `.env.systemd.example` に全て列挙できているか | 全て列挙済み / 不足あり | `run_register.sh:78` を起点にして Codex が変数を網羅したか確認が必要。不足がある場合は実装時に追記 | **Codex 調査結果（2026-02-24）: 不足あり・誤名あり**。初版の `SIP_SERVER`/`SIP_USERNAME`/`SIP_PASSWORD`/`SIP_DOMAIN`/`LOCAL_PORT` は backend 実装と不一致。正しい変数は `SIP_BIND_IP`/`RTP_PORT`/`ADVERTISED_IP`/`RECORDING_HTTP_ADDR`/`REGISTRAR_*`/`REGISTER_*`/`REGISTER_CONTACT_*`/`DATABASE_URL`/`RUST_LOG`（`src/shared/config/mod.rs:336-356`、`run_register.sh:67-69` 根拠）。§5.3 を修正済み |
| Q2 | unit ファイルの `ExecStart` / `WorkingDirectory` / `EnvironmentFile` のパス表記は実機環境（ラズパイ）の実際のインストールパスに合わせてハードコードするか、変数プレースホルダーにするか | 実機パスをハードコード（`/home/msuda/...` 等） / プレースホルダー（`@@INSTALL_DIR@@` 等） | **プレースホルダーを推奨**。ユーザーが `sed` 等で置換できる形にするとポータブル | **プレースホルダー確定**（`@@INSTALL_DIR@@` / `@@ENV_FILE@@`）。§5.2 を修正済み。§5.4 に `sed` 置換コマンドを追加済み |

---

## 9. リスク・ロールバック観点

| リスク | 影響 | 緩和策 |
|--------|------|--------|
| `KillSignal=SIGINT` で graceful stop が完了しない（タイムアウト） | `TimeoutStopSec=10` 経過後に `SIGKILL` で強制終了。REGISTER unregister が不完全になる可能性 | `TimeoutStopSec` を調整。ログで unregister 完了を確認。将来的に `SIGTERM` 対応（§5.5）で根本解消 |
| `EnvironmentFile` 内の変数名誤りや shell 構文混入 | 起動失敗（`journalctl` にエラー出力） | `.env.systemd.example` のコメントで shell 構文禁止を明示。`systemctl status` / `journalctl` で確認可能 |
| `WorkingDirectory` が存在しない場合の起動失敗 | サービス起動失敗 | unit ファイルに実際のパスを設定していることをインストール手順で確認させる |
| `run_register.sh` との二重起動 | SIP 登録の競合 | `run_register.sh` を使用する場合は `systemctl stop` 後に実行するよう手順書に明記 |

**ロールバック手順:** `sudo systemctl disable virtual-voicebot-backend && sudo systemctl stop virtual-voicebot-backend` で即時無効化。unit ファイルを削除すれば完全ロールバック可能。

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-24 | 初版作成（Codex 調査結果を元に差分仕様を記述） | Claude Sonnet 4.6 |
| 2026-02-24 | §8 Q1/Q2 オーナー回答記録、§5.2 プレースホルダー化（`@@INSTALL_DIR@@`/`@@ENV_FILE@@`）、§5.3 変数名全面修正（実装正名へ）、§5.4 sed 置換手順追加 | Claude Sonnet 4.6 |
| 2026-02-24 | §3.3 Round 1 NG 記録、①`DATABASE_URL` を PostgreSQL DSN に修正②`RECORDING_HTTP_ADDR` ポートを既定値 `18080` に修正 | Claude Sonnet 4.6 |
| 2026-02-24 | §1 ステータス Draft → Approved、§3.4 承認者記録 | @MasanoriSuda |
| 2026-02-24 | §5.1 OS ユーザー決定行追加、§5.2 `User=`/`Group=` ディレクティブ追加（`@@OS_USER@@` プレースホルダー）、§5.4 `@@OS_USER@@` 置換手順・専用ユーザー作成コメント追加（CodeRabbit 指摘対応） | Claude Sonnet 4.6 |
