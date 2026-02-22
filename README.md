# Virtual Voicebot

SIP/RTP ベースの音声対話ボットシステムです。電話着信を受けて ASR → LLM → TTS のパイプラインで自動応答します。

> **対応プラットフォーム**: 通常 PC（x86_64 Linux）/ Raspberry Pi（ARM64）

---

## アーキテクチャ概要

```
┌─────────────────┐     ┌──────────────────────────────────────────┐
│  電話機/SIP     │────▶│  virtual-voicebot-backend (Rust)        │
│  クライアント   │◀────│                                          │
└─────────────────┘     │  ┌──────────┐  ┌──────────┐  ┌───────┐  │
                        │  │ protocol │──│ service  │──│  app  │  │
                        │  │ sip/rtp  │  │call_ctrl │  └───────┘  │
                        │  └──────────┘  └──────────┘             │
                        └─────────────────────────────────────────┘
                                          │
                        ┌─────────────────┼─────────────────────────┐
                        │            AI サービス群                  │
                        │  ┌─────────┐  ┌───────────┐  ┌─────────┐ │
                        │  │ Whisper │  │Gemini/    │  │VOICEVOX │ │
                        │  │  ASR    │  │Ollama LLM │  │  TTS    │ │
                        │  └─────────┘  └───────────┘  └─────────┘ │
                        └──────────────────────────────────────────┘
```

---

## 必要なコンポーネント

| コンポーネント | 用途 | 通常PC 推奨 | Raspberry Pi 推奨 |
|---------------|------|------------|------------------|
| **ASR** | 音声→テキスト | AWS Transcribe / Whisper | Whisper（ローカル） |
| **LLM** | 対話生成 | Gemini API / Ollama + `gemma3:4b` | Ollama + `llama3.2:1b` |
| **TTS** | テキスト→音声 | VOICEVOX（ずんだもん） | VOICEVOX（ずんだもん） |

> **重要**: AI エンドポイントはコード内で **localhost 固定** です（`localhost:9000` / `localhost:11434` / `localhost:50021`）。
> `.env.example` の `OLLAMA_URL` / `VOICEVOX_URL` 等の変数は現在のコードでは参照されません。

---

## クイックスタート

### 前提条件

- Rust toolchain（1.70+）
- ネットワーク: SIP ポート（5060）・RTP ポート（10000）が開放されていること

---

### 通常PC 向けセットアップ

#### 1. AI サービスの起動

```bash
# Whisper ASR（ポート 9000）
docker run -d -p 9000:9000 onerahmet/openai-whisper-asr-webservice:latest

# Ollama LLM（ポート 11434）
docker run -d -p 11434:11434 ollama/ollama
docker exec -it <container_name> ollama pull gemma3:4b

# VOICEVOX TTS（ポート 50021）— 必須
docker run -d -p 50021:50021 voicevox/voicevox_engine:cpu-ubuntu20.04-latest
```

#### 2. Backend のビルドと起動

```bash
cd virtual-voicebot-backend

# inbound-only（ローカルソフトフォンテスト）
ADVERTISED_IP=192.168.1.100 bash run_uas.sh

# Gemini API を使う場合は追加で設定
GEMINI_API_KEY=your-api-key ADVERTISED_IP=192.168.1.100 bash run_uas.sh
```

---

### Raspberry Pi 向けセットアップ

ラズパイでは Gemini API キーなし・Ollama + `llama3.2:1b` でローカル完結させます。

#### 1. Ollama のインストールとモデル取得

```bash
# Ollama インストール（ARM64 対応）
curl -fsSL https://ollama.com/install.sh | sh

# ラズパイ推奨モデル（約 1.3 GB）
ollama pull llama3.2:1b

# 品質重視の場合（RAM 4GB 以上・約 2.0 GB）
# ollama pull llama3.2:3b
```

#### 2. Whisper ASR・VOICEVOX の起動

