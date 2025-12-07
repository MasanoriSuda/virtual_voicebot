pub mod via;
pub mod name_addr;
pub mod cseq;
pub mod content_length;
pub mod max_forwards;

pub use via::ViaHeader;
pub use name_addr::{NameAddrHeader, ContactHeader, FromHeader, ToHeader};
pub use cseq::CSeqHeader;
pub use content_length::ContentLengthHeader;
pub use max_forwards::MaxForwardsHeader;

/// ヘッダを構造化/文字列化するための共通トレイト
pub trait HeaderCodec: Sized {
    const NAME: &'static str;
    fn parse(value: &str) -> anyhow::Result<Self>;
    fn to_header(&self) -> crate::sip::message::SipHeader;
}
