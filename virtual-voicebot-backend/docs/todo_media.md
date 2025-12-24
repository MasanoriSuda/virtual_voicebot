# docs/todo_media.md

全体のTODOは `docs/todo.md` を参照。

## 目的
- media モジュールの録音設計・実装タスクを管理する。

## MVP
- [ ] session/rtp からの音声入力を media に渡すイベント/チャネル経路を用意する（HTTP配信は含めない）。
- [ ] 録音チャネルの型を定義する（StartRecording/StopRecording/PcmChunk）。PcmChunk は call_id, seq/timestamp, payload(PCM16 or μ-law) を持つ。
- [ ] 簡易整列/欠損処理を入れる（Seq/Timestamp で整列し、穴は無音で埋めるかスキップするポリシーを選ぶ）。


## NEXT
- [ ] mixed/caller/bot の複数トラック録音に対応する（トラック間の開始時刻を同期）。
- [ ] mp3/opus などへのエンコードオプションを追加する（媒体生成のみ、配信は http の責務）。
- [ ] 録音ファイルとメタデータを外部ストレージ（例: S3/MinIO）へアップロードする経路を追加する。
