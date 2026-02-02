<!-- SOURCE_OF_TRUTH: 機能設計書 -->
# 機能設計書（FDD）

> Virtual Voicebot Backend の機能設計を定義する

---

## 1. システム構成

### 1.1 モジュール構成図

```
┌─────────────────────────────────────────────────────────────────┐
│                          main.rs                                 │
│                     (エントリポイント)                            │
└─────────────────────────────────────────────────────────────────┘
                                │
        ┌───────────────────────┼───────────────────────┐
        ▼                       ▼                       ▼
┌───────────────┐      ┌───────────────┐      ┌───────────────┐
│   transport   │      │    session    │      │     http      │
│  (UDP I/O)    │◄────►│ (コール制御)   │      │ (録音配信)    │
└───────────────┘      └───────────────┘      └───────────────┘
        │                  │       │
        ▼                  ▼       ▼
┌───────────────┐    ┌─────────┐ ┌─────────┐
│      sip      │    │   app   │ │  media  │
│ (SIPプロトコル)│    │(対話制御)│ │ (録音)  │
└───────────────┘    └─────────┘ └─────────┘
        │                  │
        ▼                  ▼
┌───────────────┐    ┌─────────────────────────────┐
│      rtp      │    │            ai               │
│ (RTP/RTCP)    │    │   ┌─────┬─────┬─────┐       │
└───────────────┘    │   │ asr │ llm │ tts │       │
                     │   └─────┴─────┴─────┘       │
                     └─────────────────────────────┘
```

### 1.2 データフロー

```
【着信フロー】
Zoiper → transport → sip → session → app
                                      ↓
                                     ai::asr

【応答フロー】
ai::tts → app → session → rtp → transport → Zoiper

【録音フロー】
rtp → session → media → storage/recordings/
                         ↓
http ←─────────────── mixed.wav
```

---

## 2. 機能詳細設計

### FN-1: SIP 着信処理

#### 2.1.1 概要
SIP INVITE を受信し、通話セッションを確立する。

#### 2.1.2 処理フロー

```
1. transport: UDP パケット受信
2. sip: INVITE をパース
3. sip: 100 Trying 送信
4. session: セッション生成（call_id 採番）
5. sip: 180 Ringing 送信（100rel 対応時は RSeq 付与）
6. session: SDP 解析、RTP 設定準備
7. sip: 200 OK 送信（SDP 応答含む）
8. sip: ACK 受信待ち
9. session: Confirmed 状態へ遷移、RTP 開始
```

#### 2.1.3 入出力

| 方向 | データ | 型 |
|------|--------|-----|
| 入力 | SIP INVITE | `SipMessage` |
| 入力 | SDP Offer | `SdpOffer` |
| 出力 | SIP 100/180/200 | `SipMessage` |
| 出力 | SDP Answer | `SdpAnswer` |
| 出力 | `CallStarted` | `SessionOut` |

#### 2.1.4 状態遷移

| 現状態 | イベント | 次状態 | アクション |
|--------|---------|--------|-----------|
| Idle | INVITE受信 | Proceeding | 100/180送信 |
| Proceeding | 200送信 | Confirmed待ち | ACK待ち |
| Confirmed待ち | ACK受信 | Confirmed | RTP開始 |
| Confirmed | BYE受信 | Terminated | 200送信、終了処理 |

---

### FN-2: RTP 音声処理

#### 2.2.1 概要
RTP パケットの送受信と PCM 変換を行う。

#### 2.2.2 受信処理

```
1. transport: UDP パケット受信（ポート判別で RTP/RTCP 振り分け）
2. rtp: RTP ヘッダパース（SSRC, Seq, Timestamp, PT）
3. rtp: ペイロード抽出
4. rtp: PCMU → PCM デコード（G.711 μ-law）
5. rtp: 簡易ジッタバッファ処理
6. session: PcmInputChunk として通知
7. app: PcmReceived として受信
```

