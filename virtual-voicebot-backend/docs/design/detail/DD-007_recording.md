# Recording Design (MVP)

## 目的
- 通話（Zoiper ↔ backend）の音声を録音として保存し、Frontend から再生できるようにする
- 会話ログ（Utterance）と録音の時間軸を揃え、発話単位でシーク再生できる拡張に備える

## スコープ（MVP）
- 録音ファイルをローカルファイルとして保存する（`virtual-voicebot-backend` 内）
- Frontend は `docs/contract.md` の `recordingUrl` を使って再生する
- 形式はまず `mixed` の 1 本（混合音声）で開始する
- 将来、caller/bot の分離トラックや外部ストレージ（S3 等）に拡張可能な形にする

## 非目標（MVP）
- 署名付き URL・認証/認可の厳密化（Future）
- 高度な再エンコード（mp3/opus 等）や CDN 配信最適化（Future）
- 録音の編集・部分切り出し（Future）

---

## ディレクトリ構成
録音実体は `virtual-voicebot-backend` 配下に保存する。

```text
virtual-voicebot-backend/
  storage/
    recordings/
      <callId>/
        mixed.wav
        meta.json
```
- `<callId>` は `Call.callId` と一致させる
- ファイル名は固定（MVP）
- `mixed.wav`: 通話の混合音声
- `meta.json`: 録音メタデータ

## 役割分担（責務）
- **media モジュール（src/shared/media/）**: PCM 等の音声を受け取り、録音ファイルを生成・保存。`storage/recordings/<callId>/` 配下の生成と更新を担当し、録音タイムラインの 0 秒基準を決めてメタに保存。
- **http モジュール（src/interface/http/）**: `docs/contract.md` に沿って録音を配信。ブラウザの `<audio>` 再生・シークのため、HTTP Range に対応する（**MVP 必須**）。
- **session / rtp（protocol/session / protocol/rtp）**: 録音の開始/停止などライフサイクルの指示を出す。RTP/RTCP の詳細処理は protocol/rtp が担当し、録音は shared/media に委譲する。

## 録音のライフサイクル（MVP）
- **開始**: 通話が成立し RTP ストリームが開始した時点で録音を開始
- **終了**: 通話終了イベントを受けたら録音を finalize（ファイルクローズ、meta 確定など）

## 形式とフォーマット（MVP）
- **音声形式**: `mixed.wav`（PCM 16-bit little endian を想定）
- **メタ**: sample rate / channels は `meta.json` に明記する
- **将来**: mp3/opus などにエンコード、`mixed` 以外に `caller.wav` / `bot.wav` を追加

## meta.json（MVP 必須項目）
最低限、次の情報を保存する。

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

## 時間軸の約束
- `recordingStartedAt` を録音タイムラインの 0 秒基準とする
- `docs/contract.md` の `Utterance.startSec` / `endSec` は、この録音タイムラインの秒数として扱う前提
- MVP では `startSec` / `endSec` が null でもよい（まず再生を優先）

## 配信（MVP）
- **contract との整合**: `Call.recordingUrl` は録音再生に利用できる URL を返す。MVP では backend が直接配信し、将来短寿命の署名付き URL に置き換える可能性がある。
- **Range 対応: MVP 必須**（2025-12-27 確定、Refs Issue #7 CX-2）
  - ブラウザのシーク再生を安定させるため、HTTP Range ヘッダに対応する
  - `Accept-Ranges: bytes` / `206 Partial Content` / `416 Range Not Satisfiable` を実装する
  - 詳細は `docs/contract.md` の Transport DTO を参照

## 将来拡張（Future）
- 外部ストレージ（S3/MinIO/R2）へアップロードし、配信は署名付き URL/CDN へ移行
- caller/bot の分離トラック対応
- Utterance と録音の時間同期精度向上（より正確な `startSec` / `endSec`）
- 録音の削除ポリシー・保持期間・圧縮/エンコード戦略
