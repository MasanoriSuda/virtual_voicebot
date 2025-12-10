// src/rtp/packet.rs

/// シンプルな RTP パケット表現.
/// v1では CSRC / 拡張ヘッダは未対応（必要になったら拡張）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtpPacket {
    pub version: u8, // 通常は 2
    pub padding: bool,
    pub extension: bool,
    pub csrc_count: u8, // 今は 0 固定扱いでもOK
    pub marker: bool,
    pub payload_type: u8,
    pub sequence_number: u16,
    pub timestamp: u32,
    pub ssrc: u32,
    pub payload: Vec<u8>,
}

impl RtpPacket {
    /// よくあるデフォルトを設定して新規作成.
    pub fn new(
        payload_type: u8,
        sequence_number: u16,
        timestamp: u32,
        ssrc: u32,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            version: 2,
            padding: false,
            extension: false,
            csrc_count: 0,
            marker: false,
            payload_type,
            sequence_number,
            timestamp,
            ssrc,
            payload,
        }
    }
}
