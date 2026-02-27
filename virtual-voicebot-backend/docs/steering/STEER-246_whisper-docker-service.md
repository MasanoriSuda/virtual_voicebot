# STEER-246: Whisper Docker Compose 常駐化

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-246 |
| タイトル | Whisper Docker Compose 常駐化 |
| ステータス | Approved |
| 関連Issue | #246（前提: #245 ダッシュボード死活監視） |
| 優先度 | P1 |
| 作成日 | 2026-02-24 |

---

## 2. ストーリー（Why）

### 2.1 背景

現状、`whisper_server.py` は手動 `python` 実行で起動する構成であり、以下の問題がある。

| 問題 | 詳細 |
|------|------|
| 常駐管理されていない | Docker Compose 外で手動起動が必要。compose up 一発で起動しない |
| 監視対象 URL が不定 | localhost:9000 固定。compose ネットワーク内では参照できない（コンテナ間通信不可） |
| ヘルスチェック未実装 | `GET /healthz` エンドポイントが存在しない。ダッシュボードや compose healthcheck に使用できない |
| モデルキャッシュが非永続 | コンテナ再起動のたびにモデルをダウンロードするリスクがある（HF_HOME が volume 管理外） |
| compose.yml に whisper サービス定義がない | ルート `docker-compose.yml` / `virtual-voicebot-backend/docker-compose.dev.yml`（DevContainer）ともに whisper サービス未定義 |

Issue #245 でダッシュボードから ASR/LLM/TTS の死活監視を行う要件があるが、
Whisper が Docker Compose 常駐化されていない状態では監視対象 URL・ヘルス仕様が未確定であり、
ダッシュボード実装の手戻りが発生する。

### 2.2 目的

Whisper サーバーを Docker Compose サービスとして常駐化し、以下を実現する。

1. `docker-compose up` 一発で Whisper が起動・常駐する
2. `GET /healthz` を追加し、compose healthcheck およびダッシュボード監視に対応する
3. compose ネットワーク内の backend が `http://whisper:9000/transcribe` で ASR を呼び出せる
4. HuggingFace モデルキャッシュを named volume で永続化し、再起動コストを排除する

### 2.3 ユーザーストーリー

```text
As a システム管理者
I want to docker-compose up で Whisper が自動的に常駐起動してほしい
So that ダッシュボードで ASR の死活確認ができ、手動起動・URL 管理が不要になる

受入条件:
- [ ] docker-compose up 後、whisper サービスが自動起動・常駐する
- [ ] GET /healthz が 200 を返し、compose healthcheck が healthy になる
- [ ] backend コンテナから http://whisper:9000/transcribe で POST が通る
- [ ] モデルキャッシュが named volume に保存され、再起動時の再ダウンロードが不要になる
- [ ] CPU 環境で起動できる（GPU 対応は別 Issue で対応。既存コードは `cuda.is_available()` で自動判定済みのため CPU 環境でも動作する）
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-24 |
| 起票理由 | Issue #245 ダッシュボード死活監視の前提基盤として Whisper の常駐化が必要 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Sonnet 4.6 |
| 作成日 | 2026-02-24 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Whisper の Docker 化を行いたい。ダッシュボードの死活監視向けに常駐させることが目的。Refs #245 前提" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | Codex | 2026-02-24 | 要修正 | 重大3・中2: compose.dev.yml ターゲット不一致、pip index-url 波及、AC GPU 文言矛盾、service_healthy 残存、reazon コメント誤誘導 |
| 2 | Codex | 2026-02-24 | 要修正 | 重大2・中1: pip install 分離未実施・ffmpeg 欠落、docker-compose.dev.yml 曖昧参照残存 |
| 3 | Codex | 2026-02-24 | OK | 前回 NG 項目（ffmpeg 追加・pip 分離・参照整理）すべて解消。新規指摘なし |
| 4 | @MasanoriSuda | 2026-02-24 | OK | lgtm |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-24 |
| 承認コメント | lgtm |

### 3.5 実装（Codex 担当）

| 項目 | 値 |
|------|-----|
| 実装者 | Codex |
| 実装日 | - |
| 指示者 | @MasanoriSuda |
| 指示内容 | - |
| コードレビュー | - |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | - |
| マージ日 | - |
| マージ先 | ルート `docker-compose.yml`, `virtual-voicebot-backend/docker-compose.dev.yml`（DevContainer）, `script/whisper_server.py`（ルート `docker-compose.dev.yml` は変更なし） |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| 本ファイル（STEER-246）| 新規 | Whisper Docker 常駐化の差分仕様 |
| docs/steering/index.md | 追記 | STEER-246 エントリ追加 |

> 本体仕様書（RD/DD）への反映は、承認後・実装完了後に別途対応する（本ステアリング対象外）。

### 4.2 影響するコード

| ファイル | 変更種別 | 概要 |
|---------|---------|------|
| `script/Dockerfile.whisper` | 新規 | Whisper サーバー用 Dockerfile（kotoba 専用・CPU 版） |
| `docker-compose.yml`（ルート） | 修正 | `whisper` サービス追加、`backend.environment.ASR_LOCAL_SERVER_URL` 追加、`backend.depends_on` に `whisper` 追加 |
| `virtual-voicebot-backend/docker-compose.dev.yml`（DevContainer） | 修正 | `whisper` サービス追加、`app.environment.ASR_LOCAL_SERVER_URL` 追加、`app.depends_on` に `whisper` 追加 |
| ルート `docker-compose.dev.yml`（small override） | 変更なし | whisper サービスはルート `docker-compose.yml` に定義済みのため不要 |
| `script/whisper_server.py` | 修正 | `GET /healthz` エンドポイント追加 |

---

## 5. 差分仕様（What / How）

### 5.1 新規ファイル: `script/Dockerfile.whisper`

```dockerfile
FROM python:3.11-slim

