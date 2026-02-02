#![allow(dead_code)]

use std::net::SocketAddr;

/// RTCP 受信イベントのスタブ。未実装であることを明示するための型。
#[derive(Debug, Clone)]
pub struct RtcpEvent {
    pub raw: Vec<u8>,
    pub src: SocketAddr,
    pub dst_port: u16,
}

/// RTCP 送信要求のスタブ。SR/RR 生成は後続タスク。
#[derive(Debug, Clone)]
pub struct RtcpSendRequest {
    pub dst: SocketAddr,
    pub payload: Vec<u8>,
}

/// rtp→上位へ RTCP を通知するための I/F（現状は未使用）。
pub type RtcpEventTx = tokio::sync::mpsc::UnboundedSender<RtcpEvent>;

/// 上位→rtp へ RTCP 送信を依頼するための I/F（現状は未使用）。
pub type RtcpSendTx = tokio::sync::mpsc::UnboundedSender<RtcpSendRequest>;

/// 簡易判定: RTCP パケットかどうか（Version=2 かつ PT が 192-223 の範囲を検知）
pub fn is_rtcp_packet(data: &[u8]) -> bool {
    if data.len() < 2 {
        return false;
    }
    let v = data[0] >> 6;
    let pt = data[1];
    v == 2 && (192..=223).contains(&pt)
}

#[derive(Debug, Clone)]
pub enum RtcpPacket {
    SenderReport(RtcpSenderReport),
    ReceiverReport(RtcpReceiverReport),
}

#[derive(Debug, Clone)]
pub struct RtcpSenderReport {
    pub ssrc: u32,
    pub ntp_timestamp: u64,
    pub rtp_timestamp: u32,
    pub packet_count: u32,
    pub octet_count: u32,
}

#[derive(Debug, Clone)]
pub struct RtcpReceiverReport {
    pub ssrc: u32,
    pub report: Option<RtcpReportBlock>,
}

#[derive(Debug, Clone)]
pub struct RtcpReportBlock {
    pub ssrc: u32,
    pub fraction_lost: u8,
    pub cumulative_lost: u32,
    pub highest_seq: u32,
    pub jitter: u32,
    pub lsr: u32,
    pub dlsr: u32,
}

/// Parses consecutive RTCP packets from a byte slice into a vector of `RtcpPacket`.
///
/// The function iterates over `data`, decoding RTCP headers and extracting Sender Report
/// (PT=200) and Receiver Report (PT=201) packets when enough bytes are present. Parsing
/// stops if an RTCP header has an unsupported version, if a packet length would exceed the
/// available bytes, or if a packet is too short to be valid. Invalid or incomplete packets
/// are ignored; already-parsed packets up to that point are returned.
///
/// # Examples
///
/// ```
/// let empty = parse_rtcp_packets(&[]);
/// assert!(empty.is_empty());
/// ```
pub fn parse_rtcp_packets(data: &[u8]) -> Vec<RtcpPacket> {
    let mut packets = Vec::new();
    let mut offset = 0usize;
    while offset + 4 <= data.len() {
        let v = data[offset] >> 6;
        if v != 2 {
            break;
        }
        let rc = data[offset] & 0x1F;
        let pt = data[offset + 1];
        let length = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize;
        let bytes = (length + 1) * 4;
        if offset + bytes > data.len() || bytes < 8 {
            break;
        }
        let body = &data[offset + 4..offset + bytes];
        match pt {
            200 => {
                if body.len() >= 24 {
                    let ssrc = u32::from_be_bytes(body[0..4].try_into().unwrap());
                    let ntp_hi = u32::from_be_bytes(body[4..8].try_into().unwrap());
                    let ntp_lo = u32::from_be_bytes(body[8..12].try_into().unwrap());
                    let ntp = ((ntp_hi as u64) << 32) | (ntp_lo as u64);
                    let rtp_ts = u32::from_be_bytes(body[12..16].try_into().unwrap());
                    let pkt_count = u32::from_be_bytes(body[16..20].try_into().unwrap());
                    let oct_count = u32::from_be_bytes(body[20..24].try_into().unwrap());
                    packets.push(RtcpPacket::SenderReport(RtcpSenderReport {
                        ssrc,
                        ntp_timestamp: ntp,
                        rtp_timestamp: rtp_ts,
                        packet_count: pkt_count,
                        octet_count: oct_count,
                    }));
                }
            }
            201 => {
                if body.len() >= 4 {
                    let ssrc = u32::from_be_bytes(body[0..4].try_into().unwrap());
                    let report = if rc >= 1 && body.len() >= 24 {
                        let blk = &body[4..24];
                        Some(RtcpReportBlock {
                            ssrc: u32::from_be_bytes(blk[0..4].try_into().unwrap()),
                            fraction_lost: blk[4],
                            cumulative_lost: ((blk[5] as u32) << 16)
                                | ((blk[6] as u32) << 8)
                                | (blk[7] as u32),
                            highest_seq: u32::from_be_bytes(blk[8..12].try_into().unwrap()),
                            jitter: u32::from_be_bytes(blk[12..16].try_into().unwrap()),
                            lsr: u32::from_be_bytes(blk[16..20].try_into().unwrap()),
                            dlsr: u32::from_be_bytes(blk[20..24].try_into().unwrap()),
                        })
                    } else {
                        None
                    };
                    packets.push(RtcpPacket::ReceiverReport(RtcpReceiverReport {
                        ssrc,
                        report,
                    }));
                }
            }
            _ => {}
        }
        offset += bytes;
    }
    packets
}

