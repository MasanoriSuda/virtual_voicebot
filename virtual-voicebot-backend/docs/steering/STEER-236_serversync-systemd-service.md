# STEER-236: serversync の systemd 常駐化

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-236 |
| タイトル | serversync の systemd 常駐化 |
| ステータス | Approved |
| 関連Issue | #236 |
| 優先度 | P1 |
| 作成日 | 2026-02-24 |

---

## 2. ストーリー（Why）

### 2.1 背景

serversync バイナリ（`target/release/serversync`）は現状 `cargo run --bin serversync` による手動起動を前提としており、サーバー再起動後の自動復旧や常駐監視の仕組みがない。

| 課題 | 詳細 |
|------|------|
| サーバー再起動後の手動復旧 | OS 再起動やプロセスクラッシュ後に手動で起動し直す必要がある |
| 起動スクリプト依存 | `cargo run` 前提で release binary 直接起動に非対応 |
| graceful shutdown の保証なし | systemd デフォルトの `SIGTERM` では `serversync.rs` の `ctrl_c()` ハンドラを通らず、同期処理が中途半端な状態で終了する恐れがある（`src/bin/serversync.rs:39`） |
| 録音ファイルの権限不整合リスク | serversync は録音ファイルを読み込みアップロード後に削除するため、通話本体と OS ユーザーが違うと permission エラーが発生しやすい |

> **前例:** STEER-096 には serversync の systemd 例（`STEER-096_serversync.md:448`）が存在するが、`WorkingDirectory` / `EnvironmentFile` / `KillSignal` / `User` が省略されており本 STEER で追補する。

### 2.2 目的

`systemd` unit ファイルを追加し、serversync バイナリを常駐プロセスとして管理する。

- **Rust コード無改修**で実現する（`KillSignal=SIGINT` で既存 graceful stop を活用）
- `EnvironmentFile` 方式で env 管理を systemd と統一する（STEER-096 の `Environment=` 直書きを廃止）
- `WorkingDirectory` を明示し、録音ファイルの相対パス解決を保証する
- 通話本体（STEER-235）と**同じ OS ユーザー**で実行し、録音ファイルの read/delete 権限を揃える

### 2.3 ユーザーストーリー

