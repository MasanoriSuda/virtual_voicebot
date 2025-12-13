//! app モジュール（対話オーケストレーション層）のスタブ。
//! 現状は未配線で、責務のみをコメントとして示す（挙動は一切追加しない）。
//!
//! - session からのイベント（通話開始/終了/音声入力完了など）を受け取り、
//!   ai::{asr,llm,tts} を呼び分ける役割を担う予定。
//! - ai 呼び出し結果を解釈し、SessionOut に相当する指示（BotAudio/終了など）を返す予定。
//! - transport/sip/rtp には直接依存しない（設計 doc: docs/design.md に準拠）。

#![allow(dead_code)]

/// セッションから届くイベントのプレースホルダ。
/// 将来、channel I/F を設計した際に具体化する。
#[derive(Debug)]
pub enum AppEvent {
    #[allow(dead_code)]
    CallStarted { call_id: String },
    #[allow(dead_code)]
    AudioBuffered { call_id: String, pcm_mulaw: Vec<u8> },
    #[allow(dead_code)]
    CallEnded { call_id: String },
}

/// app から session へ返す指示のプレースホルダ。
#[derive(Debug)]
pub enum AppCommand {
    #[allow(dead_code)]
    BotAudioPcmUlaw(Vec<u8>),
    #[allow(dead_code)]
    Hangup,
}
