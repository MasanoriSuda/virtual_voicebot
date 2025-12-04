#[derive(Debug, Clone)]
// Request or Response の種別
pub enum SipMessage {
    Request(SipRequest),
    Response(SipResponse),
}

#[derive(Debug, Clone)]
pub struct SipRequest {
    pub method: SipMethod,
    pub uri: String,   // とりあえず String, 後で構造化しても良い
    #[allow(dead_code)]
    pub version: String,
    pub headers: Vec<SipHeader>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct SipResponse {
    pub version: String,
    pub status_code: u16,
    pub reason_phrase: String,
    pub headers: Vec<SipHeader>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone)]
pub enum SipMethod {
    Invite,
    Ack,
    Bye,
    Cancel,
    Options,
    Register,
    #[allow(dead_code)]
    Unknown(String),
}

#[derive(Debug, Clone)]
pub struct SipHeader {
    pub name: String,
    pub value: String,
}

impl SipHeader {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

impl SipRequest {
    pub fn header_value(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case(name))
            .map(|h| h.value.as_str())
    }
}
