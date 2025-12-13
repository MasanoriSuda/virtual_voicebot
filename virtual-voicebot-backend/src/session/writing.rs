#![allow(dead_code)]
// wiring.rs（上位＝SIP/メディア/Botとの結線）
//
// 責務: Session 内部の状態機械と、外部モジュール (sip/rtp/app) への出口を結線する。
// ここでは経路だけ定義し、実際の送信/受信はまだスタブのまま（挙動は従来どおり）。
use tokio::sync::mpsc::unbounded_channel;

use crate::session::types::*;
use crate::session::{Session, SessionHandle};

/// セッションを生成し、SessionOut を上位レイヤに配線する（挙動は従来と同じ）。
pub fn spawn_call(call_id: CallId, media_cfg: MediaConfig) -> SessionHandle {
    let (tx_up, rx_out) = unbounded_channel::<SessionOut>();
    let handle = Session::spawn(call_id.clone(), tx_up, media_cfg);

    // SIP受信側でINVITE→セッションに投げる
    // tx_in.send(SessionIn::Invite{...}).unwrap();

    // セッション→上位の指示をここで分配
    tokio::spawn(async move {
        let mut rx_out = rx_out;
        while let Some(out) = rx_out.recv().await {
            match out {
                SessionOut::SendSip180 => { /* 180 Ringing 送出 */ }
                SessionOut::SendSip200 { answer: _ } => { /* 200 OK + SDP 送出 */ }
                SessionOut::StartRtpTx {
                    dst_ip: _,
                    dst_port: _,
                    pt: _,
                } => { /* RTP送信タスク起動 */ }
                SessionOut::StopRtpTx => { /* 停止 */ }
                SessionOut::SendSipBye200 => { /* BYEに対する200 OK送出 */ }
                SessionOut::BotSynthesize { text: _ } => { /* VOICEVOX叩いて BotAudio を返送 */
                }
                SessionOut::Metrics { name: _, value: _ } => { /* メトリクス集計 */ }
            }
        }
    });

    handle
}

pub fn spawn_session(
    call_id: CallId,
    session_map: SessionMap,
    media_cfg: MediaConfig,
) -> tokio::sync::mpsc::UnboundedSender<SessionIn> {
    let handle = spawn_call(call_id.clone(), media_cfg);
    // Session manager の薄いラッパ経由で登録（挙動は従来通り）
    let registry = SessionRegistry::new(session_map);
    let _ = registry.insert(call_id, handle.tx_in.clone());
    handle.tx_in
}
