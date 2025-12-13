#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadKind {
    Pcmu,
}

/// payload type から扱うコーデックを判定する。未対応の PT は Err を返す。
pub fn classify_payload(pt: u8) -> Result<PayloadKind, UnsupportedPayload> {
    match pt {
        0 => Ok(PayloadKind::Pcmu),
        other => Err(UnsupportedPayload(other)),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnsupportedPayload(pub u8);
