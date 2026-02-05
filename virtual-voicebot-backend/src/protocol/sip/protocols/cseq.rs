use crate::protocol::sip::message::SipHeader;
use crate::protocol::sip::protocols::HeaderCodec;
use anyhow::{anyhow, Result};

#[derive(Debug, Clone)]
pub struct CSeqHeader {
    pub num: u32,
    pub method: String,
}

impl HeaderCodec for CSeqHeader {
    const NAME: &'static str = "CSeq";

    fn parse(value: &str) -> Result<Self> {
        let mut iter = value.split_whitespace();
        let num_str = iter.next().ok_or_else(|| anyhow!("CSeq missing number"))?;
        let method = iter.next().ok_or_else(|| anyhow!("CSeq missing method"))?;
        let num = num_str
            .parse::<u32>()
            .map_err(|_| anyhow!("invalid CSeq num"))?;
        Ok(Self {
            num,
            method: method.to_string(),
        })
    }

    fn to_header(&self) -> SipHeader {
        SipHeader::new(Self::NAME, format!("{} {}", self.num, self.method))
    }
}
