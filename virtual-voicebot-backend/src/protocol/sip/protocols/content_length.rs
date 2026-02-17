use crate::protocol::sip::message::SipHeader;
use crate::protocol::sip::protocols::HeaderCodec;
use anyhow::{anyhow, Result};

#[derive(Debug, Clone)]
pub struct ContentLengthHeader {
    pub length: usize,
}

impl HeaderCodec for ContentLengthHeader {
    const NAME: &'static str = "Content-Length";

    fn parse(value: &str) -> Result<Self> {
        let len = value
            .trim()
            .parse::<usize>()
            .map_err(|_| anyhow!("invalid Content-Length"))?;
        Ok(Self { length: len })
    }

    fn to_header(&self) -> SipHeader {
        SipHeader::new(Self::NAME, self.length.to_string())
    }
}