WORKDIR /app

# システム依存ライブラリ（soundfile / torch / ffmpeg 用）
# ffmpeg: transformers ASR pipeline が ffmpeg_read 経由で wav を処理するために必要
RUN apt-get update && apt-get install -y --no-install-recommends \
    libsndfile1 \
    curl \
    ffmpeg \
    && rm -rf /var/lib/apt/lists/*

# torch は CPU 版を個別インストール（--index-url を他パッケージに波及させないために分離）
ARG TORCH_INDEX_URL=https://download.pytorch.org/whl/cpu
RUN pip install --no-cache-dir torch --index-url ${TORCH_INDEX_URL}

# その他の依存ライブラリは通常 PyPI から取得
RUN pip install --no-cache-dir \
    transformers \
    fastapi \
    "uvicorn[standard]" \
    pykakasi \
    python-multipart

COPY script/whisper_server.py /app/whisper_server.py

EXPOSE 9000

HEALTHCHECK --interval=30s --timeout=10s --retries=3 --start-period=180s \
    CMD curl -f http://localhost:9000/healthz || exit 1

CMD ["python", "whisper_server.py"]
```

**設計方針:**
- **CPU 版のみ（OQ-2 確定）。** GPU 対応は別イシューで対応する。
- `whisper_server.py` L46 で `cuda.is_available()` による自動判定が実装済みのため、CPU 環境でも追加コード変更なく動作する。
- `TORCH_INDEX_URL` の build arg は将来の GPU 対応時に切り替え口として残す（今回は CPU index のみ使用）。
- **kotoba エンジンのみ対応（OQ-3 確定）。** `ASR_ENGINE=reazon` に必要な `nemo_toolkit` は含めない。reazon は別イシューで対応。
- `start_period=180s`: kotoba-whisper-v2.2 のモデルロードに時間がかかるため長めに設定（OQ-1 確定: 180s）
- `torch` だけ別 `RUN` で `--index-url` を指定: 同一 `pip install` 内で `--index-url` を指定すると全パッケージが PyTorch index から解決されるため分離する。
- `ffmpeg` を apt でインストール: `transformers` の ASR pipeline が wav 入力を `ffmpeg_read` 経由で処理するため必須。未インストールの場合 `/transcribe` が実行時に失敗する。
- `curl` を healthcheck に使用（`python -c "import urllib..."` でも可だが curl が簡潔）

---

### 5.2 `docker-compose.yml` への `whisper` サービス追加

以下の差分を `docker-compose.yml` に適用する。

**追加するサービス定義（`voicevox` サービスの後に追加）:**

```yaml
  whisper:
    build:
      context: ./virtual-voicebot-backend
      dockerfile: script/Dockerfile.whisper
    container_name: virtual-voicebot-whisper
    restart: unless-stopped
    ports:
      - "${WHISPER_HOST_PORT:-9000}:9000"
    volumes:
      - whisper-model-cache:/var/cache/huggingface
    environment:
      HF_HOME: /var/cache/huggingface
      ASR_ENGINE: kotoba  # OQ-3 確定: kotoba のみ対応（reazon は Dockerfile 未対応）
      ASR_OUTPUT_SCRIPT: hiragana  # hiragana / katakana / （空=変換なし）
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9000/healthz"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 180s
```

**`backend` サービスへの追記（`environment` と `depends_on`）:**

```yaml
  backend:
    environment:
      # 既存の設定はそのまま維持し、以下を追加
      ASR_LOCAL_SERVER_URL: http://whisper:9000/transcribe
    depends_on:
      # 既存の depends_on はそのまま維持し、以下を追加
      whisper:
        condition: service_started   # OQ-4 確定: 開発時の利便性優先
```

**`volumes` セクションへの追記:**

```yaml
volumes:
  # 既存の volume はそのまま維持し、以下を追加
  whisper-model-cache:
```

**設計方針:**
- `restart: unless-stopped`: 常駐化のため自動再起動を有効化
- `service_started`（OQ-4 確定）: backend は whisper が起動済みであれば開始する（healthcheck 通過を待たない）
- `HF_HOME` を volume にマウントし、モデルキャッシュを永続化
- `WHISPER_HOST_PORT` は他サービス同様に変数化（デフォルト 9000）

---

### 5.3 DevContainer 用 compose への `whisper` サービス追加

> **対象ファイル:** `virtual-voicebot-backend/docker-compose.dev.yml`（DevContainer 専用 compose）
> ルートの `docker-compose.dev.yml`（`backend`/`frontend` の small override）とは**別ファイル**。
> ルートの `docker-compose.dev.yml` への変更は不要（§5.2 の `docker-compose.yml` で whisper サービスが定義済み）。

以下の差分を `virtual-voicebot-backend/docker-compose.dev.yml` に適用する。

**追加するサービス定義:**

```yaml
  whisper:
    build:
      context: .              # virtual-voicebot-backend/ が context（このファイルの配置ディレクトリ相対）
      dockerfile: script/Dockerfile.whisper
    container_name: virtual-voicebot-backend-whisper
    restart: unless-stopped
    ports:
      - "${WHISPER_HOST_PORT:-9000}:9000"
    volumes:
      - whisper-model-cache:/var/cache/huggingface
    environment:
      HF_HOME: /var/cache/huggingface
      ASR_ENGINE: kotoba  # OQ-3 確定: kotoba のみ対応（reazon は Dockerfile 未対応）
      ASR_OUTPUT_SCRIPT: hiragana
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9000/healthz"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 180s
```

> **context の説明:** `virtual-voicebot-backend/docker-compose.dev.yml` は `virtual-voicebot-backend/` に配置されているため、`context: .` は `virtual-voicebot-backend/` を指す。この context では `COPY script/whisper_server.py` が `virtual-voicebot-backend/script/whisper_server.py` を正しく参照する（OQ-5 確定）。

**`app` サービスへの追記:**

```yaml
  app:
    environment:
      # 既存の設定はそのまま維持し、以下を追加
      ASR_LOCAL_SERVER_URL: http://whisper:9000/transcribe
    depends_on:
      # 既存の depends_on はそのまま維持し、以下を追加（リスト形式を維持）
      - whisper
```

> `virtual-voicebot-backend/docker-compose.dev.yml` の `app.depends_on` は条件なしのリスト形式のため、既存の形式に合わせる。

**`volumes` セクションへの追記:**

```yaml
volumes:
  # 既存の volume はそのまま維持し、以下を追加
  whisper-model-cache:
```

---

### 5.4 `script/whisper_server.py` への `GET /healthz` 追加

既存の `POST /transcribe` ルートより前（L117 付近）に以下を追加する。

```python
@app.get("/healthz")
async def healthz():
    return {"status": "ok", "engine": ASR_ENGINE}
```

**仕様:**
- パス: `GET /healthz`
- 認証: なし
- レスポンス: `200 OK`、`Content-Type: application/json`
- ボディ: `{"status": "ok", "engine": "<ASR_ENGINE の値>"}`
- 用途: Docker healthcheck / ダッシュボード死活確認 / curl による手動確認
- モデルロード完了の確認: サーバー起動後にモデルロードが完了した後にこのエンドポイントが応答する（FastAPI はモジュールロード完了後にリクエスト受付を開始するため、モデルロードが完了していれば自然に応答する）

---

### 5.5 既存 `ASR_LOCAL_SERVER_URL` との互換性

| 環境 | 設定後の `ASR_LOCAL_SERVER_URL` 値 | 補足 |
|------|----------------------------------|------|
| compose 内（docker-compose.yml） | `http://whisper:9000/transcribe` | compose service 名で解決 |
| `virtual-voicebot-backend/docker-compose.dev.yml`（DevContainer） | `http://whisper:9000/transcribe` | 同上 |
| Pi / ホスト直接起動 | `http://localhost:9000/transcribe`（デフォルト維持） | 既存動作を維持 |
| 別マシン構成 | 明示的に `ASR_LOCAL_SERVER_URL` を設定 | STEER-216 の設定化により対応済み |

> **ポイント:** Docker コンテナ内の `localhost` はコンテナ自身を指すため、compose ネットワーク経由で whisper コンテナに到達するには `whisper`（サービス名）を使う必要がある。

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #246 | STEER-246 | 起票 |
| Issue #245 | Issue #246 | 前提（ダッシュボード死活監視の基盤） |
| STEER-246 | script/Dockerfile.whisper（新規） | インフラ実装 |
| STEER-246 | docker-compose.yml（修正） | インフラ実装 |
| STEER-246 | `virtual-voicebot-backend/docker-compose.dev.yml`（DevContainer）（修正） | インフラ実装 |
| STEER-246 | script/whisper_server.py（修正） | GET /healthz 追加 |
| STEER-246 | STEER-216（ASR フォールバック設定） | ASR_LOCAL_SERVER_URL 設定を踏襲 |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [x] `start_period: 180s` の値（OQ-1 確定: 180s）
- [x] CPU 版 Dockerfile のみ（OQ-2 確定: GPU は別 Issue）
- [x] `depends_on: service_started`（OQ-4 確定: 開発利便性優先）
- [x] `script/` 配下に Dockerfile を配置（OQ-5 確定: context 整合確認済み）
- [x] reazon エンジン対応（OQ-3 確定: 今回対象外）
- [ ] モデルキャッシュ volume 名 `whisper-model-cache` に問題ないか
- [ ] `WHISPER_HOST_PORT` のデフォルト 9000 は他サービスと衝突しないか（開発環境確認）
- [ ] `virtual-voicebot-backend/docker-compose.dev.yml` と ルート `docker-compose.dev.yml` の役割の違いが実装者に伝わっているか（§5.3 の注記を参照）

### 7.2 マージ前チェック（Approved → Merged）

- [ ] `docker-compose up` で whisper サービスが起動し、healthcheck が通ることを確認
- [ ] `curl http://localhost:9000/healthz` が `{"status":"ok","engine":"kotoba"}` を返すことを確認
- [ ] backend コンテナから `http://whisper:9000/transcribe` への POST が成功することを確認
- [ ] 既存の Pi / ホスト直接起動でのデフォルト動作（`ASR_LOCAL_SERVER_URL` 未設定時）が変わらないことを確認
- [ ] モデルキャッシュ volume が永続化されていること（コンテナ再起動後に再ダウンロードされないことを確認）
- [ ] `docker compose -f docker-compose.yml -f docker-compose.dev.yml config` でマージ後の設定が意図通りか確認

---

## 8. 未確定点・質問

| # | 質問 | 選択肢 | オーナー回答 |
|---|------|--------|-------------|
| OQ-1 | `start_period` は 180s で十分か？ kotoba-whisper-v2.2 のモデルロードは CPU 環境で何秒程度かかるか？ | 120s / 180s / 300s | **180s で確定。** @MasanoriSuda 2026-02-24 |
| OQ-2 | GPU 対応を本ステアリングのスコープに含めるか？（CUDA ベースイメージへの対応） | CPU 版のみ（今回） / GPU 対応も含める | **CPU 版のみ（今回）。** GPU 対応は別イシューで検討。既存コード（L46-47）は `cuda.is_available()` で自動判定済みのため、CPU 環境でも動作する @MasanoriSuda 2026-02-24 |
| OQ-3 | `ASR_ENGINE=reazon` 使用時に必要な `nemo_toolkit` は CPU 版 Dockerfile に含めるか？（依存が大きいため） | 含める / reazon は別 Dockerfile or 今回対象外 | **今回対象外。** Dockerfile.whisper は kotoba エンジン（デフォルト）のみ対応。reazon 対応は別イシューで検討 @MasanoriSuda 2026-02-24 |
| OQ-4 | `depends_on: condition: service_healthy` を使うと compose up が whisper の healthcheck 通過まで待機するが、開発時に許容できるか？（代替: `service_started` に緩める） | service_healthy（厳格） / service_started（緩め） | **service_started（緩め）で確定。** 開発時の利便性を優先し、whisper 起動完了を待たずに backend を起動する @MasanoriSuda 2026-02-24 |
| OQ-5 | `docker-compose.yml` の `context` は `./virtual-voicebot-backend` だが、`Dockerfile.whisper` の `COPY` パスは context 相対なので `script/whisper_server.py` になる。この構成でよいか？ | OK / context を変更する | **OK。** `Dockerfile.whisper` は whisper 専用 Dockerfile。context=`./virtual-voicebot-backend` + `COPY script/whisper_server.py` の構成で問題なし @MasanoriSuda 2026-02-24 |

---

## 9. リスク・ロールバック観点

| リスク | 影響 | 緩和策 |
|--------|------|--------|
| モデルロードが遅く `start_period` を超える | compose healthcheck が `unhealthy` になり続ける | OQ-1 確定: `start_period=180s`。不足する場合は `start_period` を延長する。`service_started` 採用（OQ-4）のため backend の起動はブロックされない |
| モデルキャッシュ volume 未作成で初回起動が遅い | 初回のみ HF からダウンロードが発生 | 想定内。運用ドキュメントに初回のみ時間がかかる旨を記載 |
| `localhost:9000` 衝突 | ホスト上でポート 9000 が使用中の場合に起動失敗 | `WHISPER_HOST_PORT` 変数で回避可 |
| CPU 版 PyTorch が大きい | Dockerfile ビルド時間・イメージサイズが大きい | `torch --no-cache-dir` + CPU only で最小化。初回のみ時間がかかる |

**ロールバック手順:**
1. ルート `docker-compose.yml` と `virtual-voicebot-backend/docker-compose.dev.yml`（DevContainer）の `whisper` サービス定義と `ASR_LOCAL_SERVER_URL` を削除（ルート `docker-compose.dev.yml` は変更なしのため不要）
2. `script/Dockerfile.whisper` を削除
3. `script/whisper_server.py` の `GET /healthz` を削除
4. `ASR_LOCAL_SERVER_URL` のデフォルト（`http://localhost:9000/transcribe`）が使われる状態に戻る
5. whisper は従来通り手動起動

---

## 10. 非対象（明示的スコープ外）

| 項目 | 理由 |
|------|------|
| ダッシュボード UI へのウィジェット追加 | Issue #245 で対応（本ステアリングは監視基盤の整備のみ） |
| `GET /healthz` でのモデルロード完了チェック（推論テスト） | 通常のヘルスチェック範囲を超える。今回は FastAPI 起動確認のみとする |
| Whisper の別リポジトリ化 | 現段階では同一リポジトリ内で管理（安定後に別途検討） |
| GPU ビルドパイプライン整備 | **OQ-2 確定: 今回は対象外**。既存コードの `cuda.is_available()` により CPU 環境でも動作するため問題なし。CUDA ベースイメージへの切り替えは別イシューで対応 |
| Pi 上での Whisper Docker 化 | 別環境の問題。本ステアリングは開発 compose 環境のみ対象 |

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-24 | 初版作成 | Claude Sonnet 4.6 |
| 2026-02-24 | OQ-2 確定（CPU 版のみ）、§5.1 設計方針・§10 非対象を更新 | Claude Sonnet 4.6 |
| 2026-02-24 | OQ-1/3/4/5 確定。§5.1 設計方針・§5.2 depends_on を service_started に更新 | Claude Sonnet 4.6 |
| 2026-02-24 | Codex レビュー NG 対応（1回目）: §5.3 ターゲットファイル明確化（DevContainer compose / ルート override 分離）、§2.3 AC GPU 文言修正、§5.2 ASR_ENGINE reazon コメント削除、§7.1 OQ 確定済み項目更新、§9 リスク表修正、§4.2 影響ファイル表修正、§7.2 compose config 確認追加 | Claude Sonnet 4.6 |
| 2026-02-24 | Codex レビュー NG 対応（2回目）: §5.1 pip install 分離（torch/その他を別 RUN に）・ffmpeg 追加、残存 docker-compose.dev.yml 曖昧参照を §1/§3.6/§5.5/§6/§9 で全修正 | Claude Sonnet 4.6 |
| 2026-02-24 | Codex レビュー OK 確認。ステータス Draft → Approved（@MasanoriSuda lgtm）、§3.3 レビュー記録・§3.4 承認を記入 | Claude Sonnet 4.6 |