```bash
# Whisper ASR
docker run -d -p 9000:9000 onerahmet/openai-whisper-asr-webservice:latest

# VOICEVOX TTS
docker run -d -p 50021:50021 voicevox/voicevox_engine:cpu-ubuntu20.04-latest
```

#### 3. Backend のビルドと起動

```bash
cd virtual-voicebot-backend
cargo build --release

# GEMINI_API_KEY は未設定 → call_gemini 失敗後に Ollama へフォールバック
ADVERTISED_IP=192.168.1.xxx \
OLLAMA_MODEL=llama3.2:1b \
bash run_uas.sh
```

> `GEMINI_API_KEY` 未設定時は毎回 `call_gemini failed` の error ログが出てから Ollama を呼びます。これは正常動作です。
> ラズパイ運用では `GEMINI_API_KEY` は**意図的に未設定**にするのが推奨です。

---

## 3モード実行

### Mode 1: inbound-only（ローカルソフトフォンテスト）

Registrar への登録なし。Zoiper 等のソフトフォンから直接 SIP INVITE を受ける構成。

```bash
cd virtual-voicebot-backend
ADVERTISED_IP=192.168.1.100 bash run_uas.sh
```

### Mode 2: キャリア接続（REGISTER あり）

NTT Docomo 等のキャリア SIP サーバーへ REGISTER して着信を受ける構成。

```bash
# .env.register ファイルを作成
cat > virtual-voicebot-backend/.env.register << 'EOF'
ADVERTISED_IP=203.0.113.10
LOCAL_IP=192.168.1.100
REGISTRAR_HOST=sip.carrier.example.com
REGISTRAR_PORT=5060
REGISTRAR_TRANSPORT=udp
REGISTER_USER=09012345678
REGISTER_DOMAIN=carrier.example.com
REGISTER_AUTH_USER=09012345678
REGISTER_AUTH_PASSWORD=yourpassword
OLLAMA_MODEL=llama3.2:1b
EOF

cd virtual-voicebot-backend
bash run_register.sh
# .env.register を自動読み込みして起動します
```

> **現時点で outbound（発信側）TLS は未対応です**。外部 SIP トランク向け TLS 対応は別 Issue で対応予定です。

### Mode 3: Outbound 発信

```bash
OUTBOUND_ENABLED=true \
OUTBOUND_DOMAIN=sip.example.com \
OUTBOUND_DEFAULT_NUMBER=09000000000 \
ADVERTISED_IP=192.168.1.100 \
bash virtual-voicebot-backend/run_uas.sh
```

> `OUTBOUND_ENABLED=true` のとき、着信 INVITE の To ユーザーが `REGISTER_USER` と一致しない場合に outbound 分岐します。
> `OUTBOUND_DOMAIN` 未設定時は起動時に warn ログが出ます。

---

## 環境変数一覧

### SIP/RTP

| 変数名 | 説明 | デフォルト |
|--------|------|-----------|
| `LOCAL_IP` | ローカル IP アドレス | `0.0.0.0` |
| `ADVERTISED_IP` | SDP に記載する外部 IP | `LOCAL_IP` と同じ |
| `SIP_BIND_IP` | SIP バインド IP | `0.0.0.0` |
| `SIP_PORT` | SIP UDP/TCP ポート | `5060` |
| `SIP_TLS_PORT` | SIP TLS ポート | `5061` |
| `RTP_PORT` | RTP 受信ポート | `10000` |
| `ADVERTISED_RTP_PORT` | SDP に記載する RTP ポート | `RTP_PORT` と同じ |

### REGISTER（キャリア接続）

| 変数名 | 説明 | デフォルト |
|--------|------|-----------|
| `REGISTRAR_HOST` | Registrar ホスト名/IP | — |
| `REGISTRAR_PORT` | Registrar ポート | `5060` |
| `REGISTRAR_TRANSPORT` | トランスポート (`udp`/`tcp`/`tls`) | `udp` |
| `REGISTER_USER` | SIP ユーザー | — |
| `REGISTER_DOMAIN` | SIP ドメイン | `REGISTRAR_HOST` と同じ |
| `REGISTER_AUTH_USER` | 認証ユーザー | `REGISTER_USER` と同じ |
| `REGISTER_AUTH_PASSWORD` | 認証パスワード | — |
| `REGISTER_EXPIRES` | 登録有効期間（秒） | `3600` |
| `REGISTER_CONTACT_HOST` | Contact URI ホスト | `ADVERTISED_IP` |