```text
As a 運用者
I want to ラズパイ再起動後に serversync が自動で起動してほしい
So that 手動操作なしで Frontend への録音データ同期が復旧する

受入条件:
- [ ] OS 起動後に serversync プロセスが自動起動する
- [ ] `systemctl stop serversync` で graceful stop が実行される
- [ ] プロセスクラッシュ後に自動再起動する
- [ ] 環境変数は EnvironmentFile から読み込まれる
- [ ] 録音ファイルの read/delete が通話本体と同じ OS ユーザー権限で正常に動作する
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-24 |
| 起票理由 | serversync を systemd で常駐化し、再起動後の自動復旧と録音同期の安定稼働を実現したい |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Sonnet 4.6 |
| 作成日 | 2026-02-24 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "#235 の serversync バージョン。Codex 調査結果を踏まえてステアリングを作成してほしい" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | Codex | 2026-02-24 | NG | ①`virtual-voicebot-backend/.gitignore` は誤り→ルート `.gitignore` が正しい。unignore パスも `!.env.serversync.systemd.example` → `!virtual-voicebot-backend/.env.serversync.systemd.example` に修正が必要 ②`recording_uploader.rs`/`worker.rs`/`serversync.rs` の省略パスが実ファイルパスと不一致（`src/interface/sync/...` / `src/bin/serversync.rs` に統一が必要） |
| 2 | Codex | 2026-02-24 | NG | Round 1 の `.gitignore`・`worker.rs`・`recording_uploader.rs` 指摘は解消済み。`src/serversync.rs` がまだ実ファイルパス `src/bin/serversync.rs` と不一致（§2.1/§4.2/§5.1/§5.3/§5.5/§6/§7.1 全箇所） |
| 3 | Codex | 2026-02-24 | OK | 前回残件解消。`src/bin/serversync.rs` への統一・`.gitignore` ルート指定・`src/interface/sync/...` パス整合、すべて確認済み |

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
| 指示内容 | 承認済み STEER-236 に基づき、serversync 用 systemd unit テンプレートと `.env.serversync.systemd.example` を追加し、ルート `.gitignore` に sample の unignore を追記する |
| コードレビュー | 未実施（systemd unit / env テンプレート追加のみ、動作確認は §7.2 で実機確認） |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | - |
| マージ日 | - |
| マージ先 | `virtual-voicebot-backend/systemd/serversync.service`、`virtual-voicebot-backend/.env.serversync.systemd.example` |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| `docs/steering/STEER-236_serversync-systemd-service.md`（本ファイル） | 新規 | 本 Issue の差分仕様 |

> **参考:** `STEER-096_serversync.md:448` に serversync の systemd 記述があるが `WorkingDirectory` / `EnvironmentFile` / `KillSignal` / `User` が省略されており、本 STEER では全項目を明示する。

### 4.2 影響するコード

| ファイル | 変更種別 | 概要 |
|---------|---------|------|
| `virtual-voicebot-backend/systemd/serversync.service` | 新規追加 | serversync systemd unit ファイル |
| `virtual-voicebot-backend/.env.serversync.systemd.example` | 新規追加 | systemd 用 EnvironmentFile テンプレート（shell 構文なし） |
| `.gitignore`（ルート） | 修正 | `.env.*` パターンに対する unignore エントリ追加（`!virtual-voicebot-backend/.env.serversync.systemd.example`）。ルート `.gitignore` line 22 の `!virtual-voicebot-backend/.env.systemd.example` と同様の対応 |
| Rust コード（`src/bin/serversync.rs` 等） | **変更なし** | `KillSignal=SIGINT` で既存 `ctrl_c()` ハンドラを活用するため Rust 改修は不要 |

---

## 5. 差分仕様（What / How）

### 5.1 設計方針

| 決定事項 | 採用方針 | 根拠 |
|---------|---------|------|
| Rust コード改修 | **しない**（`KillSignal=SIGINT` で対応） | `src/bin/serversync.rs:39` は `tokio::signal::ctrl_c()` = SIGINT 待ち。SIGINT を送れば graceful stop が動作する |
| env ファイル形式 | `EnvironmentFile=`（shell 構文なし） | STEER-096 の `Environment=` 直書きは管理しにくく、systemd は `VAR=value` のみ解釈する。専用 example ファイルを用意する |
| `WorkingDirectory` | `virtual-voicebot-backend` ディレクトリに固定 | 録音ファイルの相対パス解決が `current_dir()` に依存（`src/interface/sync/worker.rs:249`） |
| OS 実行ユーザー | **通話本体（STEER-235）と同じ `@@OS_USER@@`** | serversync は録音ファイルを読み込み（`src/interface/sync/recording_uploader.rs:60`）・削除（`src/interface/sync/worker.rs:133, 150, 268`）するため、通話本体と OS ユーザーが違うと permission エラーが発生する |
| 依存関係（`After=`） | `After=virtual-voicebot-backend.service` を追加（§8 Q1 で確認） | 起動順の保証が望ましい。ただし `Requires=` / `PartOf=` は付けない（独立起動・独立停止を維持。STEER-096:467 の受入条件に準拠） |
| `serversync` | **スコープ内**（本 Issue が対象） | - |

### 5.2 systemd unit ファイル（`virtual-voicebot-backend/systemd/serversync.service`）

```ini
[Unit]
Description=Virtual VoiceBot Serversync (recording sync worker)
After=network-online.target
Wants=network-online.target
# 通話本体が先に起動していることが望ましいが、独立起動も許容する（Requires/PartOf は付けない）
After=virtual-voicebot-backend.service

