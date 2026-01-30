<!-- SOURCE_OF_TRUTH: 技術仕様書 -->
# 技術仕様書（TSD）

> Virtual Voicebot Backend の技術仕様を定義する

---

## 1. 技術スタック

### 1.1 プログラミング言語

| 項目 | 選定 | 理由 |
|------|------|------|
| 言語 | **Rust** | メモリ安全性、高性能、async/await サポート |
| エディション | 2021 | 最新の安定版機能を利用 |
| ビルドツール | Cargo | Rust 標準パッケージマネージャ |

### 1.2 主要依存クレート

| カテゴリ | クレート | バージョン | 用途 |
|---------|---------|-----------|------|
| **非同期ランタイム** | tokio | 1.x | async I/O、タスクスケジューリング |
| **エラーハンドリング** | anyhow | 1.x | 柔軟なエラー型 |
| **ログ** | log + env_logger | 0.4 / 0.11 | 構造化ログ出力 |
| **HTTP クライアント** | reqwest | 0.12 | AI API / ingest 呼び出し |
| **シリアライズ** | serde + serde_json | 1.x | JSON 変換 |
| **設定** | toml | 0.8 | 設定ファイル読み込み |
| **パーサ** | nom | 7.x | SIP メッセージパース |
| **音声ファイル** | hound | 3.x | WAV 読み書き |
| **日時** | chrono | 0.4 | タイムスタンプ処理 |
| **AWS SDK** | aws-config, aws-sdk-s3, aws-sdk-transcribe | 1.x | クラウドサービス連携 |

### 1.3 開発ツール

| ツール | 用途 |
|--------|------|
| `cargo fmt` | コードフォーマット |
| `cargo clippy` | リンター |
| `cargo test` | ユニットテスト |
| SIPp | E2E テスト（SIP シナリオ） |

---

## 2. アーキテクチャ

### 2.1 レイヤ構造

```
┌─────────────────────────────────────────────┐
│            Application Layer                │
│  ┌─────────────┐    ┌──────────────────┐    │
│  │     app     │───►│   ai (asr/llm/tts)│   │
│  │ (対話制御)   │    │  (外部API連携)    │   │
│  └─────────────┘    └──────────────────┘    │
├─────────────────────────────────────────────┤
│            Session Layer                    │
│  ┌─────────────────────────────────────┐    │
│  │              session                 │    │
│  │   (SIP Dialog + RTP Session 統合)   │    │
│  └─────────────────────────────────────┘    │
├─────────────────────────────────────────────┤
│            Protocol Layer                   │
│  ┌───────────────┐    ┌───────────────┐     │
│  │      sip      │    │      rtp      │     │
│  │ (SIP Protocol)│    │  (RTP/RTCP)   │     │
│  └───────────────┘    └───────────────┘     │
├─────────────────────────────────────────────┤
│            Transport Layer                  │
│  ┌─────────────────────────────────────┐    │
│  │         transport::packet           │    │
│  │         (UDP/TCP Socket I/O)        │    │
│  └─────────────────────────────────────┘    │
├─────────────────────────────────────────────┤
│            Infrastructure Layer             │
│  ┌────────────┐  ┌────────────┐             │
│  │   media    │  │    http    │             │
│  │  (録音)    │  │ (配信)     │             │
│  └────────────┘  └────────────┘             │
└─────────────────────────────────────────────┘
```

### 2.2 依存関係ルール

#### 許可される依存

```
app  ────► session ────► sip ────► transport
  │            │           │
  │            └────► rtp ─┘
  └────► ai (asr/llm/tts)
```

#### 禁止される依存

| 禁止パターン | 理由 |
|-------------|------|
| ai → sip/rtp/transport | プロトコル詳細からの分離 |
| session → ai | 対話ロジックは app の責務 |
| app → transport/sip/rtp | session を経由すること |
| http → sip/rtp | 配信はプロトコルに立ち入らない |
| session/app → http | HTTP に引きずられない |

### 2.3 通信パターン

| パターン | 用途 | 実装 |
|---------|------|------|
| チャネル | モジュール間イベント | `tokio::sync::mpsc` |
| Future | 1リクエスト1レスポンス | `async fn` / `.await` |
| イベント駆動 | SIP/RTP 受信処理 | イベントループ |

---

## 3. プロトコル仕様

### 3.1 SIP (RFC 3261)

#### 対応メソッド

