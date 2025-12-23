# docs/todo_media.md

全体のTODOは `docs/todo.md` を参照。

## 目的
- media モジュールの録音設計・実装タスクを管理する。

## MVP
- [x] mixed.wav + meta.json を `storage/recordings/<callId>/` に生成する録音パイプラインを追加する。
- [x] recording timeline の 0秒基準を定義し、Utterance と同期できるよう meta.json に開始時刻/長さを記録する。
- [ ] session/rtp からの音声入力を media に渡すイベント/チャネル経路を用意する（HTTP配信は含めない）。
- [ ] 録音チャネルの型を定義する（StartRecording/StopRecording/PcmChunk）。PcmChunk は call_id, seq/timestamp, payload(PCM16 or μ-law) を持つ。
- [x] RTP受信側で PCMU→PCM16 変換したデータを録音チャネルに流し、送信用PCMも合流させて mixed とする。
- [x] 送受信の双方を同一サンプルレート/チャネルで合成し、1本の mixed.wav に書き出す（重複/整列ポリシーも明記）。
- [ ] 簡易整列/欠損処理を入れる（Seq/Timestamp で整列し、穴は無音で埋めるかスキップするポリシーを選ぶ）。
- [x] WAV ライターで mixed.wav を生成し、終了時にヘッダ長を確定させる。ディレクトリ生成とクリーンアップを含む。
- [x] meta.json に callId/recordingStartedAt/sampleRate/channels/durationSec/files.mixed を書き出す。
- [x] BYE/SessionTimeout などで StopRecording を確実に発火させ、未close時も安全に終了する。

## NEXT
- [ ] mixed/caller/bot の複数トラック録音に対応する（トラック間の開始時刻を同期）。
- [ ] mp3/opus などへのエンコードオプションを追加する（媒体生成のみ、配信は http の責務）。
- [ ] 録音ファイルとメタデータを外部ストレージ（例: S3/MinIO）へアップロードする経路を追加する。
