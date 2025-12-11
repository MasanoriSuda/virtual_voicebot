// src/rtp/build.rs

use crate::rtp::packet::RtpPacket;

pub fn build_rtp_packet(pkt: &RtpPacket) -> Vec<u8> {
    let mut buf = Vec::with_capacity(12 + pkt.payload.len());

    let mut b0 = (pkt.version & 0b11) << 6;
    if pkt.padding {
        b0 |= 0b0010_0000;
    }
    if pkt.extension {
        b0 |= 0b0001_0000;
    }
    b0 |= pkt.csrc_count & 0b0000_1111;

    let mut b1 = 0u8;
    if pkt.marker {
        b1 |= 0b1000_0000;
    }
    b1 |= pkt.payload_type & 0b0111_1111;

    buf.push(b0);
    buf.push(b1);

    buf.extend_from_slice(&pkt.sequence_number.to_be_bytes());
    buf.extend_from_slice(&pkt.timestamp.to_be_bytes());
    buf.extend_from_slice(&pkt.ssrc.to_be_bytes());

    // v1では CSRC/拡張ヘッダなし前提
    buf.extend_from_slice(&pkt.payload);

    buf
}
