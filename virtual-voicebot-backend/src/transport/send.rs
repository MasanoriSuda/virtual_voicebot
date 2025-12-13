use std::net::SocketAddr;

/// transport へ「このバイト列をこの宛先に送ってほしい」と依頼するための共通型。
/// 上位層（sip/session 等）が生成し、transport が UDP 送信する。
/// `src_port` は利用したいローカルポートを明示するためのメタ情報で、単一ソケット運用時は
/// ソケットの bind ポートと一致している前提。
#[derive(Debug, Clone)]
pub struct TransportSendRequest {
    pub dst: SocketAddr,
    pub src_port: u16,
    pub payload: Vec<u8>,
}

pub type TransportSendTx = tokio::sync::mpsc::UnboundedSender<TransportSendRequest>;
pub type TransportSendRx = tokio::sync::mpsc::UnboundedReceiver<TransportSendRequest>;