### TLS

| 変数名 | 説明 | デフォルト |
|--------|------|-----------|
| `TLS_CERT_PATH` | サーバー証明書パス | — |
| `TLS_KEY_PATH` | 秘密鍵パス | — |
| `TLS_CA_PATH` | CA 証明書パス | — |

### AI サービス

> AI エンドポイントはコード内で localhost 固定です（`:9000` / `:11434` / `:50021`）。URL 変数は現在参照されません。

| 変数名 | 説明 | デフォルト | ラズパイ推奨 |
|--------|------|-----------|------------|
| `GEMINI_API_KEY` | Gemini API キー（未設定で Ollama フォールバック） | — | 未設定 |
| `GEMINI_MODEL` | Gemini モデル名 | `gemini-2.5-flash-lite` | — |
| `OLLAMA_MODEL` | Ollama モデル名 | `gemma3:4b` | `llama3.2:1b` |
| `OLLAMA_INTENT_MODEL` | 意図分類用 Ollama モデル | `OLLAMA_MODEL` と同値 | `llama3.2:1b` |
| `USE_AWS_TRANSCRIBE` | AWS Transcribe を使用 | `false` | `false` |
| `AWS_TRANSCRIBE_BUCKET` | S3 バケット名 | — | — |
| `AWS_TRANSCRIBE_PREFIX` | S3 プレフィックス | `voicebot` | — |

### Outbound

| 変数名 | 説明 | デフォルト |
|--------|------|-----------|
| `OUTBOUND_ENABLED` | Outbound 発信を有効化 | `false` |
| `OUTBOUND_DOMAIN` | 発信先 SIP ドメイン | — |
| `OUTBOUND_DEFAULT_NUMBER` | デフォルト発信番号 | — |

### Session / VAD / タイムアウト

| 変数名 | 説明 | デフォルト |
|--------|------|-----------|
| `SESSION_TIMEOUT_SEC` | セッションタイムアウト（秒）。`0` で無制限 | `1800` |
| `SESSION_MIN_SE` | 最小セッション時間（秒） | `90` |
| `IVR_TIMEOUT_SEC` | IVR タイムアウト（秒） | `10` |
| `VAD_ENERGY_THRESHOLD` | 発話検出エネルギー閾値 | `500` |
| `VAD_END_SILENCE_MS` | 発話終了判定の無音時間（ms） | `800` |
| `AI_HTTP_TIMEOUT_MS` | AI API タイムアウト（ms） | `20000` |

### ログ

| 変数名 | 説明 | デフォルト |
|--------|------|-----------|
| `RUST_LOG` | ログレベル | `info` |
| `LOG_MODE` | 出力先 (`stdout`/`file`) | `stdout` |
| `LOG_FORMAT` | フォーマット (`text`/`json`) | `text` |
| `LOG_DIR` | ログディレクトリ（`file` 時） | `logs` |
| `LOG_FILE_NAME` | ログファイル名 | `app.log` |

---

## ソフトフォン設定（SIP クライアント側）

SIP クライアント（Zoiper / Linphone 等）のコーデック設定で **PCMU（PT 0）または PCMA（PT 8）を優先**に設定してください。

| 設定項目 | 推奨値 |
|---------|--------|
| コーデック優先順位 | PCMU（G.711 µ-law）> PCMA（G.711 a-law） |
| 動的 PT（96〜127） | 優先度を下げる（または無効化） |

> **注意**: SDP 先頭コーデックが動的 PT（例: 95）の場合、`unsupported payload type` となり音声が届きません。
> PCMU/PCMA を SDP の先頭に配置するようクライアント側で設定してください。

