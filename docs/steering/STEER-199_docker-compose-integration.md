# STEER-199: Docker Compose統合（Backend + Frontend）

<!--
  ============================================================
  命名規則
  ============================================================

  ファイル名: STEER-{イシュー番号}_{slug}.md

  - イシュー番号: GitHub Issue の番号（例: 199）
  - slug: 英小文字、ハイフン区切り、20文字以内

  例: STEER-199_docker-compose-integration.md

  ============================================================
  運用ルール
  ============================================================

  - 基本: 1イシュー = 1ステアリング
  - 例外: 小さい関連変更は1つにまとめてもOK（関連Issue を複数記載）
  - マージ後: ステータスを Merged に更新してアーカイブ

  ============================================================
  修正権限
  ============================================================

  - 新規作成（Draft）: Claude Code が担当
  - Review時の修正: Codex が対応可（レビュー必須、最小差分）
  - Approved以降: Codex が段取り更新

  禁止事項:
  - Codex による新規ステアリングファイルの作成
  - ストーリー（§2）の変更（Issue で合意すべき）
  - Status の勝手な変更（人間が判断）

  詳細: AGENTS.md §ドキュメント更新の扱い を参照

  ============================================================
  Status 更新ルール
  ============================================================

  | Status | 更新者 | タイミング |
  |--------|--------|-----------|
  | Draft | Claude Code | 新規作成時 |
  | Review | オーナー | Draft完了・レビュー開始時 |
  | Approved | オーナー/PL/PO | レビュー承認時 |
  | Merged | 担当者 | PR マージ後 |

  ============================================================
-->

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-199 |
| タイトル | Docker Compose統合（Backend + Frontend） |
| ステータス | Approved |
| 関連Issue | #199 |
| 優先度 | P1 |
| 作成日 | 2026-02-17 |

---

## 2. ストーリー（Why）

### 2.1 背景

現状、Docker構成が分断されている：

- **Backend**: `virtual-voicebot-backend/` 配下に個別のDockerfile + docker-compose（dev/prod）が存在
- **Frontend**: Docker構成が未整備（Dockerfile/docker-compose.yml なし）
- **ルート**: 統合的なdocker-compose構成が存在しない

この状態では以下の問題が発生している：

1. **開発環境のセットアップが煩雑**
   - Backend/Frontend を別々に起動する必要がある
   - 新規メンバーがローカル環境を再現しづらい
   - Node.js/Rust のバージョン差分で動作が不安定

2. **Backend ↔ Frontend連携のテストが困難**
   - API通信をローカルで検証するには、両方を手動起動する必要がある
   - 環境変数（API URL等）の管理が属人化

3. **DevContainer が Backend専用**
   - `.devcontainer/devcontainer.json` は Backend のみを対象
   - Frontend開発者は独自に環境構築が必要

4. **CI/CDへの展開が非効率**
   - Docker構成が統一されていないため、GitHub Actions等でのE2Eテストが組みづらい

### 2.2 目的

モノレポ構成（Backend + Frontend）を統合的に管理できるDocker環境を構築する：

1. **単一コマンドで開発環境を起動**
   - `docker compose up` で Backend + Frontend + 依存サービス（DB/Ollama/VoiceVox）が一括起動
   - 環境差分（macOS/Windows/Linux）を吸収

2. **ホットリロード対応の開発環境**
   - Backend: `cargo watch` でRustコードの変更を即反映
   - Frontend: `pnpm dev` でNext.jsコードの変更を即反映

3. **DevContainer対応**
   - Backend/Frontend それぞれのDevContainerを整備
   - VS Code Workspaceで切り替え可能に

4. **将来のK8s移行に備えた設計**
   - 環境変数注入（12-factor）
   - ステートレスコンテナ
   - ヘルスチェック実装

### 2.3 ユーザーストーリー（該当する場合）

```
As a 開発者（Backend/Frontend両方）
I want to `docker compose up` で統合環境を起動したい
So that 環境差分を気にせず開発に集中できる

受入条件:
- [ ] ルートで `docker compose up` を実行すると、Backend + Frontend + DB + Ollama + VoiceVox が起動する
- [ ] Backend コード変更時に自動的に再ビルド・再起動される（cargo watch）
- [ ] Frontend コード変更時に自動的にホットリロードされる（pnpm dev）
- [ ] Frontend から Backend API（例: http://localhost:18080）にアクセスできる
- [ ] Frontend の本番ビルド（standalone）が成功する（E2Eテスト前提）
- [ ] 既存の Backend/Frontend 個別起動も引き続き可能
```