| メソッド | 方向 | 対応状況 |
|---------|------|---------|
| INVITE | 受信 | MVP |
| ACK | 受信 | MVP |
| BYE | 受信/送信 | MVP |
| CANCEL | 受信 | P0（未実装） |
| OPTIONS | 受信 | 将来 |
| UPDATE | 受信/送信 | P1（実装済み） |
| PRACK | 受信/送信 | P1（実装済み） |

#### トランザクションタイマ（UDP/UAS）

| タイマ | 初期値 | 最大値 | 用途 |
|--------|-------|-------|------|
| Timer A | T1 (500ms) | T1*64 | リクエスト再送（UAC） |
| Timer B | 64*T1 | - | INVITE タイムアウト（UAC） |
| Timer G | T1 | T2 | 応答再送（UAS INVITE） |
| Timer H | 64*T1 | - | ACK 待ちタイムアウト |
| Timer I | T4 (5s) | - | Confirmed → Terminated |
| Timer J | 64*T1 | - | 非 INVITE 完了待ち |

#### 状態遷移（INVITE サーバトランザクション）

```
                    INVITE受信
                        │
                        ▼
              ┌─────────────────┐
              │   Proceeding    │◄──┐
              └────────┬────────┘   │
                       │ 100/180    │ 再送INVITE
                       ▼            │
              ┌─────────────────┐   │
          ┌───│   Proceeding    │───┘
          │   └────────┬────────┘
     3xx-6xx           │ 2xx
          │            ▼
          │   ┌─────────────────┐
          │   │   Terminated    │ ← 2xx送信でトランザクション終了
          │   └─────────────────┘   （ACK待ちはUASコアで管理）
          ▼
  ┌─────────────────┐
  │   Completed     │ Timer G/H
  └────────┬────────┘
           │ ACK受信
           ▼
  ┌─────────────────┐
  │   Confirmed     │ Timer I
  └────────┬────────┘
           │
           ▼
  ┌─────────────────┐
  │   Terminated    │
  └─────────────────┘
```

### 3.2 RTP (RFC 3550)

#### パケットフォーマット

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|V=2|P|X|  CC   |M|     PT      |       Sequence Number         |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                           Timestamp                           |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|           Synchronization Source (SSRC) identifier            |
+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
|                          Payload ...                          |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

#### 対応コーデック

| コーデック | Payload Type | サンプルレート | 状態 |
|-----------|-------------|---------------|------|
| PCMU (G.711 μ-law) | 0 | 8000 Hz | MVP |
| PCMA (G.711 A-law) | 8 | 8000 Hz | MVP |
| Opus | 動的 | 48000 Hz | 将来 |

#### SSRC/Seq/Timestamp 管理

| 項目 | 仕様 |
|------|------|
| SSRC | セッション開始時にランダム32bit生成 |
| Sequence | 初期値ランダム16bit、送信毎に +1 |
| Timestamp | 初期値ランダム32bit、送信毎に +160（20ms @ 8kHz） |

### 3.3 RTCP

#### 対応レポート

| レポート | 方向 | 状態 |
|---------|------|------|
| SR (Sender Report) | 送信 | MVP |
| RR (Receiver Report) | 送信 | MVP |

#### 送信間隔

- 既定: 5000ms（`RTCP_INTERVAL_MS` で設定可）

---

## 4. データ構造

### 4.1 相関 ID 規約

| ID | スコープ | 用途 |
|----|---------|------|
| `call_id` | 通話全体 | ログ/トレース、外部 API |
| `session_id` | セッション | 内部イベント（MVP: call_id と同値） |
| `stream_id` | メディアストリーム | PCM 系イベント |

### 4.2 SIP メッセージ構造

```rust
struct SipMessage {
    start_line: StartLine,    // Request-Line or Status-Line
    headers: Headers,         // Via, From, To, Call-ID, CSeq, ...
    body: Option<Vec<u8>>,    // SDP など
}

enum StartLine {
    Request { method: Method, uri: String, version: String },
    Response { version: String, status: u16, reason: String },
}
```

### 4.3 RTP パケット構造

```rust
struct RtpPacket {
    version: u8,              // = 2
    padding: bool,
    extension: bool,
    csrc_count: u8,
    marker: bool,
    payload_type: u8,         // 0 = PCMU
    sequence: u16,
    timestamp: u32,
    ssrc: u32,
    payload: Vec<u8>,
}
```

### 4.4 PCM チャンク構造

```rust
struct PcmChunk {
    session_id: String,
    stream_id: String,
    samples: Vec<i16>,        // 8000Hz, mono, 16-bit signed
    sample_rate: u32,         // = 8000
    channels: u8,             // = 1
}
```

---