#### 2.2.3 送信処理

```
1. app: BotAudioReady（PcmOutputChunk）送信
2. session: rtp へ転送
3. rtp: PCM → PCMU エンコード
4. rtp: RTP ヘッダ組立（Seq++, Timestamp+=160）
5. transport: UDP 送信
```

#### 2.2.4 パラメータ

| 項目 | 値 | 備考 |
|------|-----|------|
| コーデック | PCMU (G.711 μ-law) | PT=0 |
| サンプルレート | 8000 Hz | |
| チャンネル | 1 (mono) | |
| フレームサイズ | 160 samples | 20ms |
| ジッタバッファ | ~100ms | 設定可 |

---

### FN-3: 音声認識（ASR）

#### 2.3.1 概要
ユーザー発話の PCM を外部 ASR サービスでテキスト化する。

#### 2.3.2 処理フロー

```
1. app: PcmReceived 受信
2. app: AsrInputPcm を ai::asr チャネルへ送信
3. asr: 外部 API 呼び出し（HTTP/WebSocket）
4. asr: 認識結果を AsrResult で返却
5. app: テキストを受信、LLM へ送信準備
```

#### 2.3.3 入出力 DTO

**入力:**
```rust
AsrInputPcm {
    session_id: String,
    stream_id: String,
    pcm: Vec<i16>,      // 8000Hz, mono
    sample_rate: u32,   // = 8000
    channels: u8,       // = 1
    chunk_ms: u32,      // ≈ 20
}
```

**出力:**
```rust
AsrResult {
    session_id: String,
    stream_id: String,
    text: String,
    is_final: bool,
    meta: Option<AsrMeta>,
}

AsrError {
    session_id: String,
    reason: String,
}
```

#### 2.3.4 エラー処理

| エラー | 対応 |
|--------|------|
| 1回目失敗 | 謝罪音声「もう一度お願いします」→ 継続 |
| 連続失敗（2回） | BYE 送信で終了 |
| タイムアウト | 謝罪音声 → 終了 |

---

### FN-4: 対話生成（LLM）

#### 2.4.1 概要
ASR テキストを LLM に送信し、応答テキストを生成する。

#### 2.4.2 処理フロー

```
1. app: AsrResult（is_final=true）受信
2. app: 履歴 + ユーザーテキストで LlmRequest 組立
3. llm: 外部 API 呼び出し（OpenAI 等）
4. llm: LlmResponse で応答テキスト返却
5. app: TTS へ送信準備
```

#### 2.4.3 入出力 DTO

**入力:**
```rust
LlmRequest {
    session_id: String,
    history: Vec<Message>,
    user_text: String,
    meta: Option<LlmMeta>,
}
```

**出力:**
```rust
LlmResponse {
    text: String,
    action: Option<String>,
    end_flag: Option<bool>,
    meta: Option<LlmMeta>,
}

LlmError {
    session_id: String,
    reason: String,
}
```

#### 2.4.4 コンテキスト管理

- 履歴は app が保持し、LlmRequest に含めて渡す
- 履歴の最大長は config で制限
- llm モジュールはステートレス（純クライアント）

---

### FN-5: 音声合成（TTS）

#### 2.5.1 概要
LLM 応答テキストを外部 TTS サービスで音声化する。

#### 2.5.2 処理フロー

```
1. app: LlmResponse 受信
2. app: TtsRequest を ai::tts チャネルへ送信
3. tts: 外部 API 呼び出し
4. tts: TtsPcmChunk を順次返却（ストリーミング）
5. app: BotAudioReady として session へ送信
6. session: rtp へ PCM 転送
```

#### 2.5.3 入出力 DTO

**入力:**
```rust
TtsRequest {
    session_id: String,
    stream_id: String,
    text: String,
    options: Option<TtsOptions>,
}
```

