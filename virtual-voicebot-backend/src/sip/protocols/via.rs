use anyhow::{anyhow, Result};

use crate::sip::message::SipHeader;
use crate::sip::protocols::HeaderCodec;

#[derive(Debug, Clone)]
pub struct ViaHeader {
    pub sent_protocol: String,
    pub sent_by: String,
    pub params: Vec<(String, String)>,
}

impl HeaderCodec for ViaHeader {
    const NAME: &'static str = "Via";

    fn parse(value: &str) -> Result<Self> {
        // 例: "SIP/2.0/UDP 192.0.2.1:5060;branch=z9hG4bK123;rport"
        let mut parts = value.splitn(2, ' ');
        let proto = parts
            .next()
            .ok_or_else(|| anyhow!("Via missing protocol"))?
            .trim();
        let rest = parts
            .next()
            .ok_or_else(|| anyhow!("Via missing sent-by"))?
            .trim();

        let mut sent_by = rest.to_string();
        let mut params = Vec::new();
        if let Some(idx) = rest.find(';') {
            sent_by = rest[..idx].trim().to_string();
            let param_str = &rest[idx + 1..];
            params = super::name_addr::parse_params(param_str);
        }

        Ok(ViaHeader {
            sent_protocol: proto.to_string(),
            sent_by,
            params,
        })
    }

    fn to_header(&self) -> SipHeader {
        let mut value = format!("{} {}", self.sent_protocol, self.sent_by);
        for (k, v) in &self.params {
            if v.is_empty() {
                value.push_str(&format!(";{}", k));
            } else {
                value.push_str(&format!(";{}={}", k, v));
            }
        }
        SipHeader::new(Self::NAME, value)
    }
}
