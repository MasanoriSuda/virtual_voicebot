use anyhow::{anyhow, Result};
use crate::sip::message::SipHeader;
use crate::sip::protocols::HeaderCodec;

#[derive(Debug, Clone)]
pub struct MaxForwardsHeader {
    pub hops: u32,
}

impl HeaderCodec for MaxForwardsHeader {
    const NAME: &'static str = "Max-Forwards";

    fn parse(value: &str) -> Result<Self> {
        let hops = value
            .trim()
            .parse::<u32>()
            .map_err(|_| anyhow!("invalid Max-Forwards"))?;
        Ok(Self { hops })
    }

    fn to_header(&self) -> SipHeader {
        SipHeader::new(Self::NAME, self.hops.to_string())
    }
}