---

## 通話フロー

```
1. INVITE 受信 → 180 Ringing → 200 OK
2. RTP 音声受信開始（PCMU/PCMA のみ対応）
3. VAD: 発話区間を検出 → WAV 保存
4. ASR: WAV → テキスト変換（Whisper または AWS Transcribe）
5. LLM: テキスト → 回答生成（Gemini → Ollama フォールバック）
6. TTS: 回答 → WAV 生成（VOICEVOX）
7. RTP 音声送信（回答再生）
8. BYE 受信 → 通話終了・録音保存
```

同時着信の2コール目は **486 Busy Here** で拒否されます（同時1コールの制限）。

---

## ディレクトリ構成

```
virtual_voicebot/
├── README.md                        # 本ファイル
├── .env.example                     # 環境変数サンプル（参考用・実装と一部未整合）
├── virtual-voicebot-backend/        # Rust バックエンド
│   ├── src/
│   │   ├── main.rs                  # エントリポイント
│   │   ├── protocol/
│   │   │   ├── sip/                 # SIP プロトコル実装
│   │   │   ├── rtp/                 # RTP/RTCP 実装
│   │   │   ├── session/             # セッション管理
│   │   │   └── transport/           # UDP/TCP/TLS トランスポート
│   │   ├── service/
│   │   │   ├── ai/                  # ASR/LLM/TTS クライアント
│   │   │   ├── call_control/        # 対話制御
│   │   │   └── recording/           # 録音生成
│   │   └── shared/
│   │       └── config/              # 設定（環境変数）
│   ├── run_uas.sh                   # inbound-only 起動スクリプト
│   └── run_register.sh              # キャリア接続起動スクリプト（.env.register 自動読込）
└── virtual-voicebot-frontend/       # フロントエンド
```

---

## トラブルシューティング

### `unsupported payload type` が出て音声が届かない

SDP 先頭のコーデックが PCMU/PCMA 以外（動的 PT）になっています。

```bash
# SIP パケットで SDP を確認
sudo tcpdump -i any port 5060 -A | grep -A5 "m=audio"
```

ソフトフォン側で PCMU/PCMA を優先コーデックに設定してください。

### 503 Service Unavailable が返る

OUTBOUND_ENABLED=true のとき必須条件が不足しています。

```bash
env | grep -E "OUTBOUND|REGISTER"
```

`OUTBOUND_DOMAIN` が未設定の場合は起動ログに warn が出ます。

### 486 Busy Here が返る

すでに別コールが接続中です。VoiceBot は同時1コールのみ対応です。既存コールを終了してから再着信してください。

### REGISTER が失敗する

```bash
env | grep -E "REGISTRAR|REGISTER|TLS"
# TLS の場合
openssl s_client -connect sip.example.com:5061
```

`REGISTRAR_TRANSPORT=tls` の場合は `TLS_CERT_PATH` / `TLS_KEY_PATH` が必要です。

### LLM（Ollama）が応答しない

```bash
# Ollama の動作確認
curl http://localhost:11434/api/tags

# モデルが取得されているか確認
ollama list

# ラズパイ: llama3.2:1b が未取得の場合
ollama pull llama3.2:1b
```

> `GEMINI_API_KEY` 未設定時は毎回 `call_gemini failed` の error ログが出てから Ollama を呼びます。これは正常動作です。

### TTS（VOICEVOX）が動作しない

```bash
curl -X POST "http://localhost:50021/audio_query?speaker=3&text=テスト" | jq
```

### ASR（Whisper）が動作しない

```bash
curl -X POST -F "file=@test.wav" http://localhost:9000/transcribe
```

---

## 関連ドキュメント

- [Backend README](virtual-voicebot-backend/README.md)
- [設計書（AI モジュール）](virtual-voicebot-backend/docs/design/detail/DD-006_ai.md)
- [プロセス定義書](virtual-voicebot-backend/docs/process/v-model.md)