[Service]
Type=simple
# Replace @@OS_USER@@ with a non-root OS user/group on the target machine.
# 通話本体（virtual-voicebot-backend.service）と同じ OS user を使うこと（録音ファイルの権限を揃えるため）
User=@@OS_USER@@
Group=@@OS_USER@@
# Replace @@INSTALL_DIR@@ with the absolute path to virtual-voicebot-backend.
ExecStart=@@INSTALL_DIR@@/target/release/serversync
WorkingDirectory=@@INSTALL_DIR@@
EnvironmentFile=@@SERVERSYNC_ENV_FILE@@
Restart=always
RestartSec=2
# Send SIGINT so the existing ctrl_c() handler performs graceful shutdown.
KillSignal=SIGINT
TimeoutStopSec=10

[Install]
WantedBy=multi-user.target
```

> **Note:** プレースホルダーは §5.4 の `sed` コマンドで置換する。`@@OS_USER@@` は STEER-235 の通話本体 unit と必ず同じ値を設定すること（録音ファイルの read/delete 権限を揃えるため）。

### 5.3 EnvironmentFile テンプレート（`virtual-voicebot-backend/.env.serversync.systemd.example`）

```dotenv
# systemd EnvironmentFile（shell 構文不可: export / $VAR 参照 不可）
# 実際の値を設定したファイルとして配置すること
# 変数名は src/shared/config/mod.rs を参照（Codex 調査 2026-02-24）

# 必須: DB 接続（routing / sync_outbox 系。backend は PgPool/PostgresAdapter を使用）
DATABASE_URL=postgres://voicebot:voicebot_dev@localhost:5432/voicebot

# 必須: Frontend への同期送信先
FRONTEND_BASE_URL=http://192.168.x.x:3000

# 推奨（デフォルトあり、明示推奨）
# src/shared/config/mod.rs:508-516 参照
SYNC_POLL_INTERVAL_SEC=300
FRONTEND_SYNC_INTERVAL_SEC=60
SYNC_BATCH_SIZE=100
SYNC_TIMEOUT_SEC=30

# ログ
RUST_LOG=info
```

> **Note:** `export` や `$VAR` 参照構文は systemd の `EnvironmentFile` では解釈されないため使用不可。`DATABASE_URL` と `FRONTEND_BASE_URL` は未設定だと serversync が起動に失敗する（`src/bin/serversync.rs:12,13`）。

### 5.4 インストール・運用手順（README / 手順書への追記案）

```bash
# ビルド（通話本体と同じ release build に serversync が含まれる）
cd virtual-voicebot-backend
cargo build --release

# EnvironmentFile の準備
cp .env.serversync.systemd.example .env.serversync.systemd
# cp は umask（通常 022）を引き継ぐため生成直後は 644（world-readable）になる。
# DATABASE_URL 等の機密情報を保護するため必ず以下の chmod を実行すること。
chmod 600 .env.serversync.systemd
vi .env.serversync.systemd  # 実際の値を設定

# @@OS_USER@@ に設定する非 root OS ユーザーを決定する
# 通話本体（STEER-235）と同じ OS ユーザーを使うこと（録音ファイルの権限を揃えるため）
OS_USER=$(whoami)
# root セッションで実行した場合は User=root になり非 root 要件を違反するため中断する
if [ "${OS_USER}" = "root" ]; then
  echo "ERROR: OS_USER=root は使用できません。非 root ユーザーで再実行してください。" >&2
  echo "       専用ユーザーを使う場合は以下を実行してから OS_USER を設定してください:" >&2
  echo "         sudo useradd -r -s /sbin/nologin voicebot" >&2
  echo "         sudo chown -R voicebot:voicebot \${INSTALL_DIR}" >&2
  echo "         OS_USER=voicebot" >&2
  exit 1
fi

# unit ファイルのプレースホルダーを実機パスで置換してインストール
INSTALL_DIR=$(pwd)
SERVERSYNC_ENV_FILE="${INSTALL_DIR}/.env.serversync.systemd"
sed -e "s|@@INSTALL_DIR@@|${INSTALL_DIR}|g" \
    -e "s|@@SERVERSYNC_ENV_FILE@@|${SERVERSYNC_ENV_FILE}|g" \
    -e "s|@@OS_USER@@|${OS_USER}|g" \
    systemd/serversync.service \
  | sudo tee /etc/systemd/system/serversync.service > /dev/null
