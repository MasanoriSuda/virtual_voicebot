<!-- SOURCE_OF_TRUTH: Session詳細設計 -->
# session モジュール詳細設計 (`src/session`)

## 1. 目的・スコープ
- 通話セッションのライフサイクル管理（生成/破棄/検索）と SIP/RTP 設定の橋渡しを行う。
- 呼状態（Idle/Early/Confirmed/Terminating/Terminated）の遷移と Session Timer/keepalive の保持。
- RTP 開始/停止と送信先設定のみを扱い、AI/アプリロジックは扱わない。

## 2. session manager API
- `create(call_id, media_cfg)` : 新規セッションを生成し内部マップに登録。ローカルSDP/RT P送信設定の初期化。
- `get(call_id)` / `remove(call_id)` : 参照/削除。BYE 受信や Session Timer 失効時に破棄。
- `list()` : デバッグ/メトリクス用。
- 保持情報: call_id、ダイアログ情報（From/To タグ、CSeq）、状態、タイマハンドル群、rtp 設定（ローカルIP/ポート/SSRC/PT）、sip/rtp へのチャネル。

## 3. Session Timer の状態管理（RFC 4028 対応）

### 3.1 Session-Expires の決定
- INVITE に `Session-Expires` ヘッダがある場合 → その値を使用
- INVITE に `Session-Expires` がない場合 → 環境変数 `SESSION_TIMEOUT_SEC`（デフォルト 1800秒）を使用
- `SESSION_TIMEOUT_SEC=0` の場合 → 無制限（タイマなし）

### 3.2 refresher の動作
| refresher | 責任 | Voicebot の動作 |
|-----------|------|----------------|
| `uas` | Voicebot | Session-Expires の 80% 経過時に UPDATE を送信 |
| `uac` | 相手側 | 相手からの re-INVITE/UPDATE を受信し 200 OK を返す |

### 3.3 持つタイマ
- **Session-Expires タイマ**: Confirmed 遷移で開始。refresher=uas の場合は 80% で UPDATE 送信をトリガ。
- **keepalive/無音監視タイマ**: RTP 送受信の監視（既存）。

### 3.4 ライフサイクル
1. **開始**: Confirmed 遷移で Session-Expires タイマを開始
2. **リフレッシュ**: re-INVITE/UPDATE 送受信成功時に残り時間をリセット
3. **終了**: BYE/エラー/終了で全タイマ停止

### 3.5 発火時の通知
- `SessionTimeout` を `session → app` に送る。
- app が `HangupRequested` を返した場合、session が BYE を sip へ送る。

## 4. keepalive / タイムアウト時の SessionOut
- keepalive運用: 20〜30ms 程度の周期 tick で無音フレーム送出やメトリクス更新を行う。送出失敗が連続する場合は `SessionOut::StopRtpTx` と `SessionOut::SendSipBye200` をトリガ。
- タイムアウト時のアクション: 原則 app に `SessionTimeout` を通知し方針を仰ぐ。即切断が必要な場合のみ直接 `SessionOut::StopRtpTx` / `SendSipBye200` を発火。
- 任意メトリクス: `SessionOut::Metrics { name: "session_timeout", value: 1 }` などを発行しても良い。

## 5. イベントと責務（RTP/PCM とコール制御の分離）

> **正本参照（2025-12-28 追記）**: session↔app イベント名の正本は [app.md](app.md) §2

- SIP起点入力: `SessionIn::SipInvite` / `SipAck` / `SipBye` / `SipTransactionTimeout`
- メディア入力: `SessionIn::PcmInputChunk`（rtp からデコード済み PCM を受ける）、keepalive tick は `MediaTimerTick`
- app入力: `SessionIn::BotAudioReady` / `HangupRequested`（app.md §2 参照）
- SIP出力: `SessionOut::SipSend180` / `SipSend200` / `SipSendBye200`
- RTP出力: `SessionOut::RtpStartTx` / `RtpStopTx` / `PcmOutputChunk`（app から受けた PCM を rtp へ転送）
- app出力: `SessionOut::CallStarted` / `PcmReceived` / `CallEnded` / `SessionTimeout`（app.md §2 参照）
- メトリクス: `SessionOut::Metrics`

## 6. 他モジュールとの責務境界
- sip: 受信 SIP を `SessionIn::SipInvite/Ack/Bye/...` で通知。応答送信は `SessionOut::SipSend*` で依頼。
- rtp: I/O と SSRC/Seq 管理・簡易ジッタは rtp。session は開始/停止と送信先設定だけ伝える（`RtpStartTx/RtpStopTx`）。rtp からは `PcmInputChunk` で PCM を受け取る。
- app/ai: 対話・ASR/LLM/TTS は app/ai で完結。session はコール制御と PCM 経路制御のみ。app からは `BotAudioReady`（payload: `PcmOutputChunk`）で TTS 音声を受け取り、`HangupRequested` で BYE 指示を受ける（app.md §2 参照）。session は受け取った `PcmOutputChunk` を rtp へ転送する。

## 7. 実装状況

### 7.1 RFC 4028 Session Timer（Step-19 で実装）
- [x] Session-Expires ヘッダ解析
- [x] Min-SE ヘッダ解析
- [x] refresher=uas 時の UPDATE 送信（80% 経過時）
- [x] refresher=uac 時の re-INVITE/UPDATE 受信・応答
- [x] Session-Expires なしの場合のデフォルト値対応

### 7.2 環境変数
| 変数名 | 説明 | デフォルト |
|--------|------|-----------|
| `SESSION_TIMEOUT_SEC` | デフォルト Session-Expires（秒）。`0` で無制限。 | `1800` |
| `SESSION_MIN_SE` | 最小 Session-Expires（秒） | `90` |

### 7.3 残タスク
- 無音監視ポリシーの高度化（将来）