```
As a 新規メンバー
I want to Docker環境だけで開発を開始したい
So that ローカル環境のセットアップ時間を最小化できる

受入条件:
- [ ] Rust/Node.js をローカルインストール不要
- [ ] `git clone` → `docker compose up` だけで起動可能
- [ ] README にDocker環境のセットアップ手順が記載されている
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-17 |
| 起票理由 | Docker上での開発の重要性が高いため |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (Claude Sonnet 4.5) |
| 作成日 | 2026-02-17 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "docker構築を行う #199 にイシュー立てました。開発するにあたりDocker上での開発の重要性が高いと感じたためです、壁打ちお願いできますか？？？" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| - | - | - | - | （Draft完了後にレビュー開始） |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-17 |
| 承認コメント | Codex レビューOK判定により承認。実装開始可能。 |

### 3.5 実装（該当する場合）

| 項目 | 値 |
|------|-----|
| 実装者 | Codex |
| 実装日 | 2026-02-17 |
| 指示者 | @MasanoriSuda |
| 指示内容 | 「承認しましたので作業お願いします、Refs #199」 |
| コードレビュー | - |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | - |
| マージ日 | - |
| マージ先 | README.md, 各種ドキュメント |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| README.md | 修正 | Docker環境のセットアップ手順を追記 |
| CONTRIBUTING.md | 修正 | 開発環境の起動方法をDocker前提に更新 |
| virtual-voicebot-backend/README.md | 修正 | Backend個別起動 vs 統合起動の使い分け説明 |
| virtual-voicebot-frontend/README.md | 追加 | Frontend開発環境のセットアップ手順 |

### 4.2 影響するファイル

| ファイル | 変更種別 | 概要 |
|---------|---------|------|
| `docker-compose.yml`（ルート） | 新規作成 | Backend + Frontend + 依存サービスの統合構成 |
| `docker-compose.dev.yml`（ルート） | 新規作成 | 開発環境用オーバーライド（ホットリロード等） |
| `docker-compose.test.yml`（ルート） | 新規作成 | E2Eテスト用構成 |
| `virtual-voicebot-frontend/Dockerfile.dev` | 新規作成 | Frontend開発用Dockerfile（Node.js + pnpm dev） |
| `virtual-voicebot-frontend/Dockerfile.prod` | 新規作成 | Frontend本番用Dockerfile（Next.js build + standalone）※E2E必須 |
| `virtual-voicebot-frontend/Dockerfile.e2e` | 新規作成 | E2Eテスト用Dockerfile（Playwright実行環境） |
| `virtual-voicebot-frontend/.dockerignore` | 新規作成 | node_modules/.next 等を除外 |
| `virtual-voicebot-frontend/package.json` | 修正 | `test:e2e` スクリプト追加 |
| `virtual-voicebot-frontend/next.config.mjs` | 修正 | `output: 'standalone'` 設定追加（本番ビルド用） |
| `virtual-voicebot-backend/Dockerfile` | 修正 | cargo watch対応、inotify設定 |
| `.devcontainer/devcontainer.json` | 修正 | 統合composeファイルを参照 |
| `.devcontainer/frontend.devcontainer.json` | 新規作成 | Frontend用DevContainer設定 |
| `.env.example` | 新規作成 | 環境変数のテンプレート |

---

## 5. 差分仕様（What / How）

### 5.1 設計方針（壁打ち結果）

以下の技術方針で設計を進める：

| 論点 | 推奨方針 | 優先度 |
|------|---------|-------|
| **Q1: Backend開発時のホットリロード** | **Yes (cargo watch)** | 高 |
| **Q2: RTPポート範囲** | **10000-10100に絞る** | 中 |
| **Q3: 本番環境の想定** | **docker-compose（当面）/ 将来K8s対応も視野** | 高 |
| **Q4: Frontend DevContainer** | **Yes（作る）** | 中 |
| **Q5: E2Eテスト用compose** | **Yes（test用を別ファイル）** | 低 |
| **Q6: Backendポート** | **18080（確定）、環境変数RECORDING_HTTP_ADDR** | 中 |
| **Q7: cargo watch コマンド** | **まず `cargo run` で動作確認、通れば `-x run`** | 中 |
| **Q8: Next.js standalone** | **E2Eテストのため本Issue内で実装必須** | 高 |

**根拠**:
- **Q1**: Rustは再ビルドが重い。`cargo watch` でループ速度を改善し、開発効率を向上させる
- **Q2**: 開発環境では同時接続数が限定的。100ポート幅で十分。運用・FW設定の簡素化
- **Q3**: いきなりK8sは過剰。まずcomposeで環境を再現可能にし、将来の移行に備えた設計（12-factor）を採用
- **Q4**: Node周りの差分（バージョン、pnpm/yarn）を吸収。新規メンバーの環境構築を簡素化
- **Q5**: 開発環境とE2E環境を分離し、依存や設定の汚染を防ぐ
- **Q6**: 実コード確認により18080を確定。環境変数 `RECORDING_HTTP_ADDR` で変更可能
- **Q7**: プロジェクト構成（workspace, bin指定等）により最適なコマンドが異なるため、動作確認後に決定
- **Q8**: E2Eテスト（§5.6.1）で本番ビルドが必要なため、本Issue内で実装必須

---

### 5.2 コンテナ構成設計

#### 5.2.1 サービス構成

```yaml
# docker-compose.yml（統合構成）
services:
  # Backend（Rust SIP/RTP サーバー）
  backend:
    build:
      context: ./virtual-voicebot-backend
      dockerfile: Dockerfile
      target: build  # 開発時はbuildステージを使用（cargo watch対応）
    container_name: virtual-voicebot-backend
    volumes:
      - ./virtual-voicebot-backend:/workspace
      - backend-target:/workspace/target  # Rustビルドキャッシュを永続化
    environment:
      SIP_BIND_IP: 0.0.0.0
      SIP_PORT: 5060
      RTP_PORT: 10000
      LOCAL_IP: 0.0.0.0
      ADVERTISED_IP: 127.0.0.1  # 開発環境: localhost
      RECORDING_HTTP_ADDR: 0.0.0.0:18080  # HTTP APIサーバーアドレス
      DATABASE_URL: postgres://voicebot:voicebot_dev@postgres:5432/voicebot
      OLLAMA_URL: http://ollama:11434
      VOICEVOX_URL: http://voicevox:50021
      ASR_PROVIDER: whisper
      LLM_PROVIDER: ollama
      TTS_PROVIDER: voicevox
      RUST_BACKTRACE: 1
      CARGO_INCREMENTAL: 1  # インクリメンタルコンパイル有効化
    ports:
      - "5060:5060/udp"      # SIP
      - "10000-10100:10000-10100/udp"  # RTP（絞った範囲）
      - "18080:18080"        # HTTP API（録音・同期ステータス）
    depends_on:
      postgres:
        condition: service_healthy
      ollama:
        condition: service_started
      voicevox:
        condition: service_started
    command: cargo watch -x run  # ホットリロード（※動作確認後に -p/--bin 追加の可能性あり）

  # Frontend（Next.js 16）
  frontend:
    build:
      context: ./virtual-voicebot-frontend
      dockerfile: Dockerfile.dev
    container_name: virtual-voicebot-frontend
    volumes:
      - ./virtual-voicebot-frontend:/app
      - frontend-node-modules:/app/node_modules  # node_modulesを永続化（高速化）
    environment:
      BACKEND_URL: http://backend:18080  # SSR時のBackend内部通信
      # ※ブラウザからのアクセスは http://localhost:18080（ホスト公開ポート）
    ports:
      - "3000:3000"  # Next.js dev server
    depends_on:
      - backend
    command: pnpm dev  # ホットリロード

  # PostgreSQL
  postgres:
    image: postgres:16-alpine
    container_name: virtual-voicebot-postgres
    restart: unless-stopped
    environment:
      POSTGRES_DB: voicebot
      POSTGRES_USER: voicebot
      POSTGRES_PASSWORD: voicebot_dev
    ports:
      - "5432:5432"
    volumes:
      - postgres-data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U voicebot"]
      interval: 5s
      timeout: 5s
      retries: 5

  # Ollama（LLM推論）
  ollama:
    image: ollama/ollama:latest
    container_name: virtual-voicebot-ollama
    restart: unless-stopped
    ports:
      - "11434:11434"
    volumes:
      - ollama-data:/root/.ollama

  # VoiceVox（音声合成）
  voicevox:
    image: voicevox/voicevox_engine:cpu-latest
    container_name: virtual-voicebot-voicevox
    restart: unless-stopped
    ports:
      - "50021:50021"

