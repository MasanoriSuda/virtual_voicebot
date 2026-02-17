use std::net::SocketAddr;

pub type ConnId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransportPeer {
    Udp(SocketAddr),
    Tcp(ConnId),
}

/// transport へ「このバイト列をこの peer に送ってほしい」と依頼するための共通型。
/// 上位層（sip/session 等）が生成し、transport が UDP/TCP 送信する。
/// `src_port` は利用したいローカルポートを明示するためのメタ情報で、単一ソケット運用時は
/// ソケットの bind ポートと一致している前提。
#[derive(Debug, Clone)]
pub struct TransportSendRequest {
    pub peer: TransportPeer,
    pub src_port: u16,
    pub payload: Vec<u8>,
}

pub type TransportSendTx = tokio::sync::mpsc::Sender<TransportSendRequest>;
pub type TransportSendRx = tokio::sync::mpsc::Receiver<TransportSendRequest>;
