#![allow(dead_code)]

use std::net::SocketAddr;

/// sip→transport 方向の送信依頼を表すスタブ。
/// 現状は transport 側の即時返信が残っているため、ここでは型のみ定義する。
#[derive(Debug, Clone)]
pub struct SipTransportRequest {
    pub dst: SocketAddr,
    pub payload: Vec<u8>,
}

/// sip モジュールが transport に送信を依頼するためのキュー I/F（スタブ）。
/// 本タスクでは定義のみで、実際の送信経路切り替えは後続タスクで行う。
pub type SipTransportTx = tokio::sync::mpsc::UnboundedSender<SipTransportRequest>;