**出力:**
```rust
TtsPcmChunk {
    session_id: String,
    stream_id: String,
    pcm: Vec<i16>,      // 8000Hz, mono
    sample_rate: u32,   // = 8000
    is_last: bool,
}

TtsError {
    session_id: String,
    reason: String,
}
```

---

### FN-6: 録音保存

#### 2.6.1 概要
通話音声を WAV ファイルとして保存する。

#### 2.6.2 処理フロー

```
1. session: RTP 開始時に media へ録音開始指示
2. media: WAV ファイル作成（storage/recordings/<callId>/mixed.wav）
3. session: PcmInputChunk を media へ転送
4. media: PCM を WAV に書き込み
5. session: 通話終了時に録音停止指示
6. media: WAV finalize、meta.json 生成
```

#### 2.6.3 ファイル構成

```
storage/recordings/<callId>/
├── mixed.wav      # 混合音声（PCM 16-bit LE, 8000Hz, mono）
└── meta.json      # メタデータ
```

#### 2.6.4 meta.json

```json
{
  "callId": "c_123",
  "recordingStartedAt": "2025-12-13T00:00:00.000Z",
  "sampleRate": 8000,
  "channels": 1,
  "durationSec": 123.45,
  "files": {
    "mixed": "mixed.wav"
  }
}
```

---

### FN-7: 録音配信

#### 2.7.1 概要
HTTP で録音ファイルを配信する（Range 対応）。

#### 2.7.2 エンドポイント

| メソッド | パス | 説明 |
|----------|------|------|
| GET | `/recordings/<callId>/mixed.wav` | 録音ファイル取得 |
| HEAD | `/recordings/<callId>/mixed.wav` | ファイル情報取得 |

#### 2.7.3 Range 対応

| リクエスト | レスポンス |
|-----------|-----------|
| 通常 GET | `200 OK` + 全ファイル |
| `Range: bytes=0-1023` | `206 Partial Content` + 1024バイト |
| `Range: bytes=0-` | `206` または `200` |
| 不正 Range | `416 Range Not Satisfiable` |
| 存在しない callId | `404 Not Found` |

#### 2.7.4 レスポンスヘッダ

```
Content-Type: audio/wav
Accept-Ranges: bytes
Content-Length: <size>
Content-Range: bytes <start>-<end>/<total>  # 206 時
```

---

### FN-8: 履歴連携

#### 2.8.1 概要
通話終了時に Frontend へ履歴情報を送信する。

#### 2.8.2 処理フロー

```
1. session: BYE 受信または終了判断
2. session: CallEnded を app へ通知
3. app: 通話情報を集約
4. session: ingest API 呼び出し（reqwest）
5. Frontend: 履歴を DB に保存
```

#### 2.8.3 API 呼び出し

**エンドポイント:** `POST /api/ingest/call`

**ペイロード:**
```json
{
  "callId": "c_123",
  "from": "sip:zoiper@example",
  "to": "sip:bot@example",
  "startedAt": "2025-12-13T00:00:00.000Z",
  "endedAt": "2025-12-13T00:05:00.000Z",
  "status": "ended",
  "summary": "配送状況の確認。住所変更あり。",
  "durationSec": 300,
  "recording": {
    "recordingUrl": "http://backend/recordings/c_123/mixed.wav",
    "durationSec": 300,
    "sampleRate": 8000,
    "channels": 1
  }
}
```

---

## 3. モジュール間インタフェース

### 3.1 session ↔ app イベント

#### session → app

| イベント | 用途 | 必須フィールド |
|----------|------|---------------|
| `CallStarted` | 通話開始通知 | call_id, session_id |
| `PcmReceived` | PCM チャンク | call_id, session_id, stream_id, pcm |
| `CallEnded` | 通話終了通知 | call_id, session_id |
| `SessionTimeout` | タイムアウト | call_id, session_id |

#### app → session

| イベント | 用途 | 必須フィールド |
|----------|------|---------------|
| `BotAudioReady` | TTS 音声送信 | call_id, session_id, stream_id, PcmOutputChunk |
| `HangupRequested` | BYE 送信指示 | call_id, session_id |