pub fn build_sr(sr: &RtcpSenderReport) -> Vec<u8> {
    let mut buf = Vec::with_capacity(28);
    buf.push(0x80); // V=2, P=0, RC=0
    buf.push(200);
    buf.extend_from_slice(&[0x00, 0x06]); // length = (28/4)-1 = 6
    buf.extend_from_slice(&sr.ssrc.to_be_bytes());
    buf.extend_from_slice(&((sr.ntp_timestamp >> 32) as u32).to_be_bytes());
    buf.extend_from_slice(&(sr.ntp_timestamp as u32).to_be_bytes());
    buf.extend_from_slice(&sr.rtp_timestamp.to_be_bytes());
    buf.extend_from_slice(&sr.packet_count.to_be_bytes());
    buf.extend_from_slice(&sr.octet_count.to_be_bytes());
    buf
}

pub fn build_rr(rr: &RtcpReceiverReport) -> Vec<u8> {
    let mut buf = Vec::with_capacity(32);
    let rc = if rr.report.is_some() { 1u8 } else { 0u8 };
    buf.push(0x80 | rc);
    buf.push(201);
    let length_words = if rc == 1 { 7 } else { 1 };
    buf.extend_from_slice(&(length_words as u16).to_be_bytes());
    buf.extend_from_slice(&rr.ssrc.to_be_bytes());
    if let Some(report) = &rr.report {
        buf.extend_from_slice(&report.ssrc.to_be_bytes());
        buf.push(report.fraction_lost);
        buf.push(((report.cumulative_lost >> 16) & 0xFF) as u8);
        buf.push(((report.cumulative_lost >> 8) & 0xFF) as u8);
        buf.push((report.cumulative_lost & 0xFF) as u8);
        buf.extend_from_slice(&report.highest_seq.to_be_bytes());
        buf.extend_from_slice(&report.jitter.to_be_bytes());
        buf.extend_from_slice(&report.lsr.to_be_bytes());
        buf.extend_from_slice(&report.dlsr.to_be_bytes());
    }
    buf
}

pub fn ntp_timestamp_now() -> u64 {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    const NTP_UNIX_EPOCH_DIFF: u64 = 2_208_988_800;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0));
    let secs = now.as_secs().wrapping_add(NTP_UNIX_EPOCH_DIFF);
    let frac = ((now.subsec_nanos() as u64) << 32) / 1_000_000_000u64;
    (secs << 32) | frac
}
