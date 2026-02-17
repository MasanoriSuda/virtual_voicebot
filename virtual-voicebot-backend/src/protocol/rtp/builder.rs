// src/rtp/build.rs

use crate::protocol::rtp::packet::RtpPacket;

pub fn build_rtp_packet(pkt: &RtpPacket) -> Vec<u8> {
    let csrc_count = pkt.csrcs.len().min(15) as u8;
    let extension = pkt.extension.is_some();
    let mut buf = Vec::with_capacity(12 + pkt.payload.len());

    let mut b0 = (pkt.version & 0b11) << 6;
    if pkt.padding {
        b0 |= 0b0010_0000;
    }
    if extension {
        b0 |= 0b0001_0000;
    }
    b0 |= csrc_count & 0b0000_1111;

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

    // CSRC
    for csrc in pkt.csrcs.iter().take(csrc_count as usize) {
        buf.extend_from_slice(&csrc.to_be_bytes());
    }

    // Extension
    if let Some(ext) = pkt.extension.as_ref() {
        let ext_len_words = ext.data.len().div_ceil(4);
        buf.extend_from_slice(&ext.profile.to_be_bytes());
        buf.extend_from_slice(&(ext_len_words as u16).to_be_bytes());
        buf.extend_from_slice(&ext.data);
        let pad_len = ext_len_words * 4 - ext.data.len();
        if pad_len > 0 {
            buf.extend(std::iter::repeat_n(0u8, pad_len));
        }
    }

    buf.extend_from_slice(&pkt.payload);

    buf
}
