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

    let csrc_len = csrc_count as usize * 4;
    let mut offset = 12 + csrc_len;
    if buf.len() < offset {
        return Err(RtpParseError::TooShort);
    }

    if extension {
        if buf.len() < offset + 4 {
            return Err(RtpParseError::TooShort);
        }
        let ext_len_words = u16::from_be_bytes([buf[offset + 2], buf[offset + 3]]) as usize;
        offset += 4 + ext_len_words * 4;
        if buf.len() < offset {
            return Err(RtpParseError::TooShort);
        }
    }

    let mut payload_end = buf.len();
    if padding {
        if payload_end <= offset {
            return Err(RtpParseError::TooShort);
        }
        let pad_len = buf[payload_end - 1] as usize;
        if pad_len == 0 || pad_len > payload_end - offset {
            return Err(RtpParseError::TooShort);
        }
        payload_end -= pad_len;
    }

    let payload = buf[offset..payload_end].to_vec();

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
