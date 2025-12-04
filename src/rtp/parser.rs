// src/rtp/parse.rs

use crate::rtp::packet::RtpPacket;

#[derive(Debug)]
pub enum RtpParseError {
    TooShort,
    #[allow(dead_code)]
    UnsupportedVersion(u8),
    // 今後、拡張ヘッダ未対応などを足していける
}

pub fn parse_rtp_packet(buf: &[u8]) -> Result<RtpPacket, RtpParseError> {
    if buf.len() < 12 {
        return Err(RtpParseError::TooShort);
    }

    let b0 = buf[0];
    let b1 = buf[1];

    let version = b0 >> 6;
    let padding = (b0 & 0b0010_0000) != 0;
    let extension = (b0 & 0b0001_0000) != 0;
    let csrc_count = b0 & 0b0000_1111;

    if version != 2 {
        // とりあえずバージョン2だけ対応
        return Err(RtpParseError::UnsupportedVersion(version));
    }

    let marker = (b1 & 0b1000_0000) != 0;
    let payload_type = b1 & 0b0111_1111;

    let sequence_number = u16::from_be_bytes([buf[2], buf[3]]);
    let timestamp = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);
    let ssrc = u32::from_be_bytes([buf[8], buf[9], buf[10], buf[11]]);

    // v1では csrc_count と extension は無視して payload をそのまま残り全部とする
    // TODO: csrc_count > 0 や extension = true のときに対応する
    let header_len = 12; // いまは固定
    if buf.len() < header_len {
        return Err(RtpParseError::TooShort);
    }

    let payload = buf[header_len..].to_vec();

    Ok(RtpPacket {
        version,
        padding,
        extension,
        csrc_count,
        marker,
        payload_type,
        sequence_number,
        timestamp,
        ssrc,
        payload,
    })
}