volumes:
  backend-target:      # Rustビルドキャッシュ（高速化）
  frontend-node-modules:  # node_modules（高速化）
  postgres-data:       # DB永続化
  ollama-data:         # モデルデータ永続化
```

#### 5.2.2 ネットワーク設計

- **デフォルトブリッジネットワーク**を使用（明示的な定義なし）
- サービス名でDNS解決（例: `http://backend:18080`, `http://postgres:5432`）
- ホスト公開ポート:
  - Frontend: 3000（ブラウザアクセス）
  - Backend: 5060/udp（SIP）, 10000-10100/udp（RTP）, 18080（HTTP API）
  - Postgres: 5432（ローカルツールからのアクセス）
  - Ollama: 11434（ローカルテスト）
  - VoiceVox: 50021（ローカルテスト）

---

### 5.3 Dockerfile仕様

#### 5.3.1 Backend Dockerfile修正

**変更点**:
- `cargo watch` インストール
- inotify設定（ボリュームマウント時のファイル監視）
- 開発ステージ（build）と本番ステージ（runtime）の明確化

**実装時の注意**:
- `cargo run` の動作確認を先に実施
  - 動作OK → `cargo watch -x run`
  - 動作NG → workspace/bin指定が必要か確認し、`cargo watch -x "run -p <package>" -x "run --bin <name>"` 等に変更

