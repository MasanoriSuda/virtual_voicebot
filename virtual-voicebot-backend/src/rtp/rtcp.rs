#![allow(dead_code)]

use std::net::SocketAddr;

/// RTCP 受信イベントのスタブ。未実装であることを明示するための型。
#[derive(Debug, Clone)]
pub struct RtcpEvent {
    pub raw: Vec<u8>,
    pub src: SocketAddr,
    pub dst_port: u16,
}

/// RTCP 送信要求のスタブ。SR/RR 生成は後続タスク。
#[derive(Debug, Clone)]
pub struct RtcpSendRequest {
    pub dst: SocketAddr,
    pub payload: Vec<u8>,
}

/// rtp→上位へ RTCP を通知するための I/F（現状は未使用）。
pub type RtcpEventTx = tokio::sync::mpsc::UnboundedSender<RtcpEvent>;

/// 上位→rtp へ RTCP 送信を依頼するための I/F（現状は未使用）。
pub type RtcpSendTx = tokio::sync::mpsc::UnboundedSender<RtcpSendRequest>;

/// 簡易判定: RTCP パケットかどうか（Version=2 かつ PT が 192-223 の範囲を検知）
pub fn is_rtcp_packet(data: &[u8]) -> bool {
    if data.len() < 2 {
        return false;
    }
    let v = data[0] >> 6;
    let pt = data[1];
    v == 2 && (192..=223).contains(&pt)
}