### 3.2 app ↔ ai イベント

#### app → ai

| イベント | 送信先 | 必須フィールド |
|----------|--------|---------------|
| `AsrInputPcm` | asr | session_id, stream_id, pcm |
| `LlmRequest` | llm | session_id, history, user_text |
| `TtsRequest` | tts | session_id, stream_id, text |

#### ai → app

| イベント | 送信元 | 必須フィールド |
|----------|--------|---------------|
| `AsrResult` | asr | session_id, stream_id, text, is_final |
| `AsrError` | asr | session_id, reason |
| `LlmResponse` | llm | text |
| `LlmError` | llm | session_id, reason |
| `TtsPcmChunk` | tts | session_id, stream_id, pcm, is_last |
| `TtsError` | tts | session_id, reason |

### 3.3 sip ↔ session イベント

#### sip → session

| イベント | 用途 |
|----------|------|
| `SipInvite` | INVITE 受信 |
| `SipAck` | ACK 受信 |
| `SipBye` | BYE 受信 |
| `SipCancel` | CANCEL 受信 |
| `SipTransactionTimeout` | トランザクションタイムアウト |

#### session → sip

| イベント | 用途 |
|----------|------|
| `SipSend100` | 100 Trying 送信 |
| `SipSend180` | 180 Ringing 送信 |
| `SipSend200` | 200 OK 送信 |
| `SipSendBye200` | BYE への 200 OK 送信 |

---

## 4. エラーハンドリング

### 4.1 エラーポリシー

| 条件 | アクション |
|------|------------|
| AI 1回目失敗 | 謝罪音声で継続 |
| AI 連続失敗（config 管理、既定: 2回） | BYE 送信で終了 |
| RTP タイムアウト | SessionTimeout → app 判断 |
| SIP トランザクションタイムアウト | TransactionTimeout 通知 |

### 4.2 フォールバックメッセージ

| エラー種別 | メッセージ |
|-----------|-----------|
| ASR 失敗 | 「聞き取れませんでした。もう一度お願いします」 |
| LLM 失敗 | 「少々お待ちください」 |
| TTS 失敗 | （無音、ログのみ） |

---

## 5. 設定項目

### 5.1 環境変数

| 変数名 | 説明 | デフォルト |
|--------|------|-----------|
| `SIP_BIND_IP` | SIP バインド IP | 0.0.0.0 |
| `SIP_PORT` | SIP ポート | 5060 |
| `ADVERTISED_IP` | SDP 記載 IP | - |
| `ADVERTISED_RTP_PORT` | RTP ポート | - |
| `RECORDING_HTTP_ADDR` | 録音配信サーバ bind | 0.0.0.0:18080 |
| `RECORDING_BASE_URL` | 録音 URL ベース | - |
| `INGEST_CALL_URL` | Frontend ingest API | - |

### 5.2 config 管理項目

| 項目 | 説明 | デフォルト |
|------|------|-----------|
| `error_threshold` | 連続失敗閾値 | 2 |
| `session_timeout_sec` | セッションタイムアウト | 120 |
| `rtp_jitter_max_reorder` | ジッタバッファ再整列上限 | 5 |
| `rtcp_interval_ms` | RTCP 送信間隔 | 5000 |

---

## 6. 参照ドキュメント

| ドキュメント | 対応機能 |
|-------------|---------|
| [sip.md](sip.md) | FN-1: SIP 着信処理 |
| [rtp.md](rtp.md) | FN-2: RTP 音声処理 |
| [ai.md](ai.md) | FN-3/4/5: ASR/LLM/TTS |
| [session.md](session.md) | セッション管理 |
| [app.md](app.md) | 対話制御 |
| [recording.md](recording.md) | FN-6: 録音保存 |
| [contract.md](../../docs/contract.md) | FN-7/8: 録音配信/履歴連携 |
