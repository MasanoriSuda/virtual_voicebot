#![allow(dead_code)]

/// RTCP 受信イベントのスタブ。未実装であることを明示するための型。
#[derive(Debug, Clone)]
pub struct RtcpEvent {
    pub raw: Vec<u8>,
}

/// RTCP 送信要求のスタブ。SR/RR 生成は後続タスク。
#[derive(Debug, Clone)]
pub struct RtcpSendRequest {
    pub payload: Vec<u8>,
}

/// rtp→上位へ RTCP を通知するための I/F（現状は未使用）。
pub type RtcpEventTx = tokio::sync::mpsc::UnboundedSender<RtcpEvent>;

/// 上位→rtp へ RTCP 送信を依頼するための I/F（現状は未使用）。
pub type RtcpSendTx = tokio::sync::mpsc::UnboundedSender<RtcpSendRequest>;
