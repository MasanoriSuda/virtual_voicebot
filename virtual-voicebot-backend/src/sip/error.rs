use thiserror::Error;

#[derive(Debug, Error)]
pub enum SipError {
    #[error("SIP error: {0}")]
    Generic(String),
}
