#![allow(dead_code, unused_imports)]

pub mod content_length;
pub mod cseq;
pub mod max_forwards;
pub mod name_addr;
pub mod via;

pub use content_length::ContentLengthHeader;
pub use cseq::CSeqHeader;
pub use max_forwards::MaxForwardsHeader;
pub use name_addr::{ContactHeader, FromHeader, NameAddrHeader, ToHeader};
pub use via::ViaHeader;

/// ヘッダを構造化/文字列化するための共通トレイト
pub trait HeaderCodec: Sized {
    const NAME: &'static str;
    fn parse(value: &str) -> anyhow::Result<Self>;
    fn to_header(&self) -> crate::sip::message::SipHeader;
}