```dockerfile
# 既存のDockerfileに以下を追加
# === ビルドステージ（開発用） ===
FROM ubuntu:22.04 AS build

# ... 既存の設定 ...

# cargo watch をインストール（ホットリロード用）
RUN cargo install cargo-watch

# inotify設定（macOS/Windows でのファイル監視）
RUN echo "fs.inotify.max_user_watches=524288" >> /etc/sysctl.conf

# ... 既存の設定 ...
```

#### 5.3.2 Frontend Dockerfile.dev（新規作成）

```dockerfile
# === 開発用Dockerfile ===
FROM node:22-alpine AS dev

WORKDIR /app

# pnpm インストール
RUN npm install -g pnpm

# 依存関係をキャッシュ（package.json/pnpm-lock.yamlが変わらない限り再利用）
COPY package.json pnpm-lock.yaml ./
RUN pnpm install

# ソースコード全体をマウント（volumes で上書き）
COPY . .

# ポート公開
EXPOSE 3000

# 開発サーバー起動
CMD ["pnpm", "dev"]
```

#### 5.3.3 Frontend Dockerfile.prod（新規作成）

**実装優先度**: 高（E2Eテストのため本Issue内で実装必須）

**前提条件**:
- `next.config.mjs` に `output: 'standalone'` 設定が必要
- E2Eテスト（§5.6.1）で本番ビルドを使用するため必須

```dockerfile
# === 本番用Dockerfile（マルチステージビルド） ===
# ※E2Eテストのため本Issue内で実装必須

FROM node:22-alpine AS deps

WORKDIR /app

RUN npm install -g pnpm

COPY package.json pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile --prod

# === ビルドステージ ===
FROM node:22-alpine AS builder

WORKDIR /app

RUN npm install -g pnpm

COPY package.json pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile

COPY . .
RUN pnpm build

# === 実行ステージ ===
FROM node:22-alpine AS runner

WORKDIR /app

ENV NODE_ENV=production

RUN addgroup --system --gid 1001 nodejs
RUN adduser --system --uid 1001 nextjs

# standalone出力を使用（最小イメージ）
COPY --from=builder /app/public ./public
COPY --from=builder --chown=nextjs:nodejs /app/.next/standalone ./
COPY --from=builder --chown=nextjs:nodejs /app/.next/static ./.next/static

USER nextjs

EXPOSE 3000

CMD ["node", "server.js"]
```

---

### 5.4 DevContainer統合

#### 5.4.1 Backend DevContainer修正

```json
// .devcontainer/devcontainer.json
{
  "name": "virtual-voicebot Backend",
  "dockerComposeFile": ["../docker-compose.yml", "../docker-compose.dev.yml"],
  "service": "backend",
  "workspaceFolder": "/workspace",

  "customizations": {
    "vscode": {
      "extensions": [
        "rust-lang.rust-analyzer",
        "ms-python.python",
        "ms-python.vscode-pylance"
      ]
    }
  },

  "remoteUser": "root",
  "postCreateCommand": "cargo build || true"
}
```

#### 5.4.2 Frontend DevContainer（新規作成）

