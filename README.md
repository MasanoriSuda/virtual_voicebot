# Virtual Voicebot
本ソフトはSIP/RTP ベースの音声対話ボットシステムです。電話着信を受けて ASR→LLM→TTS のパイプラインで自動応答します。

## アーキテクチャ概要

```
┌─────────────────┐     ┌──────────────────────────────────────────┐
│  電話機/SIP     │────▶│  virtual-voicebot-backend (Rust)        │
│  クライアント   │◀────│                                          │
└─────────────────┘     │  ┌─────────┐  ┌─────────┐  ┌─────────┐  │
                        │  │ SIP/RTP │──│ Session │──│   App   │  │
                        │  └─────────┘  └─────────┘  └────┬────┘  │
                        └─────────────────────────────────┼───────┘
                                                          │
                        ┌─────────────────────────────────┼───────┐
                        │            AI サービス群        │       │
                        │  ┌─────┐  ┌─────┐  ┌─────────┐  │       │
                        │  │ ASR │  │ LLM │  │   TTS   │  │       │
                        │  └─────┘  └─────┘  └─────────┘  │       │
                        └─────────────────────────────────────────┘
```

## 必要なコンポーネント

| コンポーネント | 用途 | 選択肢 |
|---------------|------|--------|
| **ASR** | 音声→テキスト | AWS Transcribe（推奨）/ Whisper（ローカル） |
| **LLM** | 対話生成 | Google Gemini API（推奨）/ Ollama（ローカル） |
| **TTS** | テキスト→音声 | VOICEVOX（ずんだもん） |

> **Note**: TTS は現在 VOICEVOX のみ対応。他の TTS（Google Cloud TTS 等）は PR/Issue があれば対応予定。

---

## クイックスタート

### 1. 前提条件

- Rust toolchain（1.70+）
- Docker（AI サービス用）
- ネットワーク: SIP/RTP ポートが開放されていること
####

### 2. AI サービスの起動

> **Note**: Docker Compose による一括起動は対応中です。

#### Option A: クラウド API（推奨）

```bash
# Google Gemini API キーを取得
export GEMINI_API_KEY="your-api-key"

# AWS Transcribe を使用する場合
export USE_AWS_TRANSCRIBE=true
export AWS_TRANSCRIBE_BUCKET="your-bucket"
# AWS 認証情報（~/.aws/credentials または環境変数）
```

#### Option B: ローカル AI サービス

```bash
# Whisper ASR（ポート 9000）
docker run -d -p 9000:9000 onerahmet/openai-whisper-asr-webservice:latest

# Ollama LLM（ポート 11434）
docker run -d -p 11434:11434 ollama/ollama
docker exec -it <container> ollama pull gemma3:4b

# VOICEVOX TTS（ポート 50021）- 必須
docker run -d -p 50021:50021 voicevox/voicevox_engine:cpu-ubuntu20.04-latest
```

### 3. Backend のビルドと起動

```bash
cd virtual-voicebot-backend

# ビルド
cargo build --release

# 環境変数を設定して起動
export LOCAL_IP="192.168.1.100"        # ローカル IP
export ADVERTISED_IP="192.168.1.100"   # SDP に記載する IP
export SIP_PORT=5060                   # SIP ポート
export RTP_PORT=10000                  # RTP ポート
export GEMINI_API_KEY="your-api-key"   # LLM 用

cargo run --release
```

### 4. SIP Registrar への登録（キャリア接続時）

NTT Docomo 等のキャリアから着信を受けるには REGISTER が必要です。

```bash
# 追加の環境変数
export REGISTRAR_HOST="sip.example.com"
export REGISTRAR_PORT=5061
export REGISTRAR_TRANSPORT=tls
export REGISTER_USER="your-sip-user"
export REGISTER_DOMAIN="example.com"
export REGISTER_AUTH_USER="auth-user"
export REGISTER_AUTH_PASSWORD="password"

# TLS 証明書（TLS 接続時）
export TLS_CERT_PATH="/path/to/cert.pem"
export TLS_KEY_PATH="/path/to/key.pem"
```

---

## 環境変数一覧

### 必須

| 変数名 | 説明 | デフォルト |
|--------|------|-----------|
| `LOCAL_IP` | ローカル IP アドレス | `0.0.0.0` |
| `ADVERTISED_IP` | SDP に記載する外部 IP | `LOCAL_IP` と同じ |

### SIP/RTP

| 変数名 | 説明 | デフォルト |
|--------|------|-----------|
| `SIP_BIND_IP` | SIP バインド IP | `0.0.0.0` |
| `SIP_PORT` | SIP UDP/TCP ポート | `5060` |
| `SIP_TLS_PORT` | SIP TLS ポート | `5061` |
| `RTP_PORT` | RTP ポート | `10000` |
| `ADVERTISED_RTP_PORT` | 外部 RTP ポート | `RTP_PORT` と同じ |

### REGISTER（キャリア接続）

