# RTP モジュール詳細設計 (`src/rtp`)

## 1. 目的・スコープ

- 目的:
  - RTP/RTCP パケット処理と音声ストリーム管理を担当する。
  - PCM と RTP ペイロードの相互変換を行い、ASR/TTS と連携できるようにする。
- スコープ:
- 単一 SSRC / 単一コーデックの音声ストリーム（MVPでは G.711 PCMU/PCMA / 8000Hz）
  - 安定した受信・送信パイプラインの構築
- 非スコープ:
  - 音声認識（ASR）や TTS の中身
  - SIP/SDP の解析（それは `sip`/`session` 側）

## 2. 依存関係

- 依存するモジュール:
  - `transport::packet`: RTP/RTCP 生パケットの送受信
- 依存されるモジュール:
  - `session`: SDP が決めたメディア設定の受け取り
  - `ai::asr`: PCM の入力先
  - `ai::tts`: PCM の供給元

## 3. 主な責務

1. RTP/RTCP パケット表現
   - RTP ヘッダ（SSRC, Seq, Timestamp, PayloadType, Marker, CSRC etc）
   - RTCP SR/RR の最小実装（送受信・統計の雛形）

2. パーサ / ビルダ
   - バイト列 ⇔ RTP/RTCP 構造体
   - 異常ケース（ヘッダ長不正など）のハンドリング

3. ストリーム管理
   - SSRC / (送信元 IP/Port) 単位のストリーム
   - Seq / Timestamp の管理
   - 受信バッファ（簡易ジッタバッファ）の方針
     - MVPでは「ほぼストレートに流す」でも良いが、TODO として残す

4. コーデック抽象
   - MVP: PCMU (G.711 μ-law) のみ
   - PCM ⇔ RTP ペイロードの変換 API を提供
   - 将来的なコーデック追加（PCMA/Opus 等）を視野に入れたインタフェース設計

5. ASR/TTS との連携
   - 受信: RTP → PCM → `ai::asr` へチャンク送信
   - 送信: `ai::tts` から PCM チャンクを引き取り、RTP にエンコードして送出

## 4. ストリームモデル

### 4.1 セッションとの紐付け

- `session` から渡される情報:
  - リモート IP/Port
  - ローカル送信ポート
  - コーデック (PayloadType)
  - SSRC を誰が決めるか（rtp 側で生成 or session 側から指定）
- `rtp` 内部では:
  - `(セッションID, SSRC)` をキーにストリームを管理
  - ストリームごとに Seq/Timestamp を持つ

### 4.2 受信側の流れ

1. `transport::packet` が RTP パケットを受信
2. `rtp` がヘッダ/ペイロードをパース
3. 該当ストリームを見つける（SSRC or IP/Port で）
4. 必要なら簡易的なジッタバッファ/整列処理
5. ペイロードを PCM にデコード
6. PCM チャンクを `ai::asr` に渡す（イベント or チャネル）

### 4.3 送信側の流れ

1. `ai::tts` から PCM チャンクを受け取る
2. コーデックで RTP ペイロードにエンコード
3. Seq/Timestamp をインクリメント
4. RTP ヘッダを組み立ててバイト列化
5. `transport::packet` に送信依頼

## 5. RTCP の扱い

- MVP/NEXT:
  - SR/RR の送受信を行う（送信間隔は設定で調整）
  - RR には jitter / loss / lsr / dlsr を最小限で載せる（精度は段階的に改善）
  - 受信した SR/RR はログに残す（品質観測の入口）

ここでは、「MVP 時点では何をしないか」も明示する。

## 6. ストリーム詳細設計（SSRC/Seq/Timestamp・ジッタ・RTCP）