```json
// .devcontainer/frontend.devcontainer.json
{
  "name": "virtual-voicebot Frontend",
  "dockerComposeFile": ["../docker-compose.yml", "../docker-compose.dev.yml"],
  "service": "frontend",
  "workspaceFolder": "/app",

  "customizations": {
    "vscode": {
      "extensions": [
        "dbaeumer.vscode-eslint",
        "esbenp.prettier-vscode",
        "bradlc.vscode-tailwindcss"
      ],
      "settings": {
        "editor.defaultFormatter": "esbenp.prettier-vscode",
        "editor.formatOnSave": true
      }
    }
  },

  "remoteUser": "node",
  "postCreateCommand": "pnpm install"
}
```

---

### 5.5 環境変数管理

#### 5.5.1 .env.example（新規作成）

```bash
# === Backend ===
SIP_BIND_IP=0.0.0.0
SIP_PORT=5060
RTP_PORT=10000
LOCAL_IP=0.0.0.0
ADVERTISED_IP=127.0.0.1  # 開発環境: localhost
RECORDING_HTTP_ADDR=0.0.0.0:18080  # HTTP APIサーバーアドレス
DATABASE_URL=postgres://voicebot:voicebot_dev@postgres:5432/voicebot
OLLAMA_URL=http://ollama:11434
VOICEVOX_URL=http://voicevox:50021
ASR_PROVIDER=whisper
LLM_PROVIDER=ollama
TTS_PROVIDER=voicevox

# === Frontend ===
BACKEND_URL=http://localhost:18080  # ローカル開発時のBackend URL
# ※Docker Compose環境内では http://backend:18080 を使用

# === PostgreSQL ===
POSTGRES_DB=voicebot
POSTGRES_USER=voicebot
POSTGRES_PASSWORD=voicebot_dev
```

#### 5.5.2 .gitignore

**既存ルールでカバー済み**: ルートの `.gitignore` (line 17-18) に `.env` と `.env.*` が既に記載されているため、追加不要。

---

### 5.6 E2Eテスト用構成

#### 5.6.1 docker-compose.test.yml（新規作成）

**使用方法**: オーバーライドとして使用（単体起動不可）

```bash
# E2Eテスト実行例（CI/CD用）
docker compose -f docker-compose.yml -f docker-compose.test.yml up --build --abort-on-container-exit --exit-code-from e2e
```

```yaml
# E2Eテスト用オーバーライド構成
# ※ベースの docker-compose.yml と組み合わせて使用
version: '3.9'

services:
  backend:
    build:
      target: runtime  # 本番ステージを使用
    volumes: []  # 開発用volumeを無効化（runtime/prodビルドを使用）
    environment:
      DATABASE_URL: postgres://voicebot:voicebot_test@postgres:5432/voicebot_test
      RECORDING_HTTP_ADDR: 0.0.0.0:18080  # E2E環境でも18080
    command: /workspace/virtual-voicebot-backend  # リリースビルドを実行

  frontend:
    build:
      dockerfile: Dockerfile.prod  # 本番ビルドを使用
    volumes: []  # 開発用volumeを無効化（本番ビルドを使用）
    environment:
      BACKEND_URL: http://backend:18080  # E2E環境: Backend内部通信
      NODE_ENV: production
    command: ["node", "server.js"]  # 本番ビルド実行（standalone）

  postgres:
    environment:
      POSTGRES_DB: voicebot_test
      POSTGRES_USER: voicebot
      POSTGRES_PASSWORD: voicebot_test

  # E2Eテスト実行コンテナ（Playwright）
  e2e:
    build:
      context: ./virtual-voicebot-frontend
      dockerfile: Dockerfile.e2e
    depends_on:
      - frontend
      - backend
    command: pnpm test:e2e
```

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #199 | STEER-199 | 起票 |
| STEER-199 | README.md | セットアップ手順追記 |
| STEER-199 | CONTRIBUTING.md | 開発環境起動方法更新 |
| STEER-199 | docker-compose.yml | 統合構成作成 |
| STEER-199 | Dockerfile.dev（Frontend） | 開発用イメージ作成 |
| STEER-199 | .devcontainer/ | DevContainer統合 |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] **アーキテクチャ方針は妥当か**
  - docker-compose統合 vs 個別compose の判断
  - ホットリロード対応の実現性
  - 本番環境との整合性

- [ ] **開発体験（DX）が向上するか**
  - 起動コマンドの簡素化
  - 環境差分の吸収
  - 新規メンバーのオンボーディング時間短縮