| 変数名 | 説明 | デフォルト |
|--------|------|-----------|
| `REGISTRAR_HOST` | Registrar ホスト名 | - |
| `REGISTRAR_PORT` | Registrar ポート | `5060` (UDP/TCP), `5061` (TLS) |
| `REGISTRAR_TRANSPORT` | トランスポート (`udp`/`tcp`/`tls`) | `udp` |
| `REGISTER_USER` | SIP ユーザー | - |
| `REGISTER_DOMAIN` | SIP ドメイン | `REGISTRAR_HOST` と同じ |
| `REGISTER_EXPIRES` | 登録有効期間（秒） | `3600` |
| `REGISTER_CONTACT_HOST` | Contact URI のホスト | `ADVERTISED_IP` |
| `REGISTER_CONTACT_PORT` | Contact URI のポート | `SIP_PORT` または `SIP_TLS_PORT` |
| `REGISTER_AUTH_USER` | 認証ユーザー | `REGISTER_USER` と同じ |
| `REGISTER_AUTH_PASSWORD` | 認証パスワード | - |

### TLS

| 変数名 | 説明 | デフォルト |
|--------|------|-----------|
| `TLS_CERT_PATH` | サーバー証明書パス | - |
| `TLS_KEY_PATH` | 秘密鍵パス | - |
| `TLS_CA_PATH` | CA 証明書パス（クライアント認証用） | - |

### AI サービス

| 変数名 | 説明 | デフォルト |
|--------|------|-----------|
| `GEMINI_API_KEY` | Gemini API キー | - |
| `GEMINI_MODEL` | Gemini モデル名 | `gemini-2.5-flash-lite` |
| `USE_AWS_TRANSCRIBE` | AWS Transcribe を使用 | `false` |
| `AWS_TRANSCRIBE_BUCKET` | S3 バケット名 | - |
| `AWS_TRANSCRIBE_PREFIX` | S3 プレフィックス | `voicebot` |

### タイムアウト

| 変数名 | 説明 | デフォルト |
|--------|------|-----------|
| `AI_HTTP_TIMEOUT_MS` | AI API タイムアウト | `20000` |
| `INGEST_HTTP_TIMEOUT_MS` | Ingest タイムアウト | `5000` |
| `RECORDING_IO_TIMEOUT_MS` | 録音 I/O タイムアウト | `5000` |
| `SIP_TCP_IDLE_TIMEOUT_MS` | SIP TCP アイドルタイムアウト | `30000` |

### RTP

| 変数名 | 説明 | デフォルト |
|--------|------|-----------|
| `RTP_JITTER_MAX_REORDER` | ジッタバッファ最大並び替え数 | `5` |
| `RTCP_INTERVAL_MS` | RTCP 送信間隔 | `5000` |

### ログ

| 変数名 | 説明 | デフォルト |
|--------|------|-----------|
| `RUST_LOG` | ログレベル | `info` |
| `LOG_MODE` | 出力先 (`stdout`/`file`) | `stdout` |
| `LOG_FORMAT` | フォーマット (`text`/`json`) | `text` |
| `LOG_DIR` | ログディレクトリ | `logs` |
| `LOG_FILE_NAME` | ログファイル名 | `app.log` |

---

## 通話フロー

```
1. INVITE 受信 → 180 Ringing → 200 OK
2. RTP 音声受信開始
3. 無音検出 → 発話区間を WAV 保存
4. ASR: WAV → テキスト変換
5. LLM: テキスト → 回答生成
6. TTS: 回答 → WAV 生成
7. RTP 音声送信（回答再生）
8. BYE 受信 → 通話終了
```

---

## トラブルシューティング

### SIP 接続できない

```bash
# SIP ポートの確認
netstat -an | grep 5060

# tcpdump で SIP パケット確認
sudo tcpdump -i any port 5060 -vvv
```

### REGISTER が失敗する

```bash
# 環境変数の確認
env | grep -E "REGISTRAR|REGISTER|TLS"

# TLS 証明書の確認
openssl s_client -connect sip.example.com:5061
```

### TTS が動作しない

```bash
# VOICEVOX の動作確認
curl -X POST "http://localhost:50021/audio_query?speaker=3&text=テスト" | jq
```

### ASR が動作しない

```bash
# Whisper の動作確認
curl -X POST -F "file=@test.wav" http://localhost:9000/transcribe
```

---

## ディレクトリ構成

```
virtual_voicebot/
├── README.md                 # 本ファイル
├── virtual-voicebot-backend/ # Rust バックエンド
│   ├── src/
│   │   ├── main.rs          # エントリポイント
│   │   ├── sip/             # SIP プロトコル実装
│   │   ├── rtp/             # RTP/RTCP 実装
│   │   ├── session/         # セッション管理
│   │   ├── ai/              # ASR/LLM/TTS クライアント
│   │   ├── app/             # アプリケーションロジック
│   │   ├── transport/       # UDP/TCP/TLS トランスポート
│   │   └── config.rs        # 設定管理
│   └── docs/
│       └── impl/PLAN.md     # 実装計画
└── virtual-voicebot-frontend/ # （将来）
```

---

## 関連ドキュメント

- [実装計画 (PLAN.md)](virtual-voicebot-backend/docs/impl/PLAN.md)
- [Backend README](virtual-voicebot-backend/README.md)
