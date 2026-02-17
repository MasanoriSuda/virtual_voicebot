#![allow(dead_code)]

/// RTP ストリーム管理のプレースホルダ（Seq/Timestamp/SSRC 管理用）。
/// 現状は transport 直結のままなので未使用。将来 rtp モジュールに処理を寄せるための枠だけ定義。
#[derive(Debug, Default, Clone)]
pub struct RtpStreamState {
    pub ssrc: u32,
    pub sequence_number: u16,
    pub timestamp: u32,
}

impl RtpStreamState {
    pub fn new(ssrc: u32, sequence_number: u16, timestamp: u32) -> Self {
        Self {
            ssrc,
            sequence_number,
            timestamp,
        }
    }

    /// 次の Seq/TS を進める（実際の送信は別途）。
    pub fn advance(&mut self, ts_incr: u32) {
        self.sequence_number = self.sequence_number.wrapping_add(1);
        self.timestamp = self.timestamp.wrapping_add(ts_incr);
    }
}
