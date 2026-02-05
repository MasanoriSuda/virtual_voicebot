// src/rtp/packet.rs

/// シンプルな RTP パケット表現。
/// CSRC/拡張ヘッダは保持し、必要ならビルド/パースで反映する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtpPacket {
    pub version: u8, // 通常は 2
    pub padding: bool,
    pub marker: bool,
    pub payload_type: u8,
    pub sequence_number: u16,
    pub timestamp: u32,
    pub ssrc: u32,
    pub csrcs: Vec<u32>,
    pub extension: Option<RtpExtension>,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtpExtension {
    pub profile: u16,
    pub data: Vec<u8>,
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
            marker: false,
            payload_type,
            sequence_number,
            timestamp,
            ssrc,
            csrcs: Vec::new(),
            extension: None,
            payload,
        }
    }
}