sudo systemctl daemon-reload

# 起動
sudo systemctl enable serversync
sudo systemctl start serversync

# 状態確認
sudo systemctl status serversync
journalctl -u serversync -f

# 停止（graceful: SIGINT → ctrl_c() ハンドラ）
sudo systemctl stop serversync
```

### 5.5 SIGTERM 対応の将来方針（本 Issue スコープ外）

現状は `KillSignal=SIGINT` で対応するが、将来的には `src/bin/serversync.rs` に `SIGTERM` ハンドラを追加して systemd 標準の停止シグナルに対応することを推奨する。対応時は別 Issue で実施する。

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #236 | STEER-236 | 起票 |
| STEER-235 | STEER-236 | 通話本体 systemd 化の serversync 版（OS ユーザー・プレースホルダー方式・root ガードを踏襲） |
| STEER-096:448 | STEER-236 | 旧 systemd 例を参考・不足項目（WorkingDirectory/EnvironmentFile/KillSignal/User）を補完 |
| STEER-236 | `systemd/serversync.service` | unit ファイル追加 |
| STEER-236 | serversync 用 EnvironmentFile テンプレート | env テンプレート追加 |
| STEER-236 | `src/bin/serversync.rs:39` | `ctrl_c()` = SIGINT 待ちの確認（Rust 無改修の根拠） |
| STEER-236 | `src/interface/sync/worker.rs:249` | 相対パス解決が cwd 依存（WorkingDirectory 固定の根拠） |
| STEER-236 | `src/interface/sync/recording_uploader.rs:60`, `src/interface/sync/worker.rs:133,150,268` | 録音ファイル read/delete（OS ユーザー統一の根拠） |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] `KillSignal=SIGINT` で `src/bin/serversync.rs:39` の `ctrl_c()` ハンドラが正常に動作し graceful stop できることの根拠が明確か
- [ ] `WorkingDirectory` が録音ファイルの相対パス解決（`src/interface/sync/worker.rs:249`）と整合しているか
- [ ] `EnvironmentFile` が shell 構文を排除しており systemd で正しく読み込まれることを確認しているか
- [ ] `User=`/`Group=` が通話本体（STEER-235）と同じ OS ユーザーになることが手順書で明記されているか
- [ ] `DATABASE_URL` と `FRONTEND_BASE_URL` が必須 env として明示されているか
- [x] `After=virtual-voicebot-backend.service` の `Requires=`/`PartOf=` なし方針（独立起動）が STEER-096 受入条件と整合しているか（§8 Q1 確認後に check）
- [ ] .gitignore の `.env.*` パターンと env テンプレートファイル名の関係が §8 Q2 で解決されているか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] `systemctl start serversync` で serversync が正常起動することを確認している
- [ ] `systemctl stop serversync` で graceful stop ログが出力されることを確認している
- [ ] OS 再起動後に自動起動することを確認している（`systemctl enable` 設定確認）
- [ ] プロセスクラッシュ後に `Restart=always` で自動再起動することを確認している
- [ ] 通話本体と同じ OS ユーザーで録音ファイルの read/delete が正常に動作することを確認している

---

## 8. 未確定点・質問

| # | 質問 | 選択肢 | 推奨 | オーナー回答 |
|---|------|--------|------|-------------|
| Q1 | `serversync.service` に `After=virtual-voicebot-backend.service` を追加するか | 追加する / 追加しない | **追加を推奨**（起動順の保証が望ましい。ただし `Requires=`/`PartOf=` は付けず独立停止を維持） | **追加（Requires/PartOf なし）**。同時起動時の順序だけ整え、独立起動・独立停止の思想を維持する（STEER-096 受入条件と整合）。§5.2 に反映済み |
| Q2 | serversync 用 EnvironmentFile テンプレートのファイル名をどうするか | `.env.serversync.systemd.example`（#235 と統一感あり、ルート `.gitignore` の `.env.*` パターンで無視されるためルート `.gitignore` に unignore エントリ追加が必要） / `serversync.env.systemd.example`（非隠しファイル名、.gitignore 問題なし） | **ルート `.gitignore` 追補ありで `.env.serversync.systemd.example` を推奨**（#235 との命名一貫性） | **`.env.serversync.systemd.example` に確定**。#235 の `.env.systemd.example` と命名規則を揃える。ルート `.gitignore` に `!virtual-voicebot-backend/.env.serversync.systemd.example` の unignore エントリ追加が必要（§4.2 に追記済み） |
| Q3 | `@@OS_USER@@` プレースホルダー方式を #235 と統一するか | 統一する / 別方式にする | **統一を推奨**（インストール手順の一貫性と録音ファイル権限の揃えやすさ） | **統一（強め）**。serversync は録音ファイルを読み取り・削除するため通話本体と別ユーザーだと権限エラーになりやすい。両 unit とも `User=@@OS_USER@@`/`Group=@@OS_USER@@` を使い、実機では同じ値（例: `msuda`）を置換する。§5.2・§5.4 に反映済み |

---

## 9. リスク・ロールバック観点

| リスク | 影響 | 緩和策 |
|--------|------|--------|
| `KillSignal=SIGINT` で graceful stop が完了しない（タイムアウト） | `TimeoutStopSec=10` 経過後に `SIGKILL` で強制終了。同期処理が中途半端な状態で終了する可能性 | `TimeoutStopSec` を調整。ログで sync 完了を確認。将来的に `SIGTERM` 対応（§5.5）で根本解消 |
| 通話本体と異なる OS ユーザーで動作した場合 | 録音ファイルの read/delete が permission エラーになる | インストール手順で `OS_USER` を通話本体と揃えるよう明記（§5.4 のコメント） |
| `DATABASE_URL` / `FRONTEND_BASE_URL` 未設定 | serversync 起動失敗 | EnvironmentFile テンプレートで必須と明記。`journalctl` で確認可能 |
| `EnvironmentFile` 内の shell 構文混入 | 起動失敗（`journalctl` にエラー出力） | テンプレートのコメントで shell 構文禁止を明示 |
| `run_register.sh` などとの serversync 二重起動 | 同期処理の競合・重複 | `systemctl stop serversync` 後に手動起動するよう手順書に明記 |

**ロールバック手順:** `sudo systemctl disable serversync && sudo systemctl stop serversync` で即時無効化。unit ファイルを削除すれば完全ロールバック可能。

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-24 | 初版作成（Codex 調査結果を元に差分仕様を記述。STEER-235 の設計を踏襲） | Claude Sonnet 4.6 |
| 2026-02-24 | §8 Q1/Q2/Q3 オーナー回答記録、§3.6/§4.2/§5.3/§5.4 のファイル名を `.env.serversync.systemd.example` に確定、§4.2 に `.gitignore` unignore エントリ追加スコープを明記 | Claude Sonnet 4.6 |
| 2026-02-24 | §3.3 Round 1 NG 記録、①§4.2/§8 Q2 の `.gitignore` ターゲットをルートへ修正・unignore パスを `!virtual-voicebot-backend/.env.serversync.systemd.example` に修正 ②§5.1/§5.5/§6/§7.1 のコード参照パスを `src/interface/sync/recording_uploader.rs`・`src/interface/sync/worker.rs`・`src/bin/serversync.rs` に統一 | Claude Sonnet 4.6 |
| 2026-02-24 | §3.3 Round 2 NG 記録、全箇所の `src/serversync.rs` を `src/bin/serversync.rs` に修正（実ファイル: `src/bin/serversync.rs:39`）（§2.1/§4.2/§5.1/§5.3/§5.5/§6/§7.1） | Claude Sonnet 4.6 |
| 2026-02-24 | §3.3 Round 3 OK 記録 | Claude Sonnet 4.6 |
| 2026-02-24 | §1 ステータス Draft → Approved、§3.4 承認者記録（@MasanoriSuda, lgtm） | Claude Sonnet 4.6 |
