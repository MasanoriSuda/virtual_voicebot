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

    /// よく使う基本ヘッダを構造化して返す（存在しない場合は Err）
    pub fn core_headers(&self) -> anyhow::Result<CoreHeaders> {
        let via = self
            .header_value("Via")
            .ok_or_else(|| anyhow::anyhow!("missing Via"))?
            .to_string();
        let from = self
            .header_value("From")
            .ok_or_else(|| anyhow::anyhow!("missing From"))?
            .to_string();
        let to = self
            .header_value("To")
            .ok_or_else(|| anyhow::anyhow!("missing To"))?
            .to_string();
        let call_id = self
            .header_value("Call-ID")
            .ok_or_else(|| anyhow::anyhow!("missing Call-ID"))?
            .to_string();
        let cseq_raw = self
            .header_value("CSeq")
            .ok_or_else(|| anyhow::anyhow!("missing CSeq"))?;
        let (cseq_num, cseq_method) = parse_cseq(cseq_raw)?;

        Ok(CoreHeaders {
            via,
            from,
            to,
            call_id,
            cseq: cseq_num,
            cseq_method,
        })
    }
}

#[derive(Debug, Clone)]
pub struct CoreHeaders {
    pub via: String,
    pub from: String,
    pub to: String,
    pub call_id: String,
    pub cseq: u32,
    pub cseq_method: String,
}

fn parse_cseq(raw: &str) -> anyhow::Result<(u32, String)> {
    let mut parts = raw.split_whitespace();
    let num_str = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("CSeq missing number"))?;
    let method = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("CSeq missing method"))?;
    let num: u32 = num_str
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid CSeq number"))?;
    Ok((num, method.to_string()))
}