- [ ] **セキュリティリスクは考慮されているか**
  - .env のgitignore設定
  - シークレット管理（本番環境への展開時）
  - コンテナ権限（non-root user）

- [ ] **パフォーマンスは許容範囲か**
  - ビルド時間（cargo cache, node_modules cache）
  - ディスク容量（volumes の肥大化）
  - M1/M2 Mac対応（arm64 vs amd64）

- [ ] **既存仕様との整合性**
  - 既存 Backend docker-compose.dev.yml との互換性
  - .devcontainer/devcontainer.json との統合

### 7.2 実装前チェック（Approved → 実装開始）

- [ ] **依存ツールのバージョン確認**
  - Docker Compose v2.x 以上
  - Docker Engine 20.x 以上
  - cargo watch 最新版

- [ ] **環境変数の棚卸し**
  - .env.example に全変数が列挙されているか
  - 機密情報が含まれていないか

- [ ] **ドキュメント更新準備**
  - README.md のセットアップ手順ドラフト
  - CONTRIBUTING.md の開発フローダイアグラム

---

## 8. 備考

### 8.1 リスク・制約事項

| リスク | 影響度 | 対策 |
|--------|-------|------|
| **R1: Rustビルド時間の長期化** | 高 | `backend-target` volumeでキャッシュ永続化、`cargo-chef` パターン検討 |
| **R2: ディスク容量の肥大化** | 中 | `.dockerignore` 整備、定期的な `docker system prune` |
| **R3: M1/M2 Mac対応** | 中 | `platform: linux/amd64` 指定 or Rosetta 2、arm64イメージ優先使用 |
| **R4: DevContainer複雑化** | 低 | VS Code Workspaceで切り替え、ドキュメント整備 |
| **R5: ポート衝突** | 低 | RTP範囲を絞る（10000-10100）、README に既知の衝突を記載 |

### 8.2 実装時の重要注意事項

**🚨 既存Docker構成は全面作り直し**

- **現状のDocker関連ファイルは陳腐化しているため、全て作り直してOK**
- 対象ファイル:
  - `virtual-voicebot-backend/Dockerfile`
  - `virtual-voicebot-backend/docker-compose.yml`
  - `virtual-voicebot-backend/docker-compose.dev.yml`
  - `.devcontainer/devcontainer.json`
- 既存ファイルを参考にしつつ、本ステアリング（§5）の仕様に従って再作成すること
- 既存の設定値（環境変数、ポート等）で有用なものは引き継ぐ

### 8.3 今後の拡張性

- **K8s移行時の考慮点**:
  - 環境変数注入（12-factor）は既に対応
  - ヘルスチェック実装（Postgres は実装済み、Backend/Frontend は今後）
  - Helmチャート作成時に docker-compose.yml を参照可能

- **CI/CD連携**:
  - GitHub Actions で `docker compose -f docker-compose.yml -f docker-compose.test.yml up --build --abort-on-container-exit --exit-code-from e2e` を実行
  - E2Eテスト自動化（Playwright）

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-17 | 初版作成（Draft） | Claude Code (Claude Sonnet 4.5) |
| 2026-02-17 | Q6-Q8推奨方針を反映（PORT可変化、cargo run確認、standalone優先度下げ） | Claude Code (Claude Sonnet 4.5) |
| 2026-02-17 | §8.2 実装注意事項を追加（既存Docker構成は全面作り直し） | Claude Code (Claude Sonnet 4.5) |
| 2026-02-17 | Codexレビュー第1回対応（ポート18080確定、環境変数名修正、E2E構成追加） | Claude Code (Claude Sonnet 4.5) |
| 2026-02-17 | Codexレビュー第2回対応（test.yml をオーバーライド前提に修正、全8080→18080統一） | Claude Code (Claude Sonnet 4.5) |
| 2026-02-17 | Codexレビュー第3回対応（CI/CD実行例修正、Dockerfile.prod優先度変更、Next.js 16表記） | Claude Code (Claude Sonnet 4.5) |
| 2026-02-17 | Codexレビュー第4回対応（E2E volumes無効化、CI終了条件追加、Q8優先度修正） | Claude Code (Claude Sonnet 4.5) |
| 2026-02-17 | Codexレビュー第5回対応（E2E frontend command修正、next.config.mjs追加、受入条件追加） | Claude Code (Claude Sonnet 4.5) |
| 2026-02-17 | Codexレビュー第6回でOK判定、ステータスを Approved に更新 | Claude Code (Claude Sonnet 4.5) |