### 6.1 SSRC/Seq/Timestamp 管理
- SSRC: セッションから送信開始指示を受けた際、rtp がランダム32bitを生成してストリームに紐付ける（MVPは1セッション1 SSRC固定、途中切替なし）。
- 送信 Seq: 初期値は乱数16bit。パケット送信ごとに +1、ロールオーバは自然に巻き戻し（特別処理なし）。
- 送信 Timestamp: 初期値は乱数32bit。PCMU/8000Hzで1フレーム160サンプルとし、送信ごとに +160。ロールオーバも自然に進める。
- 受信解釈: 軽微な逆順/欠損は許容。Seq が最新より大きく逆行するものは破棄。Timestamp はペイロード長相当の前進を期待し、大きく過去のものは破棄。SSRC 変更は MVP では未対応（同一 SSRC のみ受理）。

### 6.2 簡易ジッタポリシー
- 方針: ほぼストレートパスだが、古いパケットを破棄し、ごく短い整列バッファで扱う。
- バッファ: 既定は約5フレーム（≒100ms）で Seq 順に整列。`RTP_JITTER_MAX_REORDER` で上限を調整できる。
- 欠損: 無音挿入は行わずスキップ。連続欠損が目立つ場合は警告ログ（将来 `RtpLossWarning` などのイベント化を検討）。
- 遅延許容: 最新 Seq より `RTP_JITTER_MAX_REORDER` 以上遅れて到着したパケットは破棄。Timestamp も Seq に準拠して古すぎるものを捨てる。
- ASR への影響: 短時間の欠損・遅延は許容し、ASR入力はほぼリアルタイムを優先。精緻なジッタ補償は後続スプリントで拡張。

### 6.3 RTCP 送受インタフェース（実装方針）
- 方針: SR/RR を周期送信し、受信した SR/RR をログに残す。
- 送信:
  - SR: 送信ストリームの統計（packet/octet、RTP timestamp）を使って生成。
  - RR: 受信ストリームの統計（loss/jitter/lsr/dlsr）を使って生成。
  - 送信間隔は `RTCP_INTERVAL_MS` で調整する。
- 受信:
  - SR 受信時は `lsr/dlsr` 用の参照値を保持する。
  - RR 受信時は品質観測ログに残す（将来的にイベント化）。

### 6.4 上位モジュールとの関係
- `rtp → ai::asr`: デコード済み PCM を `PcmInputChunk` として渡す。Seq/Timestamp/ジッタ処理は rtp 内で吸収し、上位は PCM のみ扱う。
- `ai::tts → rtp`: PCM フレームを `PcmOutputChunk` で受け取り、rtp が Seq/Timestamp/SSRC を付与して RTP 化・送信。
- 抽象化: 上位（session/app/ai）は SSRC/Seq/Timestamp/ジッタを意識せず、PCM とイベントのみを扱う前提。時間管理・整列・廃棄ポリシーは rtp で完結させる。

## 7. 運用確認（RTCP SR/RR のキャプチャ）

- RTCP は RTP ポート + 1 を使用する（例: RTP 10000 → RTCP 10001）。
- 例: `tcpdump -n -s0 -vv udp port <rtp_port+1>` で SR/RR が周期的に流れていることを確認する。
- 送信間隔は `RTCP_INTERVAL_MS`、ジッタ整列上限は `RTP_JITTER_MAX_REORDER` で調整する。

## 8. エラー・タイムアウト処理

- 受信エラー:
  - ヘッダ異常 / ペイロード長異常 → ログ + パケット破棄
- RTP 無着信:
  - 一定時間（設定値）パケットが来ない場合 `RtpTimeout` イベントを `session` or `app` に送る
- コーデックエラー:
  - デコード不能の場合の扱い（無音扱い / スキップ / ログのみ等）

## 9. MVP と拡張範囲

- MVP で対応:
  - 単一 SSRC / 単一コーデック (PCMU/PCMA)
  - 簡易ジッタバッファ（既定約100ms、`RTP_JITTER_MAX_REORDER` で調整）と遅延パケット破棄
  - RTCP SR/RR の送受（最小統計でのレポート）
- NEXT で追加:
  - ジッタバッファと再整列ロジック
  - RTCP 統計精度の向上（jitter/lsr/dlsr/損失率の精緻化）
  - 複数コーデック対応（Opus 等）