## 5. 外部サービス連携

### 5.1 ASR（音声認識）

| 項目 | 仕様 |
|------|------|
| プロトコル | HTTP / WebSocket |
| 入力形式 | PCM 16-bit, 8000Hz, mono |
| 出力形式 | JSON（text, is_final） |
| タイムアウト | config 管理 |
| リトライ | config 管理（既定: 1回） |

### 5.2 LLM（対話生成）

| 項目 | 仕様 |
|------|------|
| プロトコル | HTTP（REST API） |
| 入力形式 | JSON（history, user_text） |
| 出力形式 | JSON（text, action, end_flag） |
| モデル | config 管理 |
| タイムアウト | config 管理 |

### 5.3 TTS（音声合成）

| 項目 | 仕様 |
|------|------|
| プロトコル | HTTP |
| 入力形式 | JSON（text, options） |
| 出力形式 | PCM 16-bit, 8000Hz, mono |
| ストリーミング | チャンク分割で返却 |
| 終端判定 | `is_last: true` |

### 5.4 Frontend 連携

| API | 方向 | 用途 |
|-----|------|------|
| `POST /api/ingest/call` | Backend → Frontend | 通話履歴プッシュ |
| `GET /recordings/<callId>/mixed.wav` | Frontend → Backend | 録音再生 |

---

## 6. ストレージ

### 6.1 録音ファイル

| 項目 | 仕様 |
|------|------|
| 保存先 | `storage/recordings/<callId>/` |
| 形式 | WAV (PCM 16-bit LE, 8000Hz, mono) |
| ファイル名 | `mixed.wav` |
| メタデータ | `meta.json` |

### 6.2 ディレクトリ構造

```
storage/
└── recordings/
    └── <callId>/
        ├── mixed.wav
        └── meta.json
```

---

## 7. ネットワーク

### 7.1 ポート構成

| 用途 | プロトコル | ポート | 備考 |
|------|----------|--------|------|
| SIP シグナリング | UDP | 5060 | 環境変数で変更可 |
| RTP メディア | UDP | 動的 | SDP で通知 |
| RTCP | UDP | RTP+1 | RFC 3550 |
| 録音配信 | HTTP | 18080 | 環境変数で変更可 |

### 7.2 パケットフロー

```
Zoiper (UAC)                Backend (UAS)
    │                            │
    │──── INVITE + SDP ─────────►│ :5060/UDP
    │◄─── 100 Trying ────────────│
    │◄─── 180 Ringing ───────────│
    │◄─── 200 OK + SDP ──────────│
    │──── ACK ──────────────────►│
    │                            │
    │◄═══ RTP (PCMU) ═══════════►│ :動的/UDP
    │◄═══ RTCP SR/RR ═══════════►│ :動的+1/UDP
    │                            │
    │──── BYE ──────────────────►│
    │◄─── 200 OK ────────────────│
```

---

## 8. エラー処理

### 8.1 エラーカテゴリ

| カテゴリ | 例 | 対応 |
|---------|-----|------|
| プロトコルエラー | 不正 SIP メッセージ | ログ + 破棄 or 4xx 応答 |
| AI エラー | ASR/LLM/TTS 失敗 | フォールバック音声 |
| ネットワークエラー | 送信失敗 | リトライ or 切断 |
| タイムアウト | RTP 無着信 | SessionTimeout 通知 |

### 8.2 リトライ戦略

| 対象 | 戦略 |
|------|------|
| SIP 応答再送 | Timer G/H に従う（指数バックオフ） |
| AI API | config 管理（既定: 1回リトライ） |
| ingest API | 1回リトライ、失敗時はログのみ |

---

## 9. セキュリティ

### 9.1 PII 取り扱い

| データ | 扱い |
|--------|------|
| 音声データ | PII として保護 |
| 文字起こし | PII として保護 |
| LLM 入出力 | PII として保護 |

### 9.2 ログマスキング

- 原文（音声内容/テキスト全文）はログに出力しない
- デバッグフラグ有効時のみ限定的に出力

### 9.3 認証（MVP）

- MVP では認証なし（ローカル/閉域想定）
- 将来: Bearer トークン、署名付き URL

---

## 10. 参照仕様

| 仕様 | ドキュメント |
|------|-------------|
| SIP Core | RFC 3261 |
| 100rel/PRACK | RFC 3262 |
| UPDATE | RFC 3311 |
| Session Timers | RFC 4028 |
| SDP | RFC 8866 |
| RTP/RTCP | RFC 3550 |
| SDP Offer/Answer | RFC 3264 |
